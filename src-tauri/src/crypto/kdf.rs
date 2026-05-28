use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const SALT_LEN: usize = 16;
pub const KEY_LEN: usize = 32;

/// Argon2id parameters stored alongside the vault so we can re-derive the key.
/// Defaults follow OWASP 2024 guidance for interactive logins.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct KdfParams {
    pub m_cost_kib: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            m_cost_kib: 64 * 1024, // 64 MiB
            t_cost: 3,
            p_cost: 1,
        }
    }
}

pub fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

pub fn derive_key(password: &str, salt: &[u8], params: KdfParams) -> AppResult<[u8; KEY_LEN]> {
    let argon_params = Params::new(
        params.m_cost_kib,
        params.t_cost,
        params.p_cost,
        Some(KEY_LEN),
    )
    .map_err(|e| AppError::Crypto(format!("argon2 params: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

    let mut key = [0u8; KEY_LEN];
    argon.hash_password_into(password.as_bytes(), salt, &mut key)?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Use a small KDF profile for tests so they run quickly. Production code
    /// MUST go through `KdfParams::default()` instead.
    fn fast_params() -> KdfParams {
        KdfParams {
            m_cost_kib: 8,
            t_cost: 1,
            p_cost: 1,
        }
    }

    #[test]
    fn derive_is_deterministic() {
        let salt = [7u8; SALT_LEN];
        let k1 = derive_key("hunter2", &salt, fast_params()).unwrap();
        let k2 = derive_key("hunter2", &salt, fast_params()).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_passwords_yield_different_keys() {
        let salt = [7u8; SALT_LEN];
        let k1 = derive_key("hunter2", &salt, fast_params()).unwrap();
        let k2 = derive_key("hunter3", &salt, fast_params()).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn different_salts_yield_different_keys() {
        let k1 = derive_key("hunter2", &[1u8; SALT_LEN], fast_params()).unwrap();
        let k2 = derive_key("hunter2", &[2u8; SALT_LEN], fast_params()).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn random_salt_is_random() {
        let s1 = random_salt();
        let s2 = random_salt();
        assert_ne!(s1, s2);
    }
}
