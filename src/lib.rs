//! minimax-cli のライブラリ本体。
//!
//! バイナリ層（`main.rs`）とテストの両方からこの API を呼ぶ。
//! stdout は成果物の出力にのみ使い、ログや診断は stderr に出す
//! （AI エージェントは stdout をコンテキストに取り込むため）。
#![warn(clippy::print_stdout)]

pub mod cli;
pub mod error;
pub mod logging;

use std::process::ExitCode;

use clap::Parser;

pub use error::Error;

/// バイナリのエントリ。引数パース → ログ初期化 → [`run`] → 終了コード決定。
pub fn main_entry() -> ExitCode {
    let args = cli::Cli::parse();
    // プロセス環境を読むのはこの 1 行だけ。以降の解決ロジックは値を引数で受け取る。
    logging::init(args.verbose, &logging::LogEnv::from_process());
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // 認証情報を含みうる値は Display に載せない方針（実装フェーズで secrecy を導入する）
            tracing::error!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

/// 実処理。環境構築フェーズでは未実装コマンドを明示的にエラーにする。
///
/// # Errors
///
/// 未実装のサブコマンドが指定された場合、help の出力に失敗した場合にエラーを返す。
pub fn run(args: &cli::Cli) -> anyhow::Result<()> {
    match &args.command {
        // サブコマンド未指定時は help を出す。help は利用者向けの正当な stdout 出力。
        None => {
            use clap::CommandFactory;

            cli::Cli::command().print_help()?;
            Ok(())
        }
        Some(cli::Command::Tts { .. }) => {
            anyhow::bail!("the `tts` subcommand is not implemented yet")
        }
    }
}
