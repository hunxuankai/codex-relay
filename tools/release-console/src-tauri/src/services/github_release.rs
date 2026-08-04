use crate::infrastructure::gh::{GhBackend, GhOperation, GhRequest};
pub use crate::models::{
    CleanupRunEvidence, DraftAssetEvidence, DraftAuditEvidence, PublishedReleaseEvidence,
    WorkflowDispatch, WorkflowJobStatus, WorkflowRunStatus, WorkflowStepStatus,
};
use crate::services::release_log::{
    NoopReleaseProgressSink, ReleaseProgressSink, ReleaseRunProgressDecision,
    ReleaseRunProgressTracker, format_run_progress,
};
use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufReader, Read};
use std::sync::Arc;
use std::time::Instant;

const TARGET_REPOSITORY: &str = "hunxuankai/codex-relay";
const RELEASE_WORKFLOW: &str = "release.yml";
const CLEANUP_WORKFLOW: &str = "cleanup-old-releases.yml";
const DEFAULT_BRANCH: &str = "main";
pub(crate) const RUN_DISCOVERY_ATTEMPTS: usize = 121;
pub(crate) const RUN_DISCOVERY_DELAY: std::time::Duration = std::time::Duration::from_secs(1);
pub(crate) const REMOTE_MONITOR_ATTEMPTS: usize = 2_881;
pub(crate) const REMOTE_MONITOR_DELAY: std::time::Duration = std::time::Duration::from_secs(5);

fn release_notes_equal(left: &str, right: &str) -> bool {
    fn normalize(notes: &str) -> String {
        notes
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .trim_end()
            .to_string()
    }

    normalize(left) == normalize(right)
}

#[derive(Debug, thiserror::Error)]
pub enum GithubReleaseError {
    #[error("GitHub CLI 调用失败")]
    BackendFailed,
    #[error("GitHub CLI 返回无效 JSON")]
    InvalidResponse,
    #[error("GitHub Actions Run 的候选提交不匹配")]
    CandidateMismatch,
    #[error("GitHub Actions Run 执行失败")]
    WorkflowRunFailed,
    #[error("未找到唯一的新 GitHub Actions Run")]
    WorkflowRunNotUnique,
    #[error("未找到唯一的目标 Draft Release")]
    DraftNotUnique,
    #[error("Draft Release 审计失败")]
    DraftAuditFailed,
    #[error("Draft Release 身份或资产证据发生漂移")]
    DraftIdentityChanged,
    #[error("公开 Release 在线复核失败")]
    PublishedAuditFailed,
    #[error("未找到唯一的历史 Release 清理 Run")]
    CleanupRunNotUnique,
    #[error("Draft Release 资产下载失败")]
    AssetDownloadFailed,
}

impl GithubReleaseError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::BackendFailed => "GITHUB_BACKEND_FAILED",
            Self::InvalidResponse => "GITHUB_RESPONSE_INVALID",
            Self::CandidateMismatch => "GITHUB_RUN_SHA_MISMATCH",
            Self::WorkflowRunFailed => "GITHUB_RUN_FAILED",
            Self::WorkflowRunNotUnique => "GITHUB_RUN_NOT_UNIQUE",
            Self::DraftNotUnique => "GITHUB_DRAFT_NOT_UNIQUE",
            Self::DraftAuditFailed => "GITHUB_DRAFT_AUDIT_FAILED",
            Self::DraftIdentityChanged => "GITHUB_DRAFT_IDENTITY_CHANGED",
            Self::PublishedAuditFailed => "GITHUB_PUBLISHED_AUDIT_FAILED",
            Self::CleanupRunNotUnique => "GITHUB_CLEANUP_RUN_NOT_UNIQUE",
            Self::AssetDownloadFailed => "GITHUB_ASSET_DOWNLOAD_FAILED",
        }
    }
}

#[derive(Default)]
pub struct DraftAuditService;

enum ReleaseAuditLookup {
    Draft,
    Latest { release_id: u64 },
}

impl DraftAuditService {
    pub fn new() -> Self {
        Self
    }

    pub async fn audit(
        &self,
        backend: &dyn GhBackend,
        target_version: &str,
        candidate_sha: &str,
        expected_notes: &str,
    ) -> Result<DraftAuditEvidence, GithubReleaseError> {
        self.audit_with_lookup(
            backend,
            target_version,
            candidate_sha,
            expected_notes,
            ReleaseAuditLookup::Draft,
        )
        .await
    }

