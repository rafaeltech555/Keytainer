use std::path::PathBuf;

use crate::error::{AppError, AppResult};

/// XDG config dir for Keytainer data:
///   $XDG_CONFIG_HOME/keytainer/   or   $HOME/.config/keytainer/
pub fn config_dir() -> AppResult<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("keytainer"));
        }
    }
    let home = std::env::var("HOME")
        .map_err(|_| AppError::Io("$HOME not set".into()))?;
    Ok(PathBuf::from(home).join(".config").join("keytainer"))
}

pub fn vault_path() -> AppResult<PathBuf> {
    Ok(config_dir()?.join("vault.dat"))
}
