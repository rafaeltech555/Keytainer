use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    pub auto_lock_seconds: u64,
    pub clipboard_clear_seconds: u64,
    pub show_totp_code: bool,
    /// UI language: "system" (follow OS), "en", or "zh-TW".
    pub locale: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_lock_seconds: 300,        // 5 minutes
            clipboard_clear_seconds: 30,
            show_totp_code: true,
            locale: "system".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_without_locale_still_deserializes() {
        // A 0.1.x config.json had no `locale` field — it must load with the
        // default rather than failing (which would silently reset settings).
        let old = r#"{"auto_lock_seconds":120,"clipboard_clear_seconds":15,"show_totp_code":false}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.auto_lock_seconds, 120);
        assert_eq!(s.clipboard_clear_seconds, 15);
        assert!(!s.show_totp_code);
        assert_eq!(s.locale, "system");
    }

    #[test]
    fn locale_round_trips() {
        let s = Settings { locale: "en".into(), ..Settings::default() };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.locale, "en");
    }
}
