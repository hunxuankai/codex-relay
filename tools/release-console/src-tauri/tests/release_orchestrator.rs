use codex_relay_release_console_lib::models::{
    CleanupRunEvidence, DraftAssetEvidence, DraftAuditEvidence, DraftIdentity,
    PublishedReleaseEvidence, ReleaseFailureEvidence, ReleaseLogLevel, ReleaseLogSource,
    ReleasePhase, ReleaseSession, WorkflowDispatch, WorkflowRunStatus,
};
use codex_relay_release_console_lib::services::git_release::GitPushOutcome;
use codex_relay_release_console_lib::services::local_verification::{
    LocalCommandEvidence, LocalVerificationBackend, LocalVerificationBackendError,
    LocalVerificationCommand, LocalVerificationFailure, LocalVerificationProcessError,
};
use codex_relay_release_console_lib::services::release_candidate::{
    ReleaseCandidatePlan, ReleaseCandidateTransaction,
};
use codex_relay_release_console_lib::services::release_log::{
    ReleaseLogRecorder, ReleaseLogStore, ReleaseProgressSink,
};
use codex_relay_release_console_lib::services::release_orchestrator::{
    ReleaseOrchestrator, ReleaseOrchestratorError, ReleasePushBackend, ReleaseRemoteBackend,
};
use codex_relay_release_console_lib::services::release_state::ReleaseStateStore;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);
const VALID_RELEASE_NOTES: &str = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";

struct TempRepository {
    root: PathBuf,
}

impl TempRepository {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-relay-release-orchestrator-{}-{id}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src-tauri/crates/codex-relay-core")).unwrap();
        fs::create_dir_all(root.join(".github")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        write_fixture(&root);
        Self { root }
    }
}

impl Drop for TempRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_fixture(root: &Path) {
    fs::write(
        root.join("package.json"),
        "{\n  \"name\": \"codex-relay\",\n  \"version\": \"0.4.0\"\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("package-lock.json"),
        "{\n  \"name\": \"codex-relay\",\n  \"version\": \"0.4.0\",\n  \"packages\": {\n    \"\": { \"version\": \"0.4.0\" }\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("src-tauri/Cargo.toml"),
        "[package]\nname = \"codex-relay\"\nversion = \"0.4.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src-tauri/crates/codex-relay-core/Cargo.toml"),
        "[package]\nname = \"codex-relay-core\"\nversion = \"0.4.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src-tauri/Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"codex-relay\"\nversion = \"0.4.0\"\n\n[[package]]\nname = \"codex-relay-core\"\nversion = \"0.4.0\"\n",
    )
    .unwrap();
    fs::write(root.join(".github/release-notes.md"), "旧说明\n").unwrap();
}

struct FailingVerificationBackend;

impl LocalVerificationBackend for FailingVerificationBackend {
    fn run<'a>(
        &'a self,
        _repository_path: &'a Path,
        command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            Ok(LocalCommandEvidence {
                id: command.id.clone(),
                exit_code: 1,
                duration_millis: 10,
            })
        })
    }
}

struct LoggingFailingVerificationBackend {
    recorder: Arc<ReleaseLogRecorder>,
}

impl LocalVerificationBackend for LoggingFailingVerificationBackend {
    fn run<'a>(
        &'a self,
        _repository_path: &'a Path,
        command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        self.recorder.record(
            command.id.clone(),
            ReleaseLogSource::Stderr,
            ReleaseLogLevel::Info,
            "compiler failure tail",
        );
        Box::pin(async move {
            Ok(LocalCommandEvidence {
                id: command.id.clone(),
                exit_code: 1,
                duration_millis: 10,
            })
        })
    }
}

struct CancelledVerificationBackend;

impl LocalVerificationBackend for CancelledVerificationBackend {
    fn run<'a>(
        &'a self,
        _repository_path: &'a Path,
        _command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async { Err(LocalVerificationBackendError::Cancelled) })
    }
}

