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
    LocalVerificationBackend, LocalVerificationError, LocalVerificationService,
};
use crate::services::release_candidate::{ReleaseCandidatePlan, ReleaseCandidateTransaction};
use crate::services::release_state::{ReleaseStateStore, RepositorySessionLock};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

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
}

impl<'a> GithubRemoteBackend<'a> {
    pub fn new(backend: &'a dyn GhBackend) -> Self {
        Self { backend }
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
            for attempt in 0..REMOTE_MONITOR_ATTEMPTS {
                let run = GithubReleaseService::new()
                    .get_release_run(self.backend, workflow.run_id, candidate_sha)
                    .await
                    .map_err(|error| error.code().to_string())?;
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
        exit_code: Option<i32>,
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
            _ => "releasePipeline",
        }
    }

    pub(crate) fn failure_message(&self) -> String {
        match self {
            Self::LocalVerificationFailed {
                exit_code: Some(exit_code),
                ..
            } => format!("本地发布门禁退出码 {exit_code}；候选文件已回滚，尚未提交或推送。"),
            Self::LocalVerificationFailed {
                exit_code: None, ..
            } => "本地发布门禁命令未能完成，且没有可用退出码；候选文件已回滚，尚未提交或推送。"
                .into(),
            _ => "发布流程失败，请查看对应阶段证据。".into(),
        }
    }
}

pub struct ReleaseOrchestrator {
    local_verification: LocalVerificationService,
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
        }
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
        ReleaseCandidateTransaction::apply(repository_path, git_dir, plan)
            .map_err(|_| ReleaseOrchestratorError::CandidateApplyFailed)?;
        state_store
            .advance(session, ReleasePhase::LocalChecks)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)?;

        let verification = self
            .local_verification
            .run(verification_backend, repository_path)
            .await;
        if let Err(error) = verification {
            if ReleaseCandidateTransaction::rollback_active(repository_path, git_dir).is_err() {
                let _ = state_store.advance(session, ReleasePhase::Failed);
                return Err(ReleaseOrchestratorError::RollbackFailed);
            }
            match error {
                LocalVerificationError::Cancelled => {
                    state_store
                        .advance(session, ReleasePhase::Cancelled)
                        .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
                    return Err(ReleaseOrchestratorError::Cancelled);
                }
                LocalVerificationError::CommandFailed {
                    command_id,
                    exit_code,
                } => {
                    state_store
                        .advance(session, ReleasePhase::Failed)
                        .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
                    return Err(ReleaseOrchestratorError::LocalVerificationFailed {
                        command_id,
                        exit_code,
                    });
                }
            }
        }

        for phase in [ReleasePhase::LocalBuild, ReleasePhase::SourceAudit] {
            state_store
                .advance(session, phase)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        let candidate_sha = match push_backend.commit(repository_path, plan).await {
            Ok(candidate_sha) => candidate_sha,
            Err(_) => {
                let index_rollback = push_backend
                    .rollback_uncommitted(repository_path, plan)
                    .await;
                let source_rollback =
                    ReleaseCandidateTransaction::rollback_active(repository_path, git_dir);
                let _ = state_store.advance(session, ReleasePhase::Failed);
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
        self.push_committed_locked(session, state_store, repository_path, git_dir, push_backend)
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
        self.push_committed_locked(session, state_store, repository_path, git_dir, push_backend)
            .await
    }

    async fn push_committed_locked(
        &self,
        session: &mut ReleaseSession,
        state_store: &ReleaseStateStore,
        repository_path: &Path,
        git_dir: &Path,
        push_backend: &dyn ReleasePushBackend,
    ) -> Result<GitPushOutcome, ReleaseOrchestratorError> {
        if session.phase != ReleasePhase::Committed {
            return Err(ReleaseOrchestratorError::RemoteStateInvalid);
        }
        let candidate_sha = session
            .candidate_sha
            .clone()
            .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
        let outcome = push_backend
            .push(repository_path, &candidate_sha)
            .await
            .map_err(|_| ReleaseOrchestratorError::PushFailed)?;
        if outcome.candidate_sha != candidate_sha || outcome.remote_main_sha != candidate_sha {
            return Err(ReleaseOrchestratorError::PushFailed);
        }
        session.remote_main_sha = Some(outcome.remote_main_sha.clone());
        state_store
            .advance(session, ReleasePhase::Pushed)
            .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        ReleaseCandidateTransaction::finalize_active(repository_path, git_dir)
            .map_err(|_| ReleaseOrchestratorError::FinalizeFailed)?;
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
        if session.phase == ReleasePhase::Pushed {
            let workflow = remote
                .dispatch(&session.target_version, &candidate_sha)
                .await
                .map_err(|_| ReleaseOrchestratorError::RemoteFailed)?;
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
            let run = remote
                .wait_for_run(workflow, &candidate_sha)
                .await
                .map_err(|_| ReleaseOrchestratorError::RemoteFailed)?;
            if run.id != workflow.run_id
                || run.url != workflow.url
                || run.head_sha != candidate_sha
                || run.status != "completed"
                || run.conclusion.as_deref() != Some("success")
            {
                return Err(ReleaseOrchestratorError::RemoteFailed);
            }
            state_store
                .advance(session, ReleasePhase::AuditingDraft)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase != ReleasePhase::AuditingDraft {
            return Err(ReleaseOrchestratorError::RemoteStateInvalid);
        }
        let draft = remote
            .audit_draft(&session.target_version, &candidate_sha, expected_notes)
            .await
            .map_err(|_| ReleaseOrchestratorError::RemoteFailed)?;
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
            return Err(ReleaseOrchestratorError::PublishIdentityMismatch);
        }

        if matches!(
            session.phase,
            ReleasePhase::Completed | ReleasePhase::CompletedWithWarnings
        ) {
            return Ok(());
        }
        if session.phase == ReleasePhase::AwaitingPublishApproval {
            state_store
                .advance(session, ReleasePhase::Publishing)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase == ReleasePhase::Publishing {
            let published = remote
                .publish(
                    &draft,
                    &session.target_version,
                    &candidate_sha,
                    expected_notes,
                )
                .await
                .map_err(|_| ReleaseOrchestratorError::RemoteFailed)?;
            if published.release_id != draft.release_id || published.tag_name != draft.tag_name {
                return Err(ReleaseOrchestratorError::RemoteFailed);
            }
            session.published = Some(published);
            state_store
                .advance(session, ReleasePhase::VerifyingPublishedRelease)
                .map_err(|_| ReleaseOrchestratorError::StateFailed)?;
        }
        if session.phase == ReleasePhase::VerifyingPublishedRelease {
            let published = session
                .published
                .as_ref()
                .ok_or(ReleaseOrchestratorError::RemoteStateInvalid)?;
            let verified = remote
                .verify_published(
                    &draft,
                    published,
                    &session.target_version,
                    &candidate_sha,
                    expected_notes,
                )
                .await
                .map_err(|_| ReleaseOrchestratorError::RemoteFailed)?;
            if verified != draft {
                return Err(ReleaseOrchestratorError::RemoteFailed);
            }
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
        match remote.monitor_cleanup(&published_at).await {
            Ok(cleanup) => {
                let succeeded = cleanup.succeeded;
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
