use crate::app_state::AppState;
use crate::error::CommandResult;
use crate::models::health::HealthReport;
use tauri::ipc::InvokeError;

pub(crate) fn run_critical_self_check_inner(state: &AppState) -> CommandResult<HealthReport> {
    CommandResult::success(state.self_check_service.run_critical_checks())
}

#[tauri::command]
pub fn run_critical_self_check(state: tauri::State<'_, AppState>) -> CommandResult<HealthReport> {
    run_critical_self_check_inner(&state)
}

pub(crate) async fn run_extended_self_check_inner(state: &AppState) -> CommandResult<HealthReport> {
    CommandResult::success(state.self_check_service.run_extended_checks().await)
}

#[tauri::command]
pub async fn run_extended_self_check(
    state: tauri::State<'_, AppState>,
) -> Result<CommandResult<HealthReport>, InvokeError> {
    Ok(run_extended_self_check_inner(&state).await)
}
