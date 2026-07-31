//! 認証情報の型・境界・ロジック。
//!
//! このモジュール本体は外部世界に到達しない（A-001）。OS の資格情報ストアへの実際のアクセスは
//! [`os_store`] の 1 モジュールに閉じ込める。

use secrecy::SecretString;

/// MiniMax の API ホストの系統。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    Global,
    China,
}

impl Region {
    /// stdout に書く識別子（`auth status` の出力。B-002 の許可リスト対象）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::China => "china",
        }
    }
}

/// API トークン。
///
/// `SecretString` を内包し、`Debug` は値を出さない形で手実装する。
/// `Display` は実装しない（B-001 / PRD DC_002）。
pub struct ApiToken(SecretString);

impl ApiToken {
    /// トークンの実値を返す。呼び出し側は値をログ・エラーメッセージに載せてはならない（B-001）。
    ///
    /// `pub(crate)` に留めるのは、この値を扱えるのはクレート内部（`tts::client` が
    /// 認証ヘッダを組み立てる箇所）だけに限るため。
    pub(crate) fn expose(&self) -> &str {
        use secrecy::ExposeSecret;

        self.0.expose_secret()
    }
}

impl std::fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiToken(<redacted>)")
    }
}

/// 保存する単位。トークン単体では保存しない（PRD FR_001_04）。
#[derive(Debug)]
pub struct StoredCredential {
    pub region: Region,
    pub token: ApiToken,
}

/// 認証情報の取得・保存・削除で起きる失敗。
///
/// 「値が無い」と「読み取りに失敗した」を同じ variant に潰すと、呼び出し側が両者を区別できず、
/// 認証情報を持たないまま処理を続行する事故につながる（design 9 節 D-4）。
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// ストアには到達できたが、このアプリのエントリが無い。
    #[error("no API token is registered")]
    NotFound,

    /// ストア自体に到達できなかった。`NotFound` とは区別する。
    #[error("could not access the OS credential store: {0}")]
    StoreUnavailable(StoreFailure),

    /// 標準入力から受け取った値が空または空白のみ。
    #[error("the token read from standard input is empty")]
    InvalidToken,

    /// 標準入力が端末に接続されている。
    #[error("standard input is a terminal; pipe the token instead")]
    InteractiveInputRejected,

    /// 標準入力の読み取り自体に失敗した。
    ///
    /// `StoreUnavailable` に変換しない。標準入力の I/O 失敗を資格情報ストアの失敗として
    /// 報告すると、この機能が防ごうとしている「原因の誤報告」を自ら犯すことになる。
    #[error("failed to read the token from standard input: {0}")]
    StdinRead(#[source] std::io::Error),
}

/// ストアに到達できなかった原因の区分。利用者への案内を変えるために分ける。
///
/// 各 variant が元のメッセージを保持するのは、デバッグ時に `keyring::Error` の詳細を
/// 失わないため（原因の断定はせず、判明した事実のみを運ぶ）。
#[derive(Debug, thiserror::Error)]
pub enum StoreFailure {
    #[error("the credential store is unavailable on this system: {0}")]
    Unavailable(String),
    #[error("permission denied by the credential store: {0}")]
    PermissionDenied(String),
    #[error("the credential store failed: {0}")]
    Other(String),
}

/// 資格情報ストアへの到達を表す境界（A-001 / PRD DC_006）。
pub trait CredentialStore {
    /// 保存する。既存があれば上書きする（PRD FR_001_05）。
    fn set(&self, region: Region, token: &ApiToken) -> Result<(), StoreFailure>;

    /// 読み出す。エントリが無い場合は `Ok(None)` を返し、
    /// 到達できない場合は `Err` を返す（この 2 つを混同しないことが要求）。
    fn get(&self) -> Result<Option<StoredCredential>, StoreFailure>;

    /// 削除する。エントリが無い場合も `Ok(false)` で成功として扱う（PRD FR_003）。
    fn delete(&self) -> Result<bool, StoreFailure>;
}

