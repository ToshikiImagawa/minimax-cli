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

/// アプリケーション層のエラー（clap の引数検証エラーではないもの）を安定して起こす引数。
///
/// 未対応の拡張子は認証情報ストアや標準入力に触れる前（送信前検査。design 4.3 節）に
/// 失敗するため、ログ・stdout の出し分けを検証する上で環境に依存しない。
const UNSUPPORTED_EXTENSION_ARGS: [&str; 7] = [
    "tts",
    "--voice",
    "v",
    "--model",
    "m",
    "--output",
    "out.unsupported",
];

/// 診断出力を stdout に載せない設計の回帰テスト。
///
/// stdout は成果物専用に空けておく必要がある（AI エージェントが stdout を
/// コンテキストに取り込むため、認証情報やエラー詳細を載せてはならない）。
#[test]
fn diagnostics_never_go_to_stdout() {
    bin()
        .args(UNSUPPORTED_EXTENSION_ARGS)
        .arg("hello")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty());
}

/// エラー表示は `main_entry` の `tracing::error!` 経由なのでログフィルタに従う。
/// ログを `off` にするとエラーメッセージまで消え、終了コードだけが残る。
///
/// CLAUDE.md に落とし穴として明記している挙動なので、意図せず変わらないよう固定する。
/// `bin()` が親の環境変数を外すため、このテストは実行環境の `RUST_LOG` に影響されない。
#[test]
fn log_off_silences_the_error_message_but_keeps_the_exit_code() {
    bin()
        .env("MINIMAX_LOG", "off")
        .args(UNSUPPORTED_EXTENSION_ARGS)
        .arg("hello")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

/// `RUST_LOG` も同じ経路で効く（`MINIMAX_LOG` 未設定時のフォールバック）。
#[test]
fn rust_log_off_also_silences_the_error_message() {
    bin()
        .env("RUST_LOG", "off")
        .args(UNSUPPORTED_EXTENSION_ARGS)
        .arg("hello")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty());
}

/// `MINIMAX_LOG` は `RUST_LOG` より優先される。
/// `RUST_LOG=off` でもエラーが出ることで、優先順位が外形からも確認できる。
#[test]
fn minimax_log_takes_precedence_over_rust_log() {
    bin()
        .env("MINIMAX_LOG", "error")
        .env("RUST_LOG", "off")
        .args(UNSUPPORTED_EXTENSION_ARGS)
        .arg("hello")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported"));
}
