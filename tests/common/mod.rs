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
pub(crate) fn bin() -> Command {
    Command::cargo_bin("minimax").expect("binary `minimax` should be built")
}

/// リポジトリのルートディレクトリ。
pub(crate) fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
