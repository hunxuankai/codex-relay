use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseProxyType {
    Http,
    Socks5,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseProxySettings {
    pub enabled: bool,
    pub proxy_type: ReleaseProxyType,
    pub host: String,
    pub port: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProbeResult {
    pub success: bool,
    pub code: Option<String>,
    pub message: String,
    pub duration_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseConnectionTestResult {
    pub git: ConnectionProbeResult,
    pub github: ConnectionProbeResult,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RepositorySyncStatus {
    Synced,
    Ahead,
    Behind,
    Diverged,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryCommitSummary {
    pub sha: String,
    pub subject: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositorySyncInspection {
    pub status: RepositorySyncStatus,
    pub ahead_count: u32,
    pub behind_count: u32,
    pub ahead_commits: Vec<RepositoryCommitSummary>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInspection {
    pub local_branch: String,
    pub default_branch: String,
    pub head_sha: String,
    pub remote_main_sha: String,
    pub remote_url: String,
    pub clean: bool,
    pub sync: RepositorySyncInspection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainInspection {
    pub git: Option<String>,
    pub node: Option<String>,
    pub npm: Option<String>,
    pub cargo: Option<String>,
    pub gh: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalPreflightSnapshot {
    pub tools: ToolchainInspection,
    pub active_release_runs: usize,
    pub conflicting_drafts: usize,
    pub latest_release_tag: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeRepositoryPushPreview {
    pub expected_head_sha: String,
    pub expected_remote_main_sha: String,
    pub commit_count: u32,
    pub commits: Vec<RepositoryCommitSummary>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeRepositoryPushRequest {
    pub repository_path: String,
    pub expected_head_sha: String,
    pub expected_remote_main_sha: String,
    pub proxy: ReleaseProxySettings,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePreflightResult {
    pub repository_path: String,
    pub repository: RepositoryInspection,
    pub external: ExternalPreflightSnapshot,
    pub release_ready: bool,
    pub blocking_reasons: Vec<String>,
    pub safe_push: Option<SafeRepositoryPushPreview>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePlanFileSummary {
    pub relative_path: String,
    pub before_sha256: String,
    pub after_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasePlanSummary {
    pub id: String,
    pub repository_path: String,
    pub previous_version: String,
    pub target_version: String,
    pub notes: String,
    pub files: Vec<ReleasePlanFileSummary>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<CommandError>,
}

impl<T> CommandResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(CommandError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDispatch {
    pub run_id: u64,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunStatus {
    pub id: u64,
    pub status: String,
    pub conclusion: Option<String>,
    pub head_sha: String,
    pub url: String,
    pub jobs: Vec<WorkflowJobStatus>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowJobStatus {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_millis: Option<u64>,
    pub steps: Vec<WorkflowStepStatus>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStepStatus {
    pub name: String,
    pub number: u64,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_millis: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftAssetEvidence {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftAuditEvidence {
    pub release_id: u64,
    pub tag_name: String,
    pub target_commit_sha: String,
    pub assets: Vec<DraftAssetEvidence>,
    pub manifest_version: String,
    pub manifest_notes: String,
    pub signature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftIdentity {
    pub release_id: u64,
    pub tag_name: String,
    pub target_commit_sha: String,
}

impl DraftAuditEvidence {
    pub fn identity(&self) -> DraftIdentity {
        DraftIdentity {
            release_id: self.release_id,
            tag_name: self.tag_name.clone(),
            target_commit_sha: self.target_commit_sha.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedReleaseEvidence {
    pub release_id: u64,
    pub tag_name: String,
    pub published_at: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRunEvidence {
    pub run_id: u64,
    pub url: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub succeeded: bool,
    pub jobs: Vec<WorkflowJobStatus>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseLogSource {
    Lifecycle,
    Stdout,
    Stderr,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReleaseLogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseLogEntry {
    pub session_id: String,
    pub sequence: u64,
    pub timestamp: String,
    pub step_id: String,
    pub source: ReleaseLogSource,
    pub level: ReleaseLogLevel,
    pub message: String,
}

impl fmt::Debug for ReleaseLogEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReleaseLogEntry")
            .field("session_id", &self.session_id)
            .field("sequence", &self.sequence)
            .field("timestamp", &self.timestamp)
            .field("step_id", &self.step_id)
            .field("source", &self.source)
            .field("level", &self.level)
            .field("message_length", &self.message.len())
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseLogPage {
    pub entries: Vec<ReleaseLogEntry>,
    pub next_before_sequence: Option<u64>,
    pub has_earlier: bool,
    pub total_entries: u64,
    pub total_bytes: u64,
    pub truncated: bool,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSessionSnapshot {
    pub session: ReleaseSession,
    pub logs: ReleaseLogPage,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ReleaseEvent {
    SessionUpdated {
        session: Box<ReleaseSession>,
    },
    StepStarted {
        step_id: String,
        started_at: String,
    },
    StepLog {
        entry: ReleaseLogEntry,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page: Option<ReleaseLogPage>,
    },
    StepCompleted {
        step_id: String,
        completed_at: String,
        duration_millis: u64,
    },
    StepFailed {
        step_id: String,
        code: String,
        message: String,
    },
    DraftReady {
        draft: DraftAuditEvidence,
    },
    ReleasePublished {
        published: PublishedReleaseEvidence,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReleasePhase {
    Idle,
    Inspected,
    Planned,
    ApplyingCandidate,
    LocalChecks,
    LocalBuild,
    SourceAudit,
    Committed,
    Pushed,
    WorkflowQueued,
    WorkflowRunning,
    AuditingDraft,
    AwaitingPublishApproval,
    Publishing,
    VerifyingPublishedRelease,
    MonitoringCleanup,
    Completed,
    CompletedWithWarnings,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseFailureEvidence {
    pub phase: ReleasePhase,
    pub step_id: String,
    pub code: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseSession {
    pub id: String,
    pub repository_path: String,
    pub target_version: String,
    pub phase: ReleasePhase,
    pub candidate_sha: Option<String>,
    pub remote_main_sha: Option<String>,
    #[serde(default)]
    pub workflow: Option<WorkflowDispatch>,
    #[serde(default)]
    pub draft: Option<DraftAuditEvidence>,
    #[serde(default)]
    pub published: Option<PublishedReleaseEvidence>,
    #[serde(default)]
    pub cleanup: Option<CleanupRunEvidence>,
    #[serde(default)]
    pub cleanup_warning: Option<String>,
    #[serde(default)]
    pub failure: Option<ReleaseFailureEvidence>,
}

impl ReleaseSession {
    pub fn new(
        id: impl Into<String>,
        repository_path: impl Into<String>,
        target_version: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            repository_path: repository_path.into(),
            target_version: target_version.into(),
            phase: ReleasePhase::Idle,
            candidate_sha: None,
            remote_main_sha: None,
            workflow: None,
            draft: None,
            published: None,
            cleanup: None,
            cleanup_warning: None,
            failure: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReleaseModelError {
    #[error("不允许从 {from:?} 转换到 {to:?}")]
    InvalidPhaseTransition {
        from: ReleasePhase,
        to: ReleasePhase,
    },
}

impl ReleaseModelError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPhaseTransition { .. } => "RELEASE_PHASE_TRANSITION_INVALID",
        }
    }
}

impl ReleasePhase {
    pub fn transition_to(self, next: Self) -> Result<Self, ReleaseModelError> {
        let is_next_phase = matches!(
            (self, next),
            (Self::Idle, Self::Inspected)
                | (Self::Inspected, Self::Planned)
                | (Self::Planned, Self::ApplyingCandidate)
                | (Self::ApplyingCandidate, Self::LocalChecks)
                | (Self::LocalChecks, Self::LocalBuild)
                | (Self::LocalBuild, Self::SourceAudit)
                | (Self::SourceAudit, Self::Committed)
                | (Self::Committed, Self::Pushed)
                | (Self::Pushed, Self::WorkflowQueued)
                | (Self::WorkflowQueued, Self::WorkflowRunning)
                | (Self::WorkflowRunning, Self::AuditingDraft)
                | (Self::AuditingDraft, Self::AwaitingPublishApproval)
                | (Self::AwaitingPublishApproval, Self::Publishing)
                | (Self::Publishing, Self::VerifyingPublishedRelease)
                | (Self::VerifyingPublishedRelease, Self::MonitoringCleanup)
                | (Self::MonitoringCleanup, Self::Completed)
                | (Self::MonitoringCleanup, Self::CompletedWithWarnings)
        );
        let is_active = !matches!(
            self,
            Self::Completed | Self::CompletedWithWarnings | Self::Failed | Self::Cancelled
        );
        let is_terminal_transition = is_active && matches!(next, Self::Failed | Self::Cancelled);

        if is_next_phase || is_terminal_transition {
            Ok(next)
        } else {
            Err(ReleaseModelError::InvalidPhaseTransition {
                from: self,
                to: next,
            })
        }
    }
}
