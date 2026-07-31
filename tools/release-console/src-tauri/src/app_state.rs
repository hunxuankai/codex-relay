use crate::models::{
    DraftIdentity, ReleaseEvent, ReleasePlanSummary, ReleasePreflightResult, ReleaseSession,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationRequest {
    Inspect {
        repository_path: String,
    },
    PreparePlan {
        repository_path: String,
        target_version: String,
        notes: Option<String>,
    },
    Start {
        plan_id: String,
    },
    GetSession {
        repository_path: String,
    },
    Resume {
        session_id: String,
    },
    Cancel {
        session_id: String,
    },
    Publish {
        session_id: String,
        expected_draft_identity: DraftIdentity,
    },
    ExportSummary {
        session_id: String,
        destination_path: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationResponse {
    Inspection(ReleasePreflightResult),
    Plan(ReleasePlanSummary),
    Session(ReleaseSession),
    OptionalSession(Option<ReleaseSession>),
    SummaryPath(String),
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct ReleaseApplicationError {
    pub code: String,
    pub message: String,
}

impl ReleaseApplicationError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait ReleaseEventSink: Send + Sync {
    fn send(&self, event: ReleaseEvent) -> Result<(), String>;
}

pub trait ReleaseApplicationBackend: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: ApplicationRequest,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Pin<
        Box<dyn Future<Output = Result<ApplicationResponse, ReleaseApplicationError>> + Send + 'a>,
    >;
}

pub struct AppState {
    backend: Arc<dyn ReleaseApplicationBackend>,
}

impl AppState {
    pub fn new(backend: Arc<dyn ReleaseApplicationBackend>) -> Self {
        Self { backend }
    }

    pub async fn execute(
        &self,
        request: ApplicationRequest,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Result<ApplicationResponse, ReleaseApplicationError> {
        self.backend.execute(request, events).await
    }

    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableApplication))
    }

    pub fn system() -> Self {
        Self::new(Arc::new(
            crate::services::release_application::SystemReleaseApplication::new(),
        ))
    }
}

struct UnavailableApplication;

impl ReleaseApplicationBackend for UnavailableApplication {
    fn execute<'a>(
        &'a self,
        _request: ApplicationRequest,
        _events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Pin<
        Box<dyn Future<Output = Result<ApplicationResponse, ReleaseApplicationError>> + Send + 'a>,
    > {
        Box::pin(async {
            Err(ReleaseApplicationError::new(
                "RELEASE_BACKEND_UNAVAILABLE",
                "发布控制台后端尚未初始化。",
            ))
        })
    }
}