/// 標準入力から読み取った文字列を正規化・検証する純粋関数（FR-002 / FR-003）。
///
/// # Errors
///
/// 前後の空白を除去した結果が空の場合 `InvalidToken` を返す。
pub fn parse_token(raw: &str) -> Result<ApiToken, CredentialError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CredentialError::InvalidToken);
    }
    Ok(ApiToken(SecretString::from(trimmed.to_owned())))
}

/// 標準入力からトークンを読む。端末に接続されている場合は拒否する（FR-005）。
///
/// `is_terminal` を引数で受け取ることで、プロセスの標準入力を見るのは境界
/// （`lib.rs::run`）の 1 行だけになる（A-001）。
///
/// # Errors
///
/// 標準入力が端末の場合、読み取りに失敗した場合、値が空の場合にエラーを返す。
pub fn read_token_from<R: std::io::Read>(
    mut reader: R,
    is_terminal: bool,
) -> Result<ApiToken, CredentialError> {
    if is_terminal {
        return Err(CredentialError::InteractiveInputRejected);
    }
    let mut raw = String::new();
    reader
        .read_to_string(&mut raw)
        .map_err(CredentialError::StdinRead)?;
    parse_token(&raw)
}

/// 登録する。既存があれば上書きする（PRD FR_001_05）。
///
/// # Errors
///
/// ストアに到達できなかった場合 `StoreUnavailable` を返す。
pub fn store(
    credential_store: &impl CredentialStore,
    credential: StoredCredential,
) -> Result<(), CredentialError> {
    credential_store
        .set(credential.region, &credential.token)
        .map_err(CredentialError::StoreUnavailable)
}

/// 読み出す。
///
/// `CredentialStore::get` が返す `Ok(None)`（エントリが無い）を `NotFound` に変換する。
/// この変換が唯一の `NotFound` の生成箇所であり、`Err`（到達できない）とは経路を分ける。
///
/// # Errors
///
/// エントリが無い場合 `NotFound`、ストアに到達できなかった場合 `StoreUnavailable` を返す。
pub fn load(credential_store: &impl CredentialStore) -> Result<StoredCredential, CredentialError> {
    match credential_store.get() {
        Ok(Some(credential)) => Ok(credential),
        Ok(None) => Err(CredentialError::NotFound),
        Err(failure) => Err(CredentialError::StoreUnavailable(failure)),
    }
}

/// 削除する。エントリが無い場合も成功として扱う（PRD FR_003）。
///
/// # Errors
///
/// ストアに到達できなかった場合 `StoreUnavailable` を返す。戻り値の `bool` は
/// 実際に削除が行われたかを示す。
pub fn remove(credential_store: &impl CredentialStore) -> Result<bool, CredentialError> {
    credential_store
        .delete()
        .map_err(CredentialError::StoreUnavailable)
}

pub mod os_store;

#[cfg(test)]
mod tests {
    use super::{
        ApiToken, CredentialError, CredentialStore, Region, StoreFailure, StoredCredential, load,
        parse_token, read_token_from, remove, store,
    };
    use secrecy::{ExposeSecret, SecretString};
    use std::cell::RefCell;
    use std::io::Read;

    #[test]
    fn secret_string_from_owned_string_compiles() {
        let _: SecretString = SecretString::from("hello".to_owned());
    }

    #[test]
    fn region_as_str_returns_the_expected_identifier() {
        assert_eq!(Region::Global.as_str(), "global");
        assert_eq!(Region::China.as_str(), "china");
    }

    #[test]
    fn api_token_debug_output_is_redacted() {
        let token = ApiToken(SecretString::from("super-secret".to_owned()));
        assert_eq!(format!("{token:?}"), "ApiToken(<redacted>)");
    }

    // --- parse_token ---

    #[test]
    fn parse_token_trims_surrounding_whitespace_and_newlines() {
        let token = parse_token("  sk-abc123\n").expect("should parse");
        assert_eq!(format!("{token:?}"), "ApiToken(<redacted>)");
    }

    #[test]
    fn parse_token_rejects_empty_input() {
        assert!(matches!(
            parse_token(""),
            Err(CredentialError::InvalidToken)
        ));
    }

    #[test]
    fn parse_token_rejects_whitespace_only_input() {
        assert!(matches!(
            parse_token("   \n\t"),
            Err(CredentialError::InvalidToken)
        ));
    }

    // --- read_token_from ---

