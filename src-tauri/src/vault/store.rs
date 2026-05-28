use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::crypto::{self, KdfParams, KEY_LEN, NONCE_LEN, SALT_LEN};
use crate::error::{AppError, AppResult};
use crate::vault::Vault;

pub const MAGIC: &[u8; 4] = b"KTNR";
pub const FORMAT_VERSION: u16 = 1;

/// On-disk header (everything before the ciphertext bytes themselves).
/// Little-endian throughout.
///
///   "KTNR" (4) | version u16 (2)
/// | m_cost u32 (4) | t_cost u32 (4) | p_cost u32 (4)
/// | salt[16] | nonce[12] | ct_len u32 (4)
const HEADER_LEN: usize = 4 + 2 + 4 + 4 + 4 + SALT_LEN + NONCE_LEN + 4;

/// Encrypted file produced by [`save`] / consumed by [`load`].
pub struct EncryptedFile {
    pub kdf: KdfParams,
    pub salt: [u8; SALT_LEN],
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext: Vec<u8>,
}

pub fn encode(file: &EncryptedFile) -> Vec<u8> {
    let ct_len = file.ciphertext.len() as u32;
    let mut buf = Vec::with_capacity(HEADER_LEN + file.ciphertext.len());
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    buf.extend_from_slice(&file.kdf.m_cost_kib.to_le_bytes());
    buf.extend_from_slice(&file.kdf.t_cost.to_le_bytes());
    buf.extend_from_slice(&file.kdf.p_cost.to_le_bytes());
    buf.extend_from_slice(&file.salt);
    buf.extend_from_slice(&file.nonce);
    buf.extend_from_slice(&ct_len.to_le_bytes());
    buf.extend_from_slice(&file.ciphertext);
    buf
}

pub fn decode(bytes: &[u8]) -> AppResult<EncryptedFile> {
    if bytes.len() < HEADER_LEN {
        return Err(AppError::VaultCorrupt);
    }
    if &bytes[0..4] != MAGIC {
        return Err(AppError::VaultCorrupt);
    }
    let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    if version != FORMAT_VERSION {
        return Err(AppError::VaultCorrupt);
    }
    let m_cost_kib = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
    let t_cost = u32::from_le_bytes(bytes[10..14].try_into().unwrap());
    let p_cost = u32::from_le_bytes(bytes[14..18].try_into().unwrap());

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&bytes[18..18 + SALT_LEN]);

    let nonce_start = 18 + SALT_LEN;
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&bytes[nonce_start..nonce_start + NONCE_LEN]);

    let len_start = nonce_start + NONCE_LEN;
    let ct_len = u32::from_le_bytes(bytes[len_start..len_start + 4].try_into().unwrap()) as usize;
    let ct_start = len_start + 4;
    if bytes.len() != ct_start + ct_len {
        return Err(AppError::VaultCorrupt);
    }
    let ciphertext = bytes[ct_start..].to_vec();

    Ok(EncryptedFile {
        kdf: KdfParams {
            m_cost_kib,
            t_cost,
            p_cost,
        },
        salt,
        nonce,
        ciphertext,
    })
}

/// Encrypt and atomically write a vault to `path`. Uses tmp + rename
/// so a crash mid-write never destroys the previous good file.
pub fn save(path: &Path, vault: &Vault, key: &[u8; KEY_LEN], kdf: KdfParams, salt: [u8; SALT_LEN]) -> AppResult<()> {
    let plaintext = serde_json::to_vec(vault)?;
    let nonce = crypto::aead::random_nonce();
    let ciphertext = crypto::encrypt(key, &nonce, &plaintext)?;

    let encoded = encode(&EncryptedFile {
        kdf,
        salt,
        nonce,
        ciphertext,
    });

    write_atomic(path, &encoded)
}

