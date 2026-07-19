use crate::app_state::AppState;
use crate::commands::command_result;
use crate::error::CommandResult;
use crate::infrastructure::file_fingerprint::FileSetFingerprint;
use crate::models::provider::{
    CreateProviderInput, ProviderListState, ProviderMutationOutcome, SwitchOutcome,
    UpdateProviderInput,
};

pub(crate) fn list_providers_inner(state: &AppState) -> CommandResult<ProviderListState> {
    command_result(state.provider_service.list_providers())
}

#[tauri::command]
pub fn list_providers(state: tauri::State<'_, AppState>) -> CommandResult<ProviderListState> {
    list_providers_inner(&state)
}

pub(crate) fn get_provider_api_key_inner(
    state: &AppState,
    provider_id: String,
) -> CommandResult<Option<String>> {
    command_result(state.provider_service.get_api_key_for_edit(&provider_id))
}

#[tauri::command]
pub fn get_provider_api_key(
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> CommandResult<Option<String>> {
    get_provider_api_key_inner(&state, provider_id)
}

pub(crate) async fn create_provider_inner(
    state: &AppState,
    input: CreateProviderInput,
) -> CommandResult<ProviderMutationOutcome> {
    command_result(state.provider_service.create_provider(input).await)
}

#[tauri::command]
pub async fn create_provider(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    input: CreateProviderInput,
) -> Result<CommandResult<ProviderMutationOutcome>, ()> {
    let application_write = match state.begin_application_write() {
        Ok(application_write) => application_write,
        Err(error) => return Ok(CommandResult::failure(&error)),
    };
    let result = create_provider_inner(&state, input).await;
    drop(application_write);
    if let Some(outcome) = result.data.as_ref() {
        crate::tray::after_provider_mutation(&app, outcome.message.clone(), false);
    }
    Ok(result)
}

pub(crate) async fn update_provider_inner(
    state: &AppState,
    input: UpdateProviderInput,
) -> CommandResult<ProviderMutationOutcome> {
    command_result(state.provider_service.update_provider(input).await)
}

#[tauri::command]
pub async fn update_provider(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    input: UpdateProviderInput,
) -> Result<CommandResult<ProviderMutationOutcome>, ()> {
    let application_write = match state.begin_application_write() {
        Ok(application_write) => application_write,
        Err(error) => return Ok(CommandResult::failure(&error)),
    };
    let result = update_provider_inner(&state, input).await;
    drop(application_write);
    if let Some(outcome) = result.data.as_ref() {
        crate::tray::after_provider_mutation(&app, outcome.message.clone(), false);
    }
    Ok(result)
}

pub(crate) async fn delete_provider_inner(
    state: &AppState,
    provider_id: String,
    expected_files: FileSetFingerprint,
) -> CommandResult<ProviderMutationOutcome> {
    command_result(
        state
            .provider_service
            .delete_provider(&provider_id, expected_files)
            .await,
    )
}

#[tauri::command]
pub async fn delete_provider(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: String,
    expected_files: FileSetFingerprint,
) -> Result<CommandResult<ProviderMutationOutcome>, ()> {
    let application_write = match state.begin_application_write() {
        Ok(application_write) => application_write,
        Err(error) => return Ok(CommandResult::failure(&error)),
    };
    let result = delete_provider_inner(&state, provider_id, expected_files).await;
    drop(application_write);
    if let Some(outcome) = result.data.as_ref() {
        crate::tray::after_provider_mutation(&app, outcome.message.clone(), false);
    }
    Ok(result)
}

pub(crate) async fn switch_provider_inner(
    state: &AppState,
    provider_id: String,
) -> CommandResult<SwitchOutcome> {
    command_result(state.provider_service.switch_provider(&provider_id).await)
}

#[tauri::command]
pub async fn switch_provider(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<CommandResult<SwitchOutcome>, ()> {
    let switch_guard = match state.tray_runtime.try_begin_switch() {
        Some(guard) => guard,
        None => {
            let error = crate::error::AppError::new(
                "SWITCH_IN_PROGRESS",
                "Provider 正在切换，请稍候。",
                "duplicate provider switch rejected",
            );
            return Ok(CommandResult::failure(&error));
        }
    };
    let _ = crate::tray::refresh_tray_from_disk(&app);
    let application_write = match state.begin_application_write() {
        Ok(application_write) => application_write,
        Err(error) => {
            drop(switch_guard);
            let _ = crate::tray::refresh_tray_from_disk(&app);
            return Ok(CommandResult::failure(&error));
        }
    };
    let result = switch_provider_inner(&state, provider_id).await;
    drop(application_write);
    drop(switch_guard);
    if let Some(outcome) = result.data.as_ref() {
        crate::tray::after_provider_mutation(&app, outcome.message.clone(), true);
    } else {
        let _ = crate::tray::refresh_tray_from_disk(&app);
    }
    Ok(result)
}

pub(crate) async fn import_current_auth_key_inner(
    state: &AppState,
    provider_id: String,
) -> CommandResult<ProviderMutationOutcome> {
    command_result(
        state
            .provider_service
            .import_current_auth_key(&provider_id)
            .await,
    )
}

#[tauri::command]
pub async fn import_current_auth_key(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<CommandResult<ProviderMutationOutcome>, ()> {
    let application_write = match state.begin_application_write() {
        Ok(application_write) => application_write,
        Err(error) => return Ok(CommandResult::failure(&error)),
    };
    let result = import_current_auth_key_inner(&state, provider_id).await;
    drop(application_write);
    if let Some(outcome) = result.data.as_ref() {
        crate::tray::after_provider_mutation(&app, outcome.message.clone(), false);
    }
    Ok(result)
}
