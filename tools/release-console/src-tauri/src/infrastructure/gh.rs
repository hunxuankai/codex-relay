use super::process::{
    ProcessError, ProcessInvocation, SafeProcessRunner, filter_release_environment,
};
use std::ffi::OsString;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::Duration;

const GH_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const TARGET_REPOSITORY: &str = "hunxuankai/codex-relay";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GhOperation {
    ConnectionTest,
    DispatchReleaseWorkflow,
    PreflightReleaseRuns,
    ListReleaseRuns,
    ViewReleaseRun,
    ListDraftReleases,
    GetRelease,
    GetTag,
    PublishRelease,
    LatestRelease,
    CleanupRuns,
}

#[derive(Clone, Eq, PartialEq)]
pub struct GhRequest {
    pub operation: GhOperation,
    pub repository: String,
    pub workflow: Option<String>,
    pub git_ref: Option<String>,
    pub tag_name: Option<String>,
    pub head_sha: Option<String>,
    pub created_after: Option<String>,
    pub resource_id: Option<u64>,
    pub stdin: Option<Vec<u8>>,
}

impl fmt::Debug for GhRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GhRequest")
            .field("operation", &self.operation)
            .field("repository", &self.repository)
            .field("workflow", &self.workflow)
            .field("git_ref", &self.git_ref)
            .field("tag_name", &self.tag_name)
            .field("head_sha", &self.head_sha)
            .field("created_after", &self.created_after)
            .field("resource_id", &self.resource_id)
            .field("stdin_len", &self.stdin.as_ref().map(Vec::len))
            .finish()
    }
}

pub struct GhResponse {
    pub stdout: Vec<u8>,
}

impl fmt::Debug for GhResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GhResponse")
            .field("stdout_len", &self.stdout.len())
            .finish()
    }
}

pub trait GhBackend: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>>;

    fn download_asset<'a>(
        &'a self,
        asset_id: u64,
        destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

pub struct SystemGhBackend {
    executable: PathBuf,
    environment: Vec<(OsString, OsString)>,
    workdir: PathBuf,
    cancel: tokio::sync::watch::Receiver<bool>,
    runner: SafeProcessRunner,
}

