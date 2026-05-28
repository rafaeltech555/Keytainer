use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use rand::RngCore;

use crate::error::{AppError, AppResult};

pub const NONCE_LEN: usize = 12;

pub fn random_nonce() -> [u8; NONCE_LEN] {
    let mut n = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut n);
    n
}

pub fn encrypt(key: &[u8; 32], nonce: &[u8; NONCE_LEN], plaintext: &[u8]) -> AppResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::Crypto(format!("invalid key: {e}")))?;
    let n = Nonce::from_slice(nonce);
    cipher
        .encrypt(n, plaintext)
        .map_err(|_| AppError::Crypto("encrypt failed".into()))
}

pub fn decrypt(key: &[u8; 32], nonce: &[u8; NONCE_LEN], ciphertext: &[u8]) -> AppResult<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| AppError::Crypto(format!("invalid key: {e}")))?;
    let n = Nonce::from_slice(nonce);
    cipher
        .decrypt(n, ciphertext)
        .map_err(|_| AppError::WrongPassword)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let key = [0x42u8; 32];
        let nonce = random_nonce();
        let pt = b"network protocols are spooky";
        let ct = encrypt(&key, &nonce, pt).unwrap();
        let back = decrypt(&key, &nonce, &ct).unwrap();
        assert_eq!(back, pt);
    }

    #[test]
    fn wrong_key_fails_as_wrong_password() {
        let nonce = random_nonce();
        let ct = encrypt(&[1u8; 32], &nonce, b"secret").unwrap();
        let err = decrypt(&[2u8; 32], &nonce, &ct).unwrap_err();
        assert!(matches!(err, AppError::WrongPassword));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = [9u8; 32];
        let nonce = random_nonce();
        let mut ct = encrypt(&key, &nonce, b"don't touch me").unwrap();
        ct[0] ^= 0xff;
        let err = decrypt(&key, &nonce, &ct).unwrap_err();
        assert!(matches!(err, AppError::WrongPassword));
    }

    #[test]
    fn nonce_is_random_each_call() {
        let n1 = random_nonce();
        let n2 = random_nonce();
        assert_ne!(n1, n2);
    }
}
