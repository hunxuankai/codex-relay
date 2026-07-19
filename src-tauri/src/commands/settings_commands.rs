use crate::app_state::AppState;
use crate::commands::command_result;
use crate::error::CommandResult;
use crate::models::settings::{Settings, SettingsState};

pub(crate) fn get_settings_inner(state: &AppState) -> CommandResult<SettingsState> {
    command_result(state.settings_state())
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> CommandResult<SettingsState> {
    get_settings_inner(&state)
}

pub(crate) fn save_settings_inner(
    state: &AppState,
    settings: Settings,
) -> CommandResult<SettingsState> {
    command_result(state.save_settings(settings))
}

#[tauri::command]
pub fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: Settings,
) -> CommandResult<SettingsState> {
    let result = save_settings_inner(&state, settings);
    if result.success {
        crate::tray::after_settings_change(&app);
    }
    result
}

pub(crate) fn set_autostart_inner(state: &AppState, enabled: bool) -> CommandResult<SettingsState> {
    command_result(state.set_autostart(enabled))
}

#[tauri::command]
pub fn set_autostart(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    enabled: bool,
) -> CommandResult<SettingsState> {
    let result = set_autostart_inner(&state, enabled);
    if result.success {
        crate::tray::after_settings_change(&app);
    }
    result
}

pub(crate) fn open_codex_directory_inner(state: &AppState) -> CommandResult<()> {
    command_result(state.open_codex_directory())
}

#[tauri::command]
pub fn open_codex_directory(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    open_codex_directory_inner(&state)
}

pub(crate) fn request_exit_inner(state: &AppState) -> CommandResult<()> {
    state.tray_runtime.request_exit();
    CommandResult::success(())
}

#[tauri::command]
pub fn exit_application(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> CommandResult<()> {
    let result = request_exit_inner(&state);
    app.exit(0);
    result
}
