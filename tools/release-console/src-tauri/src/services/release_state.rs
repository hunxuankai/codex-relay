use crate::models::{ReleasePhase, ReleaseSession};
use codex_relay_core::error::AppError;
use codex_relay_core::infrastructure::atomic_file::atomic_write;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::path::PathBuf;

const STATE_DIRECTORY: &str = "codex-relay-release-console";
const SESSION_FILE: &str = "session.json";
const SESSION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ReleaseStateError {
    #[error("无法读取发布会话状态")]
    ReadFailed,
    #[error("发布会话状态无效")]
    InvalidState,
    #[error("无法保存发布会话状态")]
    WriteFailed,
    #[error("该仓库已有活动发布进程")]
    SessionLocked,
    #[error("该仓库已有活动发布会话")]
    ActiveSessionExists,
    #[error("发布会话阶段转换无效")]
    InvalidTransition,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredReleaseSession {
    schema_version: u32,
    session: ReleaseSession,
}

pub struct ReleaseStateStore {
    state_file: PathBuf,
}

#[derive(Debug)]
pub struct RepositorySessionLock {
    file: File,
}

impl RepositorySessionLock {
    pub fn acquire(git_dir: &std::path::Path) -> Result<Self, ReleaseStateError> {
        let state_root = git_dir.join(STATE_DIRECTORY);
        fs::create_dir_all(&state_root).map_err(|_| ReleaseStateError::WriteFailed)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(state_root.join("session.lock"))
            .map_err(|_| ReleaseStateError::WriteFailed)?;
        file.try_lock_exclusive()
            .map_err(|_| ReleaseStateError::SessionLocked)?;
        Ok(Self { file })
    }
}

impl Drop for RepositorySessionLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

impl ReleaseStateStore {
    pub fn new(git_dir: PathBuf) -> Self {
        Self {
            state_file: git_dir.join(STATE_DIRECTORY).join(SESSION_FILE),
        }
    }

    pub fn save(&self, session: &ReleaseSession) -> Result<(), ReleaseStateError> {
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent).map_err(|_| ReleaseStateError::WriteFailed)?;
        }
        let stored = StoredReleaseSession {
            schema_version: SESSION_SCHEMA_VERSION,
            session: session.clone(),
        };
        let mut bytes =
            serde_json::to_vec_pretty(&stored).map_err(|_| ReleaseStateError::WriteFailed)?;
        bytes.push(b'\n');
        atomic_write(&self.state_file, &bytes, validate_stored_session)
            .map_err(|_| ReleaseStateError::WriteFailed)
    }

    pub fn initialize(&self, session: &ReleaseSession) -> Result<(), ReleaseStateError> {
        if self
            .load()?
            .as_ref()
            .is_some_and(|existing| !session_is_terminal(existing))
        {
            return Err(ReleaseStateError::ActiveSessionExists);
        }
        self.save(session)
    }

    pub fn load(&self) -> Result<Option<ReleaseSession>, ReleaseStateError> {
        let bytes = match fs::read(&self.state_file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(ReleaseStateError::ReadFailed),
        };
        let stored: StoredReleaseSession =
            serde_json::from_slice(&bytes).map_err(|_| ReleaseStateError::InvalidState)?;
        if stored.schema_version != SESSION_SCHEMA_VERSION {
            return Err(ReleaseStateError::InvalidState);
        }
        if !session_is_valid(&stored.session) {
            return Err(ReleaseStateError::InvalidState);
        }
        Ok(Some(stored.session))
    }

    pub fn advance(
        &self,
        session: &mut ReleaseSession,
        next: ReleasePhase,
    ) -> Result<(), ReleaseStateError> {
        let phase = session
            .phase
            .transition_to(next)
            .map_err(|_| ReleaseStateError::InvalidTransition)?;
        let mut updated = session.clone();
        updated.phase = phase;
        self.save(&updated)?;
        *session = updated;
        Ok(())
    }
}

