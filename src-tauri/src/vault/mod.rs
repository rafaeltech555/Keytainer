pub mod crud;
pub mod store;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct Vault {
    #[zeroize(skip)]
    pub version: u32,
    pub items: Vec<VaultItem>,
    pub tags: Vec<String>,
}

impl Default for Vault {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            items: Vec::new(),
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct PasswordHistoryEntry {
    pub password: String,
    #[zeroize(skip)]
    pub changed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct VaultItem {
    #[zeroize(skip)]
    pub id: Uuid,
    pub site_name: String,
    pub username: String,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub totp: Option<TotpEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub password_history: Vec<PasswordHistoryEntry>,
    #[zeroize(skip)]
    pub created_at: i64,
    #[zeroize(skip)]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TotpAlg {
    Sha1,
    Sha256,
    Sha512,
}

impl Default for TotpAlg {
    fn default() -> Self {
        TotpAlg::Sha1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct TotpEntry {
    pub secret: String, // base32, no padding
    #[serde(default)]
    #[zeroize(skip)]
    pub algorithm: TotpAlg,
    #[zeroize(skip)]
    pub digits: u8,
    #[zeroize(skip)]
    pub period: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
}

impl TotpEntry {
    pub fn new_default(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            algorithm: TotpAlg::Sha1,
            digits: 6,
            period: 30,
            issuer: None,
        }
    }
}
