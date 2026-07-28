//! ログ初期化。
//!
//! ログは必ず stderr に出す。stdout は成果物（生成した音声ファイルのパス等）専用に空けておく
//! （AI エージェントは stdout をコンテキストに取り込むため）。
//!
//! 環境変数を読むのは [`LogEnv::from_process`] だけ。フィルタの解決ロジックは
//! [`LogEnv`] から受け取った値しか見ないため、プロセス環境に触らずに検証できる。

use tracing_subscriber::{EnvFilter, fmt};

/// ログフィルタの解決に使う環境変数のスナップショット。
///
/// プロセス環境の読み取りを [`LogEnv::from_process`] 1 箇所に閉じるための型。
/// edition 2024 では `std::env::set_var` が unsafe で、かつ結合テストはスレッド並列に
/// 走るため、テストは環境変数を書き換える代わりに [`LogEnv::empty`] を注入する。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogEnv {
    /// `MINIMAX_LOG` の値。`RUST_LOG` より優先する。
    pub minimax_log: Option<String>,
    /// `RUST_LOG` の値。
    pub rust_log: Option<String>,
}

impl LogEnv {
    /// プロセス環境から読み出す。バイナリ層のエントリからのみ呼ぶ。
    pub fn from_process() -> Self {
        Self {
            minimax_log: std::env::var("MINIMAX_LOG").ok(),
            rust_log: std::env::var("RUST_LOG").ok(),
        }
    }

    /// 何も設定されていないソース。テストから注入する。
    pub const fn empty() -> Self {
        Self {
            minimax_log: None,
            rust_log: None,
        }
    }
}

/// `-v` の回数から既定のフィルタを決める。
const fn default_directives(verbose: u8) -> &'static str {
    match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    }
}

/// `MINIMAX_LOG` → `RUST_LOG` → `-v` の優先順位でフィルタ文字列を選ぶ。
///
/// 空文字と、不正な directive を含む値は、その候補を飛ばして次を見る。
/// 空文字を飛ばす点だけ `EnvFilter::try_from_env` を連鎖させていたときと挙動が違う。
/// `EnvFilter::try_new("")` は `Ok` を返すため、飛ばさないと `MINIMAX_LOG=` が
/// `RUST_LOG` と `-v` を黙って打ち消してしまう（`EnvFilter::new("")` は既定 directive の
/// `error` になるので、`-vv` を渡していても `error` に落ちる）。
fn resolve_directives(verbose: u8, env: &LogEnv) -> &str {
    [env.minimax_log.as_deref(), env.rust_log.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|directives| !directives.is_empty() && EnvFilter::try_new(directives).is_ok())
        .unwrap_or_else(|| default_directives(verbose))
}

/// フィルタを組み立てる。
///
/// 認証ヘッダがログにダンプされないよう、解決したフィルタに対して**後から** HTTP レイヤの
/// クレートを `warn` に固定する。
///
/// **完全な遮断ではない。** `EnvFilter` は target 文字列が長い directive から順に照合し、
/// 最初に一致した 1 本でレベルを決める。そのため固定が勝つのは同名 target（`hyper=trace`）と
/// グローバル指定（`trace`）のときだけで、`hyper::proto=trace` のように**より深い target**を
/// 指定されると貫通する（`add_directive` の順序を変えても直らない）。
/// 現状 `reqwest` / `hyper` / `h2` / `rustls` は依存グラフに 1 つも無いため実害は無い。
/// `reqwest` を依存に入れる時点で `filter::Targets` を `and` で AND 結合する形に変えて塞ぐ。
fn build_filter(verbose: u8, env: &LogEnv) -> EnvFilter {
    EnvFilter::new(resolve_directives(verbose, env))
        .add_directive("reqwest=warn".parse().expect("valid directive"))
        .add_directive("hyper=warn".parse().expect("valid directive"))
        .add_directive("hyper_util=warn".parse().expect("valid directive"))
        .add_directive("h2=warn".parse().expect("valid directive"))
        .add_directive("rustls=warn".parse().expect("valid directive"))
}

/// ログを stderr に出す subscriber を初期化する。
///
/// フィルタの優先順位は `MINIMAX_LOG` → `RUST_LOG` → `verbose` フラグ。
/// 環境変数はプロセス環境から読む。
pub fn init(verbose: u8) {
    init_with(verbose, &LogEnv::from_process());
}

