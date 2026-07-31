//! HTTP 境界。`reqwest` を呼ぶのはこの 1 モジュールだけ（A-001 / design 4.2節）。
//!
//! 認証ヘッダの組み立てをここに閉じ込め、リクエストのログ出力にヘッダを含めない（DC_006）。

use crate::credential;

use super::{SynthesisRequest, SynthesisResponse, TtsError};

/// `Region` に対応する API ホスト（design 3.1節 / PRD DC_004）。
///
/// API キーとホストが一致しないと認証エラーになるため、`Region` から一方的に導出する。
fn host(region: credential::Region) -> &'static str {
    match region {
        credential::Region::Global => "https://api.minimax.io",
        credential::Region::China => "https://api.minimaxi.com",
    }
}

const PATH: &str = "/v1/t2a_v2";

/// HTTP 境界。テストでは呼ばない（design 8.1節。実 API 呼び出しは課金を伴う）。
///
/// レート制限等のエラーでも自動再送しない（DC_003 / FR-010）。
///
/// # Errors
///
/// 通信に失敗した場合 `Transport`、レスポンスが JSON として解釈できない場合
/// `MalformedResponse` を返す。業務エラーの判定は呼び出し側の `extract_audio` が行う
/// （HTTP ステータスとは独立に本文を解釈する。design 6.1節）。
pub fn send(
    region: credential::Region,
    token: &credential::ApiToken,
    request: &SynthesisRequest,
) -> Result<SynthesisResponse, TtsError> {
    let url = format!("{}{PATH}", host(region));
    let response = reqwest::blocking::Client::new()
        .post(url)
        .bearer_auth(token.expose())
        .json(request)
        .send()
        .map_err(|error| TtsError::Transport {
            detail: error.to_string(),
        })?;

    response
        .json::<SynthesisResponse>()
        .map_err(|error| TtsError::MalformedResponse {
            detail: error.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::host;
    use crate::credential::Region;

    #[test]
    fn host_maps_each_region_to_its_documented_endpoint() {
        assert_eq!(host(Region::Global), "https://api.minimax.io");
        assert_eq!(host(Region::China), "https://api.minimaxi.com");
    }
}
