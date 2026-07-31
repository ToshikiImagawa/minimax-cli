//! 音声合成の型・境界・ロジック。
//!
//! このモジュール本体は外部世界に到達しない（A-001）。通信は [`client`] に、
//! ファイル書き出しは [`writer`] に閉じ込める。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// 同期 API のテキスト長上限。API 仕様は「10,000 文字未満」。
///
/// API 側の数え方（コードポイント / UTF-16 / grapheme）は公開されていないため、
/// Rust の char 数で近似する。境界付近では API 側のエラーになりうる（design 9 節 D-5）。
pub const TEXT_LENGTH_LIMIT: usize = 10_000;

/// 同期音声合成 API へ送るリクエスト。
#[derive(Debug, Serialize)]
pub struct SynthesisRequest {
    pub model: String,
    pub text: String,
    /// 常に false（ストリーミングは扱わない）。
    pub stream: bool,
    /// 常に "hex"（音声データを本文で受け取る）。
    pub output_format: &'static str,
    /// 省略時は "auto"。
    pub language_boost: String,
    pub voice_setting: VoiceSetting,
    pub audio_setting: AudioSetting,
}

#[derive(Debug, Serialize)]
pub struct VoiceSetting {
    pub voice_id: String,
}

#[derive(Debug, Serialize)]
pub struct AudioSetting {
    /// "mp3" / "wav" / "flac"。
    pub format: &'static str,
}

/// 同期音声合成 API のレスポンス。
///
/// **HTTP ステータスが 200 でも `base_resp.status_code` が 0 以外になりうる。**
/// `data` はエラー時に欠ける場合があるため `Option` で受ける。
#[derive(Debug, Deserialize)]
pub struct SynthesisResponse {
    pub data: Option<ResponseData>,
    pub extra_info: Option<ExtraInfo>,
    pub base_resp: BaseResp,
}

