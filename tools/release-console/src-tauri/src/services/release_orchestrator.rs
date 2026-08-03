use crate::infrastructure::gh::GhBackend;
use crate::infrastructure::git::GitBackend;
use crate::models::{
    CleanupRunEvidence, DraftAuditEvidence, DraftIdentity, PublishedReleaseEvidence, ReleasePhase,
    ReleaseSession, WorkflowDispatch, WorkflowRunStatus,
};
use crate::services::git_release::{GitPushOutcome, GitReleaseService};
use crate::services::github_release::{
    DraftAuditService, GithubReleaseService, REMOTE_MONITOR_ATTEMPTS, REMOTE_MONITOR_DELAY,
};
use crate::services::local_verification::{
    LocalVerificationBackend, LocalVerificationError, LocalVerificationFailure,
    LocalVerificationProcessError, LocalVerificationService,
};
use crate::services::release_candidate::{ReleaseCandidatePlan, ReleaseCandidateTransaction};
use crate::services::release_log::{
    NoopReleaseProgressSink, ReleaseProgressSink, ReleaseRunProgressDecision,
    ReleaseRunProgressTracker, format_run_progress,
};
use crate::services::release_state::{ReleaseStateStore, RepositorySessionLock};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

pub trait ReleasePushBackend: Send + Sync {
    fn commit<'a>(
        &'a self,
        repository_path: &'a Path,
        plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

    fn rollback_uncommitted<'a>(
        &'a self,
        _repository_path: &'a Path,
        _plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }

    fn push<'a>(
        &'a self,
        repository_path: &'a Path,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GitPushOutcome, String>> + Send + 'a>>;
}

pub struct GitReleasePushBackend {
    backend: GitBackend,
    expected_remote_sha: String,
}

impl GitReleasePushBackend {
    pub fn new(backend: GitBackend, expected_remote_sha: impl Into<String>) -> Self {
        Self {
            backend,
            expected_remote_sha: expected_remote_sha.into(),
        }
    }

    pub fn for_committed(backend: GitBackend) -> Self {
        Self {
            backend,
            expected_remote_sha: String::new(),
        }
    }
}

impl ReleasePushBackend for GitReleasePushBackend {
    fn commit<'a>(
        &'a self,
        repository_path: &'a Path,
        plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            GitReleaseService::new("main")
                .commit_candidate(
                    &self.backend,
                    repository_path,
                    plan,
                    &self.expected_remote_sha,
                )
                .await
                .map_err(|error| error.code().to_string())
        })
    }

    fn rollback_uncommitted<'a>(
        &'a self,
        repository_path: &'a Path,
        plan: &'a ReleaseCandidatePlan,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            GitReleaseService::new("main")
                .unstage_candidate(&self.backend, repository_path, plan)
                .await
                .map_err(|error| error.code().to_string())
        })
    }

    fn push<'a>(
        &'a self,
        repository_path: &'a Path,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<GitPushOutcome, String>> + Send + 'a>> {
        Box::pin(async move {
            GitReleaseService::new("main")
                .push_candidate(&self.backend, repository_path, candidate_sha)
                .await
                .map_err(|error| error.code().to_string())
        })
    }
}

pub trait ReleaseRemoteBackend: Send + Sync {
    fn dispatch<'a>(
        &'a self,
        target_version: &'a str,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<WorkflowDispatch, String>> + Send + 'a>>;

    fn wait_for_run<'a>(
        &'a self,
        workflow: &'a WorkflowDispatch,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<WorkflowRunStatus, String>> + Send + 'a>>;

    fn audit_draft<'a>(
        &'a self,
        target_version: &'a str,
        candidate_sha: &'a str,
        expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<DraftAuditEvidence, String>> + Send + 'a>>;

    fn publish<'a>(
        &'a self,
        expected_draft: &'a DraftAuditEvidence,
        target_version: &'a str,
        candidate_sha: &'a str,
        expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<PublishedReleaseEvidence, String>> + Send + 'a>>;

    fn verify_published<'a>(
        &'a self,
        expected_draft: &'a DraftAuditEvidence,
        published: &'a PublishedReleaseEvidence,
        target_version: &'a str,
        candidate_sha: &'a str,
        expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<DraftAuditEvidence, String>> + Send + 'a>>;

    fn monitor_cleanup<'a>(
        &'a self,
        published_at: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CleanupRunEvidence, String>> + Send + 'a>>;
}

pub struct GithubRemoteBackend<'a> {
    backend: &'a dyn GhBackend,
    progress: Arc<dyn ReleaseProgressSink>,
}

