use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::{AppError, AppResult};

/// Tracks the generation of the most recent clipboard write so the
/// auto-clear task can tell whether its scheduled clear is still
/// relevant — i.e., nothing newer has overwritten it. If the user copies
/// a second value within the clear window, the first clear becomes a
/// no-op and the second copy gets its own fresh 30-second timer.
#[derive(Clone, Default)]
pub struct ClipboardState {
    generation: Arc<Mutex<u64>>,
}

impl ClipboardState {
    /// Claim a fresh write generation, returning the new value. Each clipboard
    /// write takes the next generation; a scheduled auto-clear only fires while
    /// its generation is still the current one.
    #[cfg(feature = "clipboard")]
    fn next_generation(&self) -> AppResult<u64> {
        let mut g = self
            .generation
            .lock()
            .map_err(|_| AppError::Clipboard("clipboard mutex poisoned".into()))?;
        *g = g.wrapping_add(1);
        Ok(*g)
    }

    /// Whether `generation` is still the most recent write — i.e. no later copy
    /// has superseded it. A poisoned lock reports `false` so a stale clear stays
    /// a no-op rather than wiping a newer value.
    #[cfg(feature = "clipboard")]
    fn is_current(&self, generation: u64) -> bool {
        self.generation
            .lock()
            .map(|g| *g == generation)
            .unwrap_or(false)
    }

    pub fn write_with_auto_clear(&self, text: String, clear_after: Duration) -> AppResult<()> {
        #[cfg(feature = "clipboard")]
        {
            use arboard::Clipboard;

            let mut cb = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;
            cb.set_text(text).map_err(|e| AppError::Clipboard(e.to_string()))?;

            let my_gen = self.next_generation()?;

            let this = self.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(clear_after).await;
                if this.is_current(my_gen) {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(String::new());
                    }
                }
            });
            Ok(())
        }

        #[cfg(not(feature = "clipboard"))]
        {
            let _ = (text, clear_after);
            Err(AppError::Clipboard("clipboard feature not enabled".into()))
        }
    }
}

#[cfg(all(test, feature = "clipboard"))]
mod tests {
    use super::*;

    #[test]
    fn generations_increase_monotonically() {
        let state = ClipboardState::default();
        assert_eq!(state.next_generation().unwrap(), 1);
        assert_eq!(state.next_generation().unwrap(), 2);
        assert_eq!(state.next_generation().unwrap(), 3);
    }

    #[test]
    fn only_the_latest_generation_is_current() {
        let state = ClipboardState::default();
        let first = state.next_generation().unwrap();
        assert!(state.is_current(first));

        // A second copy supersedes the first: the first copy's pending clear
        // becomes a no-op, the second copy owns the live timer.
        let second = state.next_generation().unwrap();
        assert!(!state.is_current(first));
        assert!(state.is_current(second));
    }

    #[test]
    fn clones_share_one_generation_counter() {
        // The auto-clear task holds a clone of the state; it must observe
        // writes made through the original (and vice versa) via the shared Arc.
        let state = ClipboardState::default();
        let task_view = state.clone();

        let mine = state.next_generation().unwrap();
        assert!(task_view.is_current(mine));

        // A later write through the clone invalidates the earlier generation.
        let newer = task_view.next_generation().unwrap();
        assert!(!state.is_current(mine));
        assert!(state.is_current(newer));
    }
}