/// Load and decrypt the vault at `path`, deriving the key from `password`.
/// On wrong password returns [`AppError::WrongPassword`]; on malformed file
/// returns [`AppError::VaultCorrupt`].
pub fn load(path: &Path, password: &str) -> AppResult<(Vault, [u8; KEY_LEN], KdfParams, [u8; SALT_LEN])> {
    let bytes = fs::read(path)?;
    let file = decode(&bytes)?;
    let key = crypto::derive_key(password, &file.salt, file.kdf)?;
    let plaintext = crypto::decrypt(&key, &file.nonce, &file.ciphertext)?;
    let vault: Vault = serde_json::from_slice(&plaintext).map_err(|_| AppError::VaultCorrupt)?;
    Ok((vault, key, file.kdf, file.salt))
}

fn write_atomic(target: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp: PathBuf = {
        let mut name = target.file_name().unwrap_or_default().to_os_string();
        name.push(".tmp");
        target.with_file_name(name)
    };
    {
        let mut f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kdf;
    use crate::vault::{crud, VaultItem};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn fast_kdf() -> KdfParams {
        KdfParams {
            m_cost_kib: 8,
            t_cost: 1,
            p_cost: 1,
        }
    }

    fn sample_item() -> VaultItem {
        VaultItem {
            id: Uuid::nil(),
            site_name: "GitHub".into(),
            username: "alice@example.com".into(),
            password: "S3cret!".into(),
            totp: None,
            url: Some("https://github.com".into()),
            notes: Some("work account".into()),
            tags: vec!["work".into()],
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");

        let salt = kdf::random_salt();
        let key = kdf::derive_key("master-pw", &salt, fast_kdf()).unwrap();

        let mut vault = Vault::default();
        crud::add_item(&mut vault, sample_item());

        save(&path, &vault, &key, fast_kdf(), salt).unwrap();
        let (loaded, _key, kdf, salt_back) = load(&path, "master-pw").unwrap();

        assert_eq!(vault, loaded);
        assert_eq!(kdf, fast_kdf());
        assert_eq!(salt_back, salt);
    }

    #[test]
    fn wrong_password_returns_wrong_password() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");

        let salt = kdf::random_salt();
        let key = kdf::derive_key("master-pw", &salt, fast_kdf()).unwrap();
        save(&path, &Vault::default(), &key, fast_kdf(), salt).unwrap();

        let err = load(&path, "WRONG-pw").unwrap_err();
        assert!(matches!(err, AppError::WrongPassword), "got {err:?}");
    }

    #[test]
    fn flipped_byte_anywhere_in_header_or_ct_is_detected() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");

        let salt = kdf::random_salt();
        let key = kdf::derive_key("pw", &salt, fast_kdf()).unwrap();
        save(&path, &Vault::default(), &key, fast_kdf(), salt).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        // Flip a byte in the ciphertext region.
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        fs::write(&path, &bytes).unwrap();

        let err = load(&path, "pw").unwrap_err();
        // Tampered ciphertext fails GCM auth -> reported as WrongPassword
        // (we can't distinguish from a bad password without leaking timing info).
        assert!(matches!(err, AppError::WrongPassword), "got {err:?}");
    }

    #[test]
    fn bad_magic_is_vault_corrupt() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        fs::write(&path, b"NOPE_______________________________________________________").unwrap();
        let err = load(&path, "pw").unwrap_err();
        assert!(matches!(err, AppError::VaultCorrupt));
    }

    #[test]
    fn too_short_is_vault_corrupt() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        fs::write(&path, b"KTNR").unwrap();
        let err = load(&path, "pw").unwrap_err();
        assert!(matches!(err, AppError::VaultCorrupt));
    }

    #[test]
    fn atomic_write_leaves_no_tmp_on_success() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        let salt = kdf::random_salt();
        let key = kdf::derive_key("pw", &salt, fast_kdf()).unwrap();
        save(&path, &Vault::default(), &key, fast_kdf(), salt).unwrap();

        let tmp = dir.path().join("vault.dat.tmp");
        assert!(path.exists());
        assert!(!tmp.exists());
    }
}
