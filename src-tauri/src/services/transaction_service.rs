use crate::error::AppError;
use crate::infrastructure::atomic_file::atomic_write;
use crate::infrastructure::file_fingerprint::FileSetFingerprint;
use crate::infrastructure::path_service::AppPaths;
use crate::models::backup::BackupSummary;
use crate::models::transaction::{ConfigTransaction, TransactionOperation};
use crate::services::backup_service::{BackupService, FileSnapshot};
use crate::services::provider_secret_service::ProviderSecretStore;
use chrono::Utc;
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;
use toml_edit::DocumentMut;
use uuid::Uuid;

const MAX_BACKUPS: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedFileKind {
    Config,
    Auth,
    Providers,
    TransactionMarker,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WritePhase {
    Forward,
    Rollback,
}

pub trait FileOps: Send + Sync {
    fn read_optional(&self, path: &Path) -> Result<Option<Vec<u8>>, AppError>;

    fn write(
        &self,
        path: &Path,
        bytes: &[u8],
        kind: ManagedFileKind,
        phase: WritePhase,
    ) -> Result<(), AppError>;

    fn remove_if_exists(
        &self,
        path: &Path,
        kind: ManagedFileKind,
        phase: WritePhase,
    ) -> Result<(), AppError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StdFileOps;

impl FileOps for StdFileOps {
    fn read_optional(&self, path: &Path) -> Result<Option<Vec<u8>>, AppError> {
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(AppError::from(error)),
        }
    }

    fn write(
        &self,
        path: &Path,
        bytes: &[u8],
        kind: ManagedFileKind,
        phase: WritePhase,
    ) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(AppError::from)?;
        }
        atomic_write(path, bytes, |candidate| match phase {
            WritePhase::Forward => validate_managed_file(kind, candidate),
            WritePhase::Rollback => Ok(()),
        })
    }

    fn remove_if_exists(
        &self,
        path: &Path,
        _kind: ManagedFileKind,
        _phase: WritePhase,
    ) -> Result<(), AppError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AppError::from(error)),
        }
    }
}

#[derive(Clone, Default, Eq, PartialEq)]
pub enum FileChange {
    #[default]
    Unchanged,
    Write(Vec<u8>),
    Delete,
}

impl fmt::Debug for FileChange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unchanged => formatter.write_str("Unchanged"),
            Self::Write(bytes) => formatter
                .debug_struct("Write")
                .field("byte_count", &bytes.len())
                .finish(),
            Self::Delete => formatter.write_str("Delete"),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileChanges {
    pub config: FileChange,
    pub auth: FileChange,
    pub providers: FileChange,
}

#[derive(Clone)]
pub struct TransactionRequest {
    pub operation: TransactionOperation,
    pub provider_id: Option<String>,
    pub expected_files: Option<FileSetFingerprint>,
    pub changes: FileChanges,
}

impl fmt::Debug for TransactionRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TransactionRequest")
            .field("operation", &self.operation)
            .field("provider_id", &self.provider_id)
            .field("expected_files", &self.expected_files)
            .field("changes", &self.changes)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct TransactionOutcome {
    pub transaction: ConfigTransaction,
    pub backup: BackupSummary,
    pub fingerprints: FileSetFingerprint,
}

#[derive(Clone)]
pub struct TransactionService {
    paths: AppPaths,
    backup_service: BackupService,
    file_ops: Arc<dyn FileOps>,
    write_lock: Arc<AsyncMutex<()>>,
}

impl TransactionService {
    pub fn new(paths: AppPaths, backup_service: BackupService) -> Self {
        Self::with_file_ops(paths, backup_service, Arc::new(StdFileOps))
    }

