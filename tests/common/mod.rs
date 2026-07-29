//! テスト共有ヘルパー。
//!
//! cargo は「1 テストファイル = 1 テストバイナリ」なので、`mod common;` した各バイナリが
//! common の全関数をコンパイルする。そのバイナリで使っていない関数は dead_code になり、
//! `cargo clippy --all-targets -- -D warnings` でエラーになるため allow が必須。
//!
//! また `pub` ではなく `pub(crate)` を使う。`mod common;` は private module なので、
//! `pub fn` にすると `[lints.rust] unreachable_pub` に引っかかる。
#![allow(dead_code)]

use std::path::PathBuf;

use assert_cmd::Command;

/// テスト対象のバイナリを起動する [`Command`] を返す。
///
/// この CLI が読む環境変数を**子プロセスから外す**。名前は
/// [`minimax_cli::logging::LogEnv::NAMES`] を回すので、読み取り側に名前を足せば自動で追随する。
/// テスト側のプロセス環境は書き換えない（edition 2024 では `std::env::set_var` が unsafe で、
/// かつテストはスレッド並列に走る）。
///
/// 外さないと `RUST_LOG=off` を設定した開発環境で stderr の診断が消え、stderr を検証する
/// テストが落ちる。**CI の runner には未設定なので CI は green のまま、ローカルでしか再現しない。**
/// 子プロセスに値を渡したいテストは `.env(...)` で個別に上書きする。
pub(crate) fn bin() -> Command {
    let mut command = Command::cargo_bin("minimax").expect("binary `minimax` should be built");
    for name in minimax_cli::logging::LogEnv::NAMES {
        command.env_remove(name);
    }
    command
}

/// リポジトリのルートディレクトリ。
pub(crate) fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
