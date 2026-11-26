use crate::app_state::AppState;
use crate::commands::command_result;
use crate::error::CommandResult;
use crate::models::cli_profile::{
    CliProfileList, ProfileKeyEdit, ProfileKeySaved, SaveProfileKeyInput,
};
use tauri::ipc::InvokeError;

pub(crate) fn list_cli_profiles_inner(state: &AppState) -> CommandResult<CliProfileList> {
    command_result(state.cli_profile_service.list())
}

#[tauri::command]
pub fn list_cli_profiles(state: tauri::State<'_, AppState>) -> CommandResult<CliProfileList> {
    list_cli_profiles_inner(&state)
}

pub(crate) fn prepare_profile_key_edit_inner(
    state: &AppState,
    profile_name: &str,
) -> CommandResult<ProfileKeyEdit> {
    command_result(state.cli_profile_service.prepare_key_edit(profile_name))
}

#[tauri::command]
pub fn prepare_profile_key_edit(
    state: tauri::State<'_, AppState>,
    profile_name: String,
) -> CommandResult<ProfileKeyEdit> {
    prepare_profile_key_edit_inner(&state, &profile_name)
}

#[tauri::command]
pub async fn save_profile_env_key(
    state: tauri::State<'_, AppState>,
    input: SaveProfileKeyInput,
) -> Result<CommandResult<ProfileKeySaved>, InvokeError> {
    Ok(command_result(
        state.cli_profile_service.save_key(input).await,
    ))
}

#[tauri::command]
pub fn open_profile_environment_settings(state: tauri::State<'_, AppState>) -> CommandResult<()> {
    command_result(state.cli_profile_service.open_settings())
}