    pub(crate) fn with_file_ops(
        paths: AppPaths,
        backup_service: BackupService,
        file_ops: Arc<dyn FileOps>,
    ) -> Self {
        Self {
            paths,
            backup_service,
            file_ops,
            write_lock: Arc::new(AsyncMutex::new(())),
        }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub async fn execute<F>(
        &self,
        request: TransactionRequest,
        post_write_validator: F,
    ) -> Result<TransactionOutcome, AppError>
    where
        F: Fn(&AppPaths) -> Result<(), AppError> + Send + Sync,
    {
        let _guard = self.write_lock.lock().await;
        let initial_fingerprints = self.current_fingerprints()?;
        if request
            .expected_files
            .as_ref()
            .is_some_and(|expected| expected != &initial_fingerprints)
        {
            return Err(external_modification_conflict());
        }

        let snapshot = self.capture_snapshot()?;
        let transaction = ConfigTransaction {
            id: Uuid::new_v4().to_string(),
            operation: request.operation,
            provider_id: request.provider_id.clone(),
            started_at: Utc::now().to_rfc3339(),
        };
        let backup = self.backup_service.create_backup(&transaction, &snapshot)?;

        if self.current_fingerprints()? != initial_fingerprints {
            return Err(external_modification_conflict());
        }

        self.write_transaction_marker(&transaction)?;
        let forward_result = self
            .apply_changes(&request.changes)
            .and_then(|()| post_write_validator(&self.paths))
            .and_then(|()| {
                self.backup_service
                    .cleanup_old_backups(MAX_BACKUPS, Some(&transaction.id))
            });

        if let Err(operation_error) = forward_result {
            return Err(self.rollback_after_failure(&request.changes, &snapshot, &operation_error));
        }

        let fingerprints = match self.current_fingerprints() {
            Ok(fingerprints) => fingerprints,
            Err(operation_error) => {
                return Err(self.rollback_after_failure(
                    &request.changes,
                    &snapshot,
                    &operation_error,
                ));
            }
        };

        if let Err(operation_error) = self.file_ops.remove_if_exists(
            &self.transaction_marker_path(),
            ManagedFileKind::TransactionMarker,
            WritePhase::Forward,
        ) {
            return Err(self.rollback_after_failure(&request.changes, &snapshot, &operation_error));
        }

        Ok(TransactionOutcome {
            transaction,
            backup,
            fingerprints,
        })
    }

    pub async fn restore_backup(
        &self,
        directory_name: &str,
    ) -> Result<TransactionOutcome, AppError> {
        let selected_snapshot = self.backup_service.load_snapshot(directory_name)?;
        let expected_files = self.current_fingerprints()?;
        let changes = FileChanges {
            config: change_from_snapshot(selected_snapshot.config.as_deref()),
            auth: change_from_snapshot(selected_snapshot.auth.as_deref()),
            providers: change_from_snapshot(selected_snapshot.providers.as_deref()),
        };

        self.execute(
            TransactionRequest {
                operation: TransactionOperation::RestoreBackup,
                provider_id: None,
                expected_files: Some(expected_files),
                changes,
            },
            |paths| validate_snapshot_matches(paths, &selected_snapshot),
        )
        .await
    }

    fn capture_snapshot(&self) -> Result<FileSnapshot, AppError> {
        Ok(FileSnapshot {
            config: self.file_ops.read_optional(&self.paths.config_file)?,
            auth: self.file_ops.read_optional(&self.paths.auth_file)?,
            providers: self.file_ops.read_optional(&self.paths.providers_file)?,
        })
    }

    fn current_fingerprints(&self) -> Result<FileSetFingerprint, AppError> {
        FileSetFingerprint::from_paths(
            &self.paths.config_file,
            &self.paths.auth_file,
            &self.paths.providers_file,
        )
    }

    fn apply_changes(&self, changes: &FileChanges) -> Result<(), AppError> {
        self.apply_change(
            &self.paths.config_file,
            ManagedFileKind::Config,
            &changes.config,
            WritePhase::Forward,
        )?;
        self.apply_change(
            &self.paths.auth_file,
            ManagedFileKind::Auth,
            &changes.auth,
            WritePhase::Forward,
        )?;
        self.apply_change(
            &self.paths.providers_file,
            ManagedFileKind::Providers,
            &changes.providers,
            WritePhase::Forward,
        )
    }

    fn apply_change(
        &self,
        path: &Path,
        kind: ManagedFileKind,
        change: &FileChange,
        phase: WritePhase,
    ) -> Result<(), AppError> {
        match change {
            FileChange::Unchanged => Ok(()),
            FileChange::Write(bytes) => self.file_ops.write(path, bytes, kind, phase),
            FileChange::Delete => self.file_ops.remove_if_exists(path, kind, phase),
        }
    }

    fn rollback_after_failure(
        &self,
        changes: &FileChanges,
        snapshot: &FileSnapshot,
        operation_error: &AppError,
    ) -> AppError {
        let rollback_result = self.rollback_changed_files(changes, snapshot);
        match rollback_result {
            Ok(()) => {
                if let Err(marker_error) = self.file_ops.remove_if_exists(
                    &self.transaction_marker_path(),
                    ManagedFileKind::TransactionMarker,
                    WritePhase::Rollback,
                ) {
                    return rollback_incomplete(operation_error, &marker_error);
                }
                AppError::new(
                    "TRANSACTION_FAILED_ROLLED_BACK",
                    "配置修改失败。原配置已恢复，未对 Codex 配置造成更改。",
                    format!("operation failed with code {}", operation_error.code()),
                )
            }
            Err(rollback_error) => rollback_incomplete(operation_error, &rollback_error),
        }
    }

    fn rollback_changed_files(
        &self,
        changes: &FileChanges,
        snapshot: &FileSnapshot,
    ) -> Result<(), AppError> {
        let mut first_error = None;
        for (path, kind, change, original) in [
            (
                &self.paths.providers_file,
                ManagedFileKind::Providers,
                &changes.providers,
                snapshot.providers.as_deref(),
            ),
            (
                &self.paths.auth_file,
                ManagedFileKind::Auth,
                &changes.auth,
                snapshot.auth.as_deref(),
            ),
            (
                &self.paths.config_file,
                ManagedFileKind::Config,
                &changes.config,
                snapshot.config.as_deref(),
            ),
        ] {
            if matches!(change, FileChange::Unchanged) {
                continue;
            }
            let result = match original {
                Some(bytes) => self.file_ops.write(path, bytes, kind, WritePhase::Rollback),
                None => self
                    .file_ops
                    .remove_if_exists(path, kind, WritePhase::Rollback),
            };
            if first_error.is_none() {
                first_error = result.err();
            }
        }

        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn write_transaction_marker(&self, transaction: &ConfigTransaction) -> Result<(), AppError> {
        let mut json = serde_json::to_string_pretty(transaction).map_err(AppError::from)?;
        json.push('\n');
        self.file_ops.write(
            &self.transaction_marker_path(),
            json.as_bytes(),
            ManagedFileKind::TransactionMarker,
            WritePhase::Forward,
        )
    }

    fn transaction_marker_path(&self) -> std::path::PathBuf {
        self.paths.app_data_dir.join("transaction.json")
    }
}

fn validate_managed_file(kind: ManagedFileKind, bytes: &[u8]) -> Result<(), AppError> {
    match kind {
        ManagedFileKind::Config => {
            let source = std::str::from_utf8(bytes).map_err(|error| {
                AppError::new(
                    "INVALID_TEMP_CONFIG",
                    "临时 config.toml 不是有效的 UTF-8。",
                    error.to_string(),
                )
            })?;
            source.parse::<DocumentMut>().map(|_| ()).map_err(|error| {
                AppError::new(
                    "INVALID_TEMP_CONFIG",
                    "临时 config.toml 验证失败。",
                    error.to_string(),
                )
            })
        }
        ManagedFileKind::Auth => {
            let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
                AppError::new(
                    "INVALID_TEMP_AUTH",
                    "临时 auth.json 验证失败。",
                    error.to_string(),
                )
            })?;
            value
                .get("OPENAI_API_KEY")
                .and_then(serde_json::Value::as_str)
                .filter(|key| !key.is_empty())
                .map(|_| ())
                .ok_or_else(|| {
                    AppError::new(
                        "INVALID_TEMP_AUTH",
                        "临时 auth.json 缺少 OPENAI_API_KEY。",
                        "temporary auth document has no non-empty OPENAI_API_KEY",
                    )
                })
        }
        ManagedFileKind::Providers => {
            let store: ProviderSecretStore = serde_json::from_slice(bytes).map_err(|error| {
                AppError::new(
                    "INVALID_TEMP_PROVIDER_SECRETS",
                    "临时 providers.json 验证失败。",
                    error.to_string(),
                )
            })?;
            if store.version == 1 {
                Ok(())
            } else {
                Err(AppError::new(
                    "INVALID_TEMP_PROVIDER_SECRETS",
                    "临时 providers.json 版本无效。",
                    format!("temporary provider secret version is {}", store.version),
                ))
            }
        }
        ManagedFileKind::TransactionMarker => serde_json::from_slice::<serde_json::Value>(bytes)
            .map(|_| ())
            .map_err(|error| {
                AppError::new(
                    "INVALID_TRANSACTION_MARKER",
                    "事务标记写入失败。",
                    error.to_string(),
                )
            }),
    }
}

