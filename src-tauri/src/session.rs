use std::sync::Mutex;

use crate::crypto::{KdfParams, KEY_LEN, SALT_LEN};
use crate::error::{AppError, AppResult};
use crate::vault::Vault;

/// Encrypts in memory only while the vault is unlocked.
pub struct UnlockedSession {
    pub vault: Vault,
    pub key: [u8; KEY_LEN],
    pub salt: [u8; SALT_LEN],
    pub kdf: KdfParams,
}

impl Drop for UnlockedSession {
    fn drop(&mut self) {
        // Zero the key on drop so it doesn't linger in memory.
        self.key.fill(0);
    }
}

/// Holds the optional unlocked session behind a Mutex so Tauri commands
/// can mutate it without async overhead. Lives in `tauri::State`.
#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<UnlockedSession>>,
}

impl AppState {
    pub fn with_session<R>(&self, f: impl FnOnce(&mut UnlockedSession) -> AppResult<R>) -> AppResult<R> {
        let mut guard = self.session.lock().map_err(|_| AppError::Locked)?;
        let session = guard.as_mut().ok_or(AppError::Locked)?;
        f(session)
    }

    pub fn is_unlocked(&self) -> bool {
        self.session
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    pub fn lock_now(&self) {
        if let Ok(mut g) = self.session.lock() {
            *g = None; // Drop will zero the key
        }
    }
}