/// [`init`] の環境変数ソースを差し替えられる版。
///
/// プロセス環境に依存させたくない呼び出し元（テストや、設定を別経路で解決する将来の
/// サブコマンド）はこちらを使う。
pub fn init_with(verbose: u8, env: &LogEnv) {
    // without_time(): タイムスタンプは CLI のログに不要で、
    // insta のスナップショットを不安定にするため出さない。
    let _ = fmt()
        .with_env_filter(build_filter(verbose, env))
        .with_writer(std::io::stderr)
        .without_time()
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::{LogEnv, build_filter, default_directives, resolve_directives};

    /// `warn` に固定している HTTP レイヤのクレート。
    const PINNED: [&str; 5] = ["reqwest", "hyper", "hyper_util", "h2", "rustls"];

    fn source(minimax_log: Option<&str>, rust_log: Option<&str>) -> LogEnv {
        LogEnv {
            minimax_log: minimax_log.map(str::to_owned),
            rust_log: rust_log.map(str::to_owned),
        }
    }

    #[test]
    fn verbose_count_decides_the_default_filter() {
        assert_eq!(default_directives(0), "warn");
        assert_eq!(default_directives(1), "info");
        assert_eq!(default_directives(2), "debug");
        assert_eq!(default_directives(u8::MAX), "debug");
    }

    #[test]
    fn empty_source_falls_back_to_the_verbose_flag() {
        assert_eq!(resolve_directives(0, &LogEnv::empty()), "warn");
        assert_eq!(resolve_directives(1, &LogEnv::empty()), "info");
        assert_eq!(resolve_directives(2, &LogEnv::empty()), "debug");
    }

    #[test]
    fn minimax_log_wins_over_rust_log_and_the_verbose_flag() {
        let both = source(Some("trace"), Some("error"));
        assert_eq!(resolve_directives(2, &both), "trace");
    }

    #[test]
    fn rust_log_is_used_when_minimax_log_is_unset() {
        assert_eq!(resolve_directives(2, &source(None, Some("error"))), "error");
    }

    /// 不正な directive を持つ候補は飛ばして次を見る。
    #[test]
    fn invalid_directives_fall_through_to_the_next_source() {
        let with_fallback = source(Some("minimax=not_a_level"), Some("error"));
        assert_eq!(resolve_directives(2, &with_fallback), "error");

        let only_invalid = source(Some("minimax=not_a_level"), None);
        assert_eq!(resolve_directives(1, &only_invalid), "info");
    }

    /// 空文字（と空白のみ）は「未設定」として扱う。
    ///
    /// `EnvFilter::try_new("")` は `Ok` なので、明示的に弾かないと `MINIMAX_LOG=` が
    /// `RUST_LOG` と `-v` を打ち消してしまう。
    #[test]
    fn empty_values_are_treated_as_unset() {
        let falls_back_to_rust_log = source(Some(""), Some("error"));
        assert_eq!(resolve_directives(2, &falls_back_to_rust_log), "error");

        assert_eq!(resolve_directives(2, &source(Some(""), None)), "debug");
        assert_eq!(resolve_directives(0, &source(Some("   "), None)), "warn");
    }

    /// `off` も有効な directive なので採用される。
    /// これが `main_entry` の `tracing::error!` を黙らせる既知の落とし穴の入口。
    #[test]
    fn off_is_a_valid_directive() {
        assert_eq!(resolve_directives(2, &source(Some("off"), None)), "off");
        assert_eq!(resolve_directives(2, &source(None, Some("off"))), "off");
    }

    /// HTTP レイヤの `warn` 固定が、同名 target とグローバル指定に勝つこと。
    ///
    /// `build_filter` の `add_directive` 5 行を消すとこのテストが落ちる
    /// （認証ヘッダのダンプ防止が意図せず外れることを検出する）。
    #[test]
    fn http_layer_pins_beat_same_name_and_global_directives() {
        let global = build_filter(0, &source(Some("trace"), None)).to_string();

        for pinned in PINNED {
            let warn = format!("{pinned}=warn");
            let trace = format!("{pinned}=trace");

            let same_name = build_filter(0, &source(Some(&trace), None)).to_string();
            assert!(same_name.contains(&warn), "{pinned}: {same_name}");
            assert!(!same_name.contains(&trace), "{pinned}: {same_name}");

            assert!(global.contains(&warn), "{pinned}: {global}");
        }
    }

    /// 既知の制約: より深い target を指定すると `warn` 固定を貫通する。
    ///
    /// `EnvFilter` は target 文字列が長い directive を先に照合するため、`hyper=warn` より
    /// `hyper::proto=trace` が優先される。塞いだときにこのテストが落ちるので、挙動が
    /// 変わったことに気付ける（詳細は [`build_filter`] の doc コメント）。
    #[test]
    fn deeper_targets_currently_bypass_the_http_layer_pins() {
        let rendered = build_filter(0, &source(Some("hyper::proto=trace"), None)).to_string();
        assert!(rendered.contains("hyper::proto=trace"), "{rendered}");
        assert!(rendered.contains("hyper=warn"), "{rendered}");
    }
}
