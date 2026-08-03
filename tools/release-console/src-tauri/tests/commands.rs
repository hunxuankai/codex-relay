use codex_relay_release_console_lib::app_state::{
    AppState, ApplicationRequest, ApplicationResponse, ReleaseApplicationBackend,
    ReleaseApplicationError, ReleaseEventSink,
};
use codex_relay_release_console_lib::commands::{
    get_release_logs_inner, get_release_session_inner, inspect_release_repository_inner,
    push_release_repository_inner, start_release_inner, test_release_connection_inner,
};
use codex_relay_release_console_lib::models::{
    ConnectionProbeResult, ExternalPreflightSnapshot, ReleaseConnectionTestResult, ReleaseEvent,
    ReleaseLogEntry, ReleaseLogLevel, ReleaseLogPage, ReleaseLogSource, ReleasePhase,
    ReleasePreflightResult, ReleaseProxySettings, ReleaseProxyType, ReleaseSession,
    ReleaseSessionSnapshot, RepositoryInspection, RepositorySyncInspection, RepositorySyncStatus,
    SafeRepositoryPushRequest, ToolchainInspection,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

struct FixtureApplication {
    requests: Mutex<Vec<ApplicationRequest>>,
    fail: bool,
}

fn fixture_inspection() -> ReleasePreflightResult {
    ReleasePreflightResult {
        repository_path: r"D:\safe-temp\repository".into(),
        repository: RepositoryInspection {
            local_branch: "master".into(),
            default_branch: "main".into(),
            head_sha: "a".repeat(40),
            remote_main_sha: "a".repeat(40),
            remote_url: "https://github.com/hunxuankai/codex-relay.git".into(),
            clean: true,
            sync: RepositorySyncInspection {
                status: RepositorySyncStatus::Synced,
                ahead_count: 0,
                behind_count: 0,
                ahead_commits: Vec::new(),
            },
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
        release_ready: true,
        blocking_reasons: Vec::new(),
        safe_push: None,
    }
}

fn fixture_log_page() -> ReleaseLogPage {
    ReleaseLogPage {
        entries: vec![ReleaseLogEntry {
            session_id: "session-1".into(),
            sequence: 42,
            timestamp: "2026-08-03T12:00:00.000Z".into(),
            step_id: "full-project-check".into(),
            source: ReleaseLogSource::Stderr,
            level: ReleaseLogLevel::Error,
            message: "测试失败上下文".into(),
        }],
        next_before_sequence: Some(42),
        has_earlier: true,
        total_entries: 42,
        total_bytes: 4_096,
        truncated: false,
        warning: None,
    }
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
                    Ok(ApplicationResponse::Inspection(fixture_inspection()))
                }
                ApplicationRequest::PushRepository { .. } => {
                    Ok(ApplicationResponse::Inspection(fixture_inspection()))
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
                ApplicationRequest::GetSession { .. } => Ok(ApplicationResponse::OptionalSnapshot(
                    Some(ReleaseSessionSnapshot {
                        session: ReleaseSession::new(
                            "session-1",
                            r"D:\safe-temp\repository",
                            "0.5.0",
                        ),
                        logs: fixture_log_page(),
                    }),
                )),
                ApplicationRequest::GetLogs { .. } => {
                    Ok(ApplicationResponse::Logs(fixture_log_page()))
                }
                ApplicationRequest::TestConnection { .. } => Ok(
                    ApplicationResponse::ConnectionTest(ReleaseConnectionTestResult {
                        git: ConnectionProbeResult {
                            success: true,
                            code: None,
                            message: "Git 远端连接正常。".into(),
                            duration_millis: 12,
                        },
                        github: ConnectionProbeResult {
                            success: false,
                            code: Some("GITHUB_PROCESS_TIMEOUT".into()),
                            message: "GitHub API 连接超时。".into(),
                            duration_millis: 30_000,
                        },
                    }),
                ),
                _ => Err(ReleaseApplicationError::new(
                    "RELEASE_TEST_UNEXPECTED_REQUEST",
                    "测试收到了未预期请求。",
                )),
            }
        })
    }
}

