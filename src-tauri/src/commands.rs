use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::clipboard::ClipboardState;
use crate::crypto::{self, KdfParams};
use crate::error::{AppError, AppResult};
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

#[derive(Debug, Deserialize)]
pub struct ItemInput {
    #[serde(default)]
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
    fn into_vault_item(self) -> VaultItem {
        VaultItem {
            id: self.id.unwrap_or(Uuid::nil()),
            site_name: self.site_name,
            username: self.username,
            password: self.password,
            totp: self.totp,
            url: self.url,
            notes: self.notes,
            tags: self.tags,
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
pub fn create_vault(password: String, state: State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    if password.is_empty() {
        return Err(AppError::Crypto("password must not be empty".into()));
    }
    let path = paths::vault_path()?;
    if path.exists() {
        return Err(AppError::AlreadyExists);
    }

    let kdf = KdfParams::default();
    let salt = crypto::kdf::random_salt();
    let key = crypto::derive_key(&password, &salt, kdf)?;

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
pub fn unlock(password: String, state: State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    let path = paths::vault_path()?;
    if !path.exists() {
        return Err(AppError::NotInitialised);
    }
    let (vault, key, kdf, salt) = vault::store::load(&path, &password)?;
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

#[tauri::command]
pub fn list_items(query: Option<String>, state: State<'_, AppState>) -> AppResult<Vec<ItemSummary>> {
    state.with_session(|s| {
        let q = query.unwrap_or_default();
        let mut items: Vec<ItemSummary> = crud::search(&s.vault, &q)
            .into_iter()
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
    (0..length)
        .map(|_| *alphabet.choose(&mut rng).expect("alphabet is non-empty") as char)
        .collect()
}
