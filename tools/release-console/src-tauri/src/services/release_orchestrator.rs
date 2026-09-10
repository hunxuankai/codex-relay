use crate::infrastructure::gh::GhBackend;
use crate::infrastructure::git::GitBackend;
use crate::models::{
    CleanupRunEvidence, DraftAuditEvidence, DraftIdentity, PublishedReleaseEvidence, ReleasePhase,
    ReleaseSession, WorkflowDispatch, WorkflowRunStatus,
};
use crate::services::git_release::{GitPushOutcome, GitReleaseService};
use crate::services::github_release::{
    DraftAuditService, GithubReleaseError, GithubReleaseService, REMOTE_MONITOR_ATTEMPTS,
    REMOTE_MONITOR_DELAY,
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
    ) -> Pin<Box<dyn Future<Output = Result<WorkflowRunStatus, GithubReleaseError>> + Send + 'a>>;

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
    monitor_policy: RunMonitorPolicy,
}

struct RunMonitorPolicy {
    attempts: usize,
    delay: std::time::Duration,
    timeout: std::time::Duration,
    max_query_failures: usize,
}

impl Default for RunMonitorPolicy {
    fn default() -> Self {
        Self {
            attempts: REMOTE_MONITOR_ATTEMPTS,
            delay: REMOTE_MONITOR_DELAY,
            timeout: REMOTE_MONITOR_DELAY
                .saturating_mul(REMOTE_MONITOR_ATTEMPTS.saturating_sub(1) as u32),
            max_query_failures: 4,
        }
    }
}

