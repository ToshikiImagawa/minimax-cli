//! `auth` サブコマンドの結合テスト。
//!
//! design 8.1 節が定義する「ストアに触れる前に完結する経路」のみを対象にする。
//! `auth status` / `auth logout` の成功経路と `auth login` の保存経路は、実ストアへの
//! 副作用を避けるため結合テストの対象外とし、`src/credential.rs` の単体テストで担保する。
//! `auth login` の TTY 判定失敗経路も対象外（design 8.1 節 D-10。3 OS CI で pty を
//! 使わずに固定する手段が無いため、`read_token_from_rejects_a_terminal` の単体テストで担保する）。

mod common;

use common::bin;
use predicates::prelude::*;

#[test]
fn auth_without_subcommand_prints_help() {
    bin()
        .arg("auth")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Manage the MiniMax API credential",
        ));
}

#[test]
fn auth_help_prints_usage() {
    bin()
        .args(["auth", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Manage the MiniMax API credential",
        ));
}

#[test]
fn auth_login_help_prints_usage() {
    bin()
        .args(["auth", "login", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Register an API token read from standard input",
        ));
}

#[test]
fn auth_login_without_region_fails_with_exit_code_2() {
    bin()
        .args(["auth", "login"])
        .write_stdin("token\n")
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--region"));
}

#[test]
fn auth_login_with_invalid_region_fails_with_exit_code_2() {
    bin()
        .args(["auth", "login", "--region", "us"])
        .write_stdin("token\n")
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--region"));
}

#[test]
fn auth_login_with_empty_stdin_fails() {
    bin()
        .args(["auth", "login", "--region", "global"])
        .write_stdin("")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn auth_login_with_whitespace_only_stdin_fails() {
    bin()
        .args(["auth", "login", "--region", "global"])
        .write_stdin("   \n")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("empty"));
}