impl<'a> GithubRemoteBackend<'a> {
    pub fn new(backend: &'a dyn GhBackend) -> Self {
        Self {
            backend,
            progress: Arc::new(NoopReleaseProgressSink),
        }
    }

    pub fn with_progress(mut self, progress: Arc<dyn ReleaseProgressSink>) -> Self {
        self.progress = progress;
        self
    }
}

impl ReleaseRemoteBackend for GithubRemoteBackend<'_> {
    fn dispatch<'a>(
        &'a self,
        target_version: &'a str,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<WorkflowDispatch, String>> + Send + 'a>> {
        Box::pin(async move {
            GithubReleaseService::new()
                .dispatch_release(self.backend, target_version, candidate_sha)
                .await
                .map_err(|error| error.code().to_string())
        })
    }

    fn wait_for_run<'a>(
        &'a self,
        workflow: &'a WorkflowDispatch,
        candidate_sha: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<WorkflowRunStatus, String>> + Send + 'a>> {
        Box::pin(async move {
            let started = Instant::now();
            let mut tracker = ReleaseRunProgressTracker::new();
            for attempt in 0..REMOTE_MONITOR_ATTEMPTS {
                let run = GithubReleaseService::new()
                    .get_release_run(self.backend, workflow.run_id, candidate_sha)
                    .await
                    .map_err(|error| error.code().to_string())?;
                let decision = tracker.observe(started.elapsed(), &run);
                if decision != ReleaseRunProgressDecision::Silent {
                    self.progress.log(
                        "remoteRun",
                        crate::models::ReleaseLogLevel::Info,
                        &format_run_progress(&run, decision),
                    );
                }
                if run.status == "completed" {
                    return Ok(run);
                }
                if attempt + 1 < REMOTE_MONITOR_ATTEMPTS {
                    tokio::time::sleep(REMOTE_MONITOR_DELAY).await;
                }
            }
            Err("GITHUB_RUN_TIMEOUT".into())
        })
    }

    fn audit_draft<'a>(
        &'a self,
        target_version: &'a str,
        candidate_sha: &'a str,
        expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<DraftAuditEvidence, String>> + Send + 'a>> {
        Box::pin(async move {
            DraftAuditService::new()
                .audit(self.backend, target_version, candidate_sha, expected_notes)
                .await
                .map_err(|error| error.code().to_string())
        })
    }

    fn publish<'a>(
        &'a self,
        expected_draft: &'a DraftAuditEvidence,
        target_version: &'a str,
        candidate_sha: &'a str,
        expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<PublishedReleaseEvidence, String>> + Send + 'a>> {
        Box::pin(async move {
            GithubReleaseService::new()
                .publish_release(
                    self.backend,
                    expected_draft,
                    target_version,
                    candidate_sha,
                    expected_notes,
                )
                .await
                .map_err(|error| error.code().to_string())
        })
    }

    fn verify_published<'a>(
        &'a self,
        expected_draft: &'a DraftAuditEvidence,
        published: &'a PublishedReleaseEvidence,
        target_version: &'a str,
        candidate_sha: &'a str,
        expected_notes: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<DraftAuditEvidence, String>> + Send + 'a>> {
        Box::pin(async move {
            GithubReleaseService::new()
                .verify_published_release(
                    self.backend,
                    expected_draft,
                    published,
                    target_version,
                    candidate_sha,
                    expected_notes,
                )
                .await
                .map_err(|error| error.code().to_string())
        })
    }

    fn monitor_cleanup<'a>(
        &'a self,
        published_at: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<CleanupRunEvidence, String>> + Send + 'a>> {
        Box::pin(async move {
            GithubReleaseService::new()
                .with_progress(Arc::clone(&self.progress))
                .monitor_cleanup(self.backend, published_at)
                .await
                .map_err(|error| error.code().to_string())
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReleaseOrchestratorError {
    #[error("无法锁定发布仓库")]
    SessionLockFailed,
    #[error("无法保存发布会话")]
    StateFailed,
    #[error("无法应用发布候选")]
    CandidateApplyFailed,
    #[error("本地发布门禁失败：{command_id}")]
    LocalVerificationFailed {
        command_id: String,
        failure: LocalVerificationFailure,
    },
    #[error("本地发布门禁失败，且候选未能完整回滚")]
    RollbackFailed,
    #[error("发布候选提交或推送失败")]
    PushFailed,
    #[error("远端已推送，但无法清理本地回滚标记")]
    FinalizeFailed,
    #[error("发布候选已取消")]
    Cancelled,
    #[error("发布候选已推送，不能执行本地回滚取消")]
    CancelAfterPushForbidden,
    #[error("远端发布阶段失败")]
    RemoteFailed,
    #[error("发布会话缺少恢复所需的远端证据")]
    RemoteStateInvalid,
    #[error("界面确认的 Draft 身份与会话证据不一致")]
    PublishIdentityMismatch,
}

impl ReleaseOrchestratorError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::SessionLockFailed => "RELEASE_SESSION_LOCK_FAILED",
            Self::StateFailed => "RELEASE_STATE_FAILED",
            Self::CandidateApplyFailed => "RELEASE_CANDIDATE_APPLY_FAILED",
            Self::LocalVerificationFailed { .. } => "RELEASE_LOCAL_VERIFICATION_FAILED",
            Self::RollbackFailed => "RELEASE_ROLLBACK_INCOMPLETE",
            Self::PushFailed => "RELEASE_PUSH_FAILED",
            Self::FinalizeFailed => "RELEASE_FINALIZE_FAILED",
            Self::Cancelled => "RELEASE_CANCELLED",
            Self::CancelAfterPushForbidden => "RELEASE_CANCEL_AFTER_PUSH_FORBIDDEN",
            Self::RemoteFailed => "RELEASE_REMOTE_FAILED",
            Self::RemoteStateInvalid => "RELEASE_REMOTE_STATE_INVALID",
            Self::PublishIdentityMismatch => "RELEASE_PUBLISH_IDENTITY_MISMATCH",
        }
    }

    pub(crate) fn failure_step_id(&self) -> &str {
        match self {
            Self::LocalVerificationFailed { command_id, .. } => command_id,
            Self::PublishIdentityMismatch => "publishApproval",
            _ => "releasePipeline",
        }
    }

    pub(crate) fn failure_message(&self) -> String {
        match self {
            Self::LocalVerificationFailed {
                failure: LocalVerificationFailure::ExitCode(exit_code),
                ..
            } => format!("本地发布门禁退出码 {exit_code}；候选文件已回滚，尚未提交或推送。"),
            Self::LocalVerificationFailed {
                failure: LocalVerificationFailure::Process(error),
                ..
            } => {
                let reason = match error {
                    LocalVerificationProcessError::JobUnavailable
                    | LocalVerificationProcessError::JobAssignment
                    | LocalVerificationProcessError::ProcessStart
                    | LocalVerificationProcessError::ProcessResume => {
                        "本地发布门禁进程无法安全启动"
                    }
                    LocalVerificationProcessError::OutputTooLarge => "本地发布门禁输出超过安全上限",
                    LocalVerificationProcessError::Timeout => "本地发布门禁超过允许时间",
                    LocalVerificationProcessError::ProcessTreeTermination => {
                        "本地发布门禁进程树未能安全结束"
                    }
                    LocalVerificationProcessError::OutputRead => "无法完整读取本地发布门禁结果",
                    LocalVerificationProcessError::InputTooLarge
                    | LocalVerificationProcessError::InputWrite => "本地发布门禁输入边界失败",
                };
                format!("{reason}；候选文件已回滚，尚未提交或推送。")
            }
            _ => "发布流程失败，请查看对应阶段证据。".into(),
        }
    }
}

