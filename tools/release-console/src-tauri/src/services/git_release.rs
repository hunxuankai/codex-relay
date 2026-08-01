use crate::infrastructure::git::{GitBackend, GitBackendError};
pub use crate::models::{
    ExternalPreflightSnapshot, ReleasePreflightResult, RepositoryInspection, ToolchainInspection,
};
use crate::services::release_candidate::ReleaseCandidatePlan;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub trait ReleasePreflightProbe: Send + Sync {
    fn inspect(&self) -> Result<ExternalPreflightSnapshot, String>;
}

#[derive(Debug, thiserror::Error)]
pub enum ReleasePreflightError {
    #[error(transparent)]
    Git(#[from] GitReleaseError),
    #[error("无法读取工具链或 GitHub 发布状态")]
    ProbeFailed,
    #[error("发布所需工具不可用")]
    ToolMissing,
    #[error("已有活动发布工作流")]
    ActiveReleaseRun,
    #[error("已有冲突的 Draft Release")]
    ConflictingDraft,
}

pub struct ReleasePreflightService {
    repository_inspection: RepositoryInspectionService,
}

impl ReleasePreflightService {
    pub fn new(repository_inspection: RepositoryInspectionService) -> Self {
        Self {
            repository_inspection,
        }
    }

    pub async fn inspect(
        &self,
        backend: &GitBackend,
        repository_path: &Path,
        probe: &dyn ReleasePreflightProbe,
    ) -> Result<ReleasePreflightResult, ReleasePreflightError> {
        let repository = self
            .repository_inspection
            .inspect(backend, repository_path)
            .await?;
        let external = probe
            .inspect()
            .map_err(|_| ReleasePreflightError::ProbeFailed)?;
        if [
            &external.tools.git,
            &external.tools.node,
            &external.tools.npm,
            &external.tools.cargo,
            &external.tools.gh,
        ]
        .into_iter()
        .any(|version| version.as_deref().is_none_or(str::is_empty))
        {
            return Err(ReleasePreflightError::ToolMissing);
        }
        if external.active_release_runs > 0 {
            return Err(ReleasePreflightError::ActiveReleaseRun);
        }
        if external.conflicting_drafts > 0 {
            return Err(ReleasePreflightError::ConflictingDraft);
        }
        Ok(ReleasePreflightResult {
            repository_path: repository_path
                .canonicalize()
                .map_err(|_| GitReleaseError::RepositoryInvalid)?
                .to_string_lossy()
                .into_owned(),
            repository,
            external,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GitReleaseError {
    #[error("无法读取 Git 仓库")]
    RepositoryInvalid,
    #[error("Git 远端不是目标仓库")]
    RemoteMismatch,
    #[error("Git 工作区存在未提交改动")]
    WorktreeDirty,
    #[error("本地候选提交与远端 main 不一致")]
    HeadRemoteMismatch,
    #[error("工作区改动集合与发布计划不一致")]
    PlannedFilesMismatch,
    #[error("远端 main 在提交或推送前发生变化")]
    RemoteMoved,
    #[error("无法创建发布候选提交")]
    CommitFailed,
    #[error("无法清理发布候选暂存区")]
    IndexCleanupFailed,
    #[error("无法推送发布候选到远端 main")]
    PushFailed,
    #[error("推送后远端 main 未指向候选提交")]
    RemoteVerificationFailed,
    #[error(transparent)]
    Backend(#[from] GitBackendError),
}

impl GitReleaseError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RepositoryInvalid => "GIT_REPOSITORY_INVALID",
            Self::RemoteMismatch => "GIT_REMOTE_MISMATCH",
            Self::WorktreeDirty => "GIT_WORKTREE_DIRTY",
            Self::HeadRemoteMismatch => "GIT_HEAD_REMOTE_MISMATCH",
            Self::PlannedFilesMismatch => "GIT_PLANNED_FILES_MISMATCH",
            Self::RemoteMoved => "GIT_REMOTE_MOVED",
            Self::CommitFailed => "GIT_COMMIT_FAILED",
            Self::IndexCleanupFailed => "GIT_INDEX_CLEANUP_FAILED",
            Self::PushFailed => "GIT_PUSH_FAILED",
            Self::RemoteVerificationFailed => "GIT_REMOTE_VERIFICATION_FAILED",
            Self::Backend(_) => "GIT_COMMAND_FAILED",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitPushOutcome {
    pub candidate_sha: String,
    pub remote_main_sha: String,
}

pub struct GitReleaseService {
    default_branch: String,
}

impl GitReleaseService {
    pub fn new(default_branch: impl Into<String>) -> Self {
        Self {
            default_branch: default_branch.into(),
        }
    }

    pub async fn commit_candidate(
        &self,
        backend: &GitBackend,
        repository_path: &Path,
        plan: &ReleaseCandidatePlan,
        expected_remote_sha: &str,
    ) -> Result<String, GitReleaseError> {
        if plan.files.iter().any(|file| {
            std::fs::read(repository_path.join(&file.relative_path))
                .map(|bytes| bytes != file.after)
                .unwrap_or(true)
        }) {
            return Err(GitReleaseError::PlannedFilesMismatch);
        }
        let expected_files = plan
            .files
            .iter()
            .map(|file| file.relative_path.clone())
            .collect::<BTreeSet<_>>();
        let changed_files = git_name_set(
            backend
                .run(repository_path, &["diff", "--name-only"])
                .await?
                .stdout
                .as_str(),
        );
        let untracked_files = git_name_set(
            backend
                .run(
                    repository_path,
                    &["ls-files", "--others", "--exclude-standard"],
                )
                .await?
                .stdout
                .as_str(),
        );
        let staged_before = git_name_set(
            backend
                .run(repository_path, &["diff", "--cached", "--name-only"])
                .await?
                .stdout
                .as_str(),
        );
        if changed_files != expected_files
            || !untracked_files.is_empty()
            || !staged_before.is_empty()
        {
            return Err(GitReleaseError::PlannedFilesMismatch);
        }

        if remote_head(backend, repository_path, &self.default_branch).await? != expected_remote_sha
        {
            return Err(GitReleaseError::RemoteMoved);
        }

        let mut add_arguments = vec!["add".to_string(), "--".to_string()];
        add_arguments.extend(expected_files.iter().cloned());
        let add_arguments = add_arguments.iter().map(String::as_str).collect::<Vec<_>>();
        backend.run(repository_path, &add_arguments).await?;

        let staged_after = git_name_set(
            backend
                .run(repository_path, &["diff", "--cached", "--name-only"])
                .await?
                .stdout
                .as_str(),
        );
        let unstaged_after = git_name_set(
            backend
                .run(repository_path, &["diff", "--name-only"])
                .await?
                .stdout
                .as_str(),
        );
        if staged_after != expected_files || !unstaged_after.is_empty() {
            self.unstage_candidate(backend, repository_path, plan)
                .await?;
            return Err(GitReleaseError::PlannedFilesMismatch);
        }

        let message = format!("chore(release): 准备 v{} 发布", plan.target_version);
        if backend
            .run(repository_path, &["commit", "-m", &message])
            .await
            .is_err()
        {
            self.unstage_candidate(backend, repository_path, plan)
                .await?;
            return Err(GitReleaseError::CommitFailed);
        }
        let candidate_sha = backend
            .run(repository_path, &["rev-parse", "HEAD"])
            .await?
            .stdout
            .trim()
            .to_string();

        Ok(candidate_sha)
    }

    pub async fn unstage_candidate(
        &self,
        backend: &GitBackend,
        repository_path: &Path,
        plan: &ReleaseCandidatePlan,
    ) -> Result<(), GitReleaseError> {
        let backend = backend.without_cancellation();
        let staged_before = git_name_set(
            backend
                .run(repository_path, &["diff", "--cached", "--name-only"])
                .await?
                .stdout
                .as_str(),
        );
        if staged_before.is_empty() {
            return Ok(());
        }

        let expected_files = plan
            .files
            .iter()
            .map(|file| file.relative_path.clone())
            .collect::<BTreeSet<_>>();
        let mut reset_arguments = vec!["reset".to_string(), "HEAD".to_string(), "--".to_string()];
        reset_arguments.extend(expected_files);
        let reset_arguments = reset_arguments
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        backend
            .run(repository_path, &reset_arguments)
            .await
            .map_err(|_| GitReleaseError::IndexCleanupFailed)?;

        let staged_after = git_name_set(
            backend
                .run(repository_path, &["diff", "--cached", "--name-only"])
                .await?
                .stdout
                .as_str(),
        );
        if staged_after.is_empty() {
            Ok(())
        } else {
            Err(GitReleaseError::IndexCleanupFailed)
        }
    }

    pub async fn push_candidate(
        &self,
        backend: &GitBackend,
        repository_path: &Path,
        candidate_sha: &str,
    ) -> Result<GitPushOutcome, GitReleaseError> {
        let current_head = backend
            .run(repository_path, &["rev-parse", "HEAD"])
            .await?
            .stdout
            .trim()
            .to_string();
        if current_head != candidate_sha {
            return Err(GitReleaseError::CommitFailed);
        }

        let candidate_parent_ref = format!("{candidate_sha}^");
        let candidate_parent = backend
            .run(repository_path, &["rev-parse", &candidate_parent_ref])
            .await?
            .stdout
            .trim()
            .to_string();

        if remote_head(backend, repository_path, &self.default_branch).await? != candidate_parent {
            return Err(GitReleaseError::RemoteMoved);
        }
        let push_ref = format!("HEAD:refs/heads/{}", self.default_branch);
        backend
            .run(repository_path, &["push", "origin", &push_ref])
            .await
            .map_err(|_| GitReleaseError::PushFailed)?;
        let remote_main_sha = remote_head(backend, repository_path, &self.default_branch).await?;
        if remote_main_sha != candidate_sha {
            return Err(GitReleaseError::RemoteVerificationFailed);
        }

        Ok(GitPushOutcome {
            candidate_sha: candidate_sha.to_string(),
            remote_main_sha,
        })
    }
}

pub struct RepositoryInspectionService {
    default_branch: String,
    expected_remote: ExpectedRemote,
}

enum ExpectedRemote {
    LocalPath(PathBuf),
    GithubRepository(String),
}

impl RepositoryInspectionService {
    pub fn new(default_branch: impl Into<String>, expected_remote: PathBuf) -> Self {
        Self {
            default_branch: default_branch.into(),
            expected_remote: ExpectedRemote::LocalPath(expected_remote),
        }
    }

    pub fn for_codex_relay() -> Self {
        Self {
            default_branch: "main".to_string(),
            expected_remote: ExpectedRemote::GithubRepository("hunxuankai/codex-relay".to_string()),
        }
    }

    pub fn accepts_remote_url(&self, remote_url: &str) -> bool {
        match &self.expected_remote {
            ExpectedRemote::LocalPath(expected) => {
                PathBuf::from(remote_url).canonicalize().ok() == expected.canonicalize().ok()
            }
            ExpectedRemote::GithubRepository(expected) => normalize_github_remote(remote_url)
                .is_some_and(|actual| actual.eq_ignore_ascii_case(expected)),
        }
    }

    pub async fn inspect(
        &self,
        backend: &GitBackend,
        repository_path: &Path,
    ) -> Result<RepositoryInspection, GitReleaseError> {
        let repository_root = backend
            .run(repository_path, &["rev-parse", "--show-toplevel"])
            .await?
            .stdout
            .trim()
            .to_string();
        if PathBuf::from(repository_root)
            .canonicalize()
            .map_err(|_| GitReleaseError::RepositoryInvalid)?
            != repository_path
                .canonicalize()
                .map_err(|_| GitReleaseError::RepositoryInvalid)?
        {
            return Err(GitReleaseError::RepositoryInvalid);
        }

        backend
            .run(
                repository_path,
                &["fetch", "--prune", "origin", &self.default_branch],
            )
            .await?;
        let remote_url = backend
            .run(repository_path, &["remote", "get-url", "origin"])
            .await?
            .stdout
            .trim()
            .to_string();
        if !self.accepts_remote_url(&remote_url) {
            return Err(GitReleaseError::RemoteMismatch);
        }
        let status = backend
            .run(
                repository_path,
                &["status", "--porcelain=v1", "--untracked-files=all"],
            )
            .await?
            .stdout;
        if !status.trim().is_empty() {
            return Err(GitReleaseError::WorktreeDirty);
        }
        let local_branch = backend
            .run(repository_path, &["branch", "--show-current"])
            .await?
            .stdout
            .trim()
            .to_string();
        let head_sha = backend
            .run(repository_path, &["rev-parse", "HEAD"])
            .await?
            .stdout
            .trim()
            .to_string();
        let remote_ref = format!("refs/remotes/origin/{}", self.default_branch);
        let remote_main_sha = backend
            .run(repository_path, &["rev-parse", &remote_ref])
            .await?
            .stdout
            .trim()
            .to_string();
        if head_sha != remote_main_sha {
            return Err(GitReleaseError::HeadRemoteMismatch);
        }
        Ok(RepositoryInspection {
            local_branch,
            default_branch: self.default_branch.clone(),
            head_sha,
            remote_main_sha,
            remote_url,
            clean: status.trim().is_empty(),
        })
    }

    pub async fn inspect_for_recovery(
        &self,
        backend: &GitBackend,
        repository_path: &Path,
    ) -> Result<RepositoryInspection, GitReleaseError> {
        let repository_root = backend
            .run(repository_path, &["rev-parse", "--show-toplevel"])
            .await?
            .stdout
            .trim()
            .to_string();
        if PathBuf::from(repository_root)
            .canonicalize()
            .map_err(|_| GitReleaseError::RepositoryInvalid)?
            != repository_path
                .canonicalize()
                .map_err(|_| GitReleaseError::RepositoryInvalid)?
        {
            return Err(GitReleaseError::RepositoryInvalid);
        }
        let remote_url = backend
            .run(repository_path, &["remote", "get-url", "origin"])
            .await?
            .stdout
            .trim()
            .to_string();
        if !self.accepts_remote_url(&remote_url) {
            return Err(GitReleaseError::RemoteMismatch);
        }
        let status = backend
            .run(
                repository_path,
                &["status", "--porcelain=v1", "--untracked-files=all"],
            )
            .await?
            .stdout;
        let local_branch = backend
            .run(repository_path, &["branch", "--show-current"])
            .await?
            .stdout
            .trim()
            .to_string();
        let head_sha = backend
            .run(repository_path, &["rev-parse", "HEAD"])
            .await?
            .stdout
            .trim()
            .to_string();
        let remote_ref = format!("refs/remotes/origin/{}", self.default_branch);
        let remote_main_sha = backend
            .run(repository_path, &["rev-parse", &remote_ref])
            .await?
            .stdout
            .trim()
            .to_string();
        Ok(RepositoryInspection {
            local_branch,
            default_branch: self.default_branch.clone(),
            head_sha,
            remote_main_sha,
            remote_url,
            clean: status.trim().is_empty(),
        })
    }
}

fn normalize_github_remote(remote_url: &str) -> Option<String> {
    let trimmed = remote_url.trim().trim_end_matches('/');
    let path = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("git@github.com:"))
        .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))?;
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut segments = path.split('/');
    let owner = segments.next()?;
    let repository = segments.next()?;
    if owner.is_empty() || repository.is_empty() || segments.next().is_some() {
        return None;
    }
    Some(format!("{owner}/{repository}"))
}

fn git_name_set(output: &str) -> BTreeSet<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

async fn remote_head(
    backend: &GitBackend,
    repository_path: &Path,
    default_branch: &str,
) -> Result<String, GitReleaseError> {
    let remote_ref = format!("refs/heads/{default_branch}");
    let output = backend
        .run(repository_path, &["ls-remote", "origin", &remote_ref])
        .await?
        .stdout;
    output
        .split_whitespace()
        .next()
        .map(str::to_string)
        .ok_or(GitReleaseError::RemoteVerificationFailed)
}