#[test]
fn session_and_log_commands_make_one_exact_application_request_each() {
    let backend = Arc::new(FixtureApplication {
        requests: Mutex::new(Vec::new()),
        fail: false,
    });
    let state = AppState::new(backend.clone());

    let snapshot = tauri::async_runtime::block_on(get_release_session_inner(
        &state,
        r"D:\safe-temp\repository".into(),
    ));
    assert!(snapshot.success);
    let snapshot = snapshot.data.unwrap().unwrap();
    assert_eq!(snapshot.session.id, "session-1");
    assert_eq!(snapshot.logs.entries[0].sequence, 42);

    let page = tauri::async_runtime::block_on(get_release_logs_inner(
        &state,
        "session-1".into(),
        Some(42),
    ));
    assert!(page.success);
    assert_eq!(page.data.unwrap().total_entries, 42);

    assert_eq!(
        backend.requests.lock().unwrap().as_slice(),
        [
            ApplicationRequest::GetSession {
                repository_path: r"D:\safe-temp\repository".into(),
            },
            ApplicationRequest::GetLogs {
                session_id: "session-1".into(),
                before_sequence: Some(42),
            },
        ]
    );
}

#[test]
fn connection_test_command_passes_typed_proxy_settings_and_preserves_both_results() {
    let backend = Arc::new(FixtureApplication {
        requests: Mutex::new(Vec::new()),
        fail: false,
    });
    let state = AppState::new(backend.clone());
    let proxy = ReleaseProxySettings {
        enabled: true,
        proxy_type: ReleaseProxyType::Socks5,
        host: "127.0.0.1".into(),
        port: Some(1080),
    };

    let result =
        tauri::async_runtime::block_on(test_release_connection_inner(&state, proxy.clone()));

    assert!(result.success);
    let result = result.data.unwrap();
    assert!(result.git.success);
    assert_eq!(
        result.github.code.as_deref(),
        Some("GITHUB_PROCESS_TIMEOUT")
    );
    assert_eq!(
        backend.requests.lock().unwrap().as_slice(),
        [ApplicationRequest::TestConnection { proxy }]
    );
}

#[test]
fn safe_push_command_passes_only_the_repository_two_expected_shas_and_proxy() {
    let backend = Arc::new(FixtureApplication {
        requests: Mutex::new(Vec::new()),
        fail: false,
    });
    let state = AppState::new(backend.clone());
    let request = SafeRepositoryPushRequest {
        repository_path: r"D:\safe-temp\repository".into(),
        expected_head_sha: "b".repeat(40),
        expected_remote_main_sha: "a".repeat(40),
        proxy: ReleaseProxySettings {
            enabled: true,
            proxy_type: ReleaseProxyType::Http,
            host: "127.0.0.1".into(),
            port: Some(7890),
        },
    };

    let result =
        tauri::async_runtime::block_on(push_release_repository_inner(&state, request.clone()));

    assert!(result.success);
    assert_eq!(
        backend.requests.lock().unwrap().as_slice(),
        [ApplicationRequest::PushRepository { request }]
    );
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
    let proxy = ReleaseProxySettings {
        enabled: true,
        proxy_type: ReleaseProxyType::Socks5,
        host: "127.0.0.1".into(),
        port: Some(1080),
    };

    let inspection = tauri::async_runtime::block_on(inspect_release_repository_inner(
        &state,
        r"D:\safe-temp\repository".into(),
        proxy.clone(),
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
    let started = tauri::async_runtime::block_on(start_release_inner(
        &state,
        "plan-1".into(),
        proxy.clone(),
        sink.clone(),
    ));
    assert!(started.success);
    assert_eq!(started.data.unwrap().phase, ReleasePhase::Idle);
    assert!(matches!(
        sink.events.lock().unwrap().as_slice(),
        [ReleaseEvent::StepStarted { step_id, .. }] if step_id == "preflight"
    ));
    assert_eq!(
        backend.requests.lock().unwrap().as_slice(),
        [
            ApplicationRequest::Inspect {
                repository_path: r"D:\safe-temp\repository".into(),
                proxy: proxy.clone(),
            },
            ApplicationRequest::Start {
                plan_id: "plan-1".into(),
                proxy: proxy.clone(),
            },
        ]
    );

    let failing = AppState::new(Arc::new(FixtureApplication {
        requests: Mutex::new(Vec::new()),
        fail: true,
    }));
    let failed = tauri::async_runtime::block_on(inspect_release_repository_inner(
        &failing,
        r"D:\safe-temp\repository".into(),
        proxy,
    ));
    assert!(!failed.success);
    assert_eq!(failed.error.unwrap().code, "GIT_WORKTREE_DIRTY");
}
