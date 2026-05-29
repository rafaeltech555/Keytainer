use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::crypto::{self, KdfParams, NONCE_LEN, SALT_LEN};
use crate::error::{AppError, AppResult};
use crate::vault::Vault;

const FORMAT_TAG: &str = "keytainer-backup-v1";

/// Portable, self-describing backup envelope. Encoded as pretty JSON so
/// users can verify what they're looking at even before importing.
#[derive(Debug, Serialize, Deserialize)]
pub struct Backup {
    pub format: String,
    pub kdf: BackupKdf,
    pub salt_b64: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupKdf {
    pub algorithm: String, // always "argon2id" for now
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

pub fn b64_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    // Tiny base64 (RFC 4648 standard, with padding) without pulling another
    // crate. Good enough for ~few-KB vault payloads.
    const ALPHA: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let chunks = bytes.chunks(3);
    for chunk in chunks {
        let (b0, b1, b2) = match chunk.len() {
            3 => (chunk[0], chunk[1], chunk[2]),
            2 => (chunk[0], chunk[1], 0),
            1 => (chunk[0], 0, 0),
            _ => unreachable!(),
        };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        let _ = write!(out, "{}", ALPHA[((n >> 18) & 0x3f) as usize] as char);
        let _ = write!(out, "{}", ALPHA[((n >> 12) & 0x3f) as usize] as char);
        if chunk.len() >= 2 {
            let _ = write!(out, "{}", ALPHA[((n >> 6) & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() == 3 {
            let _ = write!(out, "{}", ALPHA[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn b64_decode(s: &str) -> AppResult<Vec<u8>> {
    let s = s.trim();
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' | b' ' | b'\t' => continue,
            _ => return Err(AppError::VaultCorrupt),
        };
        buf = (buf << 6) | (v as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8 & 0xff);
        }
    }
    Ok(out)
}

pub fn export_to_file(path: &Path, vault: &Vault, password: &str) -> AppResult<()> {
    let kdf = KdfParams::default();
    let salt = crypto::kdf::random_salt();
    let mut key = crypto::derive_key(password, &salt, kdf)?;
    let nonce = crypto::aead::random_nonce();
    let mut plaintext = serde_json::to_vec(vault)?;
    let ciphertext = crypto::encrypt(&key, &nonce, &plaintext)?;
    plaintext.zeroize();
    key.zeroize();

    let backup = Backup {
        format: FORMAT_TAG.into(),
        kdf: BackupKdf {
            algorithm: "argon2id".into(),
            m_cost_kib: kdf.m_cost_kib,
            t_cost: kdf.t_cost,
            p_cost: kdf.p_cost,
        },
        salt_b64: b64_encode(&salt),
        nonce_b64: b64_encode(&nonce),
        ciphertext_b64: b64_encode(&ciphertext),
    };
    let json = serde_json::to_vec_pretty(&backup)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, json)?;
    Ok(())
}

pub fn import_from_file(path: &Path, password: &str) -> AppResult<Vault> {
    let bytes = fs::read(path)?;
    let backup: Backup = serde_json::from_slice(&bytes).map_err(|_| AppError::VaultCorrupt)?;
    if backup.format != FORMAT_TAG {
        return Err(AppError::VaultCorrupt);
    }
    if backup.kdf.algorithm != "argon2id" {
        return Err(AppError::VaultCorrupt);
    }

    let salt_vec = b64_decode(&backup.salt_b64)?;
    let nonce_vec = b64_decode(&backup.nonce_b64)?;
    let ct = b64_decode(&backup.ciphertext_b64)?;
    if salt_vec.len() != SALT_LEN || nonce_vec.len() != NONCE_LEN {
        return Err(AppError::VaultCorrupt);
    }

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&salt_vec);
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&nonce_vec);

    let kdf = KdfParams {
        m_cost_kib: backup.kdf.m_cost_kib,
        t_cost: backup.kdf.t_cost,
        p_cost: backup.kdf.p_cost,
    };
    let mut key = crypto::derive_key(password, &salt, kdf)?;
    let mut plaintext = crypto::decrypt(&key, &nonce, &ct)?;
    let vault: Result<Vault, _> = serde_json::from_slice(&plaintext);
    plaintext.zeroize();
    key.zeroize();
    vault.map_err(|_| AppError::VaultCorrupt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::{crud, VaultItem};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn sample_vault() -> Vault {
        let mut v = Vault::default();
        crud::add_item(
            &mut v,
            VaultItem {
                id: Uuid::nil(),
                site_name: "Email".into(),
                username: "me".into(),
                password: "pw".into(),
                totp: None,
                url: None,
                notes: None,
                tags: vec!["personal".into()],
                created_at: 0,
                updated_at: 0,
            },
        );
        v
    }

    #[test]
    fn b64_round_trip() {
        for input in [b"".to_vec(), b"f".to_vec(), b"fo".to_vec(), b"foo".to_vec(),
                       b"foob".to_vec(), b"fooba".to_vec(), b"foobar".to_vec(),
                       vec![0u8, 255, 1, 254, 128]] {
            let encoded = b64_encode(&input);
            let back = b64_decode(&encoded).unwrap();
            assert_eq!(back, input, "round-trip failed for {input:?}");
        }
    }

    #[test]
    fn export_import_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("backup.json");
        let v = sample_vault();
        export_to_file(&path, &v, "backup-pw").unwrap();
        let restored = import_from_file(&path, "backup-pw").unwrap();
        assert_eq!(v, restored);
    }

    #[test]
    fn wrong_password_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("backup.json");
        export_to_file(&path, &sample_vault(), "right-pw").unwrap();
        let err = import_from_file(&path, "wrong-pw").unwrap_err();
        assert!(matches!(err, AppError::WrongPassword), "got {err:?}");
    }

    #[test]
    fn corrupt_file_fails_clearly() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("backup.json");
        std::fs::write(&path, b"not json").unwrap();
        let err = import_from_file(&path, "pw").unwrap_err();
        assert!(matches!(err, AppError::VaultCorrupt));
    }
}