struct TimedOutVerificationBackend;

impl LocalVerificationBackend for TimedOutVerificationBackend {
    fn run<'a>(
        &'a self,
        _repository_path: &'a Path,
        _command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async {
            Err(LocalVerificationBackendError::Process(
                LocalVerificationProcessError::Timeout,
            ))
        })
    }
}

struct UnexpectedPushBackend {
    called: AtomicBool,
}

struct SuccessfulVerificationBackend;

impl LocalVerificationBackend for SuccessfulVerificationBackend {
    fn run<'a>(
        &'a self,
        _repository_path: &'a Path,
        command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            Ok(LocalCommandEvidence {
                id: command.id.clone(),
                exit_code: 0,
                duration_millis: 10,
            })
        })
    }
}

struct SuccessfulPushBackend;

struct SuccessfulRemoteBackend {
    dispatch_calls: AtomicU64,
    publish_calls: AtomicU64,
    cleanup_succeeds: bool,
}

fn remote_draft() -> DraftAuditEvidence {
    DraftAuditEvidence {
        release_id: 42,
        tag_name: "v0.5.0".into(),
        target_commit_sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        assets: vec![DraftAssetEvidence {
            id: 501,
            name: "Codex.Relay_0.5.0_x64-setup.exe".into(),
            size: 9,
            sha256: "9c0d294c05fc1d88d698034609bb81c0c69196327594e4c69d2915c80fd9850c".into(),
        }],
        manifest_version: "0.5.0".into(),
        manifest_notes: VALID_RELEASE_NOTES.into(),
        signature: "signature-test-not-real".into(),
    }
}

impl ReleaseRemoteBackend for SuccessfulRemoteBackend {
    fn dispatch<'a>(
        &'a self,
        _target_version: &'a str,
        _candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<WorkflowDispatch, String>> + Send + 'a>> {
        self.dispatch_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(WorkflowDispatch {
                run_id: 123,
                url: "https://github.com/hunxuankai/codex-relay/actions/runs/123".into(),
            })
        })
    }

    fn wait_for_run<'a>(
        &'a self,
        workflow: &'a WorkflowDispatch,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<WorkflowRunStatus, String>> + Send + 'a>> {
        Box::pin(async move {
            Ok(WorkflowRunStatus {
                id: workflow.run_id,
                status: "completed".into(),
                conclusion: Some("success".into()),
                head_sha: candidate_sha.into(),
                url: workflow.url.clone(),
                jobs: Vec::new(),
            })
        })
    }

    fn audit_draft<'a>(
        &'a self,
        _target_version: &'a str,
        _candidate_sha: &'a str,
        _expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<DraftAuditEvidence, String>> + Send + 'a>> {
        Box::pin(async { Ok(remote_draft()) })
    }

    fn publish<'a>(
        &'a self,
        _expected_draft: &'a DraftAuditEvidence,
        _target_version: &'a str,
        _candidate_sha: &'a str,
        _expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<PublishedReleaseEvidence, String>> + Send + 'a>> {
        self.publish_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Ok(PublishedReleaseEvidence {
                release_id: 42,
                tag_name: "v0.5.0".into(),
                published_at: "2026-07-31T11:00:00Z".into(),
            })
        })
    }

    fn verify_published<'a>(
        &'a self,
        expected_draft: &'a DraftAuditEvidence,
        _published: &'a PublishedReleaseEvidence,
        _target_version: &'a str,
        _candidate_sha: &'a str,
        _expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<DraftAuditEvidence, String>> + Send + 'a>> {
        Box::pin(async move { Ok(expected_draft.clone()) })
    }

    fn monitor_cleanup<'a>(
        &'a self,
        _published_at: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CleanupRunEvidence, String>> + Send + 'a>> {
        let succeeded = self.cleanup_succeeds;
        Box::pin(async move {
            Ok(CleanupRunEvidence {
                run_id: 900,
                url: "https://github.com/hunxuankai/codex-relay/actions/runs/900".into(),
                status: "completed".into(),
                conclusion: Some(if succeeded { "success" } else { "failure" }.into()),
                succeeded,
                jobs: Vec::new(),
            })
        })
    }
}

