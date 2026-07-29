//! ログ初期化。
//!
//! ログは必ず stderr に出す。stdout は成果物（生成した音声ファイルのパス等）専用に空けておく
//! （AI エージェントは stdout をコンテキストに取り込むため）。
//!
//! 環境変数を読むのは [`LogEnv::from_process`] だけ。フィルタの解決ロジックは
//! [`LogEnv`] から受け取った値しか見ないため、プロセス環境に触らずに検証できる。

use tracing_subscriber::{
    EnvFilter, Layer,
    filter::{FilterExt, LevelFilter, Targets, combinator::And},
    fmt,
    layer::SubscriberExt,
    registry::Registry,
    util::SubscriberInitExt,
};

/// ログフィルタの解決に使う環境変数のスナップショット。
///
/// フィールドは private。空文字・空白のみを「未設定」に畳む正規化を [`LogEnv::new`] だけで行い、
/// 解決ロジック側では値の形を疑わなくて済むようにする。
///
/// edition 2024 では `std::env::set_var` が unsafe で、かつ結合テストはスレッド並列に走るため、
/// テストは環境変数を書き換える代わりに [`LogEnv::default`] を注入する。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LogEnv {
    minimax_log: Option<String>,
    rust_log: Option<String>,
}

impl LogEnv {
    /// この CLI がログフィルタのために読む環境変数の名前。`MINIMAX_LOG` が `RUST_LOG` に優先する。
    ///
    /// 読み取り側（[`LogEnv::from_process`]）がこの配列を使うため、名前の真実は 1 箇所。
    /// 子プロセスを spawn する結合テストは、ここを回して環境を隔離する。
    pub const NAMES: [&'static str; 2] = ["MINIMAX_LOG", "RUST_LOG"];

    /// プロセス環境から読み出す。バイナリ層のエントリからのみ呼ぶ。
    pub fn from_process() -> Self {
        let [minimax_log, rust_log] = Self::NAMES.map(|name| std::env::var(name).ok());
        Self::new(minimax_log, rust_log)
    }

    /// 値からスナップショットを組み立てる。
    ///
    /// 空文字と空白のみの値は「未設定」に正規化する。`EnvFilter::try_new("")` は `Ok` を返すため、
    /// ここで畳まないと `MINIMAX_LOG=` が `RUST_LOG` と `-v` を黙って打ち消す
    /// （`EnvFilter::new("")` は既定 directive の `error` になる）。
    pub fn new(minimax_log: Option<String>, rust_log: Option<String>) -> Self {
        Self {
            minimax_log: normalize(minimax_log),
            rust_log: normalize(rust_log),
        }
    }
}

/// 環境変数の値を正規化する。前後の空白を落とし、空になったら未設定として扱う。
fn normalize(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// `MINIMAX_LOG` → `RUST_LOG` → `-v` の優先順位でフィルタ文字列を選ぶ。
///
/// 不正な directive を含む候補は飛ばして次を見る。空文字は [`LogEnv::new`] が未設定へ畳んでいる。
fn resolve_directives(verbose: u8, env: &LogEnv) -> &str {
    [env.minimax_log.as_deref(), env.rust_log.as_deref()]
        .into_iter()
        .flatten()
        .find(|directives| EnvFilter::try_new(directives).is_ok())
        .unwrap_or(match verbose {
            0 => "warn",
            1 => "info",
            _ => "debug",
        })
}

/// 認証ヘッダをダンプしうる HTTP レイヤのクレート。
///
/// 依存に新しい HTTP / TLS クレートを入れるときはここへ追加する。
const HTTP_LAYER_CRATES: [&str; 5] = ["reqwest", "hyper", "hyper_util", "h2", "rustls"];

/// HTTP レイヤを `warn` で打ち止めにする上限フィルタ。
///
/// `Targets` は target を `::` 区切りの接頭辞で照合するため、`hyper` の指定が
/// `hyper::proto::h1` のような深い target にも効く。
fn http_layer_ceiling() -> Targets {
    Targets::new()
        .with_default(LevelFilter::TRACE)
        .with_targets(HTTP_LAYER_CRATES.map(|crate_name| (crate_name, LevelFilter::WARN)))
}

/// 解決したフィルタと HTTP レイヤの上限を AND 結合する。
///
/// どちらか一方でも落とすイベントは出力されないため、**利用者がどんな directive を与えても
/// HTTP レイヤは `warn` より下に開かない**（認証ヘッダのダンプ防止）。
fn build_filter(verbose: u8, env: &LogEnv) -> And<EnvFilter, Targets, Registry> {
    EnvFilter::new(resolve_directives(verbose, env)).and(http_layer_ceiling())
}

/// ログを stderr に出す subscriber を初期化する。
///
/// フィルタの優先順位は `MINIMAX_LOG` → `RUST_LOG` → `verbose` フラグ。
/// 環境変数のスナップショットは呼び出し元（バイナリ層の `main_entry`）が渡すため、
/// この関数はプロセス環境に触らない。
pub fn init(verbose: u8, env: &LogEnv) {
    // without_time(): タイムスタンプは CLI のログに不要で、
    // insta のスナップショットを不安定にするため出さない。
    let _ = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .without_time()
                .with_filter(build_filter(verbose, env)),
        )
        .try_init();
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use tracing::Level;
    use tracing_subscriber::fmt::MakeWriter;

    use super::{
        HTTP_LAYER_CRATES, Layer, LogEnv, SubscriberExt, build_filter, fmt, http_layer_ceiling,
        resolve_directives,
    };

    fn source(minimax_log: Option<&str>, rust_log: Option<&str>) -> LogEnv {
        LogEnv::new(minimax_log.map(str::to_owned), rust_log.map(str::to_owned))
    }

    /// 出力を溜めるテスト用の writer。
    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    impl Write for Buffer {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("buffer lock").extend_from_slice(data);
            Ok(data.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Buffer {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// `build_filter` で組んだ subscriber に実際にイベントを流し、出力を返す。
    fn captured(verbose: u8, env: &LogEnv, emit: impl FnOnce()) -> String {
        let buffer = Buffer::default();
        let subscriber = tracing_subscriber::registry().with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(buffer.clone())
                .without_time()
                .with_filter(build_filter(verbose, env)),
        );
        tracing::subscriber::with_default(subscriber, emit);
        let bytes = buffer.0.lock().expect("buffer lock").clone();
        String::from_utf8(bytes).expect("utf-8")
    }

    #[test]
    fn empty_source_falls_back_to_the_verbose_flag() {
        assert_eq!(resolve_directives(0, &LogEnv::default()), "warn");
        assert_eq!(resolve_directives(1, &LogEnv::default()), "info");
        assert_eq!(resolve_directives(2, &LogEnv::default()), "debug");
        assert_eq!(resolve_directives(u8::MAX, &LogEnv::default()), "debug");
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

    #[test]
    fn invalid_directives_fall_through_to_the_next_source() {
        let with_fallback = source(Some("minimax=not_a_level"), Some("error"));
        assert_eq!(resolve_directives(2, &with_fallback), "error");

        let only_invalid = source(Some("minimax=not_a_level"), None);
        assert_eq!(resolve_directives(1, &only_invalid), "info");
    }

    /// 空文字と空白は境界（[`LogEnv::new`]）で未設定に畳まれる。
    #[test]
    fn the_boundary_normalizes_empty_values() {
        assert_eq!(LogEnv::new(Some(String::new()), None), LogEnv::default());
        assert_eq!(LogEnv::new(Some("   ".to_owned()), None), LogEnv::default());
        assert_eq!(
            LogEnv::new(Some(" debug ".to_owned()), None),
            LogEnv::new(Some("debug".to_owned()), None)
        );
    }

    #[test]
    fn empty_values_are_treated_as_unset() {
        let falls_back_to_rust_log = source(Some(""), Some("error"));
        assert_eq!(resolve_directives(2, &falls_back_to_rust_log), "error");

        assert_eq!(resolve_directives(2, &source(Some(""), None)), "debug");
        assert_eq!(resolve_directives(0, &source(Some("   "), None)), "warn");
    }

    #[test]
    fn off_is_a_valid_directive() {
        assert_eq!(resolve_directives(2, &source(Some("off"), None)), "off");
        assert_eq!(resolve_directives(2, &source(None, Some("off"))), "off");
    }

    /// `Targets` の接頭辞照合が深い target にも効くこと。
    #[test]
    fn the_ceiling_matches_deeper_targets_by_prefix() {
        let ceiling = http_layer_ceiling();
        for crate_name in HTTP_LAYER_CRATES {
            let deep = format!("{crate_name}::proto::h1");
            assert!(!ceiling.would_enable(&deep, &Level::TRACE), "{deep}");
            assert!(
                !ceiling.would_enable(crate_name, &Level::TRACE),
                "{crate_name}"
            );
            assert!(
                ceiling.would_enable(crate_name, &Level::WARN),
                "{crate_name}"
            );
        }
        assert!(ceiling.would_enable("minimax_cli::tts", &Level::TRACE));
    }

    /// グローバル指定・同名 target・深い target のいずれでも HTTP レイヤは開かない。
    ///
    /// `add_directive` で後付け上書きしていたときは `hyper::proto=trace` が貫通していた。
    #[test]
    fn the_ceiling_cannot_be_bypassed_by_any_directive() {
        for directives in ["trace", "hyper=trace", "hyper::proto=trace"] {
            let rendered = captured(0, &source(Some(directives), None), || {
                tracing::trace!(target: "hyper", "authorization: Bearer leaked");
                tracing::trace!(target: "reqwest", "authorization: Bearer leaked");
                tracing::trace!(target: "h2", "authorization: Bearer leaked");
                tracing::trace!(target: "rustls", "authorization: Bearer leaked");
                tracing::trace!(target: "hyper_util", "authorization: Bearer leaked");
                tracing::trace!(target: "hyper::proto::h1", "authorization: Bearer leaked");
                tracing::warn!(target: "hyper::proto::h1", "kept at warn");
            });
            assert!(!rendered.contains("leaked"), "{directives}: {rendered}");
            assert!(
                rendered.contains("kept at warn"),
                "{directives}: {rendered}"
            );
        }
    }

    /// アプリのログは上限の影響を受けない。
    #[test]
    fn app_targets_are_not_affected_by_the_ceiling() {
        let rendered = captured(2, &LogEnv::default(), || {
            tracing::debug!(target: "minimax_cli::tts", "app event");
        });
        assert!(rendered.contains("app event"), "{rendered}");
    }

    /// `off` は従来どおり全部黙らせる（`main_entry` のエラー表示も消える既知の落とし穴）。
    #[test]
    fn off_still_silences_everything() {
        let rendered = captured(2, &source(Some("off"), None), || {
            tracing::error!("boom");
            tracing::warn!(target: "hyper", "http");
        });
        assert!(rendered.is_empty(), "{rendered}");
    }

    /// 既定（`-v` なし）では warn 以上が出る。
    #[test]
    fn default_filter_keeps_warn_and_error() {
        let rendered = captured(0, &LogEnv::default(), || {
            tracing::error!("boom");
            tracing::info!("chatter");
        });
        assert!(rendered.contains("boom"), "{rendered}");
        assert!(!rendered.contains("chatter"), "{rendered}");
    }
}
