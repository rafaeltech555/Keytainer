//! Authenticated encryption for the vault.
//!
//! v2 (current) uses **XChaCha20-Poly1305**: a 192-bit random nonce makes
//! accidental nonce reuse under a fixed session key a non-issue (vs. the
//! 96-bit birthday bound of AES-GCM), and the file/backup header is bound
//! as associated data (AAD) so KDF params, version, salt and nonce are
//! authenticated — not just the ciphertext.
//!
//! v1 vaults were written with AES-256-GCM (96-bit nonce, no AAD). We keep
//! a *decrypt-only* AES path so existing vaults still open and can be
//! transparently migrated to v2 on the next save.

// aes_gcm and chacha20poly1305 depend on different versions of the `aead`
// crate, so each cipher needs the traits from its own crate (imported
// anonymously to avoid name clashes).
use aes_gcm::aead::Aead as _;
use aes_gcm::{Aes256Gcm, KeyInit as _, Nonce};
use chacha20poly1305::aead::{Aead as _, KeyInit as _, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;

use crate::error::{AppError, AppResult};

/// XChaCha20-Poly1305 nonce length (192-bit). Used by v2.
pub const XNONCE_LEN: usize = 24;
/// AES-GCM nonce length (96-bit). Legacy, v1 read-only.
pub const GCM_NONCE_LEN: usize = 12;

pub fn random_xnonce() -> [u8; XNONCE_LEN] {
    let mut n = [0u8; XNONCE_LEN];
    rand::thread_rng().fill_bytes(&mut n);
    n
}

/// XChaCha20-Poly1305 encrypt with associated data. `aad` is authenticated
/// but not encrypted; pass the file/backup header so it can't be tampered.
pub fn xchacha_encrypt(
    key: &[u8; 32],
    nonce: &[u8; XNONCE_LEN],
    aad: &[u8],
    plaintext: &[u8],
) -> AppResult<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| AppError::Crypto(format!("invalid key: {e}")))?;
    cipher
        .encrypt(XNonce::from_slice(nonce), Payload { msg: plaintext, aad })
        .map_err(|_| AppError::Crypto("encrypt failed".into()))
}

/// XChaCha20-Poly1305 decrypt with associated data. A wrong key, tampered
/// ciphertext, or tampered AAD all surface as [`AppError::WrongPassword`]
/// (we don't distinguish, to avoid leaking which check failed).
pub fn xchacha_decrypt(
    key: &[u8; 32],
    nonce: &[u8],
    aad: &[u8],
    ciphertext: &[u8],
) -> AppResult<Vec<u8>> {
    if nonce.len() != XNONCE_LEN {
        return Err(AppError::VaultCorrupt);
    }
    let cipher = XChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| AppError::Crypto(format!("invalid key: {e}")))?;
    cipher
        .decrypt(XNonce::from_slice(nonce), Payload { msg: ciphertext, aad })
        .map_err(|_| AppError::WrongPassword)
}

/// Legacy AES-256-GCM decrypt (v1 vaults / v1 backups). No AAD.
pub fn aes_gcm_decrypt(key: &[u8; 32], nonce: &[u8], ciphertext: &[u8]) -> AppResult<Vec<u8>> {
    if nonce.len() != GCM_NONCE_LEN {
        return Err(AppError::VaultCorrupt);
    }
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::Crypto(format!("invalid key: {e}")))?;
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| AppError::WrongPassword)
}

/// Legacy AES-256-GCM encrypt. Only used by tests to build v1 fixtures for
/// the migration path; production code always writes v2 (XChaCha).
#[allow(dead_code)]
pub fn aes_gcm_encrypt(key: &[u8; 32], nonce: &[u8; GCM_NONCE_LEN], plaintext: &[u8]) -> AppResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::Crypto(format!("invalid key: {e}")))?;
    cipher
        .encrypt(Nonce::from_slice(nonce), plaintext)
        .map_err(|_| AppError::Crypto("encrypt failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xchacha_round_trip_with_aad() {
        let key = [0x42u8; 32];
        let nonce = random_xnonce();
        let aad = b"header-bytes";
        let pt = b"network protocols are spooky";
        let ct = xchacha_encrypt(&key, &nonce, aad, pt).unwrap();
        let back = xchacha_decrypt(&key, &nonce, aad, &ct).unwrap();
        assert_eq!(back, pt);
    }

    #[test]
    fn xchacha_wrong_key_fails_as_wrong_password() {
        let nonce = random_xnonce();
        let ct = xchacha_encrypt(&[1u8; 32], &nonce, b"", b"secret").unwrap();
        let err = xchacha_decrypt(&[2u8; 32], &nonce, b"", &ct).unwrap_err();
        assert!(matches!(err, AppError::WrongPassword));
    }

    #[test]
    fn xchacha_tampered_ciphertext_fails() {
        let key = [9u8; 32];
        let nonce = random_xnonce();
        let mut ct = xchacha_encrypt(&key, &nonce, b"hdr", b"don't touch me").unwrap();
        ct[0] ^= 0xff;
        let err = xchacha_decrypt(&key, &nonce, b"hdr", &ct).unwrap_err();
        assert!(matches!(err, AppError::WrongPassword));
    }

    #[test]
    fn xchacha_tampered_aad_fails() {
        // Flipping the associated data (e.g. downgrading KDF params in the
        // header) must break authentication.
        let key = [9u8; 32];
        let nonce = random_xnonce();
        let ct = xchacha_encrypt(&key, &nonce, b"original-header", b"payload").unwrap();
        let err = xchacha_decrypt(&key, &nonce, b"tampered-header", &ct).unwrap_err();
        assert!(matches!(err, AppError::WrongPassword));
    }

    #[test]
    fn xnonce_is_random_each_call() {
        assert_ne!(random_xnonce(), random_xnonce());
    }

    #[test]
    fn aes_gcm_legacy_round_trip() {
        let key = [7u8; 32];
        let nonce = [3u8; GCM_NONCE_LEN];
        let ct = aes_gcm_encrypt(&key, &nonce, b"legacy v1 data").unwrap();
        let back = aes_gcm_decrypt(&key, &nonce, &ct).unwrap();
        assert_eq!(back, b"legacy v1 data");
    }

    #[test]
    fn wrong_nonce_len_is_corrupt() {
        let key = [0u8; 32];
        assert!(matches!(
            xchacha_decrypt(&key, &[0u8; 12], b"", b"x"),
            Err(AppError::VaultCorrupt)
        ));
        assert!(matches!(
            aes_gcm_decrypt(&key, &[0u8; 24], b"x"),
            Err(AppError::VaultCorrupt)
        ));
    }
}
