use super::process::{ProcessError, ProcessInvocation, SafeProcessRunner};
use crate::services::local_verification::{
    LocalCommandEvidence, LocalExecutable, LocalVerificationBackend, LocalVerificationBackendError,
    LocalVerificationCommand, LocalVerificationProcessError,
};
use std::ffi::OsString;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{Duration, Instant};

const LOCAL_COMMAND_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

pub struct ProcessLocalVerificationBackend {
    npm_executable: PathBuf,
    cargo_executable: PathBuf,
    environment: Vec<(OsString, OsString)>,
    cancel: tokio::sync::watch::Receiver<bool>,
    runner: SafeProcessRunner,
}

impl ProcessLocalVerificationBackend {
    pub fn new(
        npm_executable: PathBuf,
        cargo_executable: PathBuf,
        environment: Vec<(OsString, OsString)>,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Self {
        Self {
            npm_executable,
            cargo_executable,
            environment,
            cancel,
            runner: SafeProcessRunner::default(),
        }
    }

    pub fn invocation_for(
        &self,
        repository_path: &Path,
        command: &LocalVerificationCommand,
    ) -> ProcessInvocation {
        let executable = match command.executable {
            LocalExecutable::Npm => self.npm_executable.clone(),
            LocalExecutable::Cargo => self.cargo_executable.clone(),
        };
        ProcessInvocation {
            executable,
            args: command.args.iter().map(OsString::from).collect(),
            env: self.environment.clone(),
            workdir: repository_path.to_path_buf(),
            stdin: None,
            stdout_file: None,
        }
    }
}

impl LocalVerificationBackend for ProcessLocalVerificationBackend {
    fn run<'a>(
        &'a self,
        repository_path: &'a Path,
        command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let started = Instant::now();
            let output = self
                .runner
                .run(
                    self.invocation_for(repository_path, command),
                    LOCAL_COMMAND_TIMEOUT,
                    self.cancel.clone(),
                    None,
                )
                .await
                .map_err(|error| match error {
                    ProcessError::Cancelled => LocalVerificationBackendError::Cancelled,
                    ProcessError::JobUnavailable => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::JobUnavailable,
                    ),
                    ProcessError::JobAssignment => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::JobAssignment,
                    ),
                    ProcessError::ProcessStart => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::ProcessStart,
                    ),
                    ProcessError::ProcessResume => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::ProcessResume,
                    ),
                    ProcessError::OutputTooLarge => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::OutputTooLarge,
                    ),
                    ProcessError::Timeout => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::Timeout,
                    ),
                    ProcessError::ProcessTreeTermination => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::ProcessTreeTermination,
                    ),
                    ProcessError::OutputRead => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::OutputRead,
                    ),
                    ProcessError::InputTooLarge => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::InputTooLarge,
                    ),
                    ProcessError::InputWrite => LocalVerificationBackendError::Process(
                        LocalVerificationProcessError::InputWrite,
                    ),
                })?;
            Ok(LocalCommandEvidence {
                id: command.id.clone(),
                exit_code: output.exit_code.unwrap_or(-1),
                duration_millis: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            })
        })
    }
}