impl ReleasePushBackend for SuccessfulPushBackend {
    fn commit<'a>(
        &'a self,
        _repository_path: &'a Path,
        _plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Ok("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()) })
    }

    fn push<'a>(
        &'a self,
        _repository_path: &'a Path,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GitPushOutcome, String>> + Send + 'a>> {
        Box::pin(async move {
            Ok(GitPushOutcome {
                candidate_sha: candidate_sha.into(),
                remote_main_sha: candidate_sha.into(),
            })
        })
    }
}

impl ReleasePushBackend for UnexpectedPushBackend {
    fn commit<'a>(
        &'a self,
        _repository_path: &'a Path,
        _plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        self.called.store(true, Ordering::SeqCst);
        Box::pin(async { Err("commit must not be called".into()) })
    }

    fn push<'a>(
        &'a self,
        _repository_path: &'a Path,
        _candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GitPushOutcome, String>> + Send + 'a>> {
        self.called.store(true, Ordering::SeqCst);
        Box::pin(async { Err("push must not be called".into()) })
    }
}

struct CommitSucceedsPushFailsBackend;

impl ReleasePushBackend for CommitSucceedsPushFailsBackend {
    fn commit<'a>(
        &'a self,
        _repository_path: &'a Path,
        _plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Ok("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()) })
    }

    fn push<'a>(
        &'a self,
        _repository_path: &'a Path,
        _candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GitPushOutcome, String>> + Send + 'a>> {
        Box::pin(async { Err("simulated push failure".into()) })
    }
}

struct CommitFailsBeforeCheckpointBackend {
    rollback_called: AtomicBool,
}

impl ReleasePushBackend for CommitFailsBeforeCheckpointBackend {
    fn commit<'a>(
        &'a self,
        _repository_path: &'a Path,
        _plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Err("simulated commit failure".into()) })
    }

    fn rollback_uncommitted<'a>(
        &'a self,
        _repository_path: &'a Path,
        _plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        self.rollback_called.store(true, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }

    fn push<'a>(
        &'a self,
        _repository_path: &'a Path,
        _candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GitPushOutcome, String>> + Send + 'a>> {
        Box::pin(async { Err("push must not be called".into()) })
    }
}

struct RetryCommittedBackend {
    commit_called: AtomicBool,
    push_called: AtomicBool,
}

impl ReleasePushBackend for RetryCommittedBackend {
    fn commit<'a>(
        &'a self,
        _repository_path: &'a Path,
        _plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        self.commit_called.store(true, Ordering::SeqCst);
        Box::pin(async { Err("commit must not be repeated".into()) })
    }

    fn push<'a>(
        &'a self,
        _repository_path: &'a Path,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GitPushOutcome, String>> + Send + 'a>> {
        self.push_called.store(true, Ordering::SeqCst);
        Box::pin(async move {
            Ok(GitPushOutcome {
                candidate_sha: candidate_sha.into(),
                remote_main_sha: candidate_sha.into(),
            })
        })
    }
}