#[derive(Debug, Deserialize)]
pub struct ResponseData {
    /// hex エンコードされた音声データ。
    pub audio: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExtraInfo {
    /// 課金対象文字数。stderr にのみ出す（成果物ではない）。
    pub usage_characters: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct BaseResp {
    pub status_code: i64,
    pub status_msg: String,
}

/// 出力音声形式。対応する拡張子と 1:1 に対応する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Flac,
}

impl AudioFormat {
    /// リクエストに載せる文字列表現。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }
}

/// 解決済みのテキスト取得元（spec 5.3 の規則の結果）。
#[derive(Debug)]
pub enum TextSource {
    Argument(String),
    File(PathBuf),
    Stdin,
}

/// この機能で起きる失敗。バイナリ層（`main_entry`）が anyhow で受けて終了コードに変換する。
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    /// テキストの経路が 2 つ以上指定された（FR-002）。
    #[error("specify the text via exactly one of: the argument, --file, or standard input")]
    TextSourceConflict,
    /// テキストが指定されていない（FR-002）。
    #[error("no text was given; pass it as an argument, --file <PATH>, or via standard input")]
    TextSourceMissing,
    /// テキストが空または空白のみ（FR-003）。
    #[error("the text to synthesize is empty")]
    EmptyText,
    /// テキストが長さ上限を超えた（FR-006）。
    #[error(
        "the text is {chars} characters, at or above the {limit}-character limit for the \
         synchronous API; split the text, or use the asynchronous API (not implemented by this CLI)"
    )]
    TextTooLong { chars: usize, limit: usize },
    /// 出力パスの拡張子が未対応（FR-004）。
    #[error("unsupported output extension: {extension:?} (expected mp3, wav, or flac)")]
    UnsupportedExtension { extension: Option<String> },
    /// 出力先に既存のファイルがある（FR-005）。
    #[error("the output file already exists: {path} (use --force to overwrite)", path = path.display())]
    OutputExists { path: PathBuf },
    /// 認証情報の取得に失敗した。**区別を保ったまま伝える**（FR-011）。
    ///
    /// `credential::CredentialError` には `tts` の文脈では発生しない variant
    /// （`InvalidToken` / `InteractiveInputRejected` / `StdinRead`。いずれも `auth login` 専用）
    /// も含まれる。これらは `credential::load` からは返らないため到達しないが、
    /// `thiserror` の `#[error(transparent)]` で `Display` をそのまま委譲することで、
    /// 呼び出し側で網羅的な match を強制しない（万一到達しても汎用的なメッセージになるだけで、
    /// 誤った案内は出さない）。
    #[error(transparent)]
    Credential(#[from] crate::credential::CredentialError),
    /// 通信に失敗した。
    #[error("failed to reach the MiniMax API: {detail}")]
    Transport { detail: String },
    /// API が業務エラーを返した（FR-007）。
    #[error(transparent)]
    Api(#[from] ApiFailure),
    /// レスポンスの解釈に失敗した（想定外の形・デコード不能）。
    #[error("could not interpret the API response: {detail}")]
    MalformedResponse { detail: String },
    /// 音声データの書き出しに失敗した（FR-008）。
    #[error("failed to write the audio file to {path}: {detail}", path = path.display())]
    Write { path: PathBuf, detail: String },
}

/// API の業務エラーの区分（PRD DC_009 / T-001）。
#[derive(Debug, thiserror::Error)]
pub enum ApiFailure {
    /// 認証に失敗した。トークンが無効な場合と、リージョンが一致しない場合の両方がありうる。
    #[error(
        "authentication failed; check that the registered token matches the registered region \
         (run `minimax auth status`)"
    )]
    Unauthorized,
    /// アカウントの残高が不足している。
    #[error("the MiniMax account balance is insufficient")]
    InsufficientBalance,
    /// レート制限に達した。自動再送はしない（FR-010）。
    ///
    /// API 由来の `detail` をそのまま見せる。実測の結果、この区分のコードは
    /// 「待てば解決する」ものだけでなく `2056`（Token Plan の枠超過。アップグレードか
    /// Credits 購入が必要）のように固定文言では誤案内になるケースがあると判明したため（design 6.2 節）。
    #[error("rate limited by the API (this CLI does not retry automatically): {detail}")]
    RateLimited { detail: String },
    /// 入力に起因するエラー。
    #[error("the API rejected the input: {detail}")]
    InvalidInput { detail: String },
    /// 上記に当てはまらないエラー。
    #[error("the API returned error {code}: {message}")]
    Other { code: i64, message: String },
}

impl ApiFailure {
    /// API のステータスコードを区分に落とす。
    ///
    /// 網羅していないコードは `Other` に落ちる。区分を増やすときは
    /// この match と、案内文を出す側の match の両方を更新する（コンパイラが漏れを検出する）。
    pub fn from_code(code: i64, message: String) -> Self {
        match code {
            1004 | 2049 => Self::Unauthorized,
            1008 => Self::InsufficientBalance,
            1002 | 1039 | 1041 | 2045 | 2056 => Self::RateLimited { detail: message },
            1026 | 1027 | 1042 | 2013 => Self::InvalidInput { detail: message },
            _ => Self::Other { code, message },
        }
    }
}

/// 位置引数・`--file`・標準入力から経路を決める（spec 5.3 の規則）。
///
/// `stdin_is_terminal` を引数で受け取るのは A-001 のため。
/// プロセスの標準入力を見るのは境界の 1 行だけにする。
///
/// # Errors
///
/// 位置引数と `--file` が同時に指定された場合 `TextSourceConflict`、
/// どちらも無く標準入力が端末の場合 `TextSourceMissing` を返す。
pub fn resolve_text_source(
    text: Option<String>,
    file: Option<PathBuf>,
    stdin_is_terminal: bool,
) -> Result<TextSource, TtsError> {
    match (text, file) {
        (Some(_), Some(_)) => Err(TtsError::TextSourceConflict),
        (Some(value), None) => Ok(TextSource::Argument(value)),
        (None, Some(path)) => Ok(TextSource::File(path)),
        (None, None) if stdin_is_terminal => Err(TtsError::TextSourceMissing),
        (None, None) => Ok(TextSource::Stdin),
    }
}

