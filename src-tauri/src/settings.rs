use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub auto_lock_seconds: u64,
    pub clipboard_clear_seconds: u64,
    pub show_totp_code: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_lock_seconds: 300,        // 5 minutes
            clipboard_clear_seconds: 30,
            show_totp_code: true,
        }
    }
}

fn config_path() -> AppResult<PathBuf> {
    Ok(paths::config_dir()?.join("config.json"))
}

/// Best-effort load. Returns defaults on any error so a corrupt config
/// never blocks the app — it just gets rewritten on the next save.
pub fn load() -> Settings {
    let path = match config_path() {
        Ok(p) => p,
        Err(_) => return Settings::default(),
    };
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .unwrap_or_default()
}

pub fn save(s: &Settings) -> AppResult<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(s)?)?;
    Ok(())
}
