//! CLI の結合テスト。実際にバイナリを起動して外形的な振る舞いを検証する。

mod common;

use common::bin;
use predicates::prelude::*;

#[test]
fn version_flag_prints_cargo_pkg_version() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_flag_prints_usage() {
    bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn no_args_prints_help() {
    bin()
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn unknown_flag_fails_with_message_on_stderr() {
    bin()
        .arg("--no-such-flag")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--no-such-flag"));
}

#[test]
fn tts_is_not_implemented_yet() {
    bin()
        .args(["tts", "hello"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not implemented"));
}

/// 診断出力を stdout に載せない設計の回帰テスト。
///
/// stdout は成果物専用に空けておく必要がある（AI エージェントが stdout を
/// コンテキストに取り込むため、認証情報やエラー詳細を載せてはならない）。
#[test]
fn diagnostics_never_go_to_stdout() {
    bin()
        .args(["tts", "hello"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty());
}
