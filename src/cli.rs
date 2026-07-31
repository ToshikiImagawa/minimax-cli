//! コマンドライン引数の定義。
//!
//! `--version` は `#[command(version)]` により Cargo.toml の `version` を参照するため、
//! バージョンの真実は Cargo.toml 1 箇所に集約される。

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Generate speech audio files with the MiniMax AI TTS API.
#[derive(Debug, Parser)]
#[command(name = "minimax", version, about, long_about = None)]
pub struct Cli {
    /// Increase log verbosity (-v for info, -vv for debug)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// サブコマンド。
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Convert text into a speech audio file
    Tts {
        /// Text to synthesize
        text: Option<String>,
        /// Read the text to synthesize from a file
        #[arg(long)]
        file: Option<PathBuf>,
        /// Voice ID to synthesize with
        #[arg(long)]
        voice: String,
        /// Model to use for synthesis
        #[arg(long)]
        model: String,
        /// Path of the audio file to write
        #[arg(long)]
        output: PathBuf,
        /// Language hint for pronunciation
        #[arg(long)]
        language: Option<String>,
        /// Overwrite the output file if it already exists
        #[arg(long)]
        force: bool,
    },
    /// Manage the MiniMax API credential
    Auth {
        #[command(subcommand)]
        command: Option<AuthCommand>,
    },
}

/// `auth` 配下のサブコマンド。
#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    /// Register an API token read from standard input
    Login {
        /// Region the API key belongs to
        #[arg(long, value_enum)]
        region: RegionArg,
    },
    /// Show whether a token is registered and for which region
    Status,
    /// Remove the registered API token
    Logout,
}

/// CLI 上のリージョン値。
///
/// `credential::Region` への変換は `lib.rs::run` が行う（`cli` は他モジュールに依存しない）。
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RegionArg {
    Global,
    China,
}