/// 出力パスの拡張子から音声形式を決める。
///
/// # Errors
///
/// 拡張子が無い、または未対応の場合 `UnsupportedExtension` を返す。
pub fn format_from_path(path: &Path) -> Result<AudioFormat, TtsError> {
    let extension = path.extension().and_then(|value| value.to_str());
    match extension.map(str::to_ascii_lowercase).as_deref() {
        Some("mp3") => Ok(AudioFormat::Mp3),
        Some("wav") => Ok(AudioFormat::Wav),
        Some("flac") => Ok(AudioFormat::Flac),
        other => Err(TtsError::UnsupportedExtension {
            extension: other.map(str::to_owned),
        }),
    }
}

/// テキストの空と長さ上限を検査する。
///
/// # Errors
///
/// 空または空白のみの場合 `EmptyText`、上限以上の場合 `TextTooLong` を返す。
pub fn validate_text(text: &str) -> Result<(), TtsError> {
    if text.trim().is_empty() {
        return Err(TtsError::EmptyText);
    }
    let chars = text.chars().count();
    if chars >= TEXT_LENGTH_LIMIT {
        return Err(TtsError::TextTooLong {
            chars,
            limit: TEXT_LENGTH_LIMIT,
        });
    }
    Ok(())
}

/// 出力先の既存ファイルを検査する（FR-005 / PRD FR_004_03）。
///
/// `exists` を**引数で受け取る**のは A-001 のため（`resolve_text_source` の
/// `stdin_is_terminal` と同じパターン）。プロセスのファイルシステムを見るのは
/// 境界（`lib.rs::run` が `path.exists()` を呼ぶ 1 行）だけにする。
///
/// # Errors
///
/// ファイルが存在し `force` が false の場合 `OutputExists` を返す。
pub fn check_output_path(path: &Path, exists: bool, force: bool) -> Result<(), TtsError> {
    if exists && !force {
        return Err(TtsError::OutputExists {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

/// レスポンスの業務エラーを検査し、音声データの hex 文字列を返す。
///
/// **HTTP ステータスとは独立に呼ぶ**（HTTP 200 でもエラーが返るため）。
///
/// # Errors
///
/// `base_resp.status_code` が 0 以外の場合 `Api`、
/// 音声データが欠けている場合 `MalformedResponse` を返す。
pub fn extract_audio(response: SynthesisResponse) -> Result<String, TtsError> {
    if response.base_resp.status_code != 0 {
        return Err(TtsError::Api(ApiFailure::from_code(
            response.base_resp.status_code,
            response.base_resp.status_msg,
        )));
    }
    response
        .data
        .and_then(|data| data.audio)
        .ok_or_else(|| TtsError::MalformedResponse {
            detail: "the response contains no audio data".to_owned(),
        })
}

pub mod client;
pub mod writer;

#[cfg(test)]
mod tests {
    use super::{
        ApiFailure, AudioFormat, BaseResp, ExtraInfo, ResponseData, SynthesisResponse,
        TEXT_LENGTH_LIMIT, TextSource, TtsError, check_output_path, extract_audio,
        format_from_path, resolve_text_source, validate_text,
    };
    use std::path::{Path, PathBuf};

    // --- resolve_text_source（spec 5.3 の表の全 5 行） ---

    #[test]
    fn resolve_text_source_rejects_argument_and_file_together() {
        let result = resolve_text_source(
            Some("hello".to_owned()),
            Some(PathBuf::from("script.txt")),
            false,
        );
        assert!(matches!(result, Err(TtsError::TextSourceConflict)));
    }

    #[test]
    fn resolve_text_source_uses_the_argument_when_only_the_argument_is_given() {
        let result = resolve_text_source(Some("hello".to_owned()), None, false);
        assert!(matches!(result, Ok(TextSource::Argument(text)) if text == "hello"));
    }

    #[test]
    fn resolve_text_source_uses_the_file_when_only_the_file_is_given() {
        let path = PathBuf::from("script.txt");
        let result = resolve_text_source(None, Some(path.clone()), false);
        assert!(matches!(result, Ok(TextSource::File(resolved)) if resolved == path));
    }

    #[test]
    fn resolve_text_source_reads_stdin_when_neither_is_given_and_stdin_is_not_a_terminal() {
        let result = resolve_text_source(None, None, false);
        assert!(matches!(result, Ok(TextSource::Stdin)));
    }

    #[test]
    fn resolve_text_source_rejects_nothing_given_when_stdin_is_a_terminal() {
        let result = resolve_text_source(None, None, true);
        assert!(matches!(result, Err(TtsError::TextSourceMissing)));
    }

    // --- format_from_path ---

    #[test]
    fn format_from_path_recognizes_the_three_supported_extensions() {
        assert_eq!(
            format_from_path(Path::new("out.mp3")).expect("should resolve"),
            AudioFormat::Mp3
        );
        assert_eq!(
            format_from_path(Path::new("out.wav")).expect("should resolve"),
            AudioFormat::Wav
        );
        assert_eq!(
            format_from_path(Path::new("out.flac")).expect("should resolve"),
            AudioFormat::Flac
        );
    }

    #[test]
    fn format_from_path_is_case_insensitive() {
        assert_eq!(
            format_from_path(Path::new("out.MP3")).expect("should resolve"),
            AudioFormat::Mp3
        );
    }

    #[test]
    fn format_from_path_rejects_a_missing_extension() {
        let result = format_from_path(Path::new("out"));
        assert!(matches!(
            result,
            Err(TtsError::UnsupportedExtension { extension: None })
        ));
    }

    #[test]
    fn format_from_path_rejects_an_unsupported_extension() {
        let result = format_from_path(Path::new("out.ogg"));
        assert!(matches!(
            result,
            Err(TtsError::UnsupportedExtension { extension: Some(ref value) }) if value == "ogg"
        ));
    }

    // --- validate_text ---

    #[test]
    fn validate_text_rejects_an_empty_string() {
        assert!(matches!(validate_text(""), Err(TtsError::EmptyText)));
    }

    #[test]
    fn validate_text_rejects_whitespace_only_text() {
        assert!(matches!(validate_text("   \n\t"), Err(TtsError::EmptyText)));
    }

    #[test]
    fn validate_text_accepts_text_just_below_the_limit() {
        let text = "a".repeat(TEXT_LENGTH_LIMIT - 1);
        assert!(validate_text(&text).is_ok());
    }

    #[test]
    fn validate_text_rejects_text_at_the_limit_boundary() {
        let text = "a".repeat(TEXT_LENGTH_LIMIT);
        let result = validate_text(&text);
        assert!(matches!(
            result,
            Err(TtsError::TextTooLong { chars, limit })
                if chars == TEXT_LENGTH_LIMIT && limit == TEXT_LENGTH_LIMIT
        ));
    }

    // --- check_output_path ---

    #[test]
    fn check_output_path_rejects_an_existing_file_without_force() {
        let path = PathBuf::from("out.mp3");
        let result = check_output_path(&path, true, false);
        assert!(matches!(result, Err(TtsError::OutputExists { path: ref p }) if *p == path));
    }

    #[test]
    fn check_output_path_allows_an_existing_file_with_force() {
        assert!(check_output_path(Path::new("out.mp3"), true, true).is_ok());
    }

    #[test]
    fn check_output_path_allows_a_missing_file_regardless_of_force() {
        assert!(check_output_path(Path::new("out.mp3"), false, false).is_ok());
        assert!(check_output_path(Path::new("out.mp3"), false, true).is_ok());
    }

    // --- extract_audio ---

    fn response(status_code: i64, audio: Option<&str>) -> SynthesisResponse {
        SynthesisResponse {
            data: audio.map(|value| ResponseData {
                audio: Some(value.to_owned()),
            }),
            extra_info: Some(ExtraInfo {
                usage_characters: Some(5),
            }),
            base_resp: BaseResp {
                status_code,
                status_msg: "success".to_owned(),
            },
        }
    }

    #[test]
    fn extract_audio_returns_the_hex_string_on_success() {
        let result = extract_audio(response(0, Some("deadbeef")));
        assert_eq!(result.expect("should succeed"), "deadbeef");
    }

    #[test]
    fn extract_audio_returns_api_failure_when_status_code_is_not_zero() {
        let result = extract_audio(response(2049, None));
        assert!(matches!(
            result,
            Err(TtsError::Api(ApiFailure::Unauthorized))
        ));
    }

    #[test]
    fn extract_audio_returns_malformed_response_when_audio_is_missing() {
        let result = extract_audio(response(0, None));
        assert!(matches!(result, Err(TtsError::MalformedResponse { .. })));
    }

    // --- ApiFailure::from_code（対応表の全コード + 未知のコード） ---

    #[test]
    fn from_code_maps_the_documented_table_exhaustively() {
        let message = || "detail".to_owned();

        for code in [1004, 2049] {
            assert!(matches!(
                ApiFailure::from_code(code, message()),
                ApiFailure::Unauthorized
            ));
        }
        assert!(matches!(
            ApiFailure::from_code(1008, message()),
            ApiFailure::InsufficientBalance
        ));
        for code in [1002, 1039, 1041, 2045, 2056] {
            assert!(matches!(
                ApiFailure::from_code(code, message()),
                ApiFailure::RateLimited { .. }
            ));
        }
        for code in [1026, 1027, 1042, 2013] {
            assert!(matches!(
                ApiFailure::from_code(code, message()),
                ApiFailure::InvalidInput { .. }
            ));
        }
    }

    #[test]
    fn from_code_falls_back_to_other_for_an_unknown_code() {
        let result = ApiFailure::from_code(9999, "mystery".to_owned());
        assert!(matches!(
            result,
            ApiFailure::Other { code: 9999, message } if message == "mystery"
        ));
    }

    /// `2056`（Token Plan の枠超過）は実測の結果、固定文言では
    /// 「待てば解決する」と誤案内になると判明した（design 6.2 節）。
    /// API 由来の `status_msg` をそのまま利用者に見せることを固定する。
    #[test]
    fn rate_limited_carries_the_api_message_through_to_display() {
        let result = ApiFailure::from_code(
            2056,
            "Token Plan usage limit reached: Upgrade your Token Plan or purchase Credits for more usage."
                .to_owned(),
        );
        assert!(matches!(
            result,
            ApiFailure::RateLimited { ref detail } if detail.contains("Upgrade your Token Plan")
        ));
        assert!(result.to_string().contains("Upgrade your Token Plan"));
    }

    // --- リクエストのシリアライズ（JSON のフィールド名が API 仕様と一致すること） ---

    #[test]
    fn synthesis_request_serializes_with_the_field_names_the_api_expects() {
        use super::{AudioSetting, SynthesisRequest, VoiceSetting};

        let request = SynthesisRequest {
            model: "speech-2.5-hd".to_owned(),
            text: "hello".to_owned(),
            stream: false,
            output_format: "hex",
            language_boost: "auto".to_owned(),
            voice_setting: VoiceSetting {
                voice_id: "voice-1".to_owned(),
            },
            audio_setting: AudioSetting {
                format: AudioFormat::Mp3.as_str(),
            },
        };

        let json = serde_json::to_value(&request).expect("should serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "model": "speech-2.5-hd",
                "text": "hello",
                "stream": false,
                "output_format": "hex",
                "language_boost": "auto",
                "voice_setting": { "voice_id": "voice-1" },
                "audio_setting": { "format": "mp3" },
            })
        );
    }
}
