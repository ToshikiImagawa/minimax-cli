//! バイナリのエントリポイント。処理の本体は lib 側に置き、ここは薄く保つ。
#![warn(clippy::print_stdout)]

use std::process::ExitCode;

fn main() -> ExitCode {
    minimax_cli::main_entry()
}
