//! `tts` サブコマンドの結合テスト。
//!
//! design 8.1 節が定義する「API に到達する前に完結する経路」のみを対象にする。
//! 成功経路（終了コード 0・stdout に音声パス）は実 API 呼び出しを要するため結合テストの
//! 対象外とし、`src/tts.rs` / `src/tts/writer.rs` の単体テストと手動確認で担保する
//! （design 8.1 節の正式な逸脱）。
//!
//! 「認証情報が未登録」の経路も対象外とする。`credential::load` は実際の OS 資格情報ストアに
//! 触れるため、開発機に既にトークンが登録されている場合にテストが環境依存になる
//! （`tests/auth.rs` が `auth status` / `auth logout` の実ストア経路を対象外にしているのと同じ理由）。
//! 「標準入力が端末」の経路も対象外（pty を使わずに固定する手段が無いため。`src/tts.rs` の
//! `resolve_text_source_rejects_nothing_given_when_stdin_is_a_terminal` が単体テストで担保する）。

mod common;

use common::bin;
use predicates::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

/// 呼び出しごとに一意な一時ディレクトリを返す（並列実行するテスト間で衝突しない）。
fn unique_temp_dir() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("minimax-cli-tts-test-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("should create a temp dir");
    dir
}

#[test]
fn tts_help_prints_usage() {
    bin()
        .args(["tts", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Convert text into a speech audio file",
        ))
        .stdout(predicate::str::contains("--voice"))
        .stdout(predicate::str::contains("--model"))
        .stdout(predicate::str::contains("--output"));
}

#[test]
fn tts_without_voice_model_or_output_fails_with_exit_code_2() {
    bin()
        .args(["tts", "hello"])
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--voice"))
        .stderr(predicate::str::contains("--model"))
        .stderr(predicate::str::contains("--output"));
}

#[test]
fn tts_rejects_the_argument_and_file_given_together() {
    let dir = unique_temp_dir();
    let file = dir.join("script.txt");
    std::fs::write(&file, "hello from a file").expect("should write script file");
    let output = dir.join("out.mp3");

    bin()
        .args(["tts", "--voice", "v", "--model", "m"])
        .arg("--output")
        .arg(&output)
        .arg("--file")
        .arg(&file)
        .arg("hello")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("exactly one"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tts_rejects_an_empty_text_piped_via_stdin() {
    let dir = unique_temp_dir();
    let output = dir.join("out.mp3");

    bin()
        .args(["tts", "--voice", "v", "--model", "m"])
        .arg("--output")
        .arg(&output)
        .write_stdin("")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("empty"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tts_rejects_text_over_the_length_limit() {
    let dir = unique_temp_dir();
    let output = dir.join("out.mp3");
    let long_text = "a".repeat(10_000);

    bin()
        .args(["tts", "--voice", "v", "--model", "m"])
        .arg("--output")
        .arg(&output)
        .write_stdin(long_text)
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("asynchronous API"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tts_rejects_an_unsupported_output_extension() {
    let dir = unique_temp_dir();
    let output = dir.join("out.ogg");

    bin()
        .args(["tts", "--voice", "v", "--model", "m"])
        .arg("--output")
        .arg(&output)
        .arg("hello")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("unsupported output extension"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tts_rejects_an_existing_output_file_without_force() {
    let dir = unique_temp_dir();
    let output = dir.join("out.mp3");
    std::fs::write(&output, "previous output").expect("should write a pre-existing file");

    bin()
        .args(["tts", "--voice", "v", "--model", "m"])
        .arg("--output")
        .arg(&output)
        .arg("hello")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("--force"));

    // 送信前検査で拒否されたので、既存ファイルの内容は変わっていない。
    assert_eq!(
        std::fs::read_to_string(&output).expect("should still exist"),
        "previous output"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
