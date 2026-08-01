use crate::infrastructure::gh::{GhBackend, GhOperation, GhRequest, SystemGhBackend};
use crate::infrastructure::git::{GitBackend, GitBackendError, GitProxyMode};
use crate::infrastructure::process::{ProcessError, filter_release_environment};
use crate::models::{
    ConnectionProbeResult, ReleaseConnectionTestResult, ReleaseProxySettings, ReleaseProxyType,
};
use std::ffi::OsString;
use std::future::Future;
use std::net::Ipv6Addr;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Instant;
use url::Host;

const PROXY_ENVIRONMENT_NAMES: &[&str] = &["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"];

#[derive(Clone, Debug)]
pub struct ReleaseNetworkProfile {
    environment: Vec<(OsString, OsString)>,
    git_proxy_mode: GitProxyMode,
}

impl ReleaseNetworkProfile {
    pub fn new<I>(
        settings: &ReleaseProxySettings,
        inherited: I,
    ) -> Result<Self, ReleaseNetworkError>
    where
        I: IntoIterator<Item = (OsString, OsString)>,
    {
        let mut environment = filter_release_environment(inherited);
        environment.push((OsString::from("GIT_TERMINAL_PROMPT"), OsString::from("0")));
        environment.push((OsString::from("GCM_INTERACTIVE"), OsString::from("Never")));
        environment.retain(|(name, _)| {
            !PROXY_ENVIRONMENT_NAMES
                .iter()
                .any(|candidate| name.to_string_lossy().eq_ignore_ascii_case(candidate))
        });

        if !settings.enabled {
            return Ok(Self {
                environment,
                git_proxy_mode: GitProxyMode::Direct,
            });
        }

        let host = render_proxy_host(settings.host.trim())?;
        let port = settings
            .port
            .filter(|port| *port > 0)
            .ok_or(ReleaseNetworkError::InvalidProxyPort)?;
        let scheme = match settings.proxy_type {
            ReleaseProxyType::Http => "http",
            ReleaseProxyType::Socks5 => "socks5",
        };
        let proxy_url = format!("{scheme}://{host}:{port}");
        environment.push((OsString::from("HTTP_PROXY"), OsString::from(&proxy_url)));
        environment.push((OsString::from("HTTPS_PROXY"), OsString::from(&proxy_url)));
        Ok(Self {
            environment,
            git_proxy_mode: GitProxyMode::Custom(proxy_url),
        })
    }

    pub fn environment(&self) -> &[(OsString, OsString)] {
        &self.environment
    }

    pub fn git_proxy_mode(&self) -> &GitProxyMode {
        &self.git_proxy_mode
    }
}

fn render_proxy_host(input: &str) -> Result<String, ReleaseNetworkError> {
    if let Ok(address) = input.parse::<Ipv6Addr>() {
        return Ok(format!("[{address}]"));
    }
    Host::parse(input)
        .map(|host| host.to_string())
        .map_err(|_| ReleaseNetworkError::InvalidProxyHost)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReleaseNetworkError {
    #[error("代理地址无效")]
    InvalidProxyHost,
    #[error("代理端口无效")]
    InvalidProxyPort,
}

impl ReleaseNetworkError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidProxyHost => "RELEASE_PROXY_HOST_INVALID",
            Self::InvalidProxyPort => "RELEASE_PROXY_PORT_INVALID",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionProbeTarget {
    Git,
    Github,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionProbeFailure {
    ToolMissing,
    ProcessStart,
    Timeout,
    Cancelled,
    ProcessTreeTermination,
    CommandFailed,
}

pub trait ReleaseConnectionProbeBackend: Send + Sync {
    fn probe<'a>(
        &'a self,
        target: ConnectionProbeTarget,
    ) -> Pin<Box<dyn Future<Output = Result<(), ConnectionProbeFailure>> + Send + 'a>>;
}

pub struct SystemReleaseConnectionProbeBackend {
    git: Option<GitBackend>,
    github: Option<SystemGhBackend>,
    workdir: PathBuf,
}

impl SystemReleaseConnectionProbeBackend {
    pub fn new(
        git_executable: Option<PathBuf>,
        gh_executable: Option<PathBuf>,
        profile: &ReleaseNetworkProfile,
        workdir: PathBuf,
    ) -> Self {
        let git = git_executable.map(|executable| {
            GitBackend::new_with_proxy(
                executable,
                profile.environment().to_vec(),
                profile.git_proxy_mode().clone(),
            )
        });
        let github = gh_executable.map(|executable| {
            let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
            SystemGhBackend::new(
                executable,
                profile.environment().to_vec(),
                workdir.clone(),
                cancel,
            )
        });
        Self {
            git,
            github,
            workdir,
        }
    }
}

impl ReleaseConnectionProbeBackend for SystemReleaseConnectionProbeBackend {
    fn probe<'a>(
        &'a self,
        target: ConnectionProbeTarget,
    ) -> Pin<Box<dyn Future<Output = Result<(), ConnectionProbeFailure>> + Send + 'a>> {
        Box::pin(async move {
            match target {
                ConnectionProbeTarget::Git => {
                    let git = self
                        .git
                        .as_ref()
                        .ok_or(ConnectionProbeFailure::ToolMissing)?;
                    git.run(
                        &self.workdir,
                        &[
                            "ls-remote",
                            "--exit-code",
                            "https://github.com/hunxuankai/codex-relay.git",
                            "refs/heads/main",
                        ],
                    )
                    .await
                    .map(|_| ())
                    .map_err(connection_git_failure)
                }
                ConnectionProbeTarget::Github => {
                    let github = self
                        .github
                        .as_ref()
                        .ok_or(ConnectionProbeFailure::ToolMissing)?;
                    github
                        .execute(GhRequest {
                            operation: GhOperation::ConnectionTest,
                            repository: "hunxuankai/codex-relay".into(),
                            workflow: None,
                            git_ref: None,
                            tag_name: None,
                            head_sha: None,
                            created_after: None,
                            resource_id: None,
                            stdin: None,
                        })
                        .await
                        .map(|_| ())
                        .map_err(|error| connection_gh_failure(&error))
                }
            }
        })
    }
}

