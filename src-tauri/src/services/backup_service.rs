use crate::error::AppError;
use crate::models::backup::{BackupMetadata, BackupSummary};
use crate::models::transaction::{ConfigTransaction, TransactionOperation};
use chrono::Utc;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "config.toml";
const AUTH_FILE_NAME: &str = "auth.json";
const PROVIDERS_FILE_NAME: &str = "providers.json";
const METADATA_FILE_NAME: &str = "metadata.json";

#[derive(Clone, Default, Eq, PartialEq)]
pub struct FileSnapshot {
    pub config: Option<Vec<u8>>,
    pub auth: Option<Vec<u8>>,
    pub providers: Option<Vec<u8>>,
}

impl fmt::Debug for FileSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileSnapshot")
            .field("config_existed", &self.config.is_some())
            .field("auth_existed", &self.auth.is_some())
            .field("providers_existed", &self.providers.is_some())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct BackupService {
    root: PathBuf,
    app_version: String,
}

impl BackupService {
    pub fn new(root: PathBuf, app_version: impl Into<String>) -> Self {
        Self {
            root,
            app_version: app_version.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create_backup(
        &self,
        transaction: &ConfigTransaction,
        snapshot: &FileSnapshot,
    ) -> Result<BackupSummary, AppError> {
        fs::create_dir_all(&self.root).map_err(AppError::from)?;
        let directory_name = format!(
            "{}-{}",
            Utc::now().format("%Y%m%d-%H%M%S-%3f"),
            safe_component(&transaction.id)
        );
        let directory = self.root.join(&directory_name);
        fs::create_dir(&directory).map_err(|error| {
            AppError::new(
                "BACKUP_CREATE_FAILED",
                "无法创建配置备份。",
                error.to_string(),
            )
        })?;

        let metadata = BackupMetadata {
            transaction_id: transaction.id.clone(),
            created_at: transaction.started_at.clone(),
            operation: operation_name(transaction.operation).into(),
            provider_id: transaction.provider_id.clone(),
            config_existed: snapshot.config.is_some(),
            auth_existed: snapshot.auth.is_some(),
            providers_existed: snapshot.providers.is_some(),
            app_version: self.app_version.clone(),
        };

        let result = (|| {
            write_optional_snapshot(&directory, CONFIG_FILE_NAME, snapshot.config.as_deref())?;
            write_optional_snapshot(&directory, AUTH_FILE_NAME, snapshot.auth.as_deref())?;
            write_optional_snapshot(
                &directory,
                PROVIDERS_FILE_NAME,
                snapshot.providers.as_deref(),
            )?;
            let mut metadata_json =
                serde_json::to_string_pretty(&metadata).map_err(AppError::from)?;
            metadata_json.push('\n');
            write_new_file(
                &directory.join(METADATA_FILE_NAME),
                metadata_json.as_bytes(),
            )
        })();

        if let Err(error) = result {
            let _ = fs::remove_dir_all(&directory);
            return Err(error);
        }

        Ok(BackupSummary {
            directory_name,
            metadata,
        })
    }

    pub fn list_backups(&self) -> Result<Vec<BackupSummary>, AppError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut backups = fs::read_dir(&self.root)
            .map_err(AppError::from)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|file_type| file_type.is_dir())
                    .map(|_| entry)
            })
            .map(|entry| {
                let directory_name = entry.file_name().to_string_lossy().into_owned();
                let metadata = read_metadata(&entry.path().join(METADATA_FILE_NAME))?;
                Ok(BackupSummary {
                    directory_name,
                    metadata,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        backups.sort_by(|left, right| right.metadata.created_at.cmp(&left.metadata.created_at));
        Ok(backups)
    }

    pub fn load_snapshot(&self, directory_name: &str) -> Result<FileSnapshot, AppError> {
        validate_backup_name(directory_name)?;
        let directory = self.root.join(directory_name);
        let metadata = read_metadata(&directory.join(METADATA_FILE_NAME))?;

        Ok(FileSnapshot {
            config: read_snapshot_file(&directory.join(CONFIG_FILE_NAME), metadata.config_existed)?,
            auth: read_snapshot_file(&directory.join(AUTH_FILE_NAME), metadata.auth_existed)?,
            providers: read_snapshot_file(
                &directory.join(PROVIDERS_FILE_NAME),
                metadata.providers_existed,
            )?,
        })
    }

    pub fn cleanup_old_backups(
        &self,
        max_backups: usize,
        active_transaction_id: Option<&str>,
    ) -> Result<(), AppError> {
        let mut backups = self.list_backups()?;
        backups.sort_by(|left, right| left.metadata.created_at.cmp(&right.metadata.created_at));

        while backups.len() > max_backups {
            let Some(index) = backups.iter().position(|backup| {
                active_transaction_id != Some(backup.metadata.transaction_id.as_str())
            }) else {
                break;
            };
            let backup = backups.remove(index);
            fs::remove_dir_all(self.root.join(backup.directory_name)).map_err(|error| {
                AppError::new(
                    "BACKUP_CLEANUP_FAILED",
                    "无法清理旧配置备份。",
                    error.to_string(),
                )
            })?;
        }
        Ok(())
    }
}

fn operation_name(operation: TransactionOperation) -> &'static str {
    match operation {
        TransactionOperation::CreateProvider => "create_provider",
        TransactionOperation::UpdateProvider => "update_provider",
        TransactionOperation::DeleteProvider => "delete_provider",
        TransactionOperation::SwitchProvider => "switch_provider",
        TransactionOperation::RestoreBackup => "restore_backup",
        TransactionOperation::SyncCurrentProvider => "sync_current_provider",
    }
}

fn write_optional_snapshot(
    directory: &Path,
    file_name: &str,
    bytes: Option<&[u8]>,
) -> Result<(), AppError> {
    if let Some(bytes) = bytes {
        write_new_file(&directory.join(file_name), bytes)?;
    }
    Ok(())
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(AppError::from)?;
    file.write_all(bytes).map_err(AppError::from)?;
    file.flush().map_err(AppError::from)?;
    file.sync_all().map_err(AppError::from)
}

fn read_metadata(path: &Path) -> Result<BackupMetadata, AppError> {
    let bytes = fs::read(path).map_err(|error| {
        AppError::new(
            "BACKUP_METADATA_READ_FAILED",
            "无法读取备份元数据。",
            error.to_string(),
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|error| {
        AppError::new(
            "INVALID_BACKUP_METADATA",
            "备份元数据格式无效。",
            error.to_string(),
        )
    })
}

fn read_snapshot_file(path: &Path, expected: bool) -> Result<Option<Vec<u8>>, AppError> {
    if !expected {
        return Ok(None);
    }
    fs::read(path).map(Some).map_err(|error| {
        AppError::new(
            "BACKUP_FILE_MISSING",
            "备份文件不完整，无法恢复。",
            error.to_string(),
        )
    })
}

fn validate_backup_name(name: &str) -> Result<(), AppError> {
    let valid = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\');
    if valid {
        Ok(())
    } else {
        Err(AppError::new(
            "INVALID_BACKUP_NAME",
            "备份名称无效。",
            "backup directory name contains path traversal characters",
        ))
    }
}

fn safe_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::transaction::{ConfigTransaction, TransactionOperation};
    use std::fs;

    fn transaction(id: &str, created_at: &str) -> ConfigTransaction {
        ConfigTransaction {
            id: id.into(),
            operation: TransactionOperation::SwitchProvider,
            provider_id: Some("provider-a".into()),
            started_at: created_at.into(),
        }
    }

    #[test]
    fn backup_contains_only_existing_files_and_secret_free_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let snapshot = FileSnapshot {
            config: Some(b"model_provider = \"provider-a\"\n".to_vec()),
            auth: Some(b"{\n  \"OPENAI_API_KEY\": \"test-key-a-not-real\"\n}\n".to_vec()),
            providers: None,
        };

        let summary = service
            .create_backup(&transaction("tx-1", "2026-07-20T22:00:00+08:00"), &snapshot)
            .unwrap();
        let backup_directory = service.root().join(&summary.directory_name);

        assert!(backup_directory.join("config.toml").exists());
        assert!(backup_directory.join("auth.json").exists());
        assert!(!backup_directory.join("providers.json").exists());
        let metadata = fs::read_to_string(backup_directory.join("metadata.json")).unwrap();
        assert!(!metadata.contains("test-key-a-not-real"));
        assert!(!metadata.contains("OPENAI_API_KEY"));
        assert!(metadata.ends_with('\n'));
        assert!(!summary.metadata.providers_existed);
    }

    #[test]
    fn list_is_newest_first_and_snapshot_round_trips_absence() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let first = FileSnapshot {
            config: Some(b"first\n".to_vec()),
            auth: None,
            providers: None,
        };
        let second = FileSnapshot {
            config: Some(b"second\n".to_vec()),
            auth: None,
            providers: Some(b"{\"version\":1,\"providers\":{}}\n".to_vec()),
        };
        service
            .create_backup(&transaction("tx-old", "2026-07-19T22:00:00+08:00"), &first)
            .unwrap();
        let newest = service
            .create_backup(&transaction("tx-new", "2026-07-20T22:00:00+08:00"), &second)
            .unwrap();

        let listed = service.list_backups().unwrap();
        let loaded = service.load_snapshot(&newest.directory_name).unwrap();

        assert_eq!(listed[0].metadata.transaction_id, "tx-new");
        assert_eq!(listed[1].metadata.transaction_id, "tx-old");
        assert_eq!(loaded.config.as_deref(), Some(b"second\n".as_slice()));
        assert!(loaded.auth.is_none());
        assert!(loaded.providers.is_some());
    }

    #[test]
    fn cleanup_retains_twenty_and_never_deletes_active_transaction() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let snapshot = FileSnapshot::default();
        for day in 1..=22 {
            service
                .create_backup(
                    &transaction(
                        &format!("tx-{day:02}"),
                        &format!("2026-07-{day:02}T22:00:00+08:00"),
                    ),
                    &snapshot,
                )
                .unwrap();
        }

        service.cleanup_old_backups(20, Some("tx-01")).unwrap();
        let listed = service.list_backups().unwrap();

        assert_eq!(listed.len(), 20);
        assert!(
            listed
                .iter()
                .any(|backup| backup.metadata.transaction_id == "tx-01")
        );
        assert!(
            !listed
                .iter()
                .any(|backup| backup.metadata.transaction_id == "tx-02")
        );
        assert!(
            !listed
                .iter()
                .any(|backup| backup.metadata.transaction_id == "tx-03")
        );
    }

    #[test]
    fn snapshot_name_rejects_path_traversal() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");

        let error = service.load_snapshot("..\\outside").unwrap_err();

        assert_eq!(error.code(), "INVALID_BACKUP_NAME");
    }
}
