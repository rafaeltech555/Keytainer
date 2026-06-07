pub mod audit;
pub mod backup;
pub mod clipboard;
pub mod commands;
pub mod crypto;
pub mod error;
pub mod generator;
pub mod keychain;
pub mod paths;
pub mod session;
pub mod settings;
pub mod totp;
pub mod vault;

use clipboard::ClipboardState;
use session::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::default())
        .manage(ClipboardState::default())
        .invoke_handler(tauri::generate_handler![
            commands::vault_exists,
            commands::is_unlocked,
            commands::create_vault,
            commands::unlock,
            commands::lock,
            commands::ping_activity,
            commands::get_system_locale,
            commands::change_password,
            commands::list_items,
            commands::list_tags,
            commands::get_item,
            commands::add_item,
            commands::update_item,
            commands::delete_item,
            commands::compute_totp,
            commands::copy_password,
            commands::copy_history_password,
            commands::copy_totp,
            commands::get_settings,
            commands::save_settings,
            commands::generate_password,
            commands::audit_passwords,
            commands::export_vault,
            commands::import_vault,
            commands::keychain_available,
            commands::keychain_enable,
            commands::keychain_disable,
            commands::keychain_is_enabled,
            commands::unlock_with_keychain,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
