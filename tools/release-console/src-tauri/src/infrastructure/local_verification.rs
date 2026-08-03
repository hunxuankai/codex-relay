use super::process::{ProcessError, ProcessEventSink, ProcessInvocation, SafeProcessRunner};
use super::release_log::ReleaseProcessLogSink;
use crate::services::local_verification::{
    LocalCommandEvidence, LocalExecutable, LocalVerificationBackend, LocalVerificationBackendError,
    LocalVerificationCommand, LocalVerificationProcessError,
};
use crate::services::release_log::ReleaseLogRecorder;
use std::ffi::OsString;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

const LOCAL_COMMAND_TIMEOUT: Duration = Duration::from_secs(2 * 60 * 60);

pub struct ProcessLocalVerificationBackend {
    npm_executable: PathBuf,
    cargo_executable: PathBuf,
    environment: Vec<(OsString, OsString)>,
    cancel: tokio::sync::watch::Receiver<bool>,
    runner: SafeProcessRunner,
    recorder: Option<Arc<ReleaseLogRecorder>>,
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
            recorder: None,
        }
    }

    pub fn with_recorder(
        npm_executable: PathBuf,
        cargo_executable: PathBuf,
        environment: Vec<(OsString, OsString)>,
        cancel: tokio::sync::watch::Receiver<bool>,
        recorder: Arc<ReleaseLogRecorder>,
    ) -> Self {
        let mut backend = Self::new(npm_executable, cargo_executable, environment, cancel);
        backend.recorder = Some(recorder);
        backend
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
            let invocation = self.invocation_for(repository_path, command);
            let mut diagnostic_sensitive_values = sensitive_environment_values(&self.environment);
            let executable = invocation.executable.to_string_lossy();
            if !executable.is_empty() {
                diagnostic_sensitive_values.push(executable.into_owned());
            }
            diagnostic_sensitive_values.sort();
            diagnostic_sensitive_values.dedup();
            let process_log = self.recorder.as_ref().map(|recorder| {
                Arc::new(ReleaseProcessLogSink::new(
                    command.id.clone(),
                    repository_path.to_path_buf(),
                    diagnostic_sensitive_values,
                    Arc::clone(recorder),
                ))
            });
            let event_sink = process_log
                .as_ref()
                .map(|sink| Arc::clone(sink) as Arc<dyn ProcessEventSink>);
            let output = self
                .runner
                .run(
                    invocation,
                    LOCAL_COMMAND_TIMEOUT,
                    self.cancel.clone(),
                    event_sink,
                )
                .await;
            if let Some(process_log) = process_log {
                process_log.finish();
            }
            let output = output.map_err(|error| match error {
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
                ProcessError::Timeout => {
                    LocalVerificationBackendError::Process(LocalVerificationProcessError::Timeout)
                }
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

fn sensitive_environment_values(environment: &[(OsString, OsString)]) -> Vec<String> {
    let mut values = environment
        .iter()
        .filter_map(|(name, value)| {
            let name = name.to_string_lossy().to_ascii_uppercase();
            let is_sensitive = ["PROXY", "TOKEN", "KEY", "SECRET", "PASSWORD", "AUTH"]
                .iter()
                .any(|marker| name.contains(marker));
            let value = value.to_string_lossy();
            (is_sensitive && !value.is_empty()).then(|| value.into_owned())
        })
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}
