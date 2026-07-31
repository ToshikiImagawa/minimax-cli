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
pub mod tts;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use credential::os_store::OsStore;

pub use error::Error;

/// バイナリのエントリ。引数パース → ログ初期化 → [`run`] → 終了コード決定。
pub fn main_entry() -> ExitCode {
    let args = cli::Cli::parse();
    // プロセス環境を読むのはこの 1 行だけ。以降の解決ロジックは値を引数で受け取る。
    logging::init(args.verbose, &logging::LogEnv::from_process());
    // ライブラリ層（run）は型付きエラーを返す。バイナリ層のこの 1 箇所だけが
    // anyhow で受け、表示・終了コードへの変換に集約する（CLAUDE.md の「エラー処理の層分け」）。
    match run(&args).map_err(anyhow::Error::from) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            // 認証情報を含みうる値は Display に載せない方針（実装フェーズで secrecy を導入する）
            tracing::error!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

/// 実処理。サブコマンドのディスパッチ。
///
/// `anyhow` はバイナリ層（[`main_entry`]）に閉じ、ライブラリ層は型付きエラーを返す
/// （CLAUDE.md の「エラー処理の層分け」）。
///
/// # Errors
///
/// help の出力に失敗した場合、各サブコマンドの処理が失敗した場合にエラーを返す。
pub fn run(args: &cli::Cli) -> Result<(), Error> {
    match &args.command {
        // サブコマンド未指定時は help を出す。help は利用者向けの正当な stdout 出力。
        None => {
            use clap::CommandFactory;

            cli::Cli::command().print_help()?;
            Ok(())
        }
        Some(cli::Command::Tts {
            text,
            file,
            voice,
            model,
            output,
            language,
            force,
        }) => tts_run(
            text.clone(),
            file.clone(),
            voice.clone(),
            model.clone(),
            output.clone(),
            language.clone(),
            *force,
        ),
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

/// `tts`。design 4.3 節の処理順序（送信前検査をすべて通ってから API を呼ぶ）を厳守する。
///
/// `5` を `7` より先に置く（既存ファイルの検査を認証情報の読み出しより先に行う）ことが
/// spec の FR-005 の要求であり、順序を入れ替えると課金回避の意図が崩れる。
///
/// `stdin.is_terminal()` と `output.exists()` を見るのはこの関数だけ（A-001）。
fn tts_run(
    text: Option<String>,
    file: Option<PathBuf>,
    voice: String,
    model: String,
    output: PathBuf,
    language: Option<String>,
    force: bool,
) -> Result<(), Error> {
    use std::io::{IsTerminal as _, Read as _, Write as _};

    // 1. テキストの経路を決定する。
    let stdin = std::io::stdin();
    let source = tts::resolve_text_source(text, file, stdin.is_terminal()).map_err(Error::Tts)?;

    // 2. テキストを取得する。
    let text = match source {
        tts::TextSource::Argument(value) => value,
        tts::TextSource::File(path) => std::fs::read_to_string(&path).map_err(Error::Io)?,
        tts::TextSource::Stdin => {
            let mut buffer = String::new();
            stdin
                .lock()
                .read_to_string(&mut buffer)
                .map_err(Error::Io)?;
            buffer
        }
    };

    // 3. テキストの空・長さ上限を検査する。
    tts::validate_text(&text).map_err(Error::Tts)?;

    // 4. 出力パスの拡張子から音声形式を決定する。
    let format = tts::format_from_path(&output).map_err(Error::Tts)?;

    // 5. 出力パスの既存ファイルを検査する。ここまでは API を呼ばないため課金されない。
    tts::check_output_path(&output, output.exists(), force).map_err(Error::Tts)?;

    // 6. 認証情報を読み出す。
    let credential =
        credential::load(&OsStore).map_err(|error| Error::Tts(tts::TtsError::Credential(error)))?;

    let request = tts::SynthesisRequest {
        model,
        text,
        stream: false,
        output_format: "hex",
        language_boost: language.unwrap_or_else(|| "auto".to_owned()),
        voice_setting: tts::VoiceSetting { voice_id: voice },
        audio_setting: tts::AudioSetting {
            format: format.as_str(),
        },
    };

    // 7. API へ送信する。
    let response =
        tts::client::send(credential.region, &credential.token, &request).map_err(Error::Tts)?;

    // 8. レスポンスの業務エラーを検査する。
    let hex_audio = tts::extract_audio(response).map_err(Error::Tts)?;

    // 9. 音声データをデコードする。
    let bytes = hex::decode(&hex_audio).map_err(|error| {
        Error::Tts(tts::TtsError::MalformedResponse {
            detail: error.to_string(),
        })
    })?;

    // 10. 一時ファイルへ書き、成功したら最終パスへ rename する。
    tts::writer::write_audio(&output, &bytes).map_err(Error::Tts)?;

    // 11. 最終パスを stdout に 1 行出力する（成果物。B-002 の許可リスト対象）。
    writeln!(std::io::stdout().lock(), "{}", output.display())?;
    Ok(())
}

/// `auth login`。標準入力からトークンを読み、リージョンと組で保存する。
///
/// プロセスの `stdin.is_terminal()` を見るのはここだけ（A-001）。
fn auth_login(region: cli::RegionArg) -> Result<(), Error> {
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
        Err(err) => return Err(Error::Credential(err)),
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
fn auth_status() -> Result<(), Error> {
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
fn auth_logout() -> Result<(), Error> {
    if !credential::remove(&OsStore).map_err(Error::Credential)? {
        tracing::warn!("no API token was registered; nothing to remove");
    }
    Ok(())
}
