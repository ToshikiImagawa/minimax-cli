//! コマンドライン引数の定義。
//!
//! `--version` は `#[command(version)]` により Cargo.toml の `version` を参照するため、
//! バージョンの真実は Cargo.toml 1 箇所に集約される。

use clap::{Parser, Subcommand};

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
    /// Convert text into a speech audio file (not implemented yet)
    Tts {
        /// Text to synthesize
        text: String,
    },
}