impl SystemGhBackend {
    pub fn new(
        executable: PathBuf,
        environment: Vec<(OsString, OsString)>,
        workdir: PathBuf,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            executable,
            environment: filter_release_environment(environment),
            workdir,
            cancel,
            runner: SafeProcessRunner::default(),
        }
    }

    pub fn invocation_for(&self, request: &GhRequest) -> Result<ProcessInvocation, String> {
        if request.repository != TARGET_REPOSITORY {
            return Err("unsupported repository".into());
        }
        let args = match request.operation {
            GhOperation::ConnectionTest => ["api", "repos/hunxuankai/codex-relay", "--silent"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            GhOperation::DispatchReleaseWorkflow => {
                let workflow = request
                    .workflow
                    .as_deref()
                    .filter(|workflow| *workflow == "release.yml")
                    .ok_or_else(|| "unsupported workflow".to_string())?;
                let git_ref = request
                    .git_ref
                    .as_deref()
                    .filter(|git_ref| *git_ref == "main")
                    .ok_or_else(|| "unsupported ref".to_string())?;
                [
                    "workflow",
                    "run",
                    workflow,
                    "--repo",
                    TARGET_REPOSITORY,
                    "--ref",
                    git_ref,
                    "--json",
                ]
                .into_iter()
                .map(OsString::from)
                .collect()
            }
            GhOperation::PreflightReleaseRuns => {
                let workflow = request
                    .workflow
                    .as_deref()
                    .filter(|workflow| *workflow == "release.yml")
                    .ok_or_else(|| "unsupported workflow".to_string())?;
                [
                    "run".into(),
                    "list".into(),
                    "--repo".into(),
                    TARGET_REPOSITORY.into(),
                    "--workflow".into(),
                    workflow.into(),
                    "--limit".into(),
                    "20".into(),
                    "--json".into(),
                    "databaseId,status,conclusion,headSha,url".into(),
                ]
                .to_vec()
            }
            GhOperation::ViewReleaseRun => {
                let run_id = request
                    .resource_id
                    .ok_or_else(|| "missing run id".to_string())?
                    .to_string();
                [
                    "run",
                    "view",
                    run_id.as_str(),
                    "--repo",
                    TARGET_REPOSITORY,
                    "--json",
                    "databaseId,status,conclusion,headSha,url,jobs",
                ]
                .into_iter()
                .map(OsString::from)
                .collect()
            }
            GhOperation::ListReleaseRuns => {
                let workflow = request
                    .workflow
                    .as_deref()
                    .filter(|workflow| *workflow == "release.yml")
                    .ok_or_else(|| "unsupported workflow".to_string())?;
                let git_ref = request
                    .git_ref
                    .as_deref()
                    .filter(|git_ref| *git_ref == "main")
                    .ok_or_else(|| "unsupported ref".to_string())?;
                let head_sha = request
                    .head_sha
                    .as_deref()
                    .filter(|sha| {
                        sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit())
                    })
                    .ok_or_else(|| "invalid head sha".to_string())?;
                let created_after = request
                    .created_after
                    .as_deref()
                    .filter(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok())
                    .ok_or_else(|| "invalid created time".to_string())?;
                [
                    "run".into(),
                    "list".into(),
                    "--repo".into(),
                    TARGET_REPOSITORY.into(),
                    "--workflow".into(),
                    workflow.into(),
                    "--branch".into(),
                    git_ref.into(),
                    "--event".into(),
                    "workflow_dispatch".into(),
                    "--commit".into(),
                    head_sha.into(),
                    "--created".into(),
                    format!(">={created_after}").into(),
                    "--limit".into(),
                    "10".into(),
                    "--json".into(),
                    "databaseId,headSha,createdAt,url".into(),
                ]
                .to_vec()
            }
            GhOperation::ListDraftReleases => {
                ["api", "repos/hunxuankai/codex-relay/releases?per_page=100"]
                    .into_iter()
                    .map(OsString::from)
                    .collect()
            }
            GhOperation::GetTag => {
                let tag_name = request
                    .tag_name
                    .as_deref()
                    .filter(|tag| tag.starts_with('v') && !tag.contains(['/', '\\']))
                    .ok_or_else(|| "invalid tag".to_string())?;
                [
                    "api".into(),
                    format!("repos/{TARGET_REPOSITORY}/git/ref/tags/{tag_name}").into(),
                ]
                .to_vec()
            }
            GhOperation::PublishRelease => {
                let release_id = request
                    .resource_id
                    .ok_or_else(|| "missing release id".to_string())?;
                let body: serde_json::Value = serde_json::from_slice(
                    request
                        .stdin
                        .as_deref()
                        .ok_or_else(|| "missing publish body".to_string())?,
                )
                .map_err(|_| "invalid publish body".to_string())?;
                if body != serde_json::json!({ "draft": false }) {
                    return Err("invalid publish body".into());
                }
                [
                    "api".into(),
                    "--method".into(),
                    "PATCH".into(),
                    format!("repos/{TARGET_REPOSITORY}/releases/{release_id}").into(),
                    "--input".into(),
                    "-".into(),
                ]
                .to_vec()
            }
            GhOperation::LatestRelease => ["api", "repos/hunxuankai/codex-relay/releases/latest"]
                .into_iter()
                .map(OsString::from)
                .collect(),
            GhOperation::CleanupRuns => {
                let workflow = request
                    .workflow
                    .as_deref()
                    .filter(|workflow| *workflow == "cleanup-old-releases.yml")
                    .ok_or_else(|| "unsupported workflow".to_string())?;
                let git_ref = request
                    .git_ref
                    .as_deref()
                    .filter(|git_ref| *git_ref == "main")
                    .ok_or_else(|| "unsupported ref".to_string())?;
                let created_after = request
                    .created_after
                    .as_deref()
                    .filter(|value| chrono::DateTime::parse_from_rfc3339(value).is_ok())
                    .ok_or_else(|| "invalid created time".to_string())?;
                [
                    "run".into(),
                    "list".into(),
                    "--repo".into(),
                    TARGET_REPOSITORY.into(),
                    "--workflow".into(),
                    workflow.into(),
                    "--branch".into(),
                    git_ref.into(),
                    "--event".into(),
                    "release".into(),
                    "--created".into(),
                    format!(">={created_after}").into(),
                    "--limit".into(),
                    "10".into(),
                    "--json".into(),
                    "databaseId,status,conclusion,createdAt,url".into(),
                ]
                .to_vec()
            }
            _ => return Err("unsupported gh operation".into()),
        };
        Ok(ProcessInvocation {
            executable: self.executable.clone(),
            args,
            env: self.environment.clone(),
            workdir: self.workdir.clone(),
            stdin: request.stdin.clone(),
            stdout_file: None,
        })
    }

    pub fn asset_download_invocation(
        &self,
        asset_id: u64,
        destination: &Path,
    ) -> ProcessInvocation {
        ProcessInvocation {
            executable: self.executable.clone(),
            args: [
                "api".into(),
                "-H".into(),
                "Accept: application/octet-stream".into(),
                format!("repos/{TARGET_REPOSITORY}/releases/assets/{asset_id}").into(),
            ]
            .to_vec(),
            env: self.environment.clone(),
            workdir: self.workdir.clone(),
            stdin: None,
            stdout_file: Some(destination.to_path_buf()),
        }
    }
}

impl GhBackend for SystemGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let invocation = self.invocation_for(&request)?;
            let output = self
                .runner
                .run(invocation, GH_COMMAND_TIMEOUT, self.cancel.clone(), None)
                .await
                .map_err(gh_process_failure)?;
            if output.exit_code != Some(0) {
                return Err("GH_COMMAND_FAILED".into());
            }
            Ok(GhResponse {
                stdout: output.stdout,
            })
        })
    }

    fn download_asset<'a>(
        &'a self,
        asset_id: u64,
        destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move {
            let output = self
                .runner
                .run(
                    self.asset_download_invocation(asset_id, destination),
                    GH_COMMAND_TIMEOUT,
                    self.cancel.clone(),
                    None,
                )
                .await
                .map_err(gh_process_failure)?;
            if output.exit_code != Some(0) || !destination.is_file() {
                return Err("GH_ASSET_DOWNLOAD_FAILED".into());
            }
            Ok(())
        })
    }
}

fn gh_process_failure(error: ProcessError) -> String {
    match error {
        ProcessError::Timeout => "GH_PROCESS_TIMEOUT",
        ProcessError::Cancelled => "GH_PROCESS_CANCELLED",
        ProcessError::ProcessTreeTermination => "GH_PROCESS_TREE_TERMINATION_FAILED",
        ProcessError::OutputTooLarge => "GH_OUTPUT_TOO_LARGE",
        ProcessError::JobUnavailable
        | ProcessError::JobAssignment
        | ProcessError::ProcessStart
        | ProcessError::ProcessResume
        | ProcessError::OutputRead
        | ProcessError::InputTooLarge
        | ProcessError::InputWrite => "GH_PROCESS_START_FAILED",
    }
    .into()
}
