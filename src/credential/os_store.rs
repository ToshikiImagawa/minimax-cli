//! `keyring` を使った [`CredentialStore`] の実装。
//!
//! OS の資格情報ストアへの実際のアクセスをこの 1 モジュールに閉じ込める（A-001）。
//! リージョンとトークンは、同じ `service` の下にある別々の `keyring::Entry`
//! （`user` を `token` / `region` で分ける）に保存する。

use secrecy::{ExposeSecret, SecretString};

use super::{ApiToken, CredentialStore, Region, StoreFailure, StoredCredential};

const SERVICE: &str = "minimax-cli";
const TOKEN_USER: &str = "token";
const REGION_USER: &str = "region";

/// OS のセキュアな資格情報ストアを使った [`CredentialStore`] の実装。
pub struct OsStore;

impl CredentialStore for OsStore {
    fn set(&self, region: Region, token: &ApiToken) -> Result<(), StoreFailure> {
        entry(TOKEN_USER)?
            .set_password(token.0.expose_secret())
            .map_err(map_error)?;
        entry(REGION_USER)?
            .set_password(region.as_str())
            .map_err(map_error)
    }

    fn get(&self) -> Result<Option<StoredCredential>, StoreFailure> {
        let raw_token = match entry(TOKEN_USER)?.get_password() {
            Ok(value) => value,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(error) => return Err(map_error(error)),
        };
        let raw_region = entry(REGION_USER)?.get_password().map_err(map_error)?;
        Ok(Some(StoredCredential {
            region: parse_region(&raw_region)?,
            token: ApiToken(SecretString::from(raw_token)),
        }))
    }

    fn delete(&self) -> Result<bool, StoreFailure> {
        let deleted = match entry(TOKEN_USER)?.delete_credential() {
            Ok(()) => true,
            Err(keyring::Error::NoEntry) => false,
            Err(error) => return Err(map_error(error)),
        };
        match entry(REGION_USER)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(deleted),
            Err(error) => Err(map_error(error)),
        }
    }
}

fn entry(user: &str) -> Result<keyring::Entry, StoreFailure> {
    keyring::Entry::new(SERVICE, user).map_err(map_error)
}

fn parse_region(raw: &str) -> Result<Region, StoreFailure> {
    match raw {
        "global" => Ok(Region::Global),
        "china" => Ok(Region::China),
        other => Err(StoreFailure::Other(format!(
            "the credential store returned an unknown region: {other}"
        ))),
    }
}

/// `keyring::Error` を [`StoreFailure`] へ落とす（design 6.1 節の対応表）。
///
/// `NoEntry` はここに来る前に呼び出し側で処理済み（`get` は `Ok(None)`、`delete` は `Ok(false)`）。
fn map_error(error: keyring::Error) -> StoreFailure {
    match error {
        keyring::Error::NoStorageAccess(inner) => StoreFailure::PermissionDenied(inner.to_string()),
        keyring::Error::PlatformFailure(inner) => StoreFailure::Unavailable(inner.to_string()),
        other => StoreFailure::Other(other.to_string()),
    }
}
