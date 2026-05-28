pub mod commands;
pub mod crypto;
pub mod error;
pub mod paths;
pub mod session;
pub mod totp;
pub mod vault;

use session::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::vault_exists,
            commands::is_unlocked,
            commands::create_vault,
            commands::unlock,
            commands::lock,
            commands::list_items,
            commands::list_tags,
            commands::get_item,
            commands::add_item,
            commands::update_item,
            commands::delete_item,
            commands::generate_password,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
