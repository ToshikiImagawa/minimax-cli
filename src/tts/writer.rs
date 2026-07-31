//! 書き出し境界。失敗時に不完全なファイルを残さない（spec FR-008 / PRD DC_005 / design 6.3節）。
//!
//! 同一ディレクトリの一時ファイルに書き、成功したら最終パスへ rename する。
//! rename はファイルシステムが同一なら原子的なため、途中経過が最終パスに見えることはない。

use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::TtsError;

/// # Errors
///
/// 一時ファイルの作成・書き込み・rename のいずれかが失敗した場合 `Write` を返す。
/// 失敗時、一時ファイルは削除され、最終パスには何も書き込まれない。
pub fn write_audio(path: &Path, bytes: &[u8]) -> Result<(), TtsError> {
    let temp_path = temp_path_for(path);
    if let Err(error) = write_temp(&temp_path, bytes) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(write_error(path, error));
    }
    std::fs::rename(&temp_path, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp_path);
        write_error(path, error)
    })
}

fn write_temp(temp_path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::File::create(temp_path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// 最終パスと同一ディレクトリの一時ファイル名（rename を同一ファイルシステム内に保つため）。
fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path.file_name().map_or_else(
        || ".minimax-tts.tmp".to_owned(),
        |name| format!(".{}.tmp", name.to_string_lossy()),
    );
    path.with_file_name(file_name)
}

fn write_error(path: &Path, error: std::io::Error) -> TtsError {
    TtsError::Write {
        path: path.to_path_buf(),
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::write_audio;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 呼び出しごとに一意なディレクトリを返す（並列実行するテスト間で衝突しない）。
    fn unique_temp_dir() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "minimax-cli-writer-test-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("should create a temp dir");
        dir
    }

    #[test]
    fn write_audio_writes_the_bytes_to_the_final_path() {
        let dir = unique_temp_dir();
        let path = dir.join("out.mp3");

        write_audio(&path, b"audio-bytes").expect("should write");

        assert_eq!(
            std::fs::read(&path).expect("should read back"),
            b"audio-bytes"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_audio_leaves_no_file_at_the_final_path_on_failure() {
        let dir = unique_temp_dir();
        // 出力先の親ディレクトリが存在しないため、一時ファイルの作成自体が失敗する。
        let path = dir.join("missing-subdir").join("out.mp3");

        let result = write_audio(&path, b"audio-bytes");

        assert!(result.is_err());
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
