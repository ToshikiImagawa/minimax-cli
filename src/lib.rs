//! minimax-cli のライブラリ本体。
//!
//! バイナリ層（`main.rs`）とテストの両方からこの API を呼ぶ。
//! stdout は成果物の出力にのみ使い、ログや診断は stderr に出す
//! （AI エージェントは stdout をコンテキストに取り込むため）。
#![warn(clippy::print_stdout)]

pub mod cli;
pub mod credential;
pub mod error;
pub mod logging;

use std::io::IsTerminal;
use std::process::ExitCode;

use clap::Parser;
use credential::os_store::OsStore;

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
        Some(cli::Command::Auth { command }) => match command {
            None => {
                use clap::CommandFactory;

                let mut root = cli::Cli::command();
                match root.find_subcommand_mut("auth") {
                    Some(auth_command) => auth_command.print_help()?,
                    None => root.print_help()?,
                }
                Ok(())
            }
            Some(cli::AuthCommand::Login { region }) => auth_login(*region),
            Some(cli::AuthCommand::Status) => auth_status(),
            Some(cli::AuthCommand::Logout) => auth_logout(),
        },
    }
}

/// `auth login`。標準入力からトークンを読み、リージョンと組で保存する。
///
/// プロセスの `stdin.is_terminal()` を見るのはここだけ（A-001）。
fn auth_login(region: cli::RegionArg) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let token = credential::read_token_from(stdin.lock(), stdin.is_terminal())
        .map_err(Error::Credential)?;

    let region = match region {
        cli::RegionArg::Global => credential::Region::Global,
        cli::RegionArg::China => credential::Region::China,
    };

    let existing = match credential::load(&OsStore) {
        Ok(_) => true,
        Err(credential::CredentialError::NotFound) => false,
        Err(err) => return Err(Error::Credential(err).into()),
    };

    credential::store(&OsStore, credential::StoredCredential { region, token })
        .map_err(Error::Credential)?;

    if existing {
        tracing::warn!(
            "overwrote the previously registered token; now registered for {}",
            region.as_str()
        );
    } else {
        tracing::warn!("registered the token for {}", region.as_str());
    }
    Ok(())
}

/// `auth status`。登録済みなら stdout にリージョンを 1 行返す（B-002 の許可リスト対象）。
fn auth_status() -> anyhow::Result<()> {
    use std::io::Write as _;

    match credential::load(&OsStore) {
        Ok(credential) => {
            writeln!(std::io::stdout().lock(), "{}", credential.region.as_str())?;
            Ok(())
        }
        Err(credential::CredentialError::NotFound) => {
            tracing::error!("no API token is registered; run `minimax auth login`");
            std::process::exit(1);
        }
        Err(err) => {
            tracing::error!("{err}");
            std::process::exit(1);
        }
    }
}

/// `auth logout`。未登録でも成功として扱う（PRD FR_003）。
fn auth_logout() -> anyhow::Result<()> {
    if !credential::remove(&OsStore).map_err(Error::Credential)? {
        tracing::warn!("no API token was registered; nothing to remove");
    }
    Ok(())
}
