//! Optional OS keychain fast-unlock: stash the 32-byte derived vault key
//! in the platform's secret store (Secret Service on Linux, Keychain on
//! macOS, Credential Manager on Windows) so the user can unlock without
//! re-running Argon2id.
//!
//! All functions are no-ops returning a clear error when the `keychain`
//! feature is disabled.

use crate::error::{AppError, AppResult};

const SERVICE: &str = "keytainer";
const ACCOUNT: &str = "vault-key";

#[cfg(feature = "keychain")]
fn entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT)
        .map_err(|e| AppError::KeychainUnavailable(e.to_string()))
}

pub fn is_supported() -> bool {
    #[cfg(feature = "keychain")]
    {
        entry().is_ok()
    }
    #[cfg(not(feature = "keychain"))]
    {
        false
    }
}

pub fn store_key(key: &[u8; 32]) -> AppResult<()> {
    #[cfg(feature = "keychain")]
    {
        use crate::backup;
        let encoded = backup::b64_encode(key);
        entry()?
            .set_password(&encoded)
            .map_err(|e| AppError::KeychainUnavailable(e.to_string()))
    }
    #[cfg(not(feature = "keychain"))]
    {
        let _ = key;
        Err(AppError::KeychainUnavailable(
            "keychain feature disabled".into(),
        ))
    }
}

pub fn load_key() -> AppResult<[u8; 32]> {
    #[cfg(feature = "keychain")]
    {
        use crate::backup;
        let encoded = entry()?
            .get_password()
            .map_err(|e| AppError::KeychainUnavailable(e.to_string()))?;
        let bytes = backup::b64_decode(&encoded)?;
        if bytes.len() != 32 {
            return Err(AppError::KeychainUnavailable("malformed key".into()));
        }
        let mut k = [0u8; 32];
        k.copy_from_slice(&bytes);
        Ok(k)
    }
    #[cfg(not(feature = "keychain"))]
    {
        Err(AppError::KeychainUnavailable(
            "keychain feature disabled".into(),
        ))
    }
}

pub fn delete_key() -> AppResult<()> {
    #[cfg(feature = "keychain")]
    {
        match entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // already gone — that's fine
            Err(e) => Err(AppError::KeychainUnavailable(e.to_string())),
        }
    }
    #[cfg(not(feature = "keychain"))]
    {
        Ok(())
    }
}
