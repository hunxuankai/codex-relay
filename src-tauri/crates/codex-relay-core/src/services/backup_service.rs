use crate::error::AppError;
use crate::models::backup::{
    BACKUP_METADATA_SCHEMA_VERSION, BackupCompatibility, BackupFileName, BackupInventory,
    BackupMetadata, BackupSummary, UnavailableBackup,
};
use crate::models::transaction::{ConfigTransaction, TransactionOperation};
use chrono::Utc;
use serde::Deserialize;
use serde_json::Value;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "config.toml";
const AUTH_FILE_NAME: &str = "auth.json";
const PROVIDERS_FILE_NAME: &str = "providers.json";
const PREFERENCES_FILE_NAME: &str = "provider-preferences.json";
const METADATA_FILE_NAME: &str = "metadata.json";

#[derive(Clone, Debug)]
struct ParsedBackupMetadata {
    metadata: BackupMetadata,
    compatibility: BackupCompatibility,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnversionedBackupMetadata {
    transaction_id: String,
    created_at: String,
    operation: String,
    provider_id: Option<String>,
    config_existed: bool,
    auth_existed: bool,
    providers_existed: bool,
    #[serde(default)]
    preferences_existed: bool,
    app_version: String,
}

#[derive(Clone, Default, Eq, PartialEq)]
pub struct FileSnapshot {
    pub config: Option<Vec<u8>>,
    pub auth: Option<Vec<u8>>,
    pub providers: Option<Vec<u8>>,
    pub preferences: Option<Vec<u8>>,
}

impl fmt::Debug for FileSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FileSnapshot")
            .field("config_existed", &self.config.is_some())
            .field("auth_existed", &self.auth.is_some())
            .field("providers_existed", &self.providers.is_some())
            .field("preferences_existed", &self.preferences.is_some())
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
            schema_version: BACKUP_METADATA_SCHEMA_VERSION,
            transaction_id: transaction.id.clone(),
            created_at: transaction.started_at.clone(),
            operation: operation_name(transaction.operation).into(),
            provider_id: transaction.provider_id.clone(),
            config_existed: snapshot.config.is_some(),
            auth_existed: snapshot.auth.is_some(),
            providers_existed: snapshot.providers.is_some(),
            preferences_existed: snapshot.preferences.is_some(),
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
            write_optional_snapshot(
                &directory,
                PREFERENCES_FILE_NAME,
                snapshot.preferences.as_deref(),
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

        let files = available_backup_files(&directory, &metadata);
        Ok(BackupSummary {
            directory_name,
            metadata,
            files,
            compatibility: BackupCompatibility::Current,
        })
    }

