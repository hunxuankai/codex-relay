use crate::app_state::AppState;
use crate::commands::command_result;
use crate::error::{AppError, CommandResult};
use crate::models::provider_availability::ProviderAvailabilityResult;
use uuid::Uuid;

fn parse_request_id(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| {
        AppError::new(
            "INVALID_PROVIDER_TEST_REQUEST_ID",
            "Provider 测试请求 ID 无效。",
            "provider availability command received malformed request id",
        )
    })
}

pub(crate) async fn test_provider_api_inner(
    state: &AppState,
    provider_id: String,
    request_id: String,
    use_proxy: bool,
) -> CommandResult<ProviderAvailabilityResult> {
    let request_id = match parse_request_id(&request_id) {
        Ok(request_id) => request_id,
        Err(error) => return CommandResult::failure(&error),
    };
    command_result(
        state
            .provider_availability_service
            .test_api(&provider_id, request_id, use_proxy)
            .await,
    )
}

#[tauri::command]
pub async fn test_provider_api(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    request_id: String,
    use_proxy: bool,
) -> Result<CommandResult<ProviderAvailabilityResult>, ()> {
    Ok(test_provider_api_inner(&state, provider_id, request_id, use_proxy).await)
}

pub(crate) async fn test_provider_codex_compatibility_inner(
    state: &AppState,
    provider_id: String,
    request_id: String,
    use_proxy: bool,
) -> CommandResult<ProviderAvailabilityResult> {
    let request_id = match parse_request_id(&request_id) {
        Ok(request_id) => request_id,
        Err(error) => return CommandResult::failure(&error),
    };
    command_result(
        state
            .provider_availability_service
            .test_codex(&provider_id, request_id, use_proxy)
            .await,
    )
}

#[tauri::command]
pub async fn test_provider_codex_compatibility(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    request_id: String,
    use_proxy: bool,
) -> Result<CommandResult<ProviderAvailabilityResult>, ()> {
    Ok(test_provider_codex_compatibility_inner(&state, provider_id, request_id, use_proxy).await)
}

pub(crate) fn cancel_provider_test_inner(
    state: &AppState,
    request_id: String,
) -> CommandResult<bool> {
    let request_id = match parse_request_id(&request_id) {
        Ok(request_id) => request_id,
        Err(error) => return CommandResult::failure(&error),
    };
    command_result(state.provider_availability_service.cancel(request_id))
}

#[tauri::command]
pub fn cancel_provider_test(
    state: tauri::State<'_, AppState>,
    request_id: String,
) -> CommandResult<bool> {
    cancel_provider_test_inner(&state, request_id)
}