    #[test]
    fn read_token_from_rejects_a_terminal() {
        let result = read_token_from(std::io::empty(), true);
        assert!(matches!(
            result,
            Err(CredentialError::InteractiveInputRejected)
        ));
    }

    #[test]
    fn read_token_from_reads_a_pipe() {
        let token = read_token_from("sk-abc123\n".as_bytes(), false).expect("should read");
        assert_eq!(format!("{token:?}"), "ApiToken(<redacted>)");
    }

    /// stdin の読み取り失敗は `StdinRead` になり、`StoreUnavailable` には変換されない。
    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("boom"))
        }
    }

    #[test]
    fn read_token_from_reports_stdin_read_failures_without_converting_them() {
        let result = read_token_from(FailingReader, false);
        assert!(matches!(result, Err(CredentialError::StdinRead(_))));
    }

    // --- store / load / remove（`InMemoryStore` を注入） ---

    #[derive(Default)]
    struct InMemoryStore {
        entry: RefCell<Option<StoredCredential>>,
    }

    impl CredentialStore for InMemoryStore {
        fn set(&self, region: Region, token: &ApiToken) -> Result<(), StoreFailure> {
            *self.entry.borrow_mut() = Some(StoredCredential {
                region,
                token: ApiToken(SecretString::from(token.0.expose_secret().to_owned())),
            });
            Ok(())
        }

        fn get(&self) -> Result<Option<StoredCredential>, StoreFailure> {
            Ok(self
                .entry
                .borrow()
                .as_ref()
                .map(|credential| StoredCredential {
                    region: credential.region,
                    token: ApiToken(SecretString::from(
                        credential.token.0.expose_secret().to_owned(),
                    )),
                }))
        }

        fn delete(&self) -> Result<bool, StoreFailure> {
            Ok(self.entry.borrow_mut().take().is_some())
        }
    }

    /// ストアが常に `Err` を返す（到達できない状況を再現する）テストダブル。
    struct UnavailableStore;

    impl CredentialStore for UnavailableStore {
        fn set(&self, _region: Region, _token: &ApiToken) -> Result<(), StoreFailure> {
            Err(StoreFailure::Unavailable("boom".to_owned()))
        }

        fn get(&self) -> Result<Option<StoredCredential>, StoreFailure> {
            Err(StoreFailure::Unavailable("boom".to_owned()))
        }

        fn delete(&self) -> Result<bool, StoreFailure> {
            Err(StoreFailure::Unavailable("boom".to_owned()))
        }
    }

    #[test]
    fn store_then_load_round_trips_and_overwrites() {
        let credential_store = InMemoryStore::default();
        store(
            &credential_store,
            StoredCredential {
                region: Region::Global,
                token: parse_token("first").expect("should parse"),
            },
        )
        .expect("should store");
        assert_eq!(
            load(&credential_store).expect("should load").region,
            Region::Global
        );

        store(
            &credential_store,
            StoredCredential {
                region: Region::China,
                token: parse_token("second").expect("should parse"),
            },
        )
        .expect("should overwrite");
        assert_eq!(
            load(&credential_store).expect("should load").region,
            Region::China
        );
    }

    #[test]
    fn load_returns_not_found_when_unregistered() {
        let credential_store = InMemoryStore::default();
        assert!(matches!(
            load(&credential_store),
            Err(CredentialError::NotFound)
        ));
    }

    #[test]
    fn remove_is_idempotent() {
        let credential_store = InMemoryStore::default();
        store(
            &credential_store,
            StoredCredential {
                region: Region::Global,
                token: parse_token("token").expect("should parse"),
            },
        )
        .expect("should store");

        assert!(remove(&credential_store).expect("should remove"));
        assert!(!remove(&credential_store).expect("removing again should still succeed"));
    }

    /// D-4 の回帰テスト: ストアが `Err` を返したとき、`load` は絶対に `NotFound` を返さない。
    #[test]
    fn load_does_not_convert_store_errors_into_not_found() {
        let result = load(&UnavailableStore);
        assert!(matches!(result, Err(CredentialError::StoreUnavailable(_))));
        assert!(!matches!(result, Err(CredentialError::NotFound)));
    }
}