    pub async fn audit_published(
        &self,
        backend: &dyn GhBackend,
        release_id: u64,
        target_version: &str,
        candidate_sha: &str,
        expected_notes: &str,
    ) -> Result<DraftAuditEvidence, GithubReleaseError> {
        self.audit_with_lookup(
            backend,
            target_version,
            candidate_sha,
            expected_notes,
            ReleaseAuditLookup::Latest { release_id },
        )
        .await
    }

    async fn audit_with_lookup(
        &self,
        backend: &dyn GhBackend,
        target_version: &str,
        candidate_sha: &str,
        expected_notes: &str,
        lookup: ReleaseAuditLookup,
    ) -> Result<DraftAuditEvidence, GithubReleaseError> {
        let tag_name = format!("v{target_version}");
        let (release, expected_draft) = match lookup {
            ReleaseAuditLookup::Draft => {
                let release_response = backend
                    .execute(GhRequest {
                        operation: GhOperation::ListDraftReleases,
                        repository: TARGET_REPOSITORY.to_string(),
                        workflow: None,
                        git_ref: None,
                        tag_name: Some(tag_name.clone()),
                        head_sha: None,
                        created_after: None,
                        resource_id: None,
                        stdin: None,
                    })
                    .await
                    .map_err(|_| GithubReleaseError::BackendFailed)?;
                let releases: Vec<RawRelease> = serde_json::from_slice(&release_response.stdout)
                    .map_err(|_| GithubReleaseError::InvalidResponse)?;
                let matching = releases
                    .into_iter()
                    .filter(|release| release.tag_name == tag_name)
                    .collect::<Vec<_>>();
                if matching.len() != 1 {
                    return Err(GithubReleaseError::DraftNotUnique);
                }
                (
                    matching
                        .into_iter()
                        .next()
                        .ok_or(GithubReleaseError::DraftNotUnique)?,
                    true,
                )
            }
            ReleaseAuditLookup::Latest { release_id } => {
                let release_response = backend
                    .execute(GhRequest {
                        operation: GhOperation::LatestRelease,
                        repository: TARGET_REPOSITORY.to_string(),
                        workflow: None,
                        git_ref: None,
                        tag_name: Some(tag_name.clone()),
                        head_sha: None,
                        created_after: None,
                        resource_id: Some(release_id),
                        stdin: None,
                    })
                    .await
                    .map_err(|_| GithubReleaseError::BackendFailed)?;
                let release: RawRelease = serde_json::from_slice(&release_response.stdout)
                    .map_err(|_| GithubReleaseError::InvalidResponse)?;
                if release.id != release_id || release.tag_name != tag_name {
                    return Err(GithubReleaseError::DraftIdentityChanged);
                }
                (release, false)
            }
        };
        if release.draft != expected_draft
            || release.prerelease
            || release.name != format!("Codex Relay v{target_version}")
            || release.target_commitish != candidate_sha
            || !release_notes_equal(&release.body, expected_notes)
        {
            return Err(GithubReleaseError::DraftAuditFailed);
        }

        let expected_asset_names = [
            format!("Codex.Relay_{target_version}_x64-setup.exe"),
            format!("Codex.Relay_{target_version}_x64-setup.exe.sig"),
            "latest.json".to_string(),
        ];
        let mut assets = release.assets;
        assets.sort_by(|left, right| left.name.cmp(&right.name));
        let mut sorted_expected = expected_asset_names.to_vec();
        sorted_expected.sort();
        if assets.iter().map(|asset| &asset.name).collect::<Vec<_>>()
            != sorted_expected.iter().collect::<Vec<_>>()
        {
            return Err(GithubReleaseError::DraftAuditFailed);
        }

        let target_commit_sha = if expected_draft {
            release.target_commitish.clone()
        } else {
            let tag_response = backend
                .execute(GhRequest {
                    operation: GhOperation::GetTag,
                    repository: TARGET_REPOSITORY.to_string(),
                    workflow: None,
                    git_ref: None,
                    tag_name: Some(tag_name.clone()),
                    head_sha: None,
                    created_after: None,
                    resource_id: None,
                    stdin: None,
                })
                .await
                .map_err(|_| GithubReleaseError::BackendFailed)?;
            let tag: RawTag = serde_json::from_slice(&tag_response.stdout)
                .map_err(|_| GithubReleaseError::InvalidResponse)?;
            if tag.object.kind != "commit" || tag.object.sha != candidate_sha {
                return Err(GithubReleaseError::DraftAuditFailed);
            }
            tag.object.sha
        };

        let temp_root = std::env::temp_dir()
            .canonicalize()
            .map_err(|_| GithubReleaseError::AssetDownloadFailed)?;
        let workspace = tempfile::Builder::new()
            .prefix("codex-relay-release-assets-")
            .tempdir_in(&temp_root)
            .map_err(|_| GithubReleaseError::AssetDownloadFailed)?;
        if !workspace.path().starts_with(&temp_root) {
            return Err(GithubReleaseError::AssetDownloadFailed);
        }

        let mut evidence = Vec::with_capacity(assets.len());
        let mut downloaded = BTreeMap::new();
        for asset in &assets {
            let destination = workspace.path().join(&asset.name);
            backend
                .download_asset(asset.id, &destination)
                .await
                .map_err(|_| GithubReleaseError::AssetDownloadFailed)?;
            let actual_size =
                fs::metadata(&destination).map_err(|_| GithubReleaseError::AssetDownloadFailed)?;
            if actual_size.len() != asset.size {
                return Err(GithubReleaseError::DraftAuditFailed);
            }
            let sha256 =
                sha256_file(&destination).map_err(|_| GithubReleaseError::AssetDownloadFailed)?;
            let expected_digest = format!("sha256:{sha256}");
            if asset
                .digest
                .as_deref()
                .is_none_or(|digest| !digest.eq_ignore_ascii_case(&expected_digest))
            {
                return Err(GithubReleaseError::DraftAuditFailed);
            }
            evidence.push(DraftAssetEvidence {
                id: asset.id,
                name: asset.name.clone(),
                size: asset.size,
                sha256,
            });
            downloaded.insert(asset.name.clone(), destination);
        }
        evidence.sort_by(|left, right| left.name.cmp(&right.name));

        let manifest_bytes = fs::read(
            downloaded
                .get("latest.json")
                .ok_or(GithubReleaseError::DraftAuditFailed)?,
        )
        .map_err(|_| GithubReleaseError::AssetDownloadFailed)?;
        let manifest: RawUpdateManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|_| GithubReleaseError::DraftAuditFailed)?;
        if manifest.version != target_version
            || !release_notes_equal(&manifest.notes, expected_notes)
        {
            return Err(GithubReleaseError::DraftAuditFailed);
        }
        let installer_name = format!("Codex.Relay_{target_version}_x64-setup.exe");
        let installer = assets
            .iter()
            .find(|asset| asset.name == installer_name)
            .ok_or(GithubReleaseError::DraftAuditFailed)?;
        let expected_url = format!(
            "https://api.github.com/repos/{TARGET_REPOSITORY}/releases/assets/{}",
            installer.id
        );
        if manifest.platforms.len() != 2 {
            return Err(GithubReleaseError::DraftAuditFailed);
        }
        let desktop = manifest
            .platforms
            .get("windows-x86_64")
            .ok_or(GithubReleaseError::DraftAuditFailed)?;
        let nsis = manifest
            .platforms
            .get("windows-x86_64-nsis")
            .ok_or(GithubReleaseError::DraftAuditFailed)?;
        let signature_file = fs::read_to_string(
            downloaded
                .get(&format!("{installer_name}.sig"))
                .ok_or(GithubReleaseError::DraftAuditFailed)?,
        )
        .map_err(|_| GithubReleaseError::DraftAuditFailed)?;
        let signature_file = signature_file.trim_end_matches(['\r', '\n']);
        if desktop.url != expected_url
            || nsis.url != expected_url
            || desktop.signature != nsis.signature
            || desktop.signature != signature_file
        {
            return Err(GithubReleaseError::DraftAuditFailed);
        }

