use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use zeroize::Zeroize;

use crate::crypto::{self, KdfParams, GCM_NONCE_LEN, KEY_LEN, SALT_LEN, XNONCE_LEN};
use crate::error::{AppError, AppResult};
use crate::vault::Vault;

pub const MAGIC: &[u8; 4] = b"KTNR";

/// Current on-disk format. v2 = XChaCha20-Poly1305 (24-byte nonce) with the
/// header bound as AAD. v1 = legacy AES-256-GCM (12-byte nonce, no AAD),
/// still readable and auto-upgraded to v2 on the next save.
pub const FORMAT_VERSION: u16 = 2;
pub const FORMAT_VERSION_V1: u16 = 1;

/// Byte offset where the variable-length nonce begins.
///   "KTNR"(4) | version u16(2) | m_cost u32(4) | t_cost u32(4) | p_cost u32(4) | salt[16]
const NONCE_OFFSET: usize = 4 + 2 + 4 + 4 + 4 + SALT_LEN;

fn nonce_len_for(version: u16) -> AppResult<usize> {
    match version {
        FORMAT_VERSION => Ok(XNONCE_LEN),
        FORMAT_VERSION_V1 => Ok(GCM_NONCE_LEN),
        _ => Err(AppError::VaultCorrupt),
    }
}

