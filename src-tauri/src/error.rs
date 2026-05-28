use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("incorrect master password")]
    WrongPassword,

    #[error("vault file is corrupt or not a Keytainer vault")]
    VaultCorrupt,

    #[error("vault is locked")]
    Locked,

    #[error("vault not initialised")]
    NotInitialised,

    #[error("vault already exists at this location")]
    AlreadyExists,

    #[error("item not found: {0}")]
    ItemNotFound(uuid::Uuid),

    #[error("invalid TOTP secret (must be base32)")]
    InvalidTotpSecret,

    #[error("keychain unavailable: {0}")]
    KeychainUnavailable(String),

    #[error("clipboard error: {0}")]
    Clipboard(String),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("serialization error: {0}")]
    Serde(String),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Serde(e.to_string())
    }
}

impl From<argon2::Error> for AppError {
    fn from(e: argon2::Error) -> Self {
        AppError::Crypto(format!("argon2: {e}"))
    }
}

/// Serialize as a tagged object so the frontend can branch on `kind`.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AppError", 2)?;
        let kind = match self {
            AppError::WrongPassword => "WrongPassword",
            AppError::VaultCorrupt => "VaultCorrupt",
            AppError::Locked => "Locked",
            AppError::NotInitialised => "NotInitialised",
            AppError::AlreadyExists => "AlreadyExists",
            AppError::ItemNotFound(_) => "ItemNotFound",
            AppError::InvalidTotpSecret => "InvalidTotpSecret",
            AppError::KeychainUnavailable(_) => "KeychainUnavailable",
            AppError::Clipboard(_) => "Clipboard",
            AppError::Crypto(_) => "Crypto",
            AppError::Io(_) => "Io",
            AppError::Serde(_) => "Serde",
        };
        s.serialize_field("kind", kind)?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
