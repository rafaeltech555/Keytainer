use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::backup;
use crate::clipboard::ClipboardState;
use crate::crypto::{self, KdfParams};
use crate::error::{AppError, AppResult};
use crate::keychain;
use crate::paths;
use crate::session::{spawn_idle_watcher, AppState, UnlockedSession};
use crate::settings::{self, Settings};
use crate::totp;
use crate::vault::{self, crud, TotpEntry, Vault, VaultItem};

#[derive(Debug, Serialize)]
pub struct ItemSummary {
    pub id: Uuid,
    pub site_name: String,
    pub username: String,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub has_totp: bool,
    pub updated_at: i64,
}

impl From<&VaultItem> for ItemSummary {
    fn from(i: &VaultItem) -> Self {
        Self {
            id: i.id,
            site_name: i.site_name.clone(),
            username: i.username.clone(),
            url: i.url.clone(),
            tags: i.tags.clone(),
            has_totp: i.totp.is_some(),
            updated_at: i.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct ItemInput {
    #[serde(default)]
    #[zeroize(skip)]
    pub id: Option<Uuid>,
    pub site_name: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub totp: Option<TotpEntry>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ItemInput {
    /// Move fields out of `self` via `mem::take` so the `ZeroizeOnDrop`
    /// `Drop` impl can still run on the (now-empty) shell.
    fn into_vault_item(mut self) -> VaultItem {
        VaultItem {
            id: self.id.take().unwrap_or(Uuid::nil()),
            site_name: std::mem::take(&mut self.site_name),
            username: std::mem::take(&mut self.username),
            password: std::mem::take(&mut self.password),
            totp: self.totp.take(),
            url: self.url.take(),
            notes: self.notes.take(),
            tags: std::mem::take(&mut self.tags),
            password_history: Vec::new(),
            created_at: 0,
            updated_at: 0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TotpState {
    pub code: String,
    pub remaining_seconds: u32,
    pub period: u32,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[tauri::command]
pub fn vault_exists() -> bool {
    paths::vault_path().map(|p| p.exists()).unwrap_or(false)
}

#[tauri::command]
pub fn is_unlocked(state: State<'_, AppState>) -> bool {
    state.is_unlocked()
}

#[tauri::command]
pub fn create_vault(password: SecretString, state: State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    if password.expose_secret().is_empty() {
        return Err(AppError::Crypto("password must not be empty".into()));
    }
    let path = paths::vault_path()?;
    if path.exists() {
        return Err(AppError::AlreadyExists);
    }

    let kdf = KdfParams::default();
    let salt = crypto::kdf::random_salt();
    let key = crypto::derive_key(password.expose_secret(), &salt, kdf)?;

    let vault = Vault::default();
    vault::store::save(&path, &vault, &key, kdf, salt)?;

    *state.session.lock().map_err(|_| AppError::Locked)? = Some(UnlockedSession {
        vault,
        key,
        salt,
        kdf,
    });
    state.touch();

    let cfg = settings::load();
    spawn_idle_watcher(app, Duration::from_secs(cfg.auto_lock_seconds));
    Ok(())
}

#[tauri::command]
pub fn unlock(password: SecretString, state: State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    let path = paths::vault_path()?;
    if !path.exists() {
        return Err(AppError::NotInitialised);
    }
    let (vault, key, kdf, salt) = vault::store::load(&path, password.expose_secret())?;
    *state.session.lock().map_err(|_| AppError::Locked)? = Some(UnlockedSession {
        vault,
        key,
        salt,
        kdf,
    });
    state.touch();

    let cfg = settings::load();
    spawn_idle_watcher(app, Duration::from_secs(cfg.auto_lock_seconds));
    Ok(())
}

#[tauri::command]
pub fn lock(state: State<'_, AppState>) {
    state.lock_now();
}

/// Reset the idle auto-lock timer. The frontend calls this on user activity
/// that doesn't otherwise hit the backend (e.g. typing in a form), so a user
/// editing an entry for a while isn't locked out mid-edit. No-op when locked.
#[tauri::command]
pub fn ping_activity(state: State<'_, AppState>) {
    if state.is_unlocked() {
        state.touch();
    }
}

/// Best-effort OS UI language (e.g. "en-US", "zh-TW"). The frontend maps this
/// to a supported locale when the user's preference is "system".
#[tauri::command]
pub fn get_system_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en".into())
}

/// Change the master password: verify the current one, derive a fresh key
/// from the new password (new salt + current default KDF params), re-encrypt
/// the whole vault, and update the in-memory session. If keychain fast-unlock
/// is enabled, the stored key is refreshed too.
#[tauri::command]
pub fn change_password(
    current: SecretString,
    new: SecretString,
    state: State<'_, AppState>,
) -> AppResult<()> {
    if new.expose_secret().is_empty() {
        return Err(AppError::Crypto("new password must not be empty".into()));
    }
    let path = paths::vault_path()?;
    state.with_session(|s| {
        // Authorize: the supplied current password must derive the live key.
        let mut check = crypto::derive_key(current.expose_secret(), &s.salt, s.kdf)?;
        let ok = check == s.key;
        check.zeroize();
        if !ok {
            return Err(AppError::WrongPassword);
        }

        let new_kdf = KdfParams::default();
        let new_salt = crypto::kdf::random_salt();
        let mut new_key = crypto::derive_key(new.expose_secret(), &new_salt, new_kdf)?;

        vault::store::save(&path, &s.vault, &new_key, new_kdf, new_salt)?;
        s.key = new_key;
        s.salt = new_salt;
        s.kdf = new_kdf;
        new_key.zeroize();

        // Keep keychain fast-unlock working after a rotation.
        if keychain::load_key().is_ok() {
            let _ = keychain::store_key(&s.key);
        }
        Ok(())
    })
}

#[tauri::command]
pub fn list_items(
    query: Option<String>,
    tag: Option<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<ItemSummary>> {
    state.with_session(|s| {
        let q = query.unwrap_or_default();
        let tag_filter = tag.unwrap_or_default();
        let mut items: Vec<ItemSummary> = crud::search(&s.vault, &q)
            .into_iter()
            .filter(|i| {
                tag_filter.is_empty()
                    || i.tags.iter().any(|t| t.eq_ignore_ascii_case(&tag_filter))
            })
            .map(ItemSummary::from)
            .collect();
        items.sort_by(|a, b| a.site_name.to_lowercase().cmp(&b.site_name.to_lowercase()));
        Ok(items)
    })
}

#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    state.with_session(|s| Ok(s.vault.tags.clone()))
}

#[tauri::command]
pub fn get_item(id: Uuid, state: State<'_, AppState>) -> AppResult<VaultItem> {
    state.with_session(|s| {
        s.vault
            .items
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or(AppError::ItemNotFound(id))
    })
}

#[tauri::command]
pub fn add_item(item: ItemInput, state: State<'_, AppState>) -> AppResult<Uuid> {
    let new_item = item.into_vault_item();
    let path = paths::vault_path()?;
    state.with_session(|s| {
        let id = crud::add_item(&mut s.vault, new_item);
        vault::store::save(&path, &s.vault, &s.key, s.kdf, s.salt)?;
        Ok(id)
    })
}

#[tauri::command]
pub fn update_item(item: ItemInput, state: State<'_, AppState>) -> AppResult<()> {
    let id = item.id.ok_or(AppError::ItemNotFound(Uuid::nil()))?;
    let mut updated = item.into_vault_item();
    updated.id = id;
    let path = paths::vault_path()?;
    state.with_session(|s| {
        crud::update_item(&mut s.vault, updated)?;
        vault::store::save(&path, &s.vault, &s.key, s.kdf, s.salt)?;
        Ok(())
    })
}

#[tauri::command]
pub fn delete_item(id: Uuid, state: State<'_, AppState>) -> AppResult<()> {
    let path = paths::vault_path()?;
    state.with_session(|s| {
        crud::delete_item(&mut s.vault, id)?;
        vault::store::save(&path, &s.vault, &s.key, s.kdf, s.salt)?;
        Ok(())
    })
}

/// Compute the current TOTP code and seconds remaining for an item.
/// Polled by the frontend once per second while a detail view is open.
#[tauri::command]
pub fn compute_totp(id: Uuid, state: State<'_, AppState>) -> AppResult<TotpState> {
    let now = now_unix();
    state.with_session(|s| {
        let item = s
            .vault
            .items
            .iter()
            .find(|i| i.id == id)
            .ok_or(AppError::ItemNotFound(id))?;
        let entry = item.totp.as_ref().ok_or(AppError::InvalidTotpSecret)?;
        Ok(TotpState {
            code: totp::code_at(entry, now)?,
            remaining_seconds: totp::remaining_seconds(entry, now),
            period: entry.period,
        })
    })
}

/// Copy an item's password to the system clipboard. Schedules an auto-clear
/// after `clipboard_clear_seconds` (from settings).
#[tauri::command]
pub fn copy_password(
    id: Uuid,
    state: State<'_, AppState>,
    clipboard: State<'_, ClipboardState>,
) -> AppResult<()> {
    let secret = state.with_session(|s| {
        s.vault
            .items
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.password.clone())
            .ok_or(AppError::ItemNotFound(id))
    })?;
    let cfg = settings::load();
    clipboard.write_with_auto_clear(secret, Duration::from_secs(cfg.clipboard_clear_seconds))
}

#[tauri::command]
pub fn copy_history_password(
    id: Uuid,
    index: usize,
    state: State<'_, AppState>,
    clipboard: State<'_, ClipboardState>,
) -> AppResult<()> {
    let secret = state.with_session(|s| {
        s.vault
            .items
            .iter()
            .find(|i| i.id == id)
            .and_then(|i| i.password_history.get(index))
            .map(|e| e.password.clone())
            .ok_or(AppError::ItemNotFound(id))
    })?;
    let cfg = settings::load();
    clipboard.write_with_auto_clear(secret, Duration::from_secs(cfg.clipboard_clear_seconds))
}

/// Copy an item's current TOTP code (not the secret) to the clipboard.
#[tauri::command]
pub fn copy_totp(
    id: Uuid,
    state: State<'_, AppState>,
    clipboard: State<'_, ClipboardState>,
) -> AppResult<()> {
    let totp_state = compute_totp(id, state)?;
    let cfg = settings::load();
    clipboard
        .write_with_auto_clear(totp_state.code, Duration::from_secs(cfg.clipboard_clear_seconds))
}

#[tauri::command]
pub fn get_settings() -> Settings {
    settings::load()
}

#[tauri::command]
pub fn save_settings(new_settings: Settings) -> AppResult<()> {
    settings::save(&new_settings)
}

/// Password generator. Produces fresh randomness from the OS CSPRNG;
/// doesn't touch the vault, so it's available even when locked.
#[tauri::command]
pub fn generate_password(length: usize, include_symbols: bool) -> String {
    use rand::seq::SliceRandom;
    let length = length.clamp(8, 128);
    let mut alphabet: Vec<u8> =
        (b'a'..=b'z').chain(b'A'..=b'Z').chain(b'0'..=b'9').collect();
    if include_symbols {
        alphabet.extend_from_slice(b"!@#$%^&*()-_=+[]{};:,.?/");
    }
    let mut rng = rand::thread_rng();
    let pw: String = (0..length)
        .map(|_| *alphabet.choose(&mut rng).expect("alphabet is non-empty") as char)
        .collect();
    // Best-effort: scrub the working alphabet. The returned String necessarily
    // crosses the IPC boundary into JS-managed memory, which we can't zeroize.
    alphabet.zeroize();
    pw
}

// ──────────────────────────── Backup / Restore ────────────────────────────

#[tauri::command]
pub fn export_vault(
    path: String,
    password: SecretString,
    state: State<'_, AppState>,
) -> AppResult<()> {
    if password.expose_secret().is_empty() {
        return Err(AppError::Crypto("backup password must not be empty".into()));
    }
    state.with_session(|s| {
        backup::export_to_file(&PathBuf::from(&path), &s.vault, password.expose_secret())
    })
}

#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub added: usize,
    pub updated: usize,
}

/// Merge items from a backup file into the current (unlocked) vault.
/// Items with a matching id replace the existing one (last-write-wins);
/// items with new ids are appended. Non-destructive otherwise.
#[tauri::command]
pub fn import_vault(
    path: String,
    password: SecretString,
    state: State<'_, AppState>,
) -> AppResult<ImportReport> {
    let mut imported = backup::import_from_file(&PathBuf::from(&path), password.expose_secret())?;
    let save_path = paths::vault_path()?;
    state.with_session(|s| {
        let mut added = 0usize;
        let mut updated = 0usize;
        // `mem::take` moves fields out of `imported` without violating its
        // `Drop` impl (added by `ZeroizeOnDrop`). The emptied `imported`
        // still zeroizes on drop.
        for item in std::mem::take(&mut imported.items) {
            match s.vault.items.iter_mut().find(|i| i.id == item.id) {
                Some(existing) => {
                    *existing = item;
                    updated += 1;
                }
                None => {
                    s.vault.items.push(item);
                    added += 1;
                }
            }
        }
        for t in std::mem::take(&mut imported.tags) {
            if !s.vault.tags.iter().any(|x| x.eq_ignore_ascii_case(&t)) {
                s.vault.tags.push(t);
            }
        }
        vault::store::save(&save_path, &s.vault, &s.key, s.kdf, s.salt)?;
        Ok(ImportReport { added, updated })
    })
}

// ───────────────────────────── OS Keychain ────────────────────────────────

#[tauri::command]
pub fn keychain_available() -> bool {
    keychain::is_supported()
}

#[tauri::command]
pub fn keychain_is_enabled() -> bool {
    keychain::load_key().is_ok()
}

/// Stash the current session's derived key in the OS keychain so the user
/// can unlock without re-entering their master password on next launch.
/// Requires an unlocked vault.
#[tauri::command]
pub fn keychain_enable(state: State<'_, AppState>) -> AppResult<()> {
    state.with_session(|s| keychain::store_key(&s.key))
}

#[tauri::command]
pub fn keychain_disable() -> AppResult<()> {
    keychain::delete_key()
}

/// Try to unlock the vault using a key cached in the OS keychain. Returns
/// `WrongPassword` if the cached key doesn't match the vault (e.g., the
/// user changed their master password elsewhere).
#[tauri::command]
pub fn unlock_with_keychain(state: State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    let path = paths::vault_path()?;
    if !path.exists() {
        return Err(AppError::NotInitialised);
    }
    let key = keychain::load_key()?;
    // Bypass the password KDF: decrypt with the cached key (handles v1 + v2).
    let (vault, kdf, salt) = vault::store::load_with_key(&path, &key)?;

    *state.session.lock().map_err(|_| AppError::Locked)? = Some(UnlockedSession {
        vault,
        key,
        salt,
        kdf,
    });
    state.touch();

    let cfg = settings::load();
    spawn_idle_watcher(app, Duration::from_secs(cfg.auto_lock_seconds));
    Ok(())
}

#[tauri::command]
pub fn audit_passwords(state: State<'_, AppState>) -> AppResult<crate::audit::AuditReport> {
    state.with_session(|s| Ok(crate::audit::audit(&s.vault)))
}

#[cfg(test)]
mod tests {
    use secrecy::{ExposeSecret, SecretString};

    /// Tauri IPC sends `password` as a JSON string. Confirm `SecretString`
    /// (with the `serde` feature on) deserializes from that without
    /// special handling on the frontend.
    #[test]
    fn secret_string_deserializes_from_json_string() {
        let s: SecretString = serde_json::from_str("\"hunter2\"").unwrap();
        assert_eq!(s.expose_secret(), "hunter2");
    }
}
