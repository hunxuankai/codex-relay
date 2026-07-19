use crate::app_state::AppState;
use crate::commands::command_result;
use crate::error::CommandResult;
use crate::models::backup::BackupSummary;
use crate::models::provider::ProviderMutationOutcome;

pub(crate) fn list_backups_inner(state: &AppState) -> CommandResult<Vec<BackupSummary>> {
    command_result(state.provider_service.list_backups())
}

#[tauri::command]
pub fn list_backups(state: tauri::State<'_, AppState>) -> CommandResult<Vec<BackupSummary>> {
    list_backups_inner(&state)
}

pub(crate) async fn restore_backup_inner(
    state: &AppState,
    directory_name: String,
) -> CommandResult<ProviderMutationOutcome> {
    command_result(state.provider_service.restore_backup(&directory_name).await)
}

#[tauri::command]
pub async fn restore_backup(
    state: tauri::State<'_, AppState>,
    directory_name: String,
) -> Result<CommandResult<ProviderMutationOutcome>, ()> {
    Ok(restore_backup_inner(&state, directory_name).await)
}
