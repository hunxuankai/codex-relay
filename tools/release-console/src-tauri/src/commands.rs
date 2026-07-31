use crate::app_state::{
    AppState, ApplicationRequest, ApplicationResponse, ReleaseApplicationError, ReleaseEventSink,
};
use crate::models::{
    CommandResult, DraftIdentity, ReleaseEvent, ReleasePlanSummary, ReleasePreflightResult,
    ReleaseSession,
};
use std::sync::Arc;
use tauri::ipc::Channel;

struct TauriReleaseEventSink(Channel<ReleaseEvent>);

impl ReleaseEventSink for TauriReleaseEventSink {
    fn send(&self, event: ReleaseEvent) -> Result<(), String> {
        self.0
            .send(event)
            .map_err(|_| "release event channel closed".to_string())
    }
}

fn failure<T>(error: ReleaseApplicationError) -> CommandResult<T> {
    CommandResult::failure(error.code, error.message)
}

fn unexpected<T>() -> CommandResult<T> {
    CommandResult::failure("RELEASE_RESPONSE_INVALID", "发布控制台后端返回了无效响应。")
}

pub async fn inspect_release_repository_inner(
    state: &AppState,
    repository_path: String,
) -> CommandResult<ReleasePreflightResult> {
    match state
        .execute(ApplicationRequest::Inspect { repository_path }, None)
        .await
    {
        Ok(ApplicationResponse::Inspection(value)) => CommandResult::success(value),
        Ok(_) => unexpected(),
        Err(error) => failure(error),
    }
}

pub async fn prepare_release_plan_inner(
    state: &AppState,
    repository_path: String,
    target_version: String,
    notes: Option<String>,
) -> CommandResult<ReleasePlanSummary> {
    match state
        .execute(
            ApplicationRequest::PreparePlan {
                repository_path,
                target_version,
                notes,
            },
            None,
        )
        .await
    {
        Ok(ApplicationResponse::Plan(value)) => CommandResult::success(value),
        Ok(_) => unexpected(),
        Err(error) => failure(error),
    }
}

pub async fn start_release_inner(
    state: &AppState,
    plan_id: String,
    events: Arc<dyn ReleaseEventSink>,
) -> CommandResult<ReleaseSession> {
    session_command(state, ApplicationRequest::Start { plan_id }, Some(events)).await
}

pub async fn get_release_session_inner(
    state: &AppState,
    repository_path: String,
) -> CommandResult<Option<ReleaseSession>> {
    match state
        .execute(ApplicationRequest::GetSession { repository_path }, None)
        .await
    {
        Ok(ApplicationResponse::OptionalSession(value)) => CommandResult::success(value),
        Ok(_) => unexpected(),
        Err(error) => failure(error),
    }
}

pub async fn resume_release_inner(
    state: &AppState,
    session_id: String,
    events: Arc<dyn ReleaseEventSink>,
) -> CommandResult<ReleaseSession> {
    session_command(
        state,
        ApplicationRequest::Resume { session_id },
        Some(events),
    )
    .await
}

pub async fn cancel_release_inner(
    state: &AppState,
    session_id: String,
) -> CommandResult<ReleaseSession> {
    session_command(state, ApplicationRequest::Cancel { session_id }, None).await
}

pub async fn publish_release_inner(
    state: &AppState,
    session_id: String,
    expected_draft_identity: DraftIdentity,
    events: Arc<dyn ReleaseEventSink>,
) -> CommandResult<ReleaseSession> {
    session_command(
        state,
        ApplicationRequest::Publish {
            session_id,
            expected_draft_identity,
        },
        Some(events),
    )
    .await
}

pub async fn export_release_summary_inner(
    state: &AppState,
    session_id: String,
    destination_path: String,
) -> CommandResult<String> {
    match state
        .execute(
            ApplicationRequest::ExportSummary {
                session_id,
                destination_path,
            },
            None,
        )
        .await
    {
        Ok(ApplicationResponse::SummaryPath(value)) => CommandResult::success(value),
        Ok(_) => unexpected(),
        Err(error) => failure(error),
    }
}

async fn session_command(
    state: &AppState,
    request: ApplicationRequest,
    events: Option<Arc<dyn ReleaseEventSink>>,
) -> CommandResult<ReleaseSession> {
    match state.execute(request, events).await {
        Ok(ApplicationResponse::Session(value)) => CommandResult::success(value),
        Ok(_) => unexpected(),
        Err(error) => failure(error),
    }
}

#[tauri::command]
pub async fn inspect_release_repository(
    state: tauri::State<'_, AppState>,
    repository_path: String,
) -> Result<CommandResult<ReleasePreflightResult>, ()> {
    Ok(inspect_release_repository_inner(&state, repository_path).await)
}

#[tauri::command]
pub async fn prepare_release_plan(
    state: tauri::State<'_, AppState>,
    repository_path: String,
    target_version: String,
    notes: Option<String>,
) -> Result<CommandResult<ReleasePlanSummary>, ()> {
    Ok(prepare_release_plan_inner(&state, repository_path, target_version, notes).await)
}

#[tauri::command]
pub async fn start_release(
    state: tauri::State<'_, AppState>,
    plan_id: String,
    on_event: Channel<ReleaseEvent>,
) -> Result<CommandResult<ReleaseSession>, ()> {
    Ok(start_release_inner(&state, plan_id, Arc::new(TauriReleaseEventSink(on_event))).await)
}

#[tauri::command]
pub async fn get_release_session(
    state: tauri::State<'_, AppState>,
    repository_path: String,
) -> Result<CommandResult<Option<ReleaseSession>>, ()> {
    Ok(get_release_session_inner(&state, repository_path).await)
}

#[tauri::command]
pub async fn resume_release(
    state: tauri::State<'_, AppState>,
    session_id: String,
    on_event: Channel<ReleaseEvent>,
) -> Result<CommandResult<ReleaseSession>, ()> {
    Ok(resume_release_inner(
        &state,
        session_id,
        Arc::new(TauriReleaseEventSink(on_event)),
    )
    .await)
}

#[tauri::command]
pub async fn cancel_release(
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<CommandResult<ReleaseSession>, ()> {
    Ok(cancel_release_inner(&state, session_id).await)
}

#[tauri::command]
pub async fn publish_release(
    state: tauri::State<'_, AppState>,
    session_id: String,
    expected_draft_identity: DraftIdentity,
    on_event: Channel<ReleaseEvent>,
) -> Result<CommandResult<ReleaseSession>, ()> {
    Ok(publish_release_inner(
        &state,
        session_id,
        expected_draft_identity,
        Arc::new(TauriReleaseEventSink(on_event)),
    )
    .await)
}

#[tauri::command]
pub async fn export_release_summary(
    state: tauri::State<'_, AppState>,
    session_id: String,
    destination_path: String,
) -> Result<CommandResult<String>, ()> {
    Ok(export_release_summary_inner(&state, session_id, destination_path).await)
}
