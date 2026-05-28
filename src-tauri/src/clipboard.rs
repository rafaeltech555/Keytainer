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
    pub fn write_with_auto_clear(&self, text: String, clear_after: Duration) -> AppResult<()> {
        #[cfg(feature = "clipboard")]
        {
            use arboard::Clipboard;

            let mut cb = Clipboard::new().map_err(|e| AppError::Clipboard(e.to_string()))?;
            cb.set_text(text).map_err(|e| AppError::Clipboard(e.to_string()))?;

            let my_gen = {
                let mut g = self
                    .generation
                    .lock()
                    .map_err(|_| AppError::Clipboard("clipboard mutex poisoned".into()))?;
                *g = g.wrapping_add(1);
                *g
            };

            let gen_arc = Arc::clone(&self.generation);
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(clear_after).await;
                let still_current = gen_arc
                    .lock()
                    .map(|g| *g == my_gen)
                    .unwrap_or(false);
                if still_current {
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
