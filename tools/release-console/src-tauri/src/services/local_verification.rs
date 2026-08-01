use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalExecutable {
    Npm,
    Cargo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalVerificationCommand {
    pub id: String,
    pub executable: LocalExecutable,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalCommandEvidence {
    pub id: String,
    pub exit_code: i32,
    pub duration_millis: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalArtifactEvidence {
    pub relative_path: PathBuf,
    pub size: u64,
    pub modified_unix_millis: u64,
    pub sha256: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LocalArtifactError {
    #[error("普通构建产物缺失")]
    MissingArtifacts,
    #[error("无法读取普通构建产物")]
    ReadFailed,
}

pub struct LocalArtifactService;

impl LocalArtifactService {
    pub fn enumerate(
        repository_path: &Path,
    ) -> Result<Vec<LocalArtifactEvidence>, LocalArtifactError> {
        let release_root = repository_path.join("src-tauri/target/release");
        let mut paths = vec![release_root.join("CodexRelay.exe")];
        let nsis_root = release_root.join("bundle/nsis");
        let mut nsis = fs::read_dir(&nsis_root)
            .map_err(|_| LocalArtifactError::MissingArtifacts)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            })
            .collect::<Vec<_>>();
        nsis.sort();
        if nsis.is_empty() || !paths[0].is_file() {
            return Err(LocalArtifactError::MissingArtifacts);
        }
        paths.extend(nsis);

        paths
            .into_iter()
            .map(|path| {
                let bytes = fs::read(&path).map_err(|_| LocalArtifactError::ReadFailed)?;
                let metadata = fs::metadata(&path).map_err(|_| LocalArtifactError::ReadFailed)?;
                let modified_unix_millis = metadata
                    .modified()
                    .map_err(|_| LocalArtifactError::ReadFailed)?
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| LocalArtifactError::ReadFailed)?
                    .as_millis()
                    .min(u128::from(u64::MAX)) as u64;
                let relative_path = path
                    .strip_prefix(repository_path)
                    .map_err(|_| LocalArtifactError::ReadFailed)?
                    .to_path_buf();
                Ok(LocalArtifactEvidence {
                    relative_path,
                    size: metadata.len(),
                    modified_unix_millis,
                    sha256: Sha256::digest(bytes)
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect(),
                })
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LocalVerificationProcessError {
    #[error("无法创建本地门禁 Job Object")]
    JobUnavailable,
    #[error("无法将本地门禁进程加入 Job Object")]
    JobAssignment,
    #[error("本地门禁进程启动失败")]
    ProcessStart,
    #[error("本地门禁进程恢复失败")]
    ProcessResume,
    #[error("本地门禁输出超过安全上限")]
    OutputTooLarge,
    #[error("本地门禁进程超时")]
    Timeout,
    #[error("本地门禁进程树未能安全终止")]
    ProcessTreeTermination,
    #[error("读取本地门禁输出失败")]
    OutputRead,
    #[error("本地门禁输入超过安全上限")]
    InputTooLarge,
    #[error("写入本地门禁输入失败")]
    InputWrite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalVerificationFailure {
    ExitCode(i32),
    Process(LocalVerificationProcessError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LocalVerificationBackendError {
    #[error("本地命令进程失败：{0}")]
    Process(LocalVerificationProcessError),
    #[error("本地命令已取消")]
    Cancelled,
}

pub trait LocalVerificationBackend: Send + Sync {
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
    >;
}

#[derive(Debug, thiserror::Error)]
pub enum LocalVerificationError {
    #[error("本地发布门禁失败：{command_id}")]
    CommandFailed {
        command_id: String,
        failure: LocalVerificationFailure,
    },
    #[error("本地发布门禁已取消")]
    Cancelled,
}

pub struct LocalVerificationService {
    commands: Vec<LocalVerificationCommand>,
}

impl Default for LocalVerificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalVerificationService {
    pub fn new() -> Self {
        Self {
            commands: vec![
                LocalVerificationCommand {
                    id: "release-structure-tests".into(),
                    executable: LocalExecutable::Npm,
                    args: [
                        "exec",
                        "--",
                        "vitest",
                        "run",
                        "src/release-request.test.ts",
                        "src/release-config.test.ts",
                        "src/release-retention.test.ts",
                        "src/release-console-structure.test.ts",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                },
                LocalVerificationCommand {
                    id: "release-console-rust-tests".into(),
                    executable: LocalExecutable::Cargo,
                    args: [
                        "test",
                        "--manifest-path",
                        "src-tauri/Cargo.toml",
                        "-p",
                        "codex-relay-release-console",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                },
                LocalVerificationCommand {
                    id: "full-project-check".into(),
                    executable: LocalExecutable::Npm,
                    args: vec!["run".into(), "check".into()],
                },
                LocalVerificationCommand {
                    id: "ordinary-build".into(),
                    executable: LocalExecutable::Npm,
                    args: vec!["run".into(), "build".into()],
                },
            ],
        }
    }

    pub async fn run(
        &self,
        backend: &dyn LocalVerificationBackend,
        repository_path: &Path,
    ) -> Result<Vec<LocalCommandEvidence>, LocalVerificationError> {
        let mut evidence = Vec::with_capacity(self.commands.len());
        for command in &self.commands {
            let item = match backend.run(repository_path, command).await {
                Ok(item) => item,
                Err(LocalVerificationBackendError::Cancelled) => {
                    return Err(LocalVerificationError::Cancelled);
                }
                Err(LocalVerificationBackendError::Process(error)) => {
                    return Err(LocalVerificationError::CommandFailed {
                        command_id: command.id.clone(),
                        failure: LocalVerificationFailure::Process(error),
                    });
                }
            };
            if item.exit_code != 0 {
                return Err(LocalVerificationError::CommandFailed {
                    command_id: command.id.clone(),
                    failure: LocalVerificationFailure::ExitCode(item.exit_code),
                });
            }
            evidence.push(item);
        }
        Ok(evidence)
    }
}