/// A decoded encrypted file. `header` holds the exact bytes preceding the
/// ciphertext — for v2 these are passed back as AAD on decrypt.
pub struct EncryptedFile {
    pub version: u16,
    pub kdf: KdfParams,
    pub salt: [u8; SALT_LEN],
    pub nonce: Vec<u8>,
    pub header: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Build the header bytes (everything before the ciphertext) for a v2 file.
fn encode_header_v2(kdf: &KdfParams, salt: &[u8; SALT_LEN], nonce: &[u8; XNONCE_LEN], ct_len: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(NONCE_OFFSET + XNONCE_LEN + 4);
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    buf.extend_from_slice(&kdf.m_cost_kib.to_le_bytes());
    buf.extend_from_slice(&kdf.t_cost.to_le_bytes());
    buf.extend_from_slice(&kdf.p_cost.to_le_bytes());
    buf.extend_from_slice(salt);
    buf.extend_from_slice(nonce);
    buf.extend_from_slice(&ct_len.to_le_bytes());
    buf
}

pub fn decode(bytes: &[u8]) -> AppResult<EncryptedFile> {
    // Need at least magic + version to even read the version.
    if bytes.len() < 6 || &bytes[0..4] != MAGIC {
        return Err(AppError::VaultCorrupt);
    }
    let version = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    let nonce_len = nonce_len_for(version)?;

    let len_offset = NONCE_OFFSET + nonce_len;
    let ct_start = len_offset + 4;
    if bytes.len() < ct_start {
        return Err(AppError::VaultCorrupt);
    }

    let m_cost_kib = u32::from_le_bytes(bytes[6..10].try_into().unwrap());
    let t_cost = u32::from_le_bytes(bytes[10..14].try_into().unwrap());
    let p_cost = u32::from_le_bytes(bytes[14..18].try_into().unwrap());

    let mut salt = [0u8; SALT_LEN];
    salt.copy_from_slice(&bytes[18..18 + SALT_LEN]);

    let nonce = bytes[NONCE_OFFSET..NONCE_OFFSET + nonce_len].to_vec();

    let ct_len = u32::from_le_bytes(bytes[len_offset..len_offset + 4].try_into().unwrap()) as usize;
    if bytes.len() != ct_start + ct_len {
        return Err(AppError::VaultCorrupt);
    }

    Ok(EncryptedFile {
        version,
        kdf: KdfParams { m_cost_kib, t_cost, p_cost },
        salt,
        nonce,
        header: bytes[0..ct_start].to_vec(),
        ciphertext: bytes[ct_start..].to_vec(),
    })
}

/// Decrypt an already-decoded file with a derived key, dispatching on the
/// format version (v2 = XChaCha+AAD, v1 = AES-GCM legacy).
fn decrypt_file(file: &EncryptedFile, key: &[u8; KEY_LEN]) -> AppResult<Vec<u8>> {
    match file.version {
        FORMAT_VERSION => crypto::aead::xchacha_decrypt(key, &file.nonce, &file.header, &file.ciphertext),
        FORMAT_VERSION_V1 => crypto::aead::aes_gcm_decrypt(key, &file.nonce, &file.ciphertext),
        _ => Err(AppError::VaultCorrupt),
    }
}

fn parse_vault(plaintext: &mut Vec<u8>) -> AppResult<Vault> {
    let vault: Result<Vault, _> = serde_json::from_slice(plaintext);
    plaintext.zeroize();
    vault.map_err(|_| AppError::VaultCorrupt)
}

/// Encrypt and atomically write a vault to `path` in the current (v2) format.
/// Uses tmp + rename + parent fsync so a crash mid-write never destroys the
/// previous good file. A fresh 192-bit nonce is generated per save.
pub fn save(path: &Path, vault: &Vault, key: &[u8; KEY_LEN], kdf: KdfParams, salt: [u8; SALT_LEN]) -> AppResult<()> {
    let mut plaintext = serde_json::to_vec(vault)?;
    let nonce = crypto::aead::random_xnonce();
    let header = encode_header_v2(&kdf, &salt, &nonce, 0); // ct_len patched below

    // ct_len is part of the AAD, so we must know it before encrypting. The GCM
    // tag adds a fixed 16 bytes to the plaintext length under both AEADs.
    let ct_len = (plaintext.len() + 16) as u32;
    let header = {
        let mut h = header;
        let len_offset = NONCE_OFFSET + XNONCE_LEN;
        h[len_offset..len_offset + 4].copy_from_slice(&ct_len.to_le_bytes());
        h
    };

    let ciphertext = crypto::aead::xchacha_encrypt(key, &nonce, &header, &plaintext)?;
    plaintext.zeroize();
    debug_assert_eq!(ciphertext.len() as u32, ct_len, "ct_len/AAD mismatch");

    let mut encoded = header;
    encoded.extend_from_slice(&ciphertext);
    write_atomic(path, &encoded)
}

/// Load and decrypt the vault at `path`, deriving the key from `password`.
/// Transparently reads both v1 and v2 files. On wrong password returns
/// [`AppError::WrongPassword`]; on a malformed file [`AppError::VaultCorrupt`].
pub fn load(path: &Path, password: &str) -> AppResult<(Vault, [u8; KEY_LEN], KdfParams, [u8; SALT_LEN])> {
    let bytes = fs::read(path)?;
    let file = decode(&bytes)?;
    let key = crypto::derive_key(password, &file.salt, file.kdf)?;
    let mut plaintext = decrypt_file(&file, &key)?;
    let vault = parse_vault(&mut plaintext)?;
    Ok((vault, key, file.kdf, file.salt))
}

/// Load and decrypt the vault using a pre-derived key (keychain fast-unlock),
/// bypassing the password KDF. Reads both v1 and v2.
pub fn load_with_key(path: &Path, key: &[u8; KEY_LEN]) -> AppResult<(Vault, KdfParams, [u8; SALT_LEN])> {
    let bytes = fs::read(path)?;
    let file = decode(&bytes)?;
    let mut plaintext = decrypt_file(&file, key)?;
    let vault = parse_vault(&mut plaintext)?;
    Ok((vault, file.kdf, file.salt))
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
    // fsync the parent directory so the rename is durable across power loss.
    // Best-effort on Unix; Windows has no portable equivalent (NTFS journals
    // the rename itself), so we skip it there.
    #[cfg(unix)]
    {
        if let Some(parent) = target.parent() {
            if let Ok(dir) = fs::File::open(parent) {
                let _ = dir.sync_all();
            }
        }
    }
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
        KdfParams { m_cost_kib: 8, t_cost: 1, p_cost: 1 }
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

    /// Write a legacy v1 (AES-GCM, no AAD) file the way 0.1.x did, so we can
    /// test the migration-on-load path.
    fn write_v1(path: &Path, vault: &Vault, key: &[u8; 32], kdf: KdfParams, salt: [u8; SALT_LEN]) {
        let plaintext = serde_json::to_vec(vault).unwrap();
        let nonce = [0x11u8; GCM_NONCE_LEN];
        let ct = crypto::aead::aes_gcm_encrypt(key, &nonce, &plaintext).unwrap();
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&FORMAT_VERSION_V1.to_le_bytes());
        buf.extend_from_slice(&kdf.m_cost_kib.to_le_bytes());
        buf.extend_from_slice(&kdf.t_cost.to_le_bytes());
        buf.extend_from_slice(&kdf.p_cost.to_le_bytes());
        buf.extend_from_slice(&salt);
        buf.extend_from_slice(&nonce);
        buf.extend_from_slice(&(ct.len() as u32).to_le_bytes());
        buf.extend_from_slice(&ct);
        fs::write(path, buf).unwrap();
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
    fn saved_file_is_v2() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        let salt = kdf::random_salt();
        let key = kdf::derive_key("pw", &salt, fast_kdf()).unwrap();
        save(&path, &Vault::default(), &key, fast_kdf(), salt).unwrap();

        let bytes = fs::read(&path).unwrap();
        let file = decode(&bytes).unwrap();
        assert_eq!(file.version, FORMAT_VERSION);
        assert_eq!(file.nonce.len(), XNONCE_LEN);
    }

