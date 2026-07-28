//! ログ初期化。
//!
//! ログは必ず stderr に出す。stdout は成果物（生成した音声ファイルのパス等）専用に空けておく
//! （AI エージェントは stdout をコンテキストに取り込むため）。

use tracing_subscriber::{EnvFilter, fmt};

/// ログを stderr に出す subscriber を初期化する。
///
/// フィルタの優先順位は `MINIMAX_LOG` → `RUST_LOG` → `verbose` フラグ。
pub fn init(verbose: u8) {
    let default = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };

    let filter = EnvFilter::try_from_env("MINIMAX_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new(default))
        // 将来 HTTP クライアントを入れたときに認証ヘッダがダンプされないよう先に遮断しておく
        .add_directive("reqwest=warn".parse().expect("valid directive"))
        .add_directive("hyper=warn".parse().expect("valid directive"))
        .add_directive("hyper_util=warn".parse().expect("valid directive"))
        .add_directive("h2=warn".parse().expect("valid directive"))
        .add_directive("rustls=warn".parse().expect("valid directive"));

    // without_time(): タイムスタンプは CLI のログに不要で、
    // insta のスナップショットを不安定にするため出さない。
    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init();
}