pub struct ReleaseOrchestrator {
    local_verification: LocalVerificationService,
    progress: Arc<dyn ReleaseProgressSink>,
}

impl Default for ReleaseOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl ReleaseOrchestrator {
    pub fn new() -> Self {
        Self {
            local_verification: LocalVerificationService::new(),
            progress: Arc::new(NoopReleaseProgressSink),
        }
    }

    pub fn with_progress(mut self, progress: Arc<dyn ReleaseProgressSink>) -> Self {
        self.progress = progress;
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn run_to_pushed(
        &self,
        session: &mut ReleaseSession,
        state_store: &ReleaseStateStore,
        repository_path: &Path,
        git_dir: &Path,
        plan: &ReleaseCandidatePlan,
        verification_backend: &dyn LocalVerificationBackend,
        push_backend: &dyn ReleasePushBackend,
    ) -> Result<GitPushOutcome, ReleaseOrchestratorError> {
        let _repository_lock = RepositorySessionLock::acquire(git_dir)
            .map_err(|_| ReleaseOrchestratorError::SessionLockFailed)?;
        for phase in [
            ReleasePhase::Inspected,
            ReleasePhase::Planned,
            ReleasePhase::ApplyingCandidate,
        ] {
            state_store
                .advance(session, phase)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        let candidate_started = Instant::now();
        self.progress.started("candidate", "开始应用发布候选文件。");
        if ReleaseCandidateTransaction::apply(repository_path, git_dir, plan).is_err() {
            self.progress.log(
                "candidate",
                crate::models::ReleaseLogLevel::Error,
                "发布候选文件应用失败。",
            );
            return Err(ReleaseOrchestratorError::CandidateApplyFailed);
        }
        self.progress.completed(
            "candidate",
            elapsed_millis(candidate_started),
            "发布候选文件已应用。",
        );
        state_store
            .advance(session, ReleasePhase::LocalChecks)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)?;

        let verification = self
            .local_verification
            .run_with_progress(
                verification_backend,
                repository_path,
                self.progress.as_ref(),
            )
            .await;
        if let Err(error) = verification {
            if ReleaseCandidateTransaction::rollback_active(repository_path, git_dir).is_err() {
                let step_id = match &error {
                    LocalVerificationError::CommandFailed { command_id, .. } => command_id,
                    LocalVerificationError::Cancelled => "releasePipeline",
                };
                let _ = state_store.fail(session, step_id, "RELEASE_ROLLBACK_INCOMPLETE");
                self.progress.log(
                    step_id,
                    crate::models::ReleaseLogLevel::Error,
                    "本地发布门禁失败，且候选文件未能完整回滚。",
                );
                return Err(ReleaseOrchestratorError::RollbackFailed);
            }
            match error {
                LocalVerificationError::Cancelled => {
                    self.progress.log(
                        "releasePipeline",
                        crate::models::ReleaseLogLevel::Warning,
                        "本地发布门禁已取消，候选文件已回滚。",
                    );
                    state_store
                        .advance(session, ReleasePhase::Cancelled)
                        .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
                    return Err(ReleaseOrchestratorError::Cancelled);
                }
                LocalVerificationError::CommandFailed {
                    command_id,
                    failure,
                } => {
                    let orchestrator_error = ReleaseOrchestratorError::LocalVerificationFailed {
                        command_id: command_id.clone(),
                        failure,
                    };
                    self.progress.log(
                        &command_id,
                        crate::models::ReleaseLogLevel::Error,
                        &orchestrator_error.failure_message(),
                    );
                    state_store
                        .fail(session, &command_id, "RELEASE_LOCAL_VERIFICATION_FAILED")
                        .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
                    return Err(orchestrator_error);
                }
            }
        }

        state_store
            .advance(session, ReleasePhase::LocalBuild)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        let source_audit_started = Instant::now();
        self.progress
            .started("sourceAudit", "开始确认本地发布源审计结果。");
        state_store
            .advance(session, ReleasePhase::SourceAudit)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        self.progress.completed(
            "sourceAudit",
            elapsed_millis(source_audit_started),
            "本地发布源审计结果已确认。",
        );
        let commit_push_started = Instant::now();
        self.progress
            .started("commitPush", "开始创建候选提交并推送固定 main 引用。");
        let candidate_sha = match push_backend.commit(repository_path, plan).await {
            Ok(candidate_sha) => {
                self.progress.log(
                    "commitPush",
                    crate::models::ReleaseLogLevel::Info,
                    &format!("候选提交已创建，SHA {}。", short_sha(&candidate_sha)),
                );
                candidate_sha
            }
            Err(_) => {
                let index_rollback = push_backend
                    .rollback_uncommitted(repository_path, plan)
                    .await;
                let source_rollback =
                    ReleaseCandidateTransaction::rollback_active(repository_path, git_dir);
                self.progress.log(
                    "commitPush",
                    crate::models::ReleaseLogLevel::Error,
                    "候选提交失败，正在保留真实回滚结果。",
                );
                let _ = state_store.fail(session, "commitPush", "RELEASE_PUSH_FAILED");
                if index_rollback.is_err() || source_rollback.is_err() {
                    return Err(ReleaseOrchestratorError::RollbackFailed);
                }
                return Err(ReleaseOrchestratorError::PushFailed);
            }
        };
        session.candidate_sha = Some(candidate_sha.clone());
        state_store
            .advance(session, ReleasePhase::Committed)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        self.push_committed_locked(
            session,
            state_store,
            repository_path,
            git_dir,
            push_backend,
            commit_push_started,
        )
        .await
    }

    pub async fn push_committed(
        &self,
        session: &mut ReleaseSession,
        state_store: &ReleaseStateStore,
        repository_path: &Path,
        git_dir: &Path,
        push_backend: &dyn ReleasePushBackend,
    ) -> Result<GitPushOutcome, ReleaseOrchestratorError> {
        let _repository_lock = RepositorySessionLock::acquire(git_dir)
            .map_err(|_| ReleaseOrchestratorError::SessionLockFailed)?;
        let started = Instant::now();
        self.progress
            .started("commitPush", "继续推送已创建的候选提交。");
        self.push_committed_locked(
            session,
            state_store,
            repository_path,
            git_dir,
            push_backend,
            started,
        )
        .await
    }

    async fn push_committed_locked(
        &self,
        session: &mut ReleaseSession,
        state_store: &ReleaseStateStore,
        repository_path: &Path,
        git_dir: &Path,
        push_backend: &dyn ReleasePushBackend,
        started: Instant,
    ) -> Result<GitPushOutcome, ReleaseOrchestratorError> {
        if session.phase != ReleasePhase::Committed {
            return Err(ReleaseOrchestratorError::RemoteStateInvalid);
        }
        let candidate_sha = session
            .candidate_sha
            .clone()
            .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
        let outcome = match push_backend.push(repository_path, &candidate_sha).await {
            Ok(outcome) => outcome,
            Err(_) => {
                self.progress.log(
                    "commitPush",
                    crate::models::ReleaseLogLevel::Error,
                    "候选推送失败，已保留 committed 检查点供安全重试。",
                );
                return Err(ReleaseOrchestratorError::PushFailed);
            }
        };
        if outcome.candidate_sha != candidate_sha || outcome.remote_main_sha != candidate_sha {
            self.progress.log(
                "commitPush",
                crate::models::ReleaseLogLevel::Error,
                "推送后的远端 main 未匹配候选提交。",
            );
            return Err(ReleaseOrchestratorError::PushFailed);
        }
        session.remote_main_sha = Some(outcome.remote_main_sha.clone());
        state_store
            .advance(session, ReleasePhase::Pushed)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        if ReleaseCandidateTransaction::finalize_active(repository_path, git_dir).is_err() {
            self.progress.log(
                "commitPush",
                crate::models::ReleaseLogLevel::Error,
                "远端推送已验证，但本地回滚标记清理失败。",
            );
            return Err(ReleaseOrchestratorError::FinalizeFailed);
        }
        self.progress.completed(
            "commitPush",
            elapsed_millis(started),
            &format!("候选提交已推送并验证，SHA {}。", short_sha(&candidate_sha)),
        );
        Ok(outcome)
    }

    pub fn cancel_active(
        &self,
        session: &mut ReleaseSession,
        state_store: &ReleaseStateStore,
        repository_path: &Path,
        git_dir: &Path,
    ) -> Result<(), ReleaseOrchestratorError> {
        let _repository_lock = RepositorySessionLock::acquire(git_dir)
            .map_err(|_| ReleaseOrchestratorError::SessionLockFailed)?;
        if matches!(
            session.phase,
            ReleasePhase::Committed
                | ReleasePhase::Pushed
                | ReleasePhase::WorkflowQueued
                | ReleasePhase::WorkflowRunning
                | ReleasePhase::AuditingDraft
                | ReleasePhase::AwaitingPublishApproval
                | ReleasePhase::Publishing
                | ReleasePhase::VerifyingPublishedRelease
                | ReleasePhase::MonitoringCleanup
                | ReleasePhase::Completed
                | ReleasePhase::CompletedWithWarnings
        ) {
            return Err(ReleaseOrchestratorError::CancelAfterPushForbidden);
        }
        if matches!(
            session.phase,
            ReleasePhase::ApplyingCandidate
                | ReleasePhase::LocalChecks
                | ReleasePhase::LocalBuild
                | ReleasePhase::SourceAudit
        ) && ReleaseCandidateTransaction::rollback_active(repository_path, git_dir).is_err()
        {
            return Err(ReleaseOrchestratorError::RollbackFailed);
        }
        state_store
            .advance(session, ReleasePhase::Cancelled)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)
    }

