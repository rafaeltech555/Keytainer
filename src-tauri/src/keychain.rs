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

/// Encode a raw vault key for storage in the OS secret store.
#[cfg(feature = "keychain")]
fn encode_key(key: &[u8; 32]) -> String {
    crate::backup::b64_encode(key)
}

/// Decode a key read back from the OS secret store, rejecting anything that
/// isn't exactly 32 bytes — a corrupted or tampered entry must not be handed
/// back as a vault key.
#[cfg(feature = "keychain")]
fn decode_key(encoded: &str) -> AppResult<[u8; 32]> {
    let bytes = crate::backup::b64_decode(encoded)?;
    if bytes.len() != 32 {
        return Err(AppError::KeychainUnavailable("malformed key".into()));
    }
    let mut k = [0u8; 32];
    k.copy_from_slice(&bytes);
    Ok(k)
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
        let encoded = encode_key(key);
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
        let encoded = entry()?
            .get_password()
            .map_err(|e| AppError::KeychainUnavailable(e.to_string()))?;
        decode_key(&encoded)
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

#[cfg(all(test, feature = "keychain"))]
mod tests {
    use super::*;

    #[test]
    fn encode_then_decode_round_trips_a_key() {
        let key = [7u8; 32];
        let restored = decode_key(&encode_key(&key)).unwrap();
        assert_eq!(restored, key);
    }

    #[test]
    fn decode_rejects_a_too_short_key() {
        // base64 of only 16 bytes — half a key.
        let short = crate::backup::b64_encode(&[1u8; 16]);
        assert!(matches!(
            decode_key(&short),
            Err(AppError::KeychainUnavailable(_))
        ));
    }

    #[test]
    fn decode_rejects_a_too_long_key() {
        let long = crate::backup::b64_encode(&[1u8; 33]);
        assert!(matches!(
            decode_key(&long),
            Err(AppError::KeychainUnavailable(_))
        ));
    }

    #[test]
    fn decode_rejects_non_base64_garbage() {
        // Not valid base64 at all — must surface an error, never a key.
        assert!(decode_key("!!! not base64 !!!").is_err());
    }
}