fn connection_git_failure(error: GitBackendError) -> ConnectionProbeFailure {
    match error {
        GitBackendError::Process(ProcessError::Timeout) => ConnectionProbeFailure::Timeout,
        GitBackendError::Process(ProcessError::Cancelled) => ConnectionProbeFailure::Cancelled,
        GitBackendError::Process(ProcessError::ProcessTreeTermination) => {
            ConnectionProbeFailure::ProcessTreeTermination
        }
        GitBackendError::Process(_) => ConnectionProbeFailure::ProcessStart,
        GitBackendError::CommandFailed | GitBackendError::InvalidUtf8 => {
            ConnectionProbeFailure::CommandFailed
        }
    }
}

fn connection_gh_failure(error: &str) -> ConnectionProbeFailure {
    match error {
        "GH_PROCESS_TIMEOUT" => ConnectionProbeFailure::Timeout,
        "GH_PROCESS_CANCELLED" => ConnectionProbeFailure::Cancelled,
        "GH_PROCESS_TREE_TERMINATION_FAILED" => ConnectionProbeFailure::ProcessTreeTermination,
        "GH_PROCESS_START_FAILED" => ConnectionProbeFailure::ProcessStart,
        _ => ConnectionProbeFailure::CommandFailed,
    }
}

pub struct ReleaseConnectionService;

impl ReleaseConnectionService {
    pub fn new() -> Self {
        Self
    }

    pub async fn test(
        &self,
        backend: &dyn ReleaseConnectionProbeBackend,
    ) -> ReleaseConnectionTestResult {
        let git = timed_probe(backend, ConnectionProbeTarget::Git);
        let github = timed_probe(backend, ConnectionProbeTarget::Github);
        let (git, github) = tokio::join!(git, github);
        ReleaseConnectionTestResult { git, github }
    }
}

impl Default for ReleaseConnectionService {
    fn default() -> Self {
        Self::new()
    }
}

async fn timed_probe(
    backend: &dyn ReleaseConnectionProbeBackend,
    target: ConnectionProbeTarget,
) -> ConnectionProbeResult {
    let started = Instant::now();
    let result = backend.probe(target).await;
    let duration_millis = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match result {
        Ok(()) => ConnectionProbeResult {
            success: true,
            code: None,
            message: match target {
                ConnectionProbeTarget::Git => "Git 远端连接正常。",
                ConnectionProbeTarget::Github => "GitHub API 连接正常。",
            }
            .into(),
            duration_millis,
        },
        Err(failure) => failed_probe(target, failure, duration_millis),
    }
}

fn failed_probe(
    target: ConnectionProbeTarget,
    failure: ConnectionProbeFailure,
    duration_millis: u64,
) -> ConnectionProbeResult {
    let (code, message) = match (target, failure) {
        (ConnectionProbeTarget::Git, ConnectionProbeFailure::ToolMissing) => {
            ("GIT_TOOL_MISSING", "未找到 Git。")
        }
        (ConnectionProbeTarget::Github, ConnectionProbeFailure::ToolMissing) => {
            ("GITHUB_TOOL_MISSING", "未找到 GitHub CLI。")
        }
        (ConnectionProbeTarget::Git, ConnectionProbeFailure::ProcessStart) => {
            ("GIT_PROCESS_START_FAILED", "Git 进程启动失败。")
        }
        (ConnectionProbeTarget::Github, ConnectionProbeFailure::ProcessStart) => {
            ("GITHUB_PROCESS_START_FAILED", "GitHub CLI 进程启动失败。")
        }
        (ConnectionProbeTarget::Git, ConnectionProbeFailure::Timeout) => {
            ("GIT_PROCESS_TIMEOUT", "Git 远端连接超时。")
        }
        (ConnectionProbeTarget::Github, ConnectionProbeFailure::Timeout) => {
            ("GITHUB_PROCESS_TIMEOUT", "GitHub API 连接超时。")
        }
        (ConnectionProbeTarget::Git, ConnectionProbeFailure::Cancelled) => {
            ("GIT_PROCESS_CANCELLED", "Git 远端连接已取消。")
        }
        (ConnectionProbeTarget::Github, ConnectionProbeFailure::Cancelled) => {
            ("GITHUB_PROCESS_CANCELLED", "GitHub API 连接已取消。")
        }
        (ConnectionProbeTarget::Git, ConnectionProbeFailure::ProcessTreeTermination) => (
            "GIT_PROCESS_TREE_TERMINATION_FAILED",
            "Git 进程树未能安全结束。",
        ),
        (ConnectionProbeTarget::Github, ConnectionProbeFailure::ProcessTreeTermination) => (
            "GITHUB_PROCESS_TREE_TERMINATION_FAILED",
            "GitHub CLI 进程树未能安全结束。",
        ),
        (ConnectionProbeTarget::Git, ConnectionProbeFailure::CommandFailed) => {
            ("GIT_COMMAND_FAILED", "Git 远端连接失败。")
        }
        (ConnectionProbeTarget::Github, ConnectionProbeFailure::CommandFailed) => {
            ("GITHUB_COMMAND_FAILED", "GitHub API 连接失败。")
        }
    };
    ConnectionProbeResult {
        success: false,
        code: Some(code.into()),
        message: message.into(),
        duration_millis,
    }
}
