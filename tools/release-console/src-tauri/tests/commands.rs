use codex_relay_release_console_lib::app_state::{
    AppState, ApplicationRequest, ApplicationResponse, ReleaseApplicationBackend,
    ReleaseApplicationError, ReleaseEventSink,
};
use codex_relay_release_console_lib::commands::{
    inspect_release_repository_inner, start_release_inner,
};
use codex_relay_release_console_lib::models::{
    ExternalPreflightSnapshot, ReleaseEvent, ReleasePhase, ReleasePreflightResult, ReleaseSession,
    RepositoryInspection, ToolchainInspection,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

struct FixtureApplication {
    requests: Mutex<Vec<ApplicationRequest>>,
    fail: bool,
}

impl ReleaseApplicationBackend for FixtureApplication {
    fn execute<'a>(
        &'a self,
        request: ApplicationRequest,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Pin<
        Box<dyn Future<Output = Result<ApplicationResponse, ReleaseApplicationError>> + Send + 'a>,
    > {
        self.requests.lock().unwrap().push(request.clone());
        Box::pin(async move {
            if self.fail {
                return Err(ReleaseApplicationError::new(
                    "GIT_WORKTREE_DIRTY",
                    "Git 工作区存在未提交改动。",
                ));
            }
            match request {
                ApplicationRequest::Inspect { .. } => {
                    Ok(ApplicationResponse::Inspection(ReleasePreflightResult {
                        repository_path: r"D:\safe-temp\repository".into(),
                        repository: RepositoryInspection {
                            local_branch: "master".into(),
                            default_branch: "main".into(),
                            head_sha: "a".repeat(40),
                            remote_main_sha: "a".repeat(40),
                            remote_url: "https://github.com/hunxuankai/codex-relay.git".into(),
                            clean: true,
                        },
                        external: ExternalPreflightSnapshot {
                            tools: ToolchainInspection {
                                git: Some("2.50".into()),
                                node: Some("24".into()),
                                npm: Some("11".into()),
                                cargo: Some("1.90".into()),
                                gh: Some("2.76".into()),
                            },
                            active_release_runs: 0,
                            conflicting_drafts: 0,
                            latest_release_tag: Some("v0.4.0".into()),
                        },
                    }))
                }
                ApplicationRequest::Start { .. } => {
                    let session =
                        ReleaseSession::new("session-1", r"D:\safe-temp\repository", "0.5.0");
                    if let Some(events) = events {
                        events
                            .send(ReleaseEvent::StepStarted {
                                step_id: "preflight".into(),
                                started_at: "2026-07-31T10:00:00Z".into(),
                            })
                            .unwrap();
                    }
                    Ok(ApplicationResponse::Session(session))
                }
                _ => Err(ReleaseApplicationError::new(
                    "RELEASE_TEST_UNEXPECTED_REQUEST",
                    "测试收到了未预期请求。",
                )),
            }
        })
    }
}

#[derive(Default)]
struct MemoryEventSink {
    events: Mutex<Vec<ReleaseEvent>>,
}

impl ReleaseEventSink for MemoryEventSink {
    fn send(&self, event: ReleaseEvent) -> Result<(), String> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[test]
fn typed_command_adapters_call_the_application_once_and_preserve_events_and_errors() {
    let backend = Arc::new(FixtureApplication {
        requests: Mutex::new(Vec::new()),
        fail: false,
    });
    let state = AppState::new(backend.clone());

    let inspection = tauri::async_runtime::block_on(inspect_release_repository_inner(
        &state,
        r"D:\safe-temp\repository".into(),
    ));
    assert!(inspection.success);
    let inspection = inspection.data.unwrap();
    assert_eq!(inspection.repository_path, r"D:\safe-temp\repository");
    assert_eq!(inspection.repository.default_branch, "main");
    assert_eq!(
        inspection.external.latest_release_tag.as_deref(),
        Some("v0.4.0")
    );

    let sink = Arc::new(MemoryEventSink::default());
    let started =
        tauri::async_runtime::block_on(start_release_inner(&state, "plan-1".into(), sink.clone()));
    assert!(started.success);
    assert_eq!(started.data.unwrap().phase, ReleasePhase::Idle);
    assert!(matches!(
        sink.events.lock().unwrap().as_slice(),
        [ReleaseEvent::StepStarted { step_id, .. }] if step_id == "preflight"
    ));
    assert_eq!(backend.requests.lock().unwrap().len(), 2);

    let failing = AppState::new(Arc::new(FixtureApplication {
        requests: Mutex::new(Vec::new()),
        fail: true,
    }));
    let failed = tauri::async_runtime::block_on(inspect_release_repository_inner(
        &failing,
        r"D:\safe-temp\repository".into(),
    ));
    assert!(!failed.success);
    assert_eq!(failed.error.unwrap().code, "GIT_WORKTREE_DIRTY");
}