fn validate_stored_session(bytes: &[u8]) -> Result<(), AppError> {
    let stored: StoredReleaseSession = serde_json::from_slice(bytes).map_err(|error| {
        AppError::new(
            "RELEASE_STATE_INVALID",
            "发布会话状态无效。",
            error.to_string(),
        )
    })?;
    if stored.schema_version != SESSION_SCHEMA_VERSION {
        return Err(AppError::new(
            "RELEASE_STATE_VERSION_UNSUPPORTED",
            "发布会话状态版本不受支持。",
            format!("schema version {}", stored.schema_version),
        ));
    }
    if !session_is_valid(&stored.session) {
        return Err(AppError::new(
            "RELEASE_STATE_INVALID",
            "发布会话状态无效。",
            "session invariants are not satisfied",
        ));
    }
    Ok(())
}

fn session_is_valid(session: &ReleaseSession) -> bool {
    use crate::models::ReleasePhase;

    if session.id.trim().is_empty()
        || session.repository_path.trim().is_empty()
        || semver::Version::parse(&session.target_version).is_err()
    {
        return false;
    }
    if matches!(session.phase, ReleasePhase::Committed) {
        return session.candidate_sha.is_some();
    }
    let is_pushed_or_later = matches!(
        session.phase,
        ReleasePhase::Pushed
            | ReleasePhase::WorkflowQueued
            | ReleasePhase::WorkflowRunning
            | ReleasePhase::AuditingDraft
            | ReleasePhase::AwaitingPublishApproval
            | ReleasePhase::Publishing
            | ReleasePhase::VerifyingPublishedRelease
            | ReleasePhase::MonitoringCleanup
            | ReleasePhase::Completed
            | ReleasePhase::CompletedWithWarnings
    );
    if is_pushed_or_later
        && (session.candidate_sha.is_none()
            || session.remote_main_sha.is_none()
            || session.candidate_sha != session.remote_main_sha)
    {
        return false;
    }

    let requires_workflow = matches!(
        session.phase,
        ReleasePhase::WorkflowQueued
            | ReleasePhase::WorkflowRunning
            | ReleasePhase::AuditingDraft
            | ReleasePhase::AwaitingPublishApproval
            | ReleasePhase::Publishing
            | ReleasePhase::VerifyingPublishedRelease
            | ReleasePhase::MonitoringCleanup
            | ReleasePhase::Completed
            | ReleasePhase::CompletedWithWarnings
    );
    if requires_workflow && session.workflow.is_none() {
        return false;
    }

    let requires_draft = matches!(
        session.phase,
        ReleasePhase::AwaitingPublishApproval
            | ReleasePhase::Publishing
            | ReleasePhase::VerifyingPublishedRelease
            | ReleasePhase::MonitoringCleanup
            | ReleasePhase::Completed
            | ReleasePhase::CompletedWithWarnings
    );
    if requires_draft {
        let Some(draft) = session.draft.as_ref() else {
            return false;
        };
        if draft.tag_name != format!("v{}", session.target_version)
            || Some(draft.target_commit_sha.as_str()) != session.candidate_sha.as_deref()
            || draft.manifest_version != session.target_version
            || draft.assets.is_empty()
        {
            return false;
        }
    }

    let requires_published = matches!(
        session.phase,
        ReleasePhase::VerifyingPublishedRelease
            | ReleasePhase::MonitoringCleanup
            | ReleasePhase::Completed
            | ReleasePhase::CompletedWithWarnings
    );
    if requires_published {
        let (Some(draft), Some(published)) = (session.draft.as_ref(), session.published.as_ref())
        else {
            return false;
        };
        if published.release_id != draft.release_id || published.tag_name != draft.tag_name {
            return false;
        }
    }

    if matches!(session.phase, ReleasePhase::Completed) {
        return session
            .cleanup
            .as_ref()
            .is_some_and(|cleanup| cleanup.succeeded)
            && session.cleanup_warning.is_none();
    }
    if matches!(session.phase, ReleasePhase::CompletedWithWarnings) {
        return session
            .cleanup
            .as_ref()
            .is_some_and(|cleanup| !cleanup.succeeded)
            || session.cleanup_warning.is_some();
    }
    true
}

fn session_is_terminal(session: &ReleaseSession) -> bool {
    matches!(
        session.phase,
        ReleasePhase::Completed
            | ReleasePhase::CompletedWithWarnings
            | ReleasePhase::Failed
            | ReleasePhase::Cancelled
    )
}