        Ok(DraftAuditEvidence {
            release_id: release.id,
            tag_name,
            target_commit_sha,
            assets: evidence,
            manifest_version: manifest.version,
            manifest_notes: manifest.notes,
            signature: desktop.signature.clone(),
        })
    }
}

pub struct GithubReleaseService {
    progress: Arc<dyn ReleaseProgressSink>,
}

impl Default for GithubReleaseService {
    fn default() -> Self {
        Self::new()
    }
}

impl GithubReleaseService {
    pub fn new() -> Self {
        Self {
            progress: Arc::new(NoopReleaseProgressSink),
        }
    }

    pub fn with_progress(mut self, progress: Arc<dyn ReleaseProgressSink>) -> Self {
        self.progress = progress;
        self
    }

    pub async fn dispatch_release(
        &self,
        backend: &dyn GhBackend,
        target_version: &str,
        candidate_sha: &str,
    ) -> Result<WorkflowDispatch, GithubReleaseError> {
        let dispatched_after = Utc::now()
            .with_nanosecond(0)
            .ok_or(GithubReleaseError::InvalidResponse)?;
        let stdin = serde_json::to_vec(&serde_json::json!({
            "expected_version": target_version,
            "expected_sha": candidate_sha,
        }))
        .map_err(|_| GithubReleaseError::InvalidResponse)?;
        let response = backend
            .execute(GhRequest {
                operation: GhOperation::DispatchReleaseWorkflow,
                repository: TARGET_REPOSITORY.to_string(),
                workflow: Some(RELEASE_WORKFLOW.to_string()),
                git_ref: Some(DEFAULT_BRANCH.to_string()),
                tag_name: None,
                head_sha: None,
                created_after: None,
                resource_id: None,
                stdin: Some(stdin),
            })
            .await
            .map_err(|_| GithubReleaseError::BackendFailed)?;
        if let Ok(dispatch) = parse_workflow_dispatch(&response.stdout) {
            return Ok(dispatch);
        }

        let created_after = dispatched_after.to_rfc3339_opts(SecondsFormat::Secs, true);
        for attempt in 0..RUN_DISCOVERY_ATTEMPTS {
            let response = backend
                .execute(GhRequest {
                    operation: GhOperation::ListReleaseRuns,
                    repository: TARGET_REPOSITORY.to_string(),
                    workflow: Some(RELEASE_WORKFLOW.to_string()),
                    git_ref: Some(DEFAULT_BRANCH.to_string()),
                    tag_name: None,
                    head_sha: Some(candidate_sha.to_string()),
                    created_after: Some(created_after.clone()),
                    resource_id: None,
                    stdin: None,
                })
                .await
                .map_err(|_| GithubReleaseError::BackendFailed)?;
            let runs: Vec<RawListedWorkflowRun> = serde_json::from_slice(&response.stdout)
                .map_err(|_| GithubReleaseError::InvalidResponse)?;
            let matching = runs
                .into_iter()
                .filter(|run| {
                    run.head_sha == candidate_sha
                        && DateTime::parse_from_rfc3339(&run.created_at)
                            .map(|created_at| created_at >= dispatched_after)
                            .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            match matching.as_slice() {
                [run] => {
                    return Ok(WorkflowDispatch {
                        run_id: run.database_id,
                        url: run.url.clone(),
                    });
                }
                [] if attempt + 1 < RUN_DISCOVERY_ATTEMPTS => {
                    tokio::time::sleep(RUN_DISCOVERY_DELAY).await;
                }
                _ => return Err(GithubReleaseError::WorkflowRunNotUnique),
            }
        }
        Err(GithubReleaseError::WorkflowRunNotUnique)
    }

    pub async fn get_release_run(
        &self,
        backend: &dyn GhBackend,
        run_id: u64,
        expected_sha: &str,
    ) -> Result<WorkflowRunStatus, GithubReleaseError> {
        let response = backend
            .execute(GhRequest {
                operation: GhOperation::ViewReleaseRun,
                repository: TARGET_REPOSITORY.to_string(),
                workflow: Some(RELEASE_WORKFLOW.to_string()),
                git_ref: Some(DEFAULT_BRANCH.to_string()),
                tag_name: None,
                head_sha: None,
                created_after: None,
                resource_id: Some(run_id),
                stdin: None,
            })
            .await
            .map_err(|_| GithubReleaseError::BackendFailed)?;
        let raw: RawWorkflowRun = serde_json::from_slice(&response.stdout)
            .map_err(|_| GithubReleaseError::InvalidResponse)?;
        if raw.head_sha != expected_sha {
            return Err(GithubReleaseError::CandidateMismatch);
        }
        if raw.status == "completed" && raw.conclusion.as_deref() != Some("success") {
            return Err(GithubReleaseError::WorkflowRunFailed);
        }
        Ok(workflow_status_from_raw(raw))
    }

    pub async fn publish_release(
        &self,
        backend: &dyn GhBackend,
        expected_draft: &DraftAuditEvidence,
        target_version: &str,
        candidate_sha: &str,
        expected_notes: &str,
    ) -> Result<PublishedReleaseEvidence, GithubReleaseError> {
        let current = DraftAuditService::new()
            .audit(backend, target_version, candidate_sha, expected_notes)
            .await;
        let current = match current {
            Ok(current) => current,
            Err(GithubReleaseError::DraftNotUnique) => {
                let published = DraftAuditService::new()
                    .audit_published(
                        backend,
                        expected_draft.release_id,
                        target_version,
                        candidate_sha,
                        expected_notes,
                    )
                    .await?;
                if &published != expected_draft {
                    return Err(GithubReleaseError::PublishedAuditFailed);
                }
                return self
                    .resolve_published_identity(backend, expected_draft)
                    .await;
            }
            Err(error) => return Err(error),
        };
        if &current != expected_draft {
            return Err(GithubReleaseError::DraftIdentityChanged);
        }
        let stdin = serde_json::to_vec(&serde_json::json!({ "draft": false }))
            .map_err(|_| GithubReleaseError::InvalidResponse)?;
        let response = backend
            .execute(GhRequest {
                operation: GhOperation::PublishRelease,
                repository: TARGET_REPOSITORY.to_string(),
                workflow: None,
                git_ref: None,
                tag_name: Some(expected_draft.tag_name.clone()),
                head_sha: Some(candidate_sha.to_string()),
                created_after: None,
                resource_id: Some(expected_draft.release_id),
                stdin: Some(stdin),
            })
            .await
            .map_err(|_| GithubReleaseError::BackendFailed)?;
        let published: RawPublishedRelease = serde_json::from_slice(&response.stdout)
            .map_err(|_| GithubReleaseError::InvalidResponse)?;
        if published.id != expected_draft.release_id
            || published.tag_name != expected_draft.tag_name
            || published.draft
            || published.prerelease
            || DateTime::parse_from_rfc3339(&published.published_at).is_err()
        {
            return Err(GithubReleaseError::DraftIdentityChanged);
        }
        Ok(PublishedReleaseEvidence {
            release_id: published.id,
            tag_name: published.tag_name,
            published_at: published.published_at,
        })
    }

    async fn resolve_published_identity(
        &self,
        backend: &dyn GhBackend,
        expected_draft: &DraftAuditEvidence,
    ) -> Result<PublishedReleaseEvidence, GithubReleaseError> {
        let response = backend
            .execute(GhRequest {
                operation: GhOperation::LatestRelease,
                repository: TARGET_REPOSITORY.to_string(),
                workflow: None,
                git_ref: None,
                tag_name: Some(expected_draft.tag_name.clone()),
                head_sha: Some(expected_draft.target_commit_sha.clone()),
                created_after: None,
                resource_id: Some(expected_draft.release_id),
                stdin: None,
            })
            .await
            .map_err(|_| GithubReleaseError::BackendFailed)?;
        let release: RawRelease = serde_json::from_slice(&response.stdout)
            .map_err(|_| GithubReleaseError::InvalidResponse)?;
        let published_at = release
            .published_at
            .filter(|value| DateTime::parse_from_rfc3339(value).is_ok())
            .ok_or(GithubReleaseError::PublishedAuditFailed)?;
        if release.id != expected_draft.release_id
            || release.tag_name != expected_draft.tag_name
            || release.draft
            || release.prerelease
        {
            return Err(GithubReleaseError::PublishedAuditFailed);
        }
        Ok(PublishedReleaseEvidence {
            release_id: release.id,
            tag_name: release.tag_name,
            published_at,
        })
    }

    pub async fn verify_published_release(
        &self,
        backend: &dyn GhBackend,
        expected_draft: &DraftAuditEvidence,
        published: &PublishedReleaseEvidence,
        target_version: &str,
        candidate_sha: &str,
        expected_notes: &str,
    ) -> Result<DraftAuditEvidence, GithubReleaseError> {
        if published.release_id != expected_draft.release_id
            || published.tag_name != expected_draft.tag_name
        {
            return Err(GithubReleaseError::PublishedAuditFailed);
        }
        let verified = DraftAuditService::new()
            .audit_published(
                backend,
                published.release_id,
                target_version,
                candidate_sha,
                expected_notes,
            )
            .await?;
        if &verified != expected_draft {
            return Err(GithubReleaseError::PublishedAuditFailed);
        }
        Ok(verified)
    }

    pub async fn monitor_cleanup(
        &self,
        backend: &dyn GhBackend,
        published_at: &str,
    ) -> Result<CleanupRunEvidence, GithubReleaseError> {
        let published_at = DateTime::parse_from_rfc3339(published_at)
            .map_err(|_| GithubReleaseError::InvalidResponse)?;
        let created_after = published_at.to_rfc3339_opts(SecondsFormat::Secs, true);
        let mut cleanup_run = None;
        for attempt in 0..RUN_DISCOVERY_ATTEMPTS {
            let response = backend
                .execute(GhRequest {
                    operation: GhOperation::CleanupRuns,
                    repository: TARGET_REPOSITORY.to_string(),
                    workflow: Some(CLEANUP_WORKFLOW.to_string()),
                    git_ref: Some(DEFAULT_BRANCH.to_string()),
                    tag_name: None,
                    head_sha: None,
                    created_after: Some(created_after.clone()),
                    resource_id: None,
                    stdin: None,
                })
                .await
                .map_err(|_| GithubReleaseError::BackendFailed)?;
            let runs: Vec<RawCleanupRun> = serde_json::from_slice(&response.stdout)
                .map_err(|_| GithubReleaseError::InvalidResponse)?;
            let matching = runs
                .into_iter()
                .filter(|run| {
                    DateTime::parse_from_rfc3339(&run.created_at)
                        .map(|created_at| created_at >= published_at)
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            match matching.as_slice() {
                [run] => {
                    cleanup_run = Some((run.database_id, run.url.clone()));
                    break;
                }
                [] if attempt + 1 < RUN_DISCOVERY_ATTEMPTS => {
                    tokio::time::sleep(RUN_DISCOVERY_DELAY).await;
                }
                _ => return Err(GithubReleaseError::CleanupRunNotUnique),
            }
        }
        let (run_id, discovered_url) =
            cleanup_run.ok_or(GithubReleaseError::CleanupRunNotUnique)?;

        let monitor_started = Instant::now();
        let mut tracker = ReleaseRunProgressTracker::new();
        for attempt in 0..REMOTE_MONITOR_ATTEMPTS {
            let response = backend
                .execute(GhRequest {
                    operation: GhOperation::ViewReleaseRun,
                    repository: TARGET_REPOSITORY.to_string(),
                    workflow: Some(CLEANUP_WORKFLOW.to_string()),
                    git_ref: Some(DEFAULT_BRANCH.to_string()),
                    tag_name: None,
                    head_sha: None,
                    created_after: None,
                    resource_id: Some(run_id),
                    stdin: None,
                })
                .await
                .map_err(|_| GithubReleaseError::BackendFailed)?;
            let raw: RawWorkflowRun = serde_json::from_slice(&response.stdout)
                .map_err(|_| GithubReleaseError::InvalidResponse)?;
            let status = workflow_status_from_raw(raw);
            if status.id != run_id || status.url != discovered_url {
                return Err(GithubReleaseError::CleanupRunNotUnique);
            }
            let decision = tracker.observe(monitor_started.elapsed(), &status);
            if decision != ReleaseRunProgressDecision::Silent {
                self.progress.log(
                    "cleanup",
                    crate::models::ReleaseLogLevel::Info,
                    &format_run_progress(&status, decision),
                );
            }
            if status.status == "completed" {
                return Ok(CleanupRunEvidence {
                    run_id,
                    url: status.url,
                    status: status.status,
                    succeeded: status.conclusion.as_deref() == Some("success"),
                    conclusion: status.conclusion,
                    jobs: status.jobs,
                });
            }
            if attempt + 1 < REMOTE_MONITOR_ATTEMPTS {
                tokio::time::sleep(REMOTE_MONITOR_DELAY).await;
            }
        }
        Err(GithubReleaseError::CleanupRunNotUnique)
    }
}

fn parse_workflow_dispatch(stdout: &[u8]) -> Result<WorkflowDispatch, GithubReleaseError> {
    if let Ok(dispatch) = serde_json::from_slice(stdout) {
        return Ok(dispatch);
    }
    let url = std::str::from_utf8(stdout)
        .map_err(|_| GithubReleaseError::InvalidResponse)?
        .trim();
    let prefix = format!("https://github.com/{TARGET_REPOSITORY}/actions/runs/");
    let run_id = url
        .strip_prefix(&prefix)
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(GithubReleaseError::InvalidResponse)?;
    Ok(WorkflowDispatch {
        run_id,
        url: url.to_string(),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWorkflowRun {
    database_id: u64,
    status: String,
    conclusion: Option<String>,
    head_sha: String,
    url: String,
    jobs: Vec<RawWorkflowJob>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawListedWorkflowRun {
    database_id: u64,
    head_sha: String,
    created_at: String,
    url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCleanupRun {
    database_id: u64,
    created_at: String,
    url: String,
}

#[derive(Deserialize)]
struct RawPublishedRelease {
    id: u64,
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWorkflowJob {
    name: String,
    status: String,
    conclusion: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    steps: Vec<RawWorkflowStep>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawWorkflowStep {
    name: String,
    number: u64,
    status: String,
    conclusion: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
}

fn duration_millis(started_at: Option<&str>, completed_at: Option<&str>) -> Option<u64> {
    let started = DateTime::parse_from_rfc3339(started_at?).ok()?;
    let completed = DateTime::parse_from_rfc3339(completed_at?).ok()?;
    u64::try_from((completed - started).num_milliseconds()).ok()
}

fn workflow_status_from_raw(raw: RawWorkflowRun) -> WorkflowRunStatus {
    WorkflowRunStatus {
        id: raw.database_id,
        status: raw.status,
        conclusion: raw.conclusion,
        head_sha: raw.head_sha,
        url: raw.url,
        jobs: raw
            .jobs
            .into_iter()
            .map(|job| WorkflowJobStatus {
                duration_millis: duration_millis(
                    job.started_at.as_deref(),
                    job.completed_at.as_deref(),
                ),
                name: job.name,
                status: job.status,
                conclusion: job.conclusion,
                started_at: job.started_at,
                completed_at: job.completed_at,
                steps: job
                    .steps
                    .into_iter()
                    .map(|step| WorkflowStepStatus {
                        duration_millis: duration_millis(
                            step.started_at.as_deref(),
                            step.completed_at.as_deref(),
                        ),
                        name: step.name,
                        number: step.number,
                        status: step.status,
                        conclusion: step.conclusion,
                        started_at: step.started_at,
                        completed_at: step.completed_at,
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[derive(Deserialize)]
struct RawRelease {
    id: u64,
    tag_name: String,
    name: String,
    target_commitish: String,
    draft: bool,
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
    body: String,
    assets: Vec<RawReleaseAsset>,
}

#[derive(Deserialize)]
struct RawReleaseAsset {
    id: u64,
    name: String,
    size: u64,
    digest: Option<String>,
}

#[derive(Deserialize)]
struct RawTag {
    object: RawTagObject,
}

#[derive(Deserialize)]
struct RawTagObject {
    #[serde(rename = "type")]
    kind: String,
    sha: String,
}

#[derive(Deserialize)]
struct RawUpdateManifest {
    version: String,
    notes: String,
    platforms: BTreeMap<String, RawUpdatePlatform>,
}

#[derive(Deserialize)]
struct RawUpdatePlatform {
    url: String,
    signature: String,
}

fn sha256_file(path: &std::path::Path) -> std::io::Result<String> {
    let mut reader = BufReader::new(fs::File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_monitor_budgets_cover_slow_github_actions_runs() {
        let discovery_budget =
            RUN_DISCOVERY_DELAY.saturating_mul(RUN_DISCOVERY_ATTEMPTS.saturating_sub(1) as u32);
        let monitor_budget =
            REMOTE_MONITOR_DELAY.saturating_mul(REMOTE_MONITOR_ATTEMPTS.saturating_sub(1) as u32);

        assert!(discovery_budget >= std::time::Duration::from_secs(2 * 60));
        assert!(monitor_budget >= std::time::Duration::from_secs(4 * 60 * 60));
    }
}