fn change_from_snapshot(bytes: Option<&[u8]>) -> FileChange {
    match bytes {
        Some(bytes) => FileChange::Write(bytes.to_vec()),
        None => FileChange::Delete,
    }
}

fn validate_snapshot_matches(paths: &AppPaths, snapshot: &FileSnapshot) -> Result<(), AppError> {
    for (path, expected) in [
        (&paths.config_file, snapshot.config.as_deref()),
        (&paths.auth_file, snapshot.auth.as_deref()),
        (&paths.providers_file, snapshot.providers.as_deref()),
    ] {
        let actual = match fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => return Err(AppError::from(error)),
        };
        if actual.as_deref() != expected {
            return Err(AppError::new(
                "RESTORE_VERIFICATION_FAILED",
                "备份恢复后的文件验证失败。",
                format!(
                    "restored file does not match selected snapshot: {}",
                    path.display()
                ),
            ));
        }
    }
    Ok(())
}

fn external_modification_conflict() -> AppError {
    AppError::new(
        "EXTERNAL_MODIFICATION_CONFLICT",
        "配置文件已被其他程序修改。请重新加载后再保存。",
        "configuration file fingerprint changed before transaction write",
    )
}

fn rollback_incomplete(operation_error: &AppError, rollback_error: &AppError) -> AppError {
    AppError::new(
        "ROLLBACK_INCOMPLETE",
        "配置修改失败，并且自动恢复未完全成功。请立即从备份页面恢复上一份配置。",
        format!(
            "operation error code {}; rollback error code {}",
            operation_error.code(),
            rollback_error.code()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::file_fingerprint::FileSetFingerprint;
    use crate::models::transaction::{ConfigTransaction, TransactionOperation};
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    const CONFIG_A: &[u8] = include_bytes!("../../../fixtures/config-multiple-providers.toml");
    const AUTH_A: &[u8] = include_bytes!("../../../fixtures/auth-api-key.json");
    const PROVIDERS: &[u8] = include_bytes!("../../../fixtures/providers-multiple.json");
    const CONFIG_B: &[u8] = br#"model_provider = "provider-b"

[model_providers.provider-a]
name = "Provider A"
base_url = "https://provider-a.example.com/v1"
wire_api = "responses"

[model_providers.provider-b]
name = "Provider B"
base_url = "https://provider-b.example.com/v1"
wire_api = "responses"
"#;
    const AUTH_B: &[u8] = b"{\n  \"OPENAI_API_KEY\": \"test-key-b-not-real\"\n}\n";

    fn create_paths(directory: &tempfile::TempDir) -> AppPaths {
        let codex = directory.path().join("codex");
        let app_data = directory.path().join("app-data");
        fs::create_dir_all(&codex).unwrap();
        fs::create_dir_all(&app_data).unwrap();
        AppPaths::for_test(codex, app_data).unwrap()
    }

    fn write_initial_files(paths: &AppPaths) {
        fs::write(&paths.config_file, CONFIG_A).unwrap();
        fs::write(&paths.auth_file, AUTH_A).unwrap();
        fs::write(&paths.providers_file, PROVIDERS).unwrap();
    }

    fn request(expected: Option<FileSetFingerprint>) -> TransactionRequest {
        TransactionRequest {
            operation: TransactionOperation::SwitchProvider,
            provider_id: Some("provider-b".into()),
            expected_files: expected,
            changes: FileChanges {
                config: FileChange::Write(CONFIG_B.to_vec()),
                auth: FileChange::Write(AUTH_B.to_vec()),
                providers: FileChange::Unchanged,
            },
        }
    }

    #[tokio::test]
    async fn successful_transaction_writes_valid_files_and_removes_marker() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let expected = FileSetFingerprint::from_paths(
            &paths.config_file,
            &paths.auth_file,
            &paths.providers_file,
        )
        .unwrap();
        let backup = BackupService::new(paths.backups_dir.clone(), "0.1.0");
        let service = TransactionService::new(paths.clone(), backup.clone());

        let outcome = service
            .execute(request(Some(expected)), |paths| {
                let config = fs::read_to_string(&paths.config_file).unwrap();
                let auth = fs::read_to_string(&paths.auth_file).unwrap();
                if !config.contains("model_provider = \"provider-b\"")
                    || !auth.contains("test-key-b-not-real")
                {
                    return Err(AppError::new(
                        "POST_WRITE_VALIDATION_FAILED",
                        "写入后验证失败。",
                        "expected provider-b files",
                    ));
                }
                Ok(())
            })
            .await
            .unwrap();

        assert_eq!(fs::read(&paths.config_file).unwrap(), CONFIG_B);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), AUTH_B);
        assert!(!paths.app_data_dir.join("transaction.json").exists());
        assert_eq!(backup.list_backups().unwrap().len(), 1);
        assert_eq!(
            outcome.transaction.provider_id.as_deref(),
            Some("provider-b")
        );
        assert!(outcome.fingerprints.config.exists);
    }

    #[tokio::test]
    async fn external_modification_conflict_stops_before_backup_or_write() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let expected = FileSetFingerprint::from_paths(
            &paths.config_file,
            &paths.auth_file,
            &paths.providers_file,
        )
        .unwrap();
        fs::write(&paths.config_file, b"model_provider = \"external\"\n").unwrap();
        let backup = BackupService::new(paths.backups_dir.clone(), "0.1.0");
        let service = TransactionService::new(paths.clone(), backup.clone());

        let error = service
            .execute(request(Some(expected)), |_| Ok(()))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "EXTERNAL_MODIFICATION_CONFLICT");
        assert_eq!(
            fs::read(&paths.config_file).unwrap(),
            b"model_provider = \"external\"\n"
        );
        assert!(backup.list_backups().unwrap().is_empty());
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum FailurePoint {
        Forward(ManagedFileKind),
        Rollback(ManagedFileKind),
    }

    struct InjectedFileOps {
        inner: StdFileOps,
        failure: Mutex<Option<FailurePoint>>,
        delay: Duration,
        active_writes: AtomicUsize,
        max_active_writes: AtomicUsize,
    }

    impl InjectedFileOps {
        fn new(failure: Option<FailurePoint>, delay: Duration) -> Self {
            Self {
                inner: StdFileOps,
                failure: Mutex::new(failure),
                delay,
                active_writes: AtomicUsize::new(0),
                max_active_writes: AtomicUsize::new(0),
            }
        }

        fn should_fail(&self, kind: ManagedFileKind, phase: WritePhase) -> bool {
            let expected = match phase {
                WritePhase::Forward => FailurePoint::Forward(kind),
                WritePhase::Rollback => FailurePoint::Rollback(kind),
            };
            let mut failure = self.failure.lock().unwrap();
            if *failure == Some(expected) {
                *failure = None;
                true
            } else {
                false
            }
        }

        fn begin_write(&self) -> ActiveWrite<'_> {
            let active = self.active_writes.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_active_writes.fetch_max(active, Ordering::SeqCst);
            ActiveWrite { owner: self }
        }
    }

    struct ActiveWrite<'a> {
        owner: &'a InjectedFileOps,
    }

    impl Drop for ActiveWrite<'_> {
        fn drop(&mut self) {
            self.owner.active_writes.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl FileOps for InjectedFileOps {
        fn read_optional(&self, path: &std::path::Path) -> Result<Option<Vec<u8>>, AppError> {
            self.inner.read_optional(path)
        }

        fn write(
            &self,
            path: &std::path::Path,
            bytes: &[u8],
            kind: ManagedFileKind,
            phase: WritePhase,
        ) -> Result<(), AppError> {
            let _active = self.begin_write();
            if !self.delay.is_zero() {
                std::thread::sleep(self.delay);
            }
            if self.should_fail(kind, phase) {
                return Err(AppError::new(
                    "INJECTED_WRITE_FAILURE",
                    "注入的写入失败。",
                    format!("injected {phase:?} failure for {kind:?}"),
                ));
            }
            self.inner.write(path, bytes, kind, phase)
        }

        fn remove_if_exists(
            &self,
            path: &std::path::Path,
            kind: ManagedFileKind,
            phase: WritePhase,
        ) -> Result<(), AppError> {
            if self.should_fail(kind, phase) {
                return Err(AppError::new(
                    "INJECTED_REMOVE_FAILURE",
                    "注入的删除失败。",
                    format!("injected {phase:?} remove failure for {kind:?}"),
                ));
            }
            self.inner.remove_if_exists(path, kind, phase)
        }
    }

    #[tokio::test]
    async fn auth_write_failure_restores_original_files() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let file_ops = Arc::new(InjectedFileOps::new(
            Some(FailurePoint::Forward(ManagedFileKind::Auth)),
            Duration::ZERO,
        ));
        let service = TransactionService::with_file_ops(
            paths.clone(),
            BackupService::new(paths.backups_dir.clone(), "0.1.0"),
            file_ops,
        );

        let error = service
            .execute(request(None), |_| Ok(()))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "TRANSACTION_FAILED_ROLLED_BACK");
        assert!(error.public_message().contains("原配置已恢复"));
        assert_eq!(fs::read(&paths.config_file).unwrap(), CONFIG_A);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), AUTH_A);
        assert!(!paths.app_data_dir.join("transaction.json").exists());
    }

    #[tokio::test]
    async fn post_write_validation_failure_restores_original_files() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let service = TransactionService::new(
            paths.clone(),
            BackupService::new(paths.backups_dir.clone(), "0.1.0"),
        );

        let error = service
            .execute(request(None), |_| {
                Err(AppError::new(
                    "POST_WRITE_VALIDATION_FAILED",
                    "写入后验证失败。",
                    "injected post-write validation failure",
                ))
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "TRANSACTION_FAILED_ROLLED_BACK");
        assert_eq!(fs::read(&paths.config_file).unwrap(), CONFIG_A);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), AUTH_A);
    }

    #[tokio::test]
    async fn invalid_temporary_toml_is_rejected_and_original_is_restored() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let service = TransactionService::new(
            paths.clone(),
            BackupService::new(paths.backups_dir.clone(), "0.1.0"),
        );
        let mut invalid = request(None);
        invalid.changes.config = FileChange::Write(b"model_provider = \"unterminated\n".to_vec());
        invalid.changes.auth = FileChange::Unchanged;

        let error = service.execute(invalid, |_| Ok(())).await.unwrap_err();

        assert_eq!(error.code(), "TRANSACTION_FAILED_ROLLED_BACK");
        assert_eq!(fs::read(&paths.config_file).unwrap(), CONFIG_A);
    }

    #[tokio::test]
    async fn invalid_temporary_json_is_rejected_and_originals_are_restored() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let service = TransactionService::new(
            paths.clone(),
            BackupService::new(paths.backups_dir.clone(), "0.1.0"),
        );
        let mut invalid_auth = request(None);
        invalid_auth.changes.config = FileChange::Unchanged;
        invalid_auth.changes.auth = FileChange::Write(b"{ invalid auth".to_vec());
        let auth_error = service.execute(invalid_auth, |_| Ok(())).await.unwrap_err();
        assert_eq!(auth_error.code(), "TRANSACTION_FAILED_ROLLED_BACK");
        assert_eq!(fs::read(&paths.auth_file).unwrap(), AUTH_A);

        let mut invalid_providers = request(None);
        invalid_providers.changes.config = FileChange::Unchanged;
        invalid_providers.changes.auth = FileChange::Unchanged;
        invalid_providers.changes.providers = FileChange::Write(b"{ invalid providers".to_vec());
        let providers_error = service
            .execute(invalid_providers, |_| Ok(()))
            .await
            .unwrap_err();
        assert_eq!(providers_error.code(), "TRANSACTION_FAILED_ROLLED_BACK");
        assert_eq!(fs::read(&paths.providers_file).unwrap(), PROVIDERS);
    }

    #[tokio::test]
    async fn rollback_failure_is_reported_truthfully_and_marker_remains() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let file_ops = Arc::new(InjectedFileOps::new(
            Some(FailurePoint::Forward(ManagedFileKind::Auth)),
            Duration::ZERO,
        ));
        let service = TransactionService::with_file_ops(
            paths.clone(),
            BackupService::new(paths.backups_dir.clone(), "0.1.0"),
            file_ops.clone(),
        );
        *file_ops.failure.lock().unwrap() = Some(FailurePoint::Forward(ManagedFileKind::Auth));

        let first_error = service
            .execute(request(None), |_| Ok(()))
            .await
            .unwrap_err();
        assert_eq!(first_error.code(), "TRANSACTION_FAILED_ROLLED_BACK");

        *file_ops.failure.lock().unwrap() = Some(FailurePoint::Rollback(ManagedFileKind::Config));
        // Force a forward validation failure after config has been written, then fail config rollback.
        let error = service
            .execute(request(None), |_| {
                Err(AppError::new(
                    "POST_WRITE_VALIDATION_FAILED",
                    "写入后验证失败。",
                    "injected validation failure",
                ))
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "ROLLBACK_INCOMPLETE");
        assert!(!error.public_message().contains("原配置已恢复"));
        assert!(paths.app_data_dir.join("transaction.json").exists());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn simultaneous_transactions_are_serialized_by_one_lock() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let file_ops = Arc::new(InjectedFileOps::new(None, Duration::from_millis(30)));
        let service = Arc::new(TransactionService::with_file_ops(
            paths.clone(),
            BackupService::new(paths.backups_dir.clone(), "0.1.0"),
            file_ops.clone(),
        ));

        let left = {
            let service = service.clone();
            tokio::spawn(async move { service.execute(request(None), |_| Ok(())).await })
        };
        let right = {
            let service = service.clone();
            tokio::spawn(async move { service.execute(request(None), |_| Ok(())).await })
        };
        left.await.unwrap().unwrap();
        right.await.unwrap().unwrap();

        assert_eq!(file_ops.max_active_writes.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn restore_backup_first_backs_up_current_state_and_restores_exact_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_initial_files(&paths);
        let backup = BackupService::new(paths.backups_dir.clone(), "0.1.0");
        let selected = backup
            .create_backup(
                &ConfigTransaction {
                    id: "selected-backup".into(),
                    operation: TransactionOperation::SwitchProvider,
                    provider_id: Some("provider-b".into()),
                    started_at: "2026-07-20T22:00:00+08:00".into(),
                },
                &FileSnapshot {
                    config: Some(CONFIG_B.to_vec()),
                    auth: Some(AUTH_B.to_vec()),
                    providers: Some(PROVIDERS.to_vec()),
                },
            )
            .unwrap();
        let service = TransactionService::new(paths.clone(), backup.clone());

        service
            .restore_backup(&selected.directory_name)
            .await
            .unwrap();

        assert_eq!(fs::read(&paths.config_file).unwrap(), CONFIG_B);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), AUTH_B);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), PROVIDERS);
        assert_eq!(backup.list_backups().unwrap().len(), 2);
    }
}
