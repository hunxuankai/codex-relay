use super::process::{ProcessError, ProcessInvocation, SafeProcessRunner};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GitProxyMode {
    Direct,
    Custom(String),
}

#[derive(Clone)]
pub struct GitBackend {
    executable: PathBuf,
    environment: Vec<(OsString, OsString)>,
    proxy_mode: GitProxyMode,
    cancel: tokio::sync::watch::Receiver<bool>,
    runner: SafeProcessRunner,
}

impl GitBackend {
    pub fn new(executable: PathBuf, environment: Vec<(OsString, OsString)>) -> Self {
        Self::new_with_proxy(executable, environment, GitProxyMode::Direct)
    }

    pub fn new_with_proxy(
        executable: PathBuf,
        environment: Vec<(OsString, OsString)>,
        proxy_mode: GitProxyMode,
    ) -> Self {
        let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
        Self::new_cancellable_with_proxy(executable, environment, proxy_mode, cancel)
    }

    pub fn new_cancellable(
        executable: PathBuf,
        environment: Vec<(OsString, OsString)>,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self::new_cancellable_with_proxy(executable, environment, GitProxyMode::Direct, cancel)
    }

    pub fn new_cancellable_with_proxy(
        executable: PathBuf,
        environment: Vec<(OsString, OsString)>,
        proxy_mode: GitProxyMode,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            executable,
            environment,
            proxy_mode,
            cancel,
            runner: SafeProcessRunner::default(),
        }
    }

    pub fn without_cancellation(&self) -> Self {
        Self::new_with_proxy(
            self.executable.clone(),
            self.environment.clone(),
            self.proxy_mode.clone(),
        )
    }

    pub fn invocation_for(&self, workdir: &Path, args: &[&str]) -> ProcessInvocation {
        let proxy_url = match &self.proxy_mode {
            GitProxyMode::Direct => "",
            GitProxyMode::Custom(url) => url,
        };
        let mut invocation_args = vec![
            OsString::from("-c"),
            OsString::from(format!("http.proxy={proxy_url}")),
            OsString::from("-c"),
            OsString::from(format!("https.proxy={proxy_url}")),
        ];
        invocation_args.extend(args.iter().map(OsString::from));
        ProcessInvocation {
            executable: self.executable.clone(),
            args: invocation_args,
            env: self.environment.clone(),
            workdir: workdir.to_path_buf(),
            stdin: None,
            stdout_file: None,
        }
    }

    pub async fn run(
        &self,
        workdir: &Path,
        args: &[&str],
    ) -> Result<GitCommandOutput, GitBackendError> {
        let output = self
            .runner
            .run(
                self.invocation_for(workdir, args),
                GIT_COMMAND_TIMEOUT,
                self.cancel.clone(),
                None,
            )
            .await
            .map_err(GitBackendError::Process)?;
        if output.exit_code != Some(0) {
            return Err(GitBackendError::CommandFailed);
        }
        let stdout = String::from_utf8(output.stdout).map_err(|_| GitBackendError::InvalidUtf8)?;
        let stderr = String::from_utf8(output.stderr).map_err(|_| GitBackendError::InvalidUtf8)?;
        Ok(GitCommandOutput { stdout, stderr })
    }
}

pub struct GitCommandOutput {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum GitBackendError {
    #[error("Git 子进程失败")]
    Process(ProcessError),
    #[error("Git 命令返回失败状态")]
    CommandFailed,
    #[error("Git 输出不是有效 UTF-8")]
    InvalidUtf8,
}

impl GitBackendError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Process(ProcessError::Timeout) => "GIT_PROCESS_TIMEOUT",
            Self::Process(ProcessError::Cancelled) => "GIT_PROCESS_CANCELLED",
            Self::Process(ProcessError::ProcessTreeTermination) => {
                "GIT_PROCESS_TREE_TERMINATION_FAILED"
            }
            Self::Process(ProcessError::OutputTooLarge) => "GIT_PROCESS_OUTPUT_TOO_LARGE",
            Self::Process(_) => "GIT_PROCESS_START_FAILED",
            Self::CommandFailed | Self::InvalidUtf8 => "GIT_COMMAND_FAILED",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::process::filter_release_environment;

    #[test]
    fn backend_errors_expose_distinct_stable_process_codes() {
        assert_eq!(
            GitBackendError::Process(ProcessError::ProcessStart).code(),
            "GIT_PROCESS_START_FAILED"
        );
        assert_eq!(
            GitBackendError::Process(ProcessError::Timeout).code(),
            "GIT_PROCESS_TIMEOUT"
        );
        assert_eq!(
            GitBackendError::Process(ProcessError::Cancelled).code(),
            "GIT_PROCESS_CANCELLED"
        );
        assert_eq!(
            GitBackendError::Process(ProcessError::ProcessTreeTermination).code(),
            "GIT_PROCESS_TREE_TERMINATION_FAILED"
        );
        assert_eq!(GitBackendError::CommandFailed.code(), "GIT_COMMAND_FAILED");
    }

    #[tokio::test]
    async fn cancellable_backend_terminates_the_active_process_tree() {
        let directory = tempfile::tempdir().unwrap();
        let (cancel_sender, cancel) = tokio::sync::watch::channel(false);
        let backend = GitBackend::new_cancellable(
            PathBuf::from("powershell.exe"),
            filter_release_environment(std::env::vars_os()),
            cancel,
        );
        let run = tokio::spawn(async move {
            backend
                .run(
                    directory.path(),
                    &[
                        "-NoLogo",
                        "-NoProfile",
                        "-NonInteractive",
                        "-Command",
                        "Start-Sleep -Seconds 30",
                    ],
                )
                .await
        });

        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel_sender.send(true).unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), run)
            .await
            .expect("cancelled process should stop within the test budget")
            .unwrap();
        let error = match result {
            Err(error) => error,
            Ok(_) => panic!("cancelled Git backend unexpectedly completed successfully"),
        };

        assert!(matches!(
            error,
            GitBackendError::Process(ProcessError::Cancelled)
        ));
    }
}
