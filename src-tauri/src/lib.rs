#![allow(linker_messages)]

pub mod app_state;
pub mod commands;
pub mod error;
pub mod infrastructure;
pub mod models;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::provider_commands::list_providers,
            commands::provider_commands::get_provider_api_key,
            commands::provider_commands::create_provider,
            commands::provider_commands::update_provider,
            commands::provider_commands::delete_provider,
            commands::provider_commands::switch_provider,
            commands::provider_commands::import_current_auth_key,
            commands::settings_commands::get_settings,
            commands::settings_commands::save_settings,
            commands::settings_commands::set_autostart,
            commands::settings_commands::open_codex_directory,
            commands::backup_commands::list_backups,
            commands::backup_commands::restore_backup,
            commands::self_check_commands::run_critical_self_check,
            commands::self_check_commands::run_extended_self_check,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Relay");
}