    pub fn list_backups(&self) -> Result<BackupInventory, AppError> {
        if !self.root.exists() {
            return Ok(BackupInventory::default());
        }

        let mut backups = Vec::new();
        let mut unavailable_backups = Vec::new();
        for entry in fs::read_dir(&self.root)
            .map_err(AppError::from)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|file_type| file_type.is_dir())
                    .map(|_| entry)
            })
        {
            let directory_name = entry.file_name().to_string_lossy().into_owned();
            let directory = entry.path();
            match read_metadata(&directory.join(METADATA_FILE_NAME)) {
                Ok(parsed) => {
                    let files = available_backup_files(&directory, &parsed.metadata);
                    backups.push(BackupSummary {
                        directory_name,
                        metadata: parsed.metadata,
                        files,
                        compatibility: parsed.compatibility,
                    });
                }
                Err(error) if is_unavailable_backup_metadata_error(&error) => {
                    unavailable_backups.push(unavailable_backup(
                        directory_name,
                        &directory,
                        &error,
                    ));
                }
                Err(error) => return Err(error),
            }
        }

        backups.sort_by(|left, right| right.metadata.created_at.cmp(&left.metadata.created_at));
        unavailable_backups.sort_by(|left, right| left.directory_name.cmp(&right.directory_name));
        Ok(BackupInventory {
            backups,
            unavailable_backups,
        })
    }

    pub fn load_snapshot(&self, directory_name: &str) -> Result<FileSnapshot, AppError> {
        validate_backup_name(directory_name)?;
        let directory = self.root.join(directory_name);
        let metadata = read_metadata(&directory.join(METADATA_FILE_NAME))?.metadata;

        Ok(FileSnapshot {
            config: read_snapshot_file(&directory.join(CONFIG_FILE_NAME), metadata.config_existed)?,
            auth: read_snapshot_file(&directory.join(AUTH_FILE_NAME), metadata.auth_existed)?,
            providers: read_snapshot_file(
                &directory.join(PROVIDERS_FILE_NAME),
                metadata.providers_existed,
            )?,
            preferences: read_snapshot_file(
                &directory.join(PREFERENCES_FILE_NAME),
                metadata.preferences_existed,
            )?,
        })
    }

    pub fn resolve_backup_file(
        &self,
        directory_name: &str,
        file_name: BackupFileName,
    ) -> Result<PathBuf, AppError> {
        validate_backup_name(directory_name)?;
        let root = fs::canonicalize(&self.root).map_err(|error| {
            AppError::new(
                "BACKUP_DIRECTORY_NOT_FOUND",
                "备份目录不存在或无法访问。",
                error.to_string(),
            )
        })?;
        let directory = fs::canonicalize(self.root.join(directory_name)).map_err(|error| {
            AppError::new(
                "BACKUP_NOT_FOUND",
                "所选备份不存在或无法访问。",
                error.to_string(),
            )
        })?;
        if directory.parent() != Some(root.as_path()) {
            return Err(invalid_backup_path(
                "backup directory resolved outside backup root",
            ));
        }

        let metadata_path = resolve_file_inside_directory(&directory, METADATA_FILE_NAME)?;
        if file_name == BackupFileName::Metadata {
            return Ok(metadata_path);
        }
        let metadata = read_metadata(&metadata_path)?.metadata;
        if !file_name.existed_in(&metadata) {
            return Err(AppError::new(
                "BACKUP_FILE_NOT_FOUND",
                "该备份中不存在所选文件。",
                "backup metadata records selected file as absent",
            ));
        }
        resolve_file_inside_directory(&directory, file_name.as_str())
    }

    pub fn cleanup_old_backups(
        &self,
        max_backups: usize,
        active_transaction_id: Option<&str>,
    ) -> Result<(), AppError> {
        let mut backups = self.list_backups()?.backups;
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
        TransactionOperation::UpdateProviderPreference => "update_provider_preference",
        TransactionOperation::SaveProviderBaseUrls => "save_provider_base_urls",
        TransactionOperation::SelectProviderBaseUrl => "select_provider_base_url",
        TransactionOperation::SaveProviderApiKeys => "save_provider_api_keys",
        TransactionOperation::SelectProviderApiKey => "select_provider_api_key",
        TransactionOperation::ImportCurrentApiKey => "import_current_api_key",
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

fn available_backup_files(directory: &Path, metadata: &BackupMetadata) -> Vec<BackupFileName> {
    metadata
        .files()
        .into_iter()
        .filter(|file_name| directory.join(file_name.as_str()).is_file())
        .collect()
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

fn read_metadata(path: &Path) -> Result<ParsedBackupMetadata, AppError> {
    let bytes = fs::read(path).map_err(|error| {
        AppError::new(
            "BACKUP_METADATA_READ_FAILED",
            "无法读取备份元数据。",
            error.to_string(),
        )
    })?;
    let value = serde_json::from_slice::<Value>(&bytes).map_err(invalid_backup_metadata)?;
    let object = value.as_object().ok_or_else(|| {
        AppError::new(
            "INVALID_BACKUP_METADATA",
            "备份元数据格式无效。",
            "backup metadata JSON root is not an object",
        )
    })?;

    match object.get("schemaVersion") {
        Some(Value::Number(version)) => {
            let version = version.as_u64().ok_or_else(|| {
                AppError::new(
                    "INVALID_BACKUP_METADATA",
                    "备份元数据格式无效。",
                    "backup metadata schemaVersion is not an unsigned integer",
                )
            })?;
            if version != u64::from(BACKUP_METADATA_SCHEMA_VERSION) {
                return Err(AppError::new(
                    "UNSUPPORTED_BACKUP_METADATA_VERSION",
                    "备份元数据版本不受支持。",
                    format!("unsupported backup metadata schema version: {version}"),
                ));
            }
            let metadata =
                serde_json::from_value::<BackupMetadata>(value).map_err(invalid_backup_metadata)?;
            Ok(ParsedBackupMetadata {
                metadata,
                compatibility: BackupCompatibility::Current,
            })
        }
        Some(_) => Err(AppError::new(
            "INVALID_BACKUP_METADATA",
            "备份元数据格式无效。",
            "backup metadata schemaVersion is not a number",
        )),
        None => {
            let compatibility = if object.contains_key("preferencesExisted") {
                BackupCompatibility::Current
            } else {
                BackupCompatibility::LegacyWithoutPreferences
            };
            let legacy = serde_json::from_value::<UnversionedBackupMetadata>(value)
                .map_err(invalid_backup_metadata)?;
            Ok(ParsedBackupMetadata {
                metadata: BackupMetadata {
                    schema_version: if compatibility == BackupCompatibility::Current {
                        BACKUP_METADATA_SCHEMA_VERSION
                    } else {
                        1
                    },
                    transaction_id: legacy.transaction_id,
                    created_at: legacy.created_at,
                    operation: legacy.operation,
                    provider_id: legacy.provider_id,
                    config_existed: legacy.config_existed,
                    auth_existed: legacy.auth_existed,
                    providers_existed: legacy.providers_existed,
                    preferences_existed: legacy.preferences_existed,
                    app_version: legacy.app_version,
                },
                compatibility,
            })
        }
    }
}

fn invalid_backup_metadata(error: serde_json::Error) -> AppError {
    AppError::new(
        "INVALID_BACKUP_METADATA",
        "备份元数据格式无效。",
        error.to_string(),
    )
}

fn is_unavailable_backup_metadata_error(error: &AppError) -> bool {
    matches!(
        error.code(),
        "BACKUP_METADATA_READ_FAILED"
            | "INVALID_BACKUP_METADATA"
            | "UNSUPPORTED_BACKUP_METADATA_VERSION"
    )
}

fn unavailable_backup(
    directory_name: String,
    directory: &Path,
    error: &AppError,
) -> UnavailableBackup {
    let message = match error.code() {
        "UNSUPPORTED_BACKUP_METADATA_VERSION" => {
            "此备份使用当前版本不支持的元数据格式，已保留，无法安全恢复。"
        }
        _ => "无法读取此备份的元数据，已保留，无法安全恢复。",
    };
    UnavailableBackup {
        directory_name,
        code: error.code().into(),
        message: message.into(),
        can_open_metadata: directory.join(METADATA_FILE_NAME).is_file(),
    }
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

fn invalid_backup_path(detail: &str) -> AppError {
    AppError::new("INVALID_BACKUP_PATH", "备份文件路径无效。", detail)
}

fn resolve_file_inside_directory(directory: &Path, file_name: &str) -> Result<PathBuf, AppError> {
    let path = fs::canonicalize(directory.join(file_name)).map_err(|error| {
        AppError::new(
            "BACKUP_FILE_MISSING",
            "备份文件不存在或无法访问。",
            error.to_string(),
        )
    })?;
    if path.parent() != Some(directory) || !path.is_file() {
        return Err(invalid_backup_path(
            "backup file resolved outside selected backup directory or is not a file",
        ));
    }
    Ok(path)
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
    use crate::models::backup::BackupFileName;
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
    fn provider_multi_credential_operations_have_stable_backup_names() {
        assert_eq!(
            operation_name(TransactionOperation::SaveProviderBaseUrls),
            "save_provider_base_urls"
        );
        assert_eq!(
            operation_name(TransactionOperation::SelectProviderBaseUrl),
            "select_provider_base_url"
        );
        assert_eq!(
            operation_name(TransactionOperation::SaveProviderApiKeys),
            "save_provider_api_keys"
        );
        assert_eq!(
            operation_name(TransactionOperation::SelectProviderApiKey),
            "select_provider_api_key"
        );
        assert_eq!(
            operation_name(TransactionOperation::ImportCurrentApiKey),
            "import_current_api_key"
        );
    }

    #[test]
    fn backup_contains_only_existing_files_and_secret_free_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let snapshot = FileSnapshot {
            config: Some(b"model_provider = \"provider-a\"\n".to_vec()),
            auth: Some(b"{\n  \"OPENAI_API_KEY\": \"test-key-a-not-real\"\n}\n".to_vec()),
            providers: None,
            preferences: None,
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
        assert!(metadata.contains("\"schemaVersion\": 2"));
        assert!(metadata.ends_with('\n'));
        assert!(!summary.metadata.providers_existed);
        assert_eq!(
            summary.files,
            vec![
                BackupFileName::Config,
                BackupFileName::Auth,
                BackupFileName::Metadata,
            ]
        );
    }

    #[test]
    fn list_is_newest_first_and_snapshot_round_trips_absence() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let first = FileSnapshot {
            config: Some(b"first\n".to_vec()),
            auth: None,
            providers: None,
            preferences: None,
        };
        let second = FileSnapshot {
            config: Some(b"second\n".to_vec()),
            auth: None,
            providers: Some(b"{\"version\":1,\"providers\":{}}\n".to_vec()),
            preferences: None,
        };
        service
            .create_backup(&transaction("tx-old", "2026-07-19T22:00:00+08:00"), &first)
            .unwrap();
        let newest = service
            .create_backup(&transaction("tx-new", "2026-07-20T22:00:00+08:00"), &second)
            .unwrap();

        let listed = service.list_backups().unwrap();
        let loaded = service.load_snapshot(&newest.directory_name).unwrap();

        assert_eq!(listed.backups[0].metadata.transaction_id, "tx-new");
        assert_eq!(listed.backups[1].metadata.transaction_id, "tx-old");
        assert_eq!(loaded.config.as_deref(), Some(b"second\n".as_slice()));
        assert!(loaded.auth.is_none());
        assert!(loaded.providers.is_some());
    }

    #[test]
    fn legacy_metadata_without_preferences_is_recoverable_without_rewriting_it() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let backup_directory = service.root().join("legacy-backup");
        let metadata = br#"{
  "transactionId": "legacy-transaction",
  "createdAt": "2026-07-19T22:00:00+08:00",
  "operation": "switch_provider",
  "providerId": "provider-a",
  "configExisted": true,
  "authExisted": false,
  "providersExisted": false,
  "appVersion": "0.1.0"
}
"#;
        fs::create_dir_all(&backup_directory).unwrap();
        fs::write(backup_directory.join(METADATA_FILE_NAME), metadata).unwrap();
        fs::write(
            backup_directory.join(CONFIG_FILE_NAME),
            b"model_provider = \"provider-a\"\n",
        )
        .unwrap();

        let listed = service.list_backups().unwrap();
        let loaded = service.load_snapshot("legacy-backup").unwrap();
        let resolved = service
            .resolve_backup_file("legacy-backup", BackupFileName::Config)
            .unwrap();
        service.cleanup_old_backups(20, None).unwrap();

        assert_eq!(listed.backups.len(), 1);
        assert_eq!(
            listed.backups[0].metadata.transaction_id,
            "legacy-transaction"
        );
        assert!(!listed.backups[0].metadata.preferences_existed);
        assert_eq!(
            listed.backups[0].compatibility,
            BackupCompatibility::LegacyWithoutPreferences
        );
        assert_eq!(
            listed.backups[0].files,
            vec![BackupFileName::Config, BackupFileName::Metadata]
        );
        assert_eq!(
            loaded.config.as_deref(),
            Some(b"model_provider = \"provider-a\"\n".as_slice())
        );
        assert!(loaded.preferences.is_none());
        assert_eq!(
            resolved,
            fs::canonicalize(backup_directory.join(CONFIG_FILE_NAME)).unwrap()
        );
        assert!(backup_directory.exists());
        assert_eq!(
            fs::read(backup_directory.join(METADATA_FILE_NAME)).unwrap(),
            metadata
        );
    }

    #[test]
    fn unversioned_metadata_with_preferences_keeps_the_preferences_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let backup_directory = service.root().join("unversioned-v2-backup");
        let metadata = br#"{
  "transactionId": "unversioned-v2",
  "createdAt": "2026-07-20T22:00:00+08:00",
  "operation": "switch_provider",
  "providerId": "provider-a",
  "configExisted": false,
  "authExisted": false,
  "providersExisted": false,
  "preferencesExisted": true,
  "appVersion": "0.1.2"
}
"#;
        fs::create_dir_all(&backup_directory).unwrap();
        fs::write(backup_directory.join(METADATA_FILE_NAME), metadata).unwrap();
        fs::write(
            backup_directory.join(PREFERENCES_FILE_NAME),
            b"{\n  \"version\": 2,\n  \"providers\": {}\n}\n",
        )
        .unwrap();

        let listed = service.list_backups().unwrap();
        let loaded = service.load_snapshot("unversioned-v2-backup").unwrap();

        assert_eq!(listed.backups.len(), 1);
        assert_eq!(
            listed.backups[0].compatibility,
            BackupCompatibility::Current
        );
        assert!(listed.backups[0].metadata.preferences_existed);
        assert_eq!(
            listed.backups[0].metadata.schema_version,
            BACKUP_METADATA_SCHEMA_VERSION
        );
        assert!(loaded.preferences.is_some());
    }

    #[test]
    fn unsupported_metadata_version_is_listed_as_unavailable() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let unavailable_directory = service.root().join("future-backup");
        fs::create_dir_all(&unavailable_directory).unwrap();
        fs::write(
            unavailable_directory.join(METADATA_FILE_NAME),
            br#"{
  "schemaVersion": 3,
  "transactionId": "future-transaction"
}
"#,
        )
        .unwrap();

        let listed = service.list_backups().unwrap();

        assert!(listed.backups.is_empty());
        assert_eq!(listed.unavailable_backups.len(), 1);
        assert_eq!(
            listed.unavailable_backups[0].code,
            "UNSUPPORTED_BACKUP_METADATA_VERSION"
        );
        assert_eq!(
            listed.unavailable_backups[0].message,
            "此备份使用当前版本不支持的元数据格式，已保留，无法安全恢复。"
        );
    }

    #[test]
    fn invalid_metadata_does_not_hide_recoverable_backups_or_block_cleanup() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let valid = service
            .create_backup(
                &transaction("tx-valid", "2026-07-20T22:00:00+08:00"),
                &FileSnapshot::default(),
            )
            .unwrap();
        let unavailable_directory = service.root().join("unavailable-backup");
        fs::create_dir_all(&unavailable_directory).unwrap();
        fs::write(
            unavailable_directory.join(METADATA_FILE_NAME),
            b"{ invalid json",
        )
        .unwrap();

        let listed = service.list_backups().unwrap();
        service.cleanup_old_backups(20, None).unwrap();

        assert_eq!(listed.backups.len(), 1);
        assert_eq!(listed.backups[0].directory_name, valid.directory_name);
        assert_eq!(listed.unavailable_backups.len(), 1);
        assert_eq!(
            listed.unavailable_backups[0].directory_name,
            "unavailable-backup"
        );
        assert_eq!(
            listed.unavailable_backups[0].code,
            "INVALID_BACKUP_METADATA"
        );
        assert!(listed.unavailable_backups[0].can_open_metadata);
        assert_eq!(
            listed.unavailable_backups[0].message,
            "无法读取此备份的元数据，已保留，无法安全恢复。"
        );
        assert!(unavailable_directory.exists());
    }

    #[test]
    fn missing_metadata_is_listed_without_an_open_action() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        fs::create_dir_all(service.root().join("missing-metadata")).unwrap();

        let listed = service.list_backups().unwrap();

        assert!(listed.backups.is_empty());
        assert_eq!(listed.unavailable_backups.len(), 1);
        assert_eq!(
            listed.unavailable_backups[0].code,
            "BACKUP_METADATA_READ_FAILED"
        );
        assert!(!listed.unavailable_backups[0].can_open_metadata);
    }

    #[test]
    fn metadata_file_can_be_opened_when_its_contents_are_unreadable() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let backup_directory = service.root().join("unavailable-backup");
        fs::create_dir_all(&backup_directory).unwrap();
        fs::write(backup_directory.join(METADATA_FILE_NAME), b"{ invalid json").unwrap();

        let resolved = service
            .resolve_backup_file("unavailable-backup", BackupFileName::Metadata)
            .unwrap();

        assert_eq!(
            resolved,
            fs::canonicalize(backup_directory.join(METADATA_FILE_NAME)).unwrap()
        );
    }

    #[test]
    fn resolves_an_existing_backup_file_inside_the_selected_directory() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let snapshot = FileSnapshot {
            config: Some(b"model_provider = \"provider-a\"\n".to_vec()),
            auth: Some(b"{\"OPENAI_API_KEY\":\"test-key-a-not-real\"}\n".to_vec()),
            providers: None,
            preferences: None,
        };
        let summary = service
            .create_backup(
                &transaction("tx-open", "2026-07-20T22:00:00+08:00"),
                &snapshot,
            )
            .unwrap();

        let resolved = service
            .resolve_backup_file(&summary.directory_name, BackupFileName::Auth)
            .unwrap();

        assert_eq!(
            resolved,
            fs::canonicalize(
                service
                    .root()
                    .join(summary.directory_name)
                    .join("auth.json")
            )
            .unwrap()
        );
    }

    #[test]
    fn rejects_a_snapshot_file_recorded_as_absent() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let summary = service
            .create_backup(
                &transaction("tx-absent", "2026-07-20T22:00:00+08:00"),
                &FileSnapshot {
                    config: Some(b"model_provider = \"provider-a\"\n".to_vec()),
                    auth: None,
                    providers: None,
                    preferences: None,
                },
            )
            .unwrap();

        let error = service
            .resolve_backup_file(&summary.directory_name, BackupFileName::Auth)
            .unwrap_err();

        assert_eq!(error.code(), "BACKUP_FILE_NOT_FOUND");
    }

    #[test]
    fn rejects_a_snapshot_file_missing_from_disk() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let summary = service
            .create_backup(
                &transaction("tx-missing", "2026-07-20T22:00:00+08:00"),
                &FileSnapshot {
                    config: None,
                    auth: Some(b"{\"OPENAI_API_KEY\":\"test-key-a-not-real\"}\n".to_vec()),
                    providers: None,
                    preferences: None,
                },
            )
            .unwrap();
        fs::remove_file(
            service
                .root()
                .join(&summary.directory_name)
                .join("auth.json"),
        )
        .unwrap();

        let error = service
            .resolve_backup_file(&summary.directory_name, BackupFileName::Auth)
            .unwrap_err();

        assert_eq!(error.code(), "BACKUP_FILE_MISSING");
    }

    #[test]
    fn listed_files_exclude_a_snapshot_missing_from_disk() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");
        let summary = service
            .create_backup(
                &transaction("tx-list-missing", "2026-07-20T22:00:00+08:00"),
                &FileSnapshot {
                    config: None,
                    auth: Some(b"{\"OPENAI_API_KEY\":\"test-key-a-not-real\"}\n".to_vec()),
                    providers: None,
                    preferences: None,
                },
            )
            .unwrap();
        fs::remove_file(
            service
                .root()
                .join(&summary.directory_name)
                .join("auth.json"),
        )
        .unwrap();

        let listed = service.list_backups().unwrap();

        assert_eq!(listed.backups[0].files, vec![BackupFileName::Metadata]);
    }

    #[test]
    fn resolving_a_backup_file_rejects_path_traversal() {
        let directory = tempfile::tempdir().unwrap();
        let service = BackupService::new(directory.path().join("backups"), "0.1.0");

        let error = service
            .resolve_backup_file("..\\outside", BackupFileName::Metadata)
            .unwrap_err();

        assert_eq!(error.code(), "INVALID_BACKUP_NAME");
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

        assert_eq!(listed.backups.len(), 20);
        assert!(
            listed
                .backups
                .iter()
                .any(|backup| backup.metadata.transaction_id == "tx-01")
        );
        assert!(
            !listed
                .backups
                .iter()
                .any(|backup| backup.metadata.transaction_id == "tx-02")
        );
        assert!(
            !listed
                .backups
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
