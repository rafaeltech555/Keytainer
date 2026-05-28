use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

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
pub struct AppState {
    pub session: Mutex<Option<UnlockedSession>>,
    last_activity: Mutex<Instant>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            session: Mutex::new(None),
            last_activity: Mutex::new(Instant::now()),
        }
    }
}

impl AppState {
    /// Run `f` against the unlocked session, also bumping the idle timer.
    /// Returns [`AppError::Locked`] if there is no unlocked session.
    pub fn with_session<R>(&self, f: impl FnOnce(&mut UnlockedSession) -> AppResult<R>) -> AppResult<R> {
        let mut guard = self.session.lock().map_err(|_| AppError::Locked)?;
        let session = guard.as_mut().ok_or(AppError::Locked)?;
        let result = f(session)?;
        self.touch();
        Ok(result)
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

    /// Mark "the user did something just now", resetting the idle countdown.
    pub fn touch(&self) {
        if let Ok(mut t) = self.last_activity.lock() {
            *t = Instant::now();
        }
    }

    pub fn idle_for(&self) -> Duration {
        self.last_activity
            .lock()
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO)
    }
}

/// Spawn an idle-watcher task that locks the vault after `timeout` of
/// inactivity. Emits the `vault-locked` event so the frontend can navigate
/// back to the Unlock screen. The task exits on its own once the vault is
/// no longer unlocked, so callers can safely call this every time the
/// vault is unlocked without leaking tasks.
pub fn spawn_idle_watcher(app: AppHandle, timeout: Duration) {
    use tauri::Manager;

    tauri::async_runtime::spawn(async move {
        let poll = Duration::from_secs(5);
        loop {
            tokio::time::sleep(poll).await;
            let state: tauri::State<AppState> = app.state();
            if !state.is_unlocked() {
                break; // someone else (manual lock) already cleared it
            }
            if state.idle_for() >= timeout {
                state.lock_now();
                let _ = app.emit("vault-locked", "idle");
                break;
            }
        }
    });
}