    #[test]
    fn v1_file_loads_and_migrates_to_v2() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        let salt = kdf::random_salt();
        let key = kdf::derive_key("legacy-pw", &salt, fast_kdf()).unwrap();

        let mut vault = Vault::default();
        crud::add_item(&mut vault, sample_item());
        write_v1(&path, &vault, &key, fast_kdf(), salt);

        // Reads the v1 file...
        assert_eq!(decode(&fs::read(&path).unwrap()).unwrap().version, FORMAT_VERSION_V1);
        let (loaded, key_back, kdf, salt_back) = load(&path, "legacy-pw").unwrap();
        assert_eq!(loaded, vault);

        // ...and re-saving upgrades it to v2 in place.
        save(&path, &loaded, &key_back, kdf, salt_back).unwrap();
        assert_eq!(decode(&fs::read(&path).unwrap()).unwrap().version, FORMAT_VERSION);
        let (again, ..) = load(&path, "legacy-pw").unwrap();
        assert_eq!(again, vault);
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
    fn tampering_header_kdf_params_is_detected() {
        // The header (incl. KDF params) is AAD in v2, so flipping a param
        // byte must fail authentication rather than silently downgrading.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        let salt = kdf::random_salt();
        let key = kdf::derive_key("pw", &salt, fast_kdf()).unwrap();
        save(&path, &Vault::default(), &key, fast_kdf(), salt).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        bytes[6] ^= 0x01; // first byte of m_cost_kib
        fs::write(&path, &bytes).unwrap();

        let err = load(&path, "pw").unwrap_err();
        // Either the AAD check fails (WrongPassword) or the length sanity
        // check rejects it (VaultCorrupt) — both are safe refusals.
        assert!(
            matches!(err, AppError::WrongPassword | AppError::VaultCorrupt),
            "got {err:?}"
        );
    }

    #[test]
    fn flipped_byte_in_ciphertext_is_detected() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        let salt = kdf::random_salt();
        let key = kdf::derive_key("pw", &salt, fast_kdf()).unwrap();
        save(&path, &Vault::default(), &key, fast_kdf(), salt).unwrap();

        let mut bytes = fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        fs::write(&path, &bytes).unwrap();

        let err = load(&path, "pw").unwrap_err();
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
    fn load_with_key_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        let salt = kdf::random_salt();
        let key = kdf::derive_key("pw", &salt, fast_kdf()).unwrap();
        let mut vault = Vault::default();
        crud::add_item(&mut vault, sample_item());
        save(&path, &vault, &key, fast_kdf(), salt).unwrap();

        let (loaded, _kdf, _salt) = load_with_key(&path, &key).unwrap();
        assert_eq!(loaded, vault);
    }

    #[test]
    fn atomic_write_leaves_no_tmp_on_success() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.dat");
        let salt = kdf::random_salt();
        let key = kdf::derive_key("pw", &salt, fast_kdf()).unwrap();
        save(&path, &Vault::default(), &key, fast_kdf(), salt).unwrap();

        assert!(path.exists());
        assert!(!dir.path().join("vault.dat.tmp").exists());
    }
}
