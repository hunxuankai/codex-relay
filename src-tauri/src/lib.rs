#![allow(linker_messages)]

use crate::app_state::AppState;
use crate::infrastructure::path_service::{PathMode, resolve_paths};
use crate::infrastructure::safe_log::init_logging;
use crate::services::autostart_service::{AutostartService, TauriAutostartBackend};
use crate::services::file_watch_service::FileWatchService;
use crate::services::self_check_service::SystemCodexCommandProbe;
use crate::tray::TauriFileWatchSink;
use std::sync::Arc;
use tauri::Manager;

pub mod app_state;
pub mod commands;
pub mod error;
pub mod infrastructure;
pub mod models;
pub mod services;
pub mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _cwd| {
                let _ = tray::show_main_window(app);
            },
        ))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--autostart")
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            tray::create_initial_tray(app.handle())?;

            let paths = resolve_paths(PathMode::Production)?;
            let log_guard = init_logging(&paths.logs_dir)?;
            let autostart_service =
                AutostartService::new(Arc::new(TauriAutostartBackend::new(app.handle().clone())));
            let state = AppState::new(
                paths.clone(),
                app.package_info().version.to_string(),
                autostart_service,
                Arc::new(SystemCodexCommandProbe),
            )?;
            let settings = state.settings_service.load_or_create()?;
            app.manage(state);

            let managed = app.state::<AppState>();
            managed.hold_log_guard(log_guard)?;
            let file_watch = FileWatchService::start(
                paths,
                Arc::new(TauriFileWatchSink::new(app.handle().clone())),
            )?;
            managed.install_file_watch_service(file_watch)?;

            tray::refresh_tray_from_disk(app.handle())?;
            tray::restore_window_bounds(app.handle(), &settings.window);
            let arguments = std::env::args().collect::<Vec<_>>();
            if tray::should_show_window(tray::is_autostart_launch(&arguments), &settings) {
                tray::show_main_window(app.handle())?;
            }
            tray::run_startup_checks(app.handle());
            Ok(())
        })
        .on_window_event(tray::handle_window_event)
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
            commands::settings_commands::exit_application,
            commands::backup_commands::list_backups,
            commands::backup_commands::restore_backup,
            commands::self_check_commands::run_critical_self_check,
            commands::self_check_commands::run_extended_self_check,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Codex Relay");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let tray_exit_requested = app
                .try_state::<AppState>()
                .map(|state| state.tray_runtime.exit_requested())
                .unwrap_or(false);
            if !tray_exit_requested {
                api.prevent_exit();
            }
        }
    });
}