#[test]
fn local_failure_before_commit_rolls_back_candidate_and_persists_failed_phase() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session =
        ReleaseSession::new("session-test-1", repository.root.to_string_lossy(), "0.5.0");
    let push = UnexpectedPushBackend {
        called: AtomicBool::new(false),
    };

    let error = tauri::async_runtime::block_on(ReleaseOrchestrator::new().run_to_pushed(
        &mut session,
        &store,
        &repository.root,
        &git_dir,
        &plan,
        &FailingVerificationBackend,
        &push,
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        ReleaseOrchestratorError::LocalVerificationFailed {
            command_id,
            failure: LocalVerificationFailure::ExitCode(1),
        } if command_id == "release-structure-tests"
    ));
    assert!(!push.called.load(Ordering::SeqCst));
    assert_eq!(session.phase, ReleasePhase::Failed);
    let persisted = store.load().unwrap().unwrap();
    assert_eq!(persisted.phase, ReleasePhase::Failed);
    assert_eq!(
        persisted.failure,
        Some(ReleaseFailureEvidence {
            phase: ReleasePhase::LocalChecks,
            step_id: "release-structure-tests".into(),
            code: "RELEASE_LOCAL_VERIFICATION_FAILED".into(),
        })
    );
    for file in &plan.files {
        assert_eq!(
            fs::read(repository.root.join(&file.relative_path)).unwrap(),
            file.before
        );
    }
    assert!(
        !git_dir
            .join("codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

#[test]
fn local_failure_logs_output_tail_before_stable_failure_and_stops_later_steps() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let state_store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session = ReleaseSession::new(
        "session-test-log-order",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    let log_store = ReleaseLogStore::new(git_dir.clone());
    log_store.initialize(&session.id).unwrap();
    let recorder = Arc::new(ReleaseLogRecorder::new(
        session.id.clone(),
        log_store,
        0,
        None,
    ));
    let verification = LoggingFailingVerificationBackend {
        recorder: Arc::clone(&recorder),
    };
    let push = UnexpectedPushBackend {
        called: AtomicBool::new(false),
    };
    let orchestrator =
        ReleaseOrchestrator::new().with_progress(recorder.clone() as Arc<dyn ReleaseProgressSink>);

    let error = tauri::async_runtime::block_on(orchestrator.run_to_pushed(
        &mut session,
        &state_store,
        &repository.root,
        &git_dir,
        &plan,
        &verification,
        &push,
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        ReleaseOrchestratorError::LocalVerificationFailed {
            failure: LocalVerificationFailure::ExitCode(1),
            ..
        }
    ));
    let entries = ReleaseLogStore::new(git_dir)
        .load_page(&session.id, None)
        .unwrap()
        .entries;
    let tail = entries
        .iter()
        .position(|entry| entry.message == "compiler failure tail")
        .expect("process tail should be retained");
    let failure = entries
        .iter()
        .position(|entry| {
            entry.step_id == "release-structure-tests"
                && entry.level == ReleaseLogLevel::Error
                && entry.message.contains("退出码 1")
        })
        .expect("stable local failure should be logged");
    assert!(tail < failure);
    assert!(!entries.iter().any(|entry| {
        matches!(
            entry.step_id.as_str(),
            "release-console-rust-tests"
                | "full-project-check"
                | "ordinary-build"
                | "sourceAudit"
                | "commitPush"
        )
    }));
}

#[test]
fn local_process_failure_rolls_back_candidate_and_preserves_safe_classification() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session = ReleaseSession::new(
        "session-test-process-failure",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    let push = UnexpectedPushBackend {
        called: AtomicBool::new(false),
    };

    let error = tauri::async_runtime::block_on(ReleaseOrchestrator::new().run_to_pushed(
        &mut session,
        &store,
        &repository.root,
        &git_dir,
        &plan,
        &TimedOutVerificationBackend,
        &push,
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        ReleaseOrchestratorError::LocalVerificationFailed {
            command_id,
            failure: LocalVerificationFailure::Process(
                LocalVerificationProcessError::Timeout
            ),
        } if command_id == "release-structure-tests"
    ));
    assert!(!push.called.load(Ordering::SeqCst));
    assert_eq!(session.phase, ReleasePhase::Failed);
    for file in &plan.files {
        assert_eq!(
            fs::read(repository.root.join(&file.relative_path)).unwrap(),
            file.before
        );
    }
}

#[test]
fn commit_failure_unstages_and_rolls_back_candidate_before_marking_failed() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session = ReleaseSession::new(
        "session-test-commit-failure",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    let push = CommitFailsBeforeCheckpointBackend {
        rollback_called: AtomicBool::new(false),
    };

    let error = tauri::async_runtime::block_on(ReleaseOrchestrator::new().run_to_pushed(
        &mut session,
        &store,
        &repository.root,
        &git_dir,
        &plan,
        &SuccessfulVerificationBackend,
        &push,
    ))
    .unwrap_err();

    assert!(matches!(error, ReleaseOrchestratorError::PushFailed));
    assert!(push.rollback_called.load(Ordering::SeqCst));
    assert_eq!(session.phase, ReleasePhase::Failed);
    let persisted = store.load().unwrap().unwrap();
    assert_eq!(persisted.phase, ReleasePhase::Failed);
    assert_eq!(
        persisted.failure,
        Some(ReleaseFailureEvidence {
            phase: ReleasePhase::SourceAudit,
            step_id: "commitPush".into(),
            code: "RELEASE_PUSH_FAILED".into(),
        })
    );
    for file in &plan.files {
        assert_eq!(
            fs::read(repository.root.join(&file.relative_path)).unwrap(),
            file.before
        );
    }
    assert!(
        !git_dir
            .join("codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

#[test]
fn pushed_session_keeps_candidate_bytes_and_removes_rollback_marker() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session =
        ReleaseSession::new("session-test-1", repository.root.to_string_lossy(), "0.5.0");

    let outcome = tauri::async_runtime::block_on(ReleaseOrchestrator::new().run_to_pushed(
        &mut session,
        &store,
        &repository.root,
        &git_dir,
        &plan,
        &SuccessfulVerificationBackend,
        &SuccessfulPushBackend,
    ))
    .unwrap();

    assert_eq!(session.phase, ReleasePhase::Pushed);
    assert_eq!(
        session.candidate_sha.as_deref(),
        Some(outcome.candidate_sha.as_str())
    );
    assert_eq!(
        session.remote_main_sha.as_deref(),
        Some(outcome.remote_main_sha.as_str())
    );
    for file in &plan.files {
        assert_eq!(
            fs::read(repository.root.join(&file.relative_path)).unwrap(),
            file.after
        );
    }
    assert!(
        !git_dir
            .join("codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
    assert_eq!(store.load().unwrap().unwrap(), session);
}

#[test]
fn successful_local_pipeline_logs_each_fixed_step_through_commit_and_push() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let state_store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session = ReleaseSession::new(
        "session-test-success-logs",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    let log_store = ReleaseLogStore::new(git_dir.clone());
    log_store.initialize(&session.id).unwrap();
    let recorder = Arc::new(ReleaseLogRecorder::new(
        session.id.clone(),
        log_store,
        0,
        None,
    ));
    let orchestrator =
        ReleaseOrchestrator::new().with_progress(recorder.clone() as Arc<dyn ReleaseProgressSink>);

    tauri::async_runtime::block_on(orchestrator.run_to_pushed(
        &mut session,
        &state_store,
        &repository.root,
        &git_dir,
        &plan,
        &SuccessfulVerificationBackend,
        &SuccessfulPushBackend,
    ))
    .unwrap();

    let entries = ReleaseLogStore::new(git_dir)
        .load_page(&session.id, None)
        .unwrap()
        .entries;
    for step_id in [
        "candidate",
        "release-structure-tests",
        "release-console-rust-tests",
        "full-project-check",
        "ordinary-build",
        "sourceAudit",
        "commitPush",
    ] {
        assert!(
            entries
                .iter()
                .filter(|entry| entry.step_id == step_id)
                .count()
                >= 2,
            "missing start/completion evidence for {step_id}"
        );
    }
    assert!(entries.iter().any(|entry| {
        entry.step_id == "commitPush"
            && entry.message.contains("aaaaaaaa")
            && !entry.message.contains("aaaaaaaaaaaaaaaa")
    }));
}

#[test]
fn push_failure_persists_the_committed_checkpoint_for_retry() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session = ReleaseSession::new(
        "session-test-commit",
        repository.root.to_string_lossy(),
        "0.5.0",
    );

    let error = tauri::async_runtime::block_on(ReleaseOrchestrator::new().run_to_pushed(
        &mut session,
        &store,
        &repository.root,
        &git_dir,
        &plan,
        &SuccessfulVerificationBackend,
        &CommitSucceedsPushFailsBackend,
    ))
    .unwrap_err();

    assert!(matches!(error, ReleaseOrchestratorError::PushFailed));
    assert_eq!(session.phase, ReleasePhase::Committed);
    assert_eq!(
        session.candidate_sha.as_deref(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(session.remote_main_sha, None);
    assert_eq!(store.load().unwrap().unwrap(), session);
    assert!(
        git_dir
            .join("codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

#[test]
fn committed_checkpoint_retries_only_the_push_and_then_finalizes() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session = ReleaseSession::new(
        "session-test-retry",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    let orchestrator = ReleaseOrchestrator::new();
    tauri::async_runtime::block_on(orchestrator.run_to_pushed(
        &mut session,
        &store,
        &repository.root,
        &git_dir,
        &plan,
        &SuccessfulVerificationBackend,
        &CommitSucceedsPushFailsBackend,
    ))
    .unwrap_err();
    let retry = RetryCommittedBackend {
        commit_called: AtomicBool::new(false),
        push_called: AtomicBool::new(false),
    };

    let outcome = tauri::async_runtime::block_on(orchestrator.push_committed(
        &mut session,
        &store,
        &repository.root,
        &git_dir,
        &retry,
    ))
    .unwrap();

    assert!(!retry.commit_called.load(Ordering::SeqCst));
    assert!(retry.push_called.load(Ordering::SeqCst));
    assert_eq!(session.phase, ReleasePhase::Pushed);
    assert_eq!(session.remote_main_sha, Some(outcome.remote_main_sha));
    assert_eq!(store.load().unwrap().unwrap(), session);
    assert!(
        !git_dir
            .join("codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

#[test]
fn cancellation_before_commit_rolls_back_and_persists_cancelled_phase() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut session =
        ReleaseSession::new("session-test-1", repository.root.to_string_lossy(), "0.5.0");
    let push = UnexpectedPushBackend {
        called: AtomicBool::new(false),
    };

    let error = tauri::async_runtime::block_on(ReleaseOrchestrator::new().run_to_pushed(
        &mut session,
        &store,
        &repository.root,
        &git_dir,
        &plan,
        &CancelledVerificationBackend,
        &push,
    ))
    .unwrap_err();

    assert!(matches!(error, ReleaseOrchestratorError::Cancelled));
    assert_eq!(session.phase, ReleasePhase::Cancelled);
    assert!(!push.called.load(Ordering::SeqCst));
    for file in &plan.files {
        assert_eq!(
            fs::read(repository.root.join(&file.relative_path)).unwrap(),
            file.before
        );
    }
}

#[test]
fn restarted_local_session_can_cancel_using_persisted_state_and_marker() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let plan =
        ReleaseCandidateTransaction::plan(&repository.root, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    let mut original =
        ReleaseSession::new("session-test-1", repository.root.to_string_lossy(), "0.5.0");
    for phase in [
        ReleasePhase::Inspected,
        ReleasePhase::Planned,
        ReleasePhase::ApplyingCandidate,
    ] {
        store.advance(&mut original, phase).unwrap();
    }
    ReleaseCandidateTransaction::apply(&repository.root, &git_dir, &plan).unwrap();
    store
        .advance(&mut original, ReleasePhase::LocalChecks)
        .unwrap();
    let mut resumed = store.load().unwrap().unwrap();

    ReleaseOrchestrator::new()
        .cancel_active(&mut resumed, &store, &repository.root, &git_dir)
        .unwrap();

    assert_eq!(resumed.phase, ReleasePhase::Cancelled);
    assert_eq!(
        store.load().unwrap().unwrap().phase,
        ReleasePhase::Cancelled
    );
    for file in &plan.files {
        assert_eq!(
            fs::read(repository.root.join(&file.relative_path)).unwrap(),
            file.before
        );
    }
}

#[test]
fn remote_pipeline_persists_run_and_draft_then_stops_for_publish_approval() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let mut session = ReleaseSession::new(
        "session-test-remote",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    session.phase = ReleasePhase::Pushed;
    session.candidate_sha = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
    session.remote_main_sha = session.candidate_sha.clone();
    store.save(&session).unwrap();
    let remote = SuccessfulRemoteBackend {
        dispatch_calls: AtomicU64::new(0),
        publish_calls: AtomicU64::new(0),
        cleanup_succeeds: true,
    };
    let log_store = ReleaseLogStore::new(git_dir.clone());
    log_store.initialize(&session.id).unwrap();
    let recorder = Arc::new(ReleaseLogRecorder::new(
        session.id.clone(),
        log_store,
        0,
        None,
    ));
    let orchestrator =
        ReleaseOrchestrator::new().with_progress(recorder.clone() as Arc<dyn ReleaseProgressSink>);

    let draft = tauri::async_runtime::block_on(orchestrator.run_remote_to_draft(
        &mut session,
        &store,
        &git_dir,
        VALID_RELEASE_NOTES,
        &remote,
    ))
    .unwrap();

    assert_eq!(draft.release_id, 42);
    assert_eq!(session.phase, ReleasePhase::AwaitingPublishApproval);
    assert_eq!(session.workflow.as_ref().unwrap().run_id, 123);
    assert_eq!(session.draft.as_ref(), Some(&draft));
    assert_eq!(remote.dispatch_calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.load().unwrap().unwrap(), session);
    let entries = ReleaseLogStore::new(git_dir)
        .load_page(&session.id, None)
        .unwrap()
        .entries;
    for step_id in ["remoteRun", "draftAudit"] {
        assert!(
            entries
                .iter()
                .filter(|entry| entry.step_id == step_id)
                .count()
                >= 2,
            "missing start/completion evidence for {step_id}"
        );
    }
    let public_text = entries
        .iter()
        .map(|entry| entry.message.as_str())
        .collect::<String>();
    assert!(public_text.contains("Run 123"));
    assert!(public_text.contains("SHA aaaaaaaa"));
    assert!(public_text.contains("Release 42"));
    assert!(public_text.contains("v0.5.0"));
    assert!(!public_text.contains("aaaaaaaaaaaaaaaa"));
}

#[test]
fn cleanup_failure_finishes_with_warnings_without_losing_published_evidence() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let mut session = ReleaseSession::new(
        "session-test-publish",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    session.phase = ReleasePhase::AwaitingPublishApproval;
    session.candidate_sha = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
    session.remote_main_sha = session.candidate_sha.clone();
    session.workflow = Some(WorkflowDispatch {
        run_id: 123,
        url: "https://github.com/hunxuankai/codex-relay/actions/runs/123".into(),
    });
    session.draft = Some(remote_draft());
    store.save(&session).unwrap();
    let identity = DraftIdentity {
        release_id: 42,
        tag_name: "v0.5.0".into(),
        target_commit_sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    };
    let remote = SuccessfulRemoteBackend {
        dispatch_calls: AtomicU64::new(0),
        publish_calls: AtomicU64::new(0),
        cleanup_succeeds: false,
    };
    let log_store = ReleaseLogStore::new(git_dir.clone());
    log_store.initialize(&session.id).unwrap();
    let recorder = Arc::new(ReleaseLogRecorder::new(
        session.id.clone(),
        log_store,
        0,
        None,
    ));
    let orchestrator =
        ReleaseOrchestrator::new().with_progress(recorder.clone() as Arc<dyn ReleaseProgressSink>);

    tauri::async_runtime::block_on(orchestrator.publish_and_finalize(
        &mut session,
        &store,
        &git_dir,
        &identity,
        VALID_RELEASE_NOTES,
        &remote,
    ))
    .unwrap();

    assert_eq!(session.phase, ReleasePhase::CompletedWithWarnings);
    assert_eq!(session.published.as_ref().unwrap().release_id, 42);
    assert!(!session.cleanup.as_ref().unwrap().succeeded);
    assert_eq!(remote.publish_calls.load(Ordering::SeqCst), 1);
    assert_eq!(store.load().unwrap().unwrap(), session);
    let entries = ReleaseLogStore::new(git_dir)
        .load_page(&session.id, None)
        .unwrap()
        .entries;
    for step_id in ["publishApproval", "onlineVerification", "cleanup"] {
        assert!(
            entries
                .iter()
                .filter(|entry| entry.step_id == step_id)
                .count()
                >= 2,
            "missing lifecycle evidence for {step_id}"
        );
    }
    assert!(entries.iter().any(|entry| {
        entry.step_id == "cleanup"
            && entry.level == ReleaseLogLevel::Warning
            && entry.message.contains("failure")
    }));
}

#[test]
fn resumed_workflow_session_reuses_the_persisted_run_without_dispatching_again() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let mut session = ReleaseSession::new(
        "session-test-resume",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    session.phase = ReleasePhase::WorkflowQueued;
    session.candidate_sha = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
    session.remote_main_sha = session.candidate_sha.clone();
    session.workflow = Some(WorkflowDispatch {
        run_id: 123,
        url: "https://github.com/hunxuankai/codex-relay/actions/runs/123".into(),
    });
    store.save(&session).unwrap();
    let remote = SuccessfulRemoteBackend {
        dispatch_calls: AtomicU64::new(0),
        publish_calls: AtomicU64::new(0),
        cleanup_succeeds: true,
    };

    tauri::async_runtime::block_on(ReleaseOrchestrator::new().run_remote_to_draft(
        &mut session,
        &store,
        &git_dir,
        VALID_RELEASE_NOTES,
        &remote,
    ))
    .unwrap();

    assert_eq!(remote.dispatch_calls.load(Ordering::SeqCst), 0);
    assert_eq!(session.phase, ReleasePhase::AwaitingPublishApproval);
}

#[test]
fn resumed_published_session_skips_publish_and_continues_online_verification() {
    let repository = TempRepository::new();
    let git_dir = repository.root.join(".git");
    let store = ReleaseStateStore::new(git_dir.clone());
    let mut session = ReleaseSession::new(
        "session-test-published",
        repository.root.to_string_lossy(),
        "0.5.0",
    );
    session.phase = ReleasePhase::VerifyingPublishedRelease;
    session.candidate_sha = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
    session.remote_main_sha = session.candidate_sha.clone();
    session.workflow = Some(WorkflowDispatch {
        run_id: 123,
        url: "https://github.com/hunxuankai/codex-relay/actions/runs/123".into(),
    });
    session.draft = Some(remote_draft());
    session.published = Some(PublishedReleaseEvidence {
        release_id: 42,
        tag_name: "v0.5.0".into(),
        published_at: "2026-07-31T11:00:00Z".into(),
    });
    store.save(&session).unwrap();
    let identity = session.draft.as_ref().unwrap().identity();
    let remote = SuccessfulRemoteBackend {
        dispatch_calls: AtomicU64::new(0),
        publish_calls: AtomicU64::new(0),
        cleanup_succeeds: true,
    };

    tauri::async_runtime::block_on(ReleaseOrchestrator::new().publish_and_finalize(
        &mut session,
        &store,
        &git_dir,
        &identity,
        VALID_RELEASE_NOTES,
        &remote,
    ))
    .unwrap();

    assert_eq!(remote.publish_calls.load(Ordering::SeqCst), 0);
    assert_eq!(session.phase, ReleasePhase::Completed);
    assert!(session.cleanup.as_ref().unwrap().succeeded);
}
