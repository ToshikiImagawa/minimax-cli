//! ライブラリ層のエラー型。
//!
//! バイナリ層は [`anyhow`] で受けて表示する。
//! エラーメッセージに認証情報を載せてはならない（AI エージェントが読む経路に乗るため）。

/// ライブラリ層のエラー。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 設定値が不正。
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// 入出力エラー。
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
