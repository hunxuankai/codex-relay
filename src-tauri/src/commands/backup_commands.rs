use crate::app_state::AppState;
use crate::commands::command_result;
use crate::error::CommandResult;
use crate::models::backup::{BackupFileName, BackupInventory};
use crate::models::provider::ProviderMutationOutcome;

pub(crate) fn list_backups_inner(state: &AppState) -> CommandResult<BackupInventory> {
    command_result(state.provider_service.list_backups())
}

#[tauri::command]
pub fn list_backups(state: tauri::State<'_, AppState>) -> CommandResult<BackupInventory> {
    list_backups_inner(&state)
}

pub(crate) fn open_backup_file_inner(
    state: &AppState,
    directory_name: String,
    file_name: BackupFileName,
) -> CommandResult<()> {
    command_result(state.open_backup_file(&directory_name, file_name))
}

#[tauri::command]
pub fn open_backup_file(
    state: tauri::State<'_, AppState>,
    directory_name: String,
    file_name: BackupFileName,
) -> CommandResult<()> {
    open_backup_file_inner(&state, directory_name, file_name)
}

pub(crate) async fn restore_backup_inner(
    state: &AppState,
    directory_name: String,
) -> CommandResult<ProviderMutationOutcome> {
    command_result(state.provider_service.restore_backup(&directory_name).await)
}

#[tauri::command]
pub async fn restore_backup(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    directory_name: String,
) -> Result<CommandResult<ProviderMutationOutcome>, ()> {
    let application_write = match state.begin_application_write() {
        Ok(application_write) => application_write,
        Err(error) => return Ok(CommandResult::failure(&error)),
    };
    let result = restore_backup_inner(&state, directory_name).await;
    drop(application_write);
    if let Some(outcome) = result.data.as_ref() {
        crate::tray::after_provider_mutation(&app, outcome.message.clone(), false);
    }
    Ok(result)
}