    pub async fn run_remote_to_draft(
        &self,
        session: &mut ReleaseSession,
        state_store: &ReleaseStateStore,
        git_dir: &Path,
        expected_notes: &str,
        remote: &dyn ReleaseRemoteBackend,
    ) -> Result<DraftAuditEvidence, ReleaseOrchestratorError> {
        let _repository_lock = RepositorySessionLock::acquire(git_dir)
            .map_err(|_| ReleaseOrchestratorError::SessionLockFailed)?;
        let candidate_sha = session
            .candidate_sha
            .clone()
            .filter(|sha| session.remote_main_sha.as_deref() == Some(sha.as_str()))
            .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;

        if session.phase == ReleasePhase::AwaitingPublishApproval {
            return session
                .draft
                .clone()
                .ok_or(ReleaseOrchestratorError::RemoteStateInvalid);
        }
        let remote_run_started = Instant::now();
        if matches!(
            session.phase,
            ReleasePhase::Pushed | ReleasePhase::WorkflowQueued | ReleasePhase::WorkflowRunning
        ) {
            self.progress
                .started("remoteRun", "开始触发或继续监控 GitHub 发布 Run。");
        }
        if session.phase == ReleasePhase::Pushed {
            let workflow = match remote
                .dispatch(&session.target_version, &candidate_sha)
                .await
            {
                Ok(workflow) => workflow,
                Err(_) => {
                    self.progress.log(
                        "remoteRun",
                        crate::models::ReleaseLogLevel::Error,
                        "GitHub 发布 Run 触发失败（RELEASE_REMOTE_FAILED）。",
                    );
                    return Err(ReleaseOrchestratorError::RemoteFailed);
                }
            };
            self.progress.log(
                "remoteRun",
                crate::models::ReleaseLogLevel::Info,
                &format!(
                    "Run {} 已触发，SHA {}。",
                    workflow.run_id,
                    short_sha(&candidate_sha)
                ),
            );
            session.workflow = Some(workflow);
            state_store
                .advance(session, ReleasePhase::WorkflowQueued)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase == ReleasePhase::WorkflowQueued {
            state_store
                .advance(session, ReleasePhase::WorkflowRunning)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase == ReleasePhase::WorkflowRunning {
            let workflow = session
                .workflow
                .as_ref()
                .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
            let run = match remote.wait_for_run(workflow, &candidate_sha).await {
                Ok(run) => run,
                Err(_) => {
                    self.progress.log(
                        "remoteRun",
                        crate::models::ReleaseLogLevel::Error,
                        "GitHub 发布 Run 监控失败（RELEASE_REMOTE_FAILED）。",
                    );
                    return Err(ReleaseOrchestratorError::RemoteFailed);
                }
            };
            if run.id != workflow.run_id
                || run.url != workflow.url
                || run.head_sha != candidate_sha
                || run.status != "completed"
                || run.conclusion.as_deref() != Some("success")
            {
                self.progress.log(
                    "remoteRun",
                    crate::models::ReleaseLogLevel::Error,
                    "GitHub 发布 Run 身份或成功结论未通过验证。",
                );
                return Err(ReleaseOrchestratorError::RemoteFailed);
            }
            self.progress.completed(
                "remoteRun",
                elapsed_millis(remote_run_started),
                &format!(
                    "Run {} 已成功完成并验证，SHA {}。",
                    run.id,
                    short_sha(&candidate_sha)
                ),
            );
            state_store
                .advance(session, ReleasePhase::AuditingDraft)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase != ReleasePhase::AuditingDraft {
            return Err(ReleaseOrchestratorError::RemoteStateInvalid);
        }
        let draft_audit_started = Instant::now();
        self.progress
            .started("draftAudit", "开始审计 GitHub Draft Release。");
        let draft = match remote
            .audit_draft(&session.target_version, &candidate_sha, expected_notes)
            .await
        {
            Ok(draft) => draft,
            Err(_) => {
                self.progress.log(
                    "draftAudit",
                    crate::models::ReleaseLogLevel::Error,
                    "GitHub Draft Release 审计失败（RELEASE_REMOTE_FAILED）。",
                );
                return Err(ReleaseOrchestratorError::RemoteFailed);
            }
        };
        self.progress.completed(
            "draftAudit",
            elapsed_millis(draft_audit_started),
            &format!(
                "Release {} ({}) Draft 审计完成，资产 {} 项。",
                draft.release_id,
                draft.tag_name,
                draft.assets.len()
            ),
        );
        session.draft = Some(draft.clone());
        state_store
            .advance(session, ReleasePhase::AwaitingPublishApproval)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        Ok(draft)
    }

    pub async fn publish_and_finalize(
        &self,
        session: &mut ReleaseSession,
        state_store: &ReleaseStateStore,
        git_dir: &Path,
        expected_identity: &DraftIdentity,
        expected_notes: &str,
        remote: &dyn ReleaseRemoteBackend,
    ) -> Result<(), ReleaseOrchestratorError> {
        let _repository_lock = RepositorySessionLock::acquire(git_dir)
            .map_err(|_| ReleaseOrchestratorError::SessionLockFailed)?;
        let candidate_sha = session
            .candidate_sha
            .clone()
            .filter(|sha| session.remote_main_sha.as_deref() == Some(sha.as_str()))
            .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
        let draft = session
            .draft
            .clone()
            .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
        if &draft.identity() != expected_identity
            || expected_identity.target_commit_sha != candidate_sha
        {
            self.progress.log(
                "publishApproval",
                crate::models::ReleaseLogLevel::Error,
                "确认的 Draft 身份与会话证据不一致（RELEASE_PUBLISH_IDENTITY_MISMATCH）。",
            );
            return Err(ReleaseOrchestratorError::PublishIdentityMismatch);
        }

        if matches!(
            session.phase,
            ReleasePhase::Completed | ReleasePhase::CompletedWithWarnings
        ) {
            return Ok(());
        }
        let publish_started = Instant::now();
        if matches!(
            session.phase,
            ReleasePhase::AwaitingPublishApproval | ReleasePhase::Publishing
        ) {
            self.progress.started(
                "publishApproval",
                "开始复核确认信息并公开同一 Draft Release。",
            );
        }
        if session.phase == ReleasePhase::AwaitingPublishApproval {
            state_store
                .advance(session, ReleasePhase::Publishing)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase == ReleasePhase::Publishing {
            let published = match remote
                .publish(
                    &draft,
                    &session.target_version,
                    &candidate_sha,
                    expected_notes,
                )
                .await
            {
                Ok(published) => published,
                Err(_) => {
                    self.progress.log(
                        "publishApproval",
                        crate::models::ReleaseLogLevel::Error,
                        "Draft Release 公开失败（RELEASE_REMOTE_FAILED）。",
                    );
                    return Err(ReleaseOrchestratorError::RemoteFailed);
                }
            };
            if published.release_id != draft.release_id || published.tag_name != draft.tag_name {
                self.progress.log(
                    "publishApproval",
                    crate::models::ReleaseLogLevel::Error,
                    "公开后的 Release 身份与已审计 Draft 不一致。",
                );
                return Err(ReleaseOrchestratorError::RemoteFailed);
            }
            self.progress.completed(
                "publishApproval",
                elapsed_millis(publish_started),
                &format!(
                    "Release {} ({}) 已公开。",
                    published.release_id, published.tag_name
                ),
            );
            session.published = Some(published);
            state_store
                .advance(session, ReleasePhase::VerifyingPublishedRelease)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase == ReleasePhase::VerifyingPublishedRelease {
            let online_verification_started = Instant::now();
            self.progress
                .started("onlineVerification", "开始在线复核公开 Release。");
            let published = session
                .published
                .as_ref()
                .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
            let verified = match remote
                .verify_published(
                    &draft,
                    published,
                    &session.target_version,
                    &candidate_sha,
                    expected_notes,
                )
                .await
            {
                Ok(verified) => verified,
                Err(_) => {
                    self.progress.log(
                        "onlineVerification",
                        crate::models::ReleaseLogLevel::Error,
                        "公开 Release 在线复核失败（RELEASE_REMOTE_FAILED）。",
                    );
                    return Err(ReleaseOrchestratorError::RemoteFailed);
                }
            };
            if verified != draft {
                self.progress.log(
                    "onlineVerification",
                    crate::models::ReleaseLogLevel::Error,
                    "公开 Release 在线证据与 Draft 审计结果不一致。",
                );
                return Err(ReleaseOrchestratorError::RemoteFailed);
            }
            self.progress.completed(
                "onlineVerification",
                elapsed_millis(online_verification_started),
                &format!(
                    "Release {} ({}) 在线复核完成。",
                    published.release_id, published.tag_name
                ),
            );
            state_store
                .advance(session, ReleasePhase::MonitoringCleanup)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase != ReleasePhase::MonitoringCleanup {
            return Err(ReleaseOrchestratorError::RemoteStateInvalid);
        }
        let published_at = session
            .published
            .as_ref()
            .map(|published| published.published_at.clone())
            .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
        let cleanup_started = Instant::now();
        self.progress
            .started("cleanup", "开始监控历史 Release cleanup Run。");
        match remote.monitor_cleanup(&published_at).await {
            Ok(cleanup) => {
                let succeeded = cleanup.succeeded;
                if !succeeded {
                    self.progress.log(
                        "cleanup",
                        crate::models::ReleaseLogLevel::Warning,
                        &format!(
                            "cleanup Run {} 已结束，conclusion={}；已公开 Release 保持有效。",
                            cleanup.run_id,
                            cleanup.conclusion.as_deref().unwrap_or("unknown")
                        ),
                    );
                }
                self.progress.completed(
                    "cleanup",
                    elapsed_millis(cleanup_started),
                    &format!(
                        "cleanup Run {} 监控完成，conclusion={}。",
                        cleanup.run_id,
                        cleanup.conclusion.as_deref().unwrap_or("unknown")
                    ),
                );
                session.cleanup = Some(cleanup);
                session.cleanup_warning = None;
                state_store
                    .advance(
                        session,
                        if succeeded {
                            ReleasePhase::Completed
                        } else {
                            ReleasePhase::CompletedWithWarnings
                        },
                    )
                    .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
            }
            Err(_) => {
                self.progress.log(
                    "cleanup",
                    crate::models::ReleaseLogLevel::Warning,
                    "cleanup Run 监控失败（GITHUB_CLEANUP_MONITOR_FAILED）；已公开 Release 保持有效。",
                );
                self.progress.completed(
                    "cleanup",
                    elapsed_millis(cleanup_started),
                    "cleanup Run 监控已结束，但未能确认清理结果。",
                );
                session.cleanup = None;
                session.cleanup_warning = Some("GITHUB_CLEANUP_MONITOR_FAILED".into());
                state_store
                    .advance(session, ReleasePhase::CompletedWithWarnings)
                    .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
            }
        }
        Ok(())
    }
}

fn elapsed_millis(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn short_sha(sha: &str) -> String {
    sha.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::{GithubRemoteBackend, ReleaseRemoteBackend};
    use crate::infrastructure::gh::{GhBackend, GhRequest, GhResponse};
    use crate::models::{ReleaseLogLevel, WorkflowDispatch};
    use crate::services::release_log::{ReleaseLogRecorder, ReleaseLogStore, ReleaseProgressSink};
    use std::future::Future;
    use std::path::Path;
    use std::pin::Pin;
    use std::sync::Arc;

    struct CompletedRunBackend;

    impl GhBackend for CompletedRunBackend {
        fn execute<'a>(
            &'a self,
            _request: GhRequest,
        ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
            Box::pin(async {
                Ok(GhResponse {
                    stdout: r#"{
  "databaseId": 42,
  "status": "completed",
  "conclusion": "success",
  "headSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "url": "https://github.com/hunxuankai/codex-relay/actions/runs/42",
  "jobs": [{
    "name": "发布 Windows 更新",
    "status": "completed",
    "conclusion": "success",
    "startedAt": "2026-08-03T10:00:00Z",
    "completedAt": "2026-08-03T10:01:00Z",
    "steps": [{
      "name": "运行检查",
      "number": 3,
      "status": "completed",
      "conclusion": "success",
      "startedAt": "2026-08-03T10:00:10Z",
      "completedAt": "2026-08-03T10:00:50Z"
    }]
  }]
}"#
                    .as_bytes()
                    .to_vec(),
                })
            })
        }

        fn download_asset<'a>(
            &'a self,
            _asset_id: u64,
            _destination: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async { Err("download not expected".into()) })
        }
    }

    #[test]
    fn github_remote_backend_logs_the_first_completed_run_projection() {
        let git_dir = tempfile::tempdir().unwrap();
        let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let backend = CompletedRunBackend;
        let remote = GithubRemoteBackend::new(&backend)
            .with_progress(recorder.clone() as Arc<dyn ReleaseProgressSink>);
        let workflow = WorkflowDispatch {
            run_id: 42,
            url: "https://github.com/hunxuankai/codex-relay/actions/runs/42".into(),
        };

        let run = tauri::async_runtime::block_on(
            remote.wait_for_run(&workflow, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        )
        .unwrap();

        assert_eq!(run.status, "completed");
        let page = ReleaseLogStore::new(git_dir.path().to_path_buf())
            .load_page("session-a", None)
            .unwrap();
        assert_eq!(page.entries.len(), 1);
        let entry = &page.entries[0];
        assert_eq!(entry.step_id, "remoteRun");
        assert_eq!(entry.level, ReleaseLogLevel::Info);
        assert!(entry.message.contains("Run 42"));
        assert!(entry.message.contains("Job 发布 Windows 更新"));
        assert!(entry.message.contains("Step #3 运行检查"));
        assert!(!entry.message.contains("https://"));
        assert!(!entry.message.contains("aaaaaaaaaaaaaaaa"));
    }
}
