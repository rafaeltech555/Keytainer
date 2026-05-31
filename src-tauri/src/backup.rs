use std::fs;
use std::path::Path;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::crypto::{self, KdfParams, GCM_NONCE_LEN, SALT_LEN, XNONCE_LEN};
use crate::error::{AppError, AppResult};
use crate::vault::Vault;

/// v2 uses XChaCha20-Poly1305 with the envelope metadata bound as AAD.
/// v1 (AES-256-GCM, no AAD) is still importable so old backups keep working.
const FORMAT_TAG_V2: &str = "keytainer-backup-v2";
const FORMAT_TAG_V1: &str = "keytainer-backup-v1";

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
    pub algorithm: String, // KDF: always "argon2id"
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

pub fn b64_encode(bytes: &[u8]) -> String {
    B64.encode(bytes)
}

pub fn b64_decode(s: &str) -> AppResult<Vec<u8>> {
    B64.decode(s.trim()).map_err(|_| AppError::VaultCorrupt)
}

/// Associated data binding the v2 envelope metadata to the ciphertext, so an
/// attacker can't swap KDF params / salt / nonce without breaking auth.
fn backup_aad_v2(kdf: &BackupKdf, salt: &[u8], nonce: &[u8]) -> Vec<u8> {
    let mut aad = Vec::new();
    aad.extend_from_slice(FORMAT_TAG_V2.as_bytes());
    aad.extend_from_slice(kdf.algorithm.as_bytes());
    aad.extend_from_slice(&kdf.m_cost_kib.to_le_bytes());
    aad.extend_from_slice(&kdf.t_cost.to_le_bytes());
    aad.extend_from_slice(&kdf.p_cost.to_le_bytes());
    aad.extend_from_slice(salt);
    aad.extend_from_slice(nonce);
    aad
}

pub fn export_to_file(path: &Path, vault: &Vault, password: &str) -> AppResult<()> {
    let kdf = KdfParams::default();
    let salt = crypto::kdf::random_salt();
    let nonce = crypto::aead::random_xnonce();
    let mut key = crypto::derive_key(password, &salt, kdf)?;

    let backup_kdf = BackupKdf {
        algorithm: "argon2id".into(),
        m_cost_kib: kdf.m_cost_kib,
        t_cost: kdf.t_cost,
        p_cost: kdf.p_cost,
    };
    let aad = backup_aad_v2(&backup_kdf, &salt, &nonce);

    let mut plaintext = serde_json::to_vec(vault)?;
    let ciphertext = crypto::aead::xchacha_encrypt(&key, &nonce, &aad, &plaintext)?;
    plaintext.zeroize();
    key.zeroize();

    let backup = Backup {
        format: FORMAT_TAG_V2.into(),
        kdf: backup_kdf,
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
    let is_v2 = match backup.format.as_str() {
        FORMAT_TAG_V2 => true,
        FORMAT_TAG_V1 => false,
        _ => return Err(AppError::VaultCorrupt),
    };
    if backup.kdf.algorithm != "argon2id" {
        return Err(AppError::VaultCorrupt);
    }

    let salt_vec = b64_decode(&backup.salt_b64)?;
    let nonce_vec = b64_decode(&backup.nonce_b64)?;
    let ct = b64_decode(&backup.ciphertext_b64)?;
    let expected_nonce = if is_v2 { XNONCE_LEN } else { GCM_NONCE_LEN };
    if salt_vec.len() != SALT_LEN || nonce_vec.len() != expected_nonce {
        return Err(AppError::VaultCorrupt);
    }

    let kdf = KdfParams {
        m_cost_kib: backup.kdf.m_cost_kib,
        t_cost: backup.kdf.t_cost,
        p_cost: backup.kdf.p_cost,
    };
    let mut key = crypto::derive_key(password, &salt_vec, kdf)?;
    let mut plaintext = if is_v2 {
        let aad = backup_aad_v2(&backup.kdf, &salt_vec, &nonce_vec);
        crypto::aead::xchacha_decrypt(&key, &nonce_vec, &aad, &ct)?
    } else {
        crypto::aead::aes_gcm_decrypt(&key, &nonce_vec, &ct)?
    };
    key.zeroize();
    let vault: Result<Vault, _> = serde_json::from_slice(&plaintext);
    plaintext.zeroize();
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
        for input in [
            b"".to_vec(), b"f".to_vec(), b"fo".to_vec(), b"foo".to_vec(),
            b"foob".to_vec(), b"fooba".to_vec(), b"foobar".to_vec(),
            vec![0u8, 255, 1, 254, 128],
        ] {
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
    fn exports_v2_format() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("backup.json");
        export_to_file(&path, &sample_vault(), "pw").unwrap();
        let backup: Backup = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(backup.format, FORMAT_TAG_V2);
        assert_eq!(b64_decode(&backup.nonce_b64).unwrap().len(), XNONCE_LEN);
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
    fn tampered_kdf_params_fail_auth() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("backup.json");
        export_to_file(&path, &sample_vault(), "pw").unwrap();

        // Tamper with the m_cost in the JSON envelope (it's AAD in v2).
        let mut backup: Backup = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        backup.kdf.m_cost_kib = backup.kdf.m_cost_kib.wrapping_add(1);
        fs::write(&path, serde_json::to_vec(&backup).unwrap()).unwrap();

        let err = import_from_file(&path, "pw").unwrap_err();
        assert!(matches!(err, AppError::WrongPassword), "got {err:?}");
    }

    #[test]
    fn imports_legacy_v1_backup() {
        // Build a v1 (AES-GCM, no AAD) envelope by hand.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("backup.json");
        let v = sample_vault();
        let kdf = KdfParams::default();
        let salt = crypto::kdf::random_salt();
        let nonce = [0x22u8; GCM_NONCE_LEN];
        let key = crypto::derive_key("v1-pw", &salt, kdf).unwrap();
        let ct = crypto::aead::aes_gcm_encrypt(&key, &nonce, &serde_json::to_vec(&v).unwrap()).unwrap();
        let backup = Backup {
            format: FORMAT_TAG_V1.into(),
            kdf: BackupKdf { algorithm: "argon2id".into(), m_cost_kib: kdf.m_cost_kib, t_cost: kdf.t_cost, p_cost: kdf.p_cost },
            salt_b64: b64_encode(&salt),
            nonce_b64: b64_encode(&nonce),
            ciphertext_b64: b64_encode(&ct),
        };
        fs::write(&path, serde_json::to_vec_pretty(&backup).unwrap()).unwrap();

        let restored = import_from_file(&path, "v1-pw").unwrap();
        assert_eq!(restored, v);
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