impl<'a> GithubRemoteBackend<'a> {
    pub fn new(backend: &'a dyn GhBackend) -> Self {
        Self {
            backend,
            progress: Arc::new(NoopReleaseProgressSink),
            monitor_policy: RunMonitorPolicy::default(),
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
    ) -> Pin<Box<dyn Future<Output = Result<WorkflowRunStatus, GithubReleaseError>> + Send + 'a>>
    {
        Box::pin(async move {
            let started = tokio::time::Instant::now();
            let mut tracker = ReleaseRunProgressTracker::new();
            let mut query_failures = 0;
            for attempt in 0..self.monitor_policy.attempts {
                // 不在外层丢弃进程 future；当前查询由 SafeProcessRunner 安全结束。
                if started.elapsed() >= self.monitor_policy.timeout {
                    break;
                }
                let result = GithubReleaseService::new()
                    .get_release_run_status(self.backend, workflow.run_id, candidate_sha)
                    .await;
                match result {
                    Ok(run) => {
                        if run.url != workflow.url {
                            return Err(GithubReleaseError::WorkflowRunIdentityMismatch);
                        }
                        if query_failures > 0 {
                            self.progress.log(
                                "remoteRun",
                                crate::models::ReleaseLogLevel::Info,
                                &format!("Run {} 查询已恢复，继续监控同一 Run。", workflow.run_id),
                            );
                        }
                        query_failures = 0;
                        let decision = tracker.observe(started.elapsed(), &run);
                        if decision != ReleaseRunProgressDecision::Silent {
                            self.progress.log(
                                "remoteRun",
                                crate::models::ReleaseLogLevel::Info,
                                &format_run_progress(&run, decision),
                            );
                        }
                        if run.status == "completed" {
                            return if run.conclusion.as_deref() == Some("success") {
                                Ok(run)
                            } else {
                                Err(GithubReleaseError::WorkflowRunFailed)
                            };
                        }
                    }
                    Err(error) if error.is_retryable_run_query() => {
                        query_failures += 1;
                        if query_failures >= self.monitor_policy.max_query_failures {
                            return Err(error);
                        }
                        if attempt + 1 >= self.monitor_policy.attempts
                            || started.elapsed().saturating_add(self.monitor_policy.delay)
                                >= self.monitor_policy.timeout
                        {
                            return Err(GithubReleaseError::WorkflowRunTimeout);
                        }
                        self.progress.log(
                            "remoteRun",
                            crate::models::ReleaseLogLevel::Warning,
                            &format!(
                                "Run {} 查询暂时失败（{}），连续第 {}/{} 次；将重试查询同一 Run。",
                                workflow.run_id,
                                error.code(),
                                query_failures,
                                self.monitor_policy.max_query_failures,
                            ),
                        );
                    }
                    Err(error) => return Err(error),
                }
                if attempt + 1 < self.monitor_policy.attempts {
                    let remaining = self
                        .monitor_policy
                        .timeout
                        .saturating_sub(started.elapsed());
                    tokio::time::sleep(self.monitor_policy.delay.min(remaining)).await;
                }
            }
            Err(GithubReleaseError::WorkflowRunTimeout)
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
    #[error("GitHub Run 监控停止：{0}")]
    RunMonitoringFailed(GithubReleaseError),
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
            Self::RunMonitoringFailed(error) => error.code(),
            Self::RemoteStateInvalid => "RELEASE_REMOTE_STATE_INVALID",
            Self::PublishIdentityMismatch => "RELEASE_PUBLISH_IDENTITY_MISMATCH",
        }
    }

    pub(crate) fn failure_step_id(&self) -> &str {
        match self {
            Self::LocalVerificationFailed { command_id, .. } => command_id,
            Self::PublishIdentityMismatch => "publishApproval",
            Self::RunMonitoringFailed(_) => "remoteRun",
            _ => "releasePipeline",
        }
    }

    pub(crate) fn failure_message(&self) -> String {
        match self {
            Self::RunMonitoringFailed(error) => {
                if error.is_retryable_run_query()
                    || *error == GithubReleaseError::WorkflowRunTimeout
                {
                    format!("{error}；已保留 Run，可检查 GitHub 连接后继续监控同一 Run。")
                } else {
                    format!("{error}；已停止监控，请核对 GitHub Run。")
                }
            }
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
        let has_candidate_changes = plan.has_changes();
        self.progress.started(
            "commitPush",
            if has_candidate_changes {
                "开始创建候选提交并确认固定 main 引用。"
            } else {
                "候选文件已是目标状态，开始复用已同步 HEAD 并确认固定 main 引用。"
            },
        );
        let candidate_sha = match push_backend.commit(repository_path, plan).await {
            Ok(candidate_sha) => {
                self.progress.log(
                    "commitPush",
                    crate::models::ReleaseLogLevel::Info,
                    &if has_candidate_changes {
                        format!("候选提交已创建，SHA {}。", short_sha(&candidate_sha))
                    } else {
                        format!(
                            "候选文件无变化，复用已同步 HEAD，SHA {}。",
                            short_sha(&candidate_sha)
                        )
                    },
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
            .started("commitPush", "继续确认已记录候选提交与远端 main。");
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
                "远端 main 已验证，但本地回滚标记清理失败。",
            );
            return Err(ReleaseOrchestratorError::FinalizeFailed);
        }
        self.progress.completed(
            "commitPush",
            elapsed_millis(started),
            &format!(
                "候选提交与远端 main 已验证一致，SHA {}。",
                short_sha(&candidate_sha)
            ),
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
        let repository_lock = RepositorySessionLock::acquire(git_dir)
            .map_err(|_| ReleaseOrchestratorError::SessionLockFailed)?;
        if session.phase == ReleasePhase::Failed {
            let previous_code = session
                .failure
                .as_ref()
                .map(|failure| failure.code.clone())
                .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
            state_store
                .resume_remote_monitoring(session, &repository_lock)
                .map_err(|_| ReleaseOrchestratorError::RemoteStateInvalid)?;
            self.progress.log(
                "remoteRun",
                crate::models::ReleaseLogLevel::Info,
                &format!("已恢复原 GitHub Run 的监控检查点；上次失败代码 {previous_code}。"),
            );
        }
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
                Err(cause) => {
                    let error = ReleaseOrchestratorError::RunMonitoringFailed(cause);
                    self.progress.log(
                        "remoteRun",
                        crate::models::ReleaseLogLevel::Error,
                        &format!("{}：{}", error.code(), error.failure_message()),
                    );
                    return Err(error);
                }
            };
            if run.id != workflow.run_id || run.url != workflow.url || run.head_sha != candidate_sha
            {
                return Err(ReleaseOrchestratorError::RunMonitoringFailed(
                    GithubReleaseError::WorkflowRunIdentityMismatch,
                ));
            }
            if run.status != "completed" || run.conclusion.as_deref().is_none_or(str::is_empty) {
                return Err(ReleaseOrchestratorError::RunMonitoringFailed(
                    GithubReleaseError::InvalidResponse,
                ));
            }
            if run.conclusion.as_deref() != Some("success") {
                return Err(ReleaseOrchestratorError::RunMonitoringFailed(
                    GithubReleaseError::WorkflowRunFailed,
                ));
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
    use crate::infrastructure::gh::{GhBackend, GhOperation, GhRequest, GhResponse};
    use crate::models::{ReleaseLogLevel, WorkflowDispatch};
    use crate::services::release_log::{ReleaseLogRecorder, ReleaseLogStore, ReleaseProgressSink};
    use std::collections::VecDeque;
    use std::future::Future;
    use std::path::Path;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    struct ScriptedRunBackend {
        responses: Mutex<VecDeque<Result<GhResponse, String>>>,
        requests: Mutex<Vec<GhRequest>>,
    }

    impl GhBackend for ScriptedRunBackend {
        fn execute<'a>(
            &'a self,
            request: GhRequest,
        ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
            self.requests.lock().unwrap().push(request);
            let response = self.responses.lock().unwrap().pop_front().unwrap();
            Box::pin(async move { response })
        }

        fn download_asset<'a>(
            &'a self,
            _asset_id: u64,
            _destination: &'a Path,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async { panic!("monitoring must not download assets") })
        }
    }

    fn run_response(status: &str, conclusion: Option<&str>) -> GhResponse {
        GhResponse {
            stdout: serde_json::to_vec(&serde_json::json!({
                "databaseId": 42,
                "status": status,
                "conclusion": conclusion,
                "headSha": "a".repeat(40),
                "url": "https://github.com/hunxuankai/codex-relay/actions/runs/42",
                "jobs": [],
            }))
            .unwrap(),
        }
    }

    #[test]
    fn run_monitor_recovers_after_one_failed_query_without_dispatching() {
        let git_dir = tempfile::tempdir().unwrap();
        let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
        store.initialize("session-retry").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-retry", store, 0, None));
        let backend = ScriptedRunBackend {
            responses: Mutex::new(VecDeque::from([
                Err("GH_COMMAND_FAILED".into()),
                Ok(run_response("completed", Some("success"))),
            ])),
            requests: Mutex::new(Vec::new()),
        };
        let remote = GithubRemoteBackend::new(&backend).with_progress(recorder);
        let workflow = WorkflowDispatch {
            run_id: 42,
            url: "https://github.com/hunxuankai/codex-relay/actions/runs/42".into(),
        };

        let run = tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
            .expect("a temporary query failure must not fail the release run");

        assert_eq!(run.status, "completed");
        assert_eq!(run.conclusion.as_deref(), Some("success"));
        let requests = backend.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests.iter().all(|request| {
            request.operation == GhOperation::ViewReleaseRun && request.resource_id == Some(42)
        }));
        let page = ReleaseLogStore::new(git_dir.path().to_path_buf())
            .load_page("session-retry", None)
            .unwrap();
        assert!(page.entries.iter().any(|entry| {
            entry.step_id == "remoteRun"
                && entry.level == ReleaseLogLevel::Warning
                && entry.message.contains("GITHUB_COMMAND_FAILED")
        }));
    }

    fn monitor_fixture(
        responses: Vec<Result<GhResponse, String>>,
    ) -> (ScriptedRunBackend, WorkflowDispatch) {
        (
            ScriptedRunBackend {
                responses: Mutex::new(responses.into()),
                requests: Mutex::new(Vec::new()),
            },
            WorkflowDispatch {
                run_id: 42,
                url: "https://github.com/hunxuankai/codex-relay/actions/runs/42".into(),
            },
        )
    }

    #[test]
    fn run_monitor_resets_consecutive_failures_after_a_valid_running_response() {
        let mut responses = Vec::new();
        for _ in 0..2 {
            responses.extend((0..3).map(|_| Err("GH_PROCESS_TIMEOUT".into())));
            responses.push(Ok(run_response("in_progress", None)));
        }
        responses.push(Ok(run_response("completed", Some("success"))));
        let (backend, workflow) = monitor_fixture(responses);
        let mut remote = GithubRemoteBackend::new(&backend);
        remote.monitor_policy.delay = std::time::Duration::ZERO;

        let run = tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
            .unwrap();

        assert_eq!(run.conclusion.as_deref(), Some("success"));
        assert_eq!(backend.requests.lock().unwrap().len(), 9);
    }

    #[test]
    fn run_monitor_stops_after_four_consecutive_query_failures_with_the_specific_code() {
        for (backend_code, public_code) in [
            ("GH_COMMAND_FAILED", "GITHUB_COMMAND_FAILED"),
            ("GH_PROCESS_TIMEOUT", "GITHUB_PROCESS_TIMEOUT"),
        ] {
            let mut responses = (0..4).map(|_| Err(backend_code.into())).collect::<Vec<_>>();
            responses.push(Ok(run_response("completed", Some("success"))));
            let (backend, workflow) = monitor_fixture(responses);
            let mut remote = GithubRemoteBackend::new(&backend);
            remote.monitor_policy.delay = std::time::Duration::ZERO;

            let error =
                tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
                    .unwrap_err();

            assert_eq!(error.code(), public_code);
            assert_eq!(backend.requests.lock().unwrap().len(), 4);
        }
    }

    #[test]
    fn run_monitor_never_retries_cancellation_safety_or_unknown_backend_errors() {
        for (backend_code, public_code) in [
            ("GH_PROCESS_CANCELLED", "GITHUB_PROCESS_CANCELLED"),
            ("GH_PROCESS_START_FAILED", "GITHUB_PROCESS_START_FAILED"),
            (
                "GH_PROCESS_TREE_TERMINATION_FAILED",
                "GITHUB_PROCESS_TREE_TERMINATION_FAILED",
            ),
            ("GH_OUTPUT_TOO_LARGE", "GITHUB_OUTPUT_TOO_LARGE"),
            ("test-key-backend-error-not-real", "GITHUB_BACKEND_FAILED"),
        ] {
            let (backend, workflow) = monitor_fixture(vec![
                Err(backend_code.into()),
                Ok(run_response("completed", Some("success"))),
            ]);
            let remote = GithubRemoteBackend::new(&backend);

            let error =
                tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
                    .unwrap_err();

            assert_eq!(error.code(), public_code);
            assert!(!format!("{error:?} {error}").contains("test-key"));
            assert_eq!(backend.requests.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn run_monitor_rejects_identity_response_and_failed_conclusion_without_retrying() {
        for (field, value, public_code) in [
            (
                "databaseId",
                serde_json::json!(43),
                "GITHUB_RUN_IDENTITY_MISMATCH",
            ),
            (
                "url",
                serde_json::json!("https://example.invalid/run/42"),
                "GITHUB_RUN_IDENTITY_MISMATCH",
            ),
            (
                "headSha",
                serde_json::json!("b".repeat(40)),
                "GITHUB_RUN_SHA_MISMATCH",
            ),
            (
                "status",
                serde_json::json!("unrecognized"),
                "GITHUB_RESPONSE_INVALID",
            ),
            (
                "conclusion",
                serde_json::Value::Null,
                "GITHUB_RESPONSE_INVALID",
            ),
            (
                "conclusion",
                serde_json::json!("future_conclusion"),
                "GITHUB_RESPONSE_INVALID",
            ),
            (
                "conclusion",
                serde_json::json!("  "),
                "GITHUB_RESPONSE_INVALID",
            ),
            (
                "conclusion",
                serde_json::json!("failure"),
                "GITHUB_RUN_FAILED",
            ),
        ] {
            let mut raw: serde_json::Value =
                serde_json::from_slice(&run_response("completed", Some("success")).stdout).unwrap();
            raw[field] = value;
            let (backend, workflow) = monitor_fixture(vec![
                Ok(GhResponse {
                    stdout: serde_json::to_vec(&raw).unwrap(),
                }),
                Ok(run_response("completed", Some("success"))),
            ]);
            let remote = GithubRemoteBackend::new(&backend);

            let error =
                tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
                    .unwrap_err();

            assert_eq!(error.code(), public_code);
            assert_eq!(backend.requests.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn run_monitor_logs_a_failed_terminal_projection_before_stopping() {
        let git_dir = tempfile::tempdir().unwrap();
        let log_store = ReleaseLogStore::new(git_dir.path().to_path_buf());
        log_store.initialize("session-failed-run").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new(
            "session-failed-run",
            log_store,
            0,
            None,
        ));
        let mut failed: serde_json::Value =
            serde_json::from_slice(&run_response("completed", Some("failure")).stdout).unwrap();
        failed["jobs"] = serde_json::json!([{
            "name": "release", "status": "completed", "conclusion": "failure",
            "steps": [{"name": "运行完整检查", "number": 7, "status": "completed", "conclusion": "failure"}]
        }]);
        let (backend, workflow) = monitor_fixture(vec![
            Ok(run_response("in_progress", None)),
            Ok(GhResponse {
                stdout: serde_json::to_vec(&failed).unwrap(),
            }),
            Ok(run_response("completed", Some("success"))),
        ]);
        let mut remote = GithubRemoteBackend::new(&backend).with_progress(recorder);
        remote.monitor_policy.delay = std::time::Duration::ZERO;

        let error = tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
            .unwrap_err();

        assert_eq!(error.code(), "GITHUB_RUN_FAILED");
        assert_eq!(backend.requests.lock().unwrap().len(), 2);
        let page = ReleaseLogStore::new(git_dir.path().to_path_buf())
            .load_page("session-failed-run", None)
            .unwrap();
        assert!(page.entries.iter().any(|entry| {
            entry.message.contains("conclusion=failure")
                && entry.message.contains("Step #7 运行完整检查")
        }));
    }

    #[test]
    fn run_monitor_does_not_announce_a_retry_after_the_last_attempt() {
        let git_dir = tempfile::tempdir().unwrap();
        let log_store = ReleaseLogStore::new(git_dir.path().to_path_buf());
        log_store.initialize("session-last-query").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new(
            "session-last-query",
            log_store,
            0,
            None,
        ));
        let (backend, workflow) = monitor_fixture(vec![Err("GH_COMMAND_FAILED".into())]);
        let mut remote = GithubRemoteBackend::new(&backend).with_progress(recorder);
        remote.monitor_policy.attempts = 1;

        let error = tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
            .unwrap_err();

        assert_eq!(error.code(), "GITHUB_RUN_TIMEOUT");
        let page = ReleaseLogStore::new(git_dir.path().to_path_buf())
            .load_page("session-last-query", None)
            .unwrap();
        assert!(
            !page
                .entries
                .iter()
                .any(|entry| entry.message.contains("将重试"))
        );
    }

    #[test]
    fn run_monitor_does_not_start_queries_after_the_deadline_or_attempt_budget() {
        let (backend, workflow) = monitor_fixture(Vec::new());
        let mut remote = GithubRemoteBackend::new(&backend);
        remote.monitor_policy.timeout = std::time::Duration::ZERO;
        let error = tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
            .unwrap_err();
        assert_eq!(error.code(), "GITHUB_RUN_TIMEOUT");
        assert!(backend.requests.lock().unwrap().is_empty());

        let (backend, workflow) = monitor_fixture(
            (0..3)
                .map(|_| Ok(run_response("in_progress", None)))
                .collect(),
        );
        let mut remote = GithubRemoteBackend::new(&backend);
        remote.monitor_policy.delay = std::time::Duration::ZERO;
        remote.monitor_policy.attempts = 3;
        let error = tauri::async_runtime::block_on(remote.wait_for_run(&workflow, &"a".repeat(40)))
            .unwrap_err();
        assert_eq!(error.code(), "GITHUB_RUN_TIMEOUT");
        assert_eq!(backend.requests.lock().unwrap().len(), 3);
    }

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
