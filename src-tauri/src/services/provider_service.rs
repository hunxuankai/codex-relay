use crate::error::AppError;
use crate::infrastructure::file_fingerprint::FileSetFingerprint;
use crate::infrastructure::path_service::AppPaths;
use crate::models::backup::BackupSummary;
use crate::models::provider::{
    ApiKeyChange, CreateProviderInput, ProviderListState, ProviderMutationOutcome, ProviderProfile,
    SwitchOutcome, UpdateProviderInput, WireApi,
};
use crate::models::transaction::TransactionOperation;
use crate::services::auth_service::{AuthService, render_auth_json};
use crate::services::backup_service::BackupService;
use crate::services::config_service::{
    self, ProviderConfig, ProviderInput, ValidatedProviderInput,
};
use crate::services::provider_secret_service::{
    ProviderSecret, ProviderSecretService, ProviderSecretStore, normalize_api_key, serialize_store,
};
use crate::services::transaction_service::{
    FileChange, FileChanges, FileOps, StdFileOps, TransactionRequest, TransactionService,
};
use std::fs;
use std::io::ErrorKind;
use std::sync::Arc;

const CONSISTENT_READ_ATTEMPTS: usize = 3;

#[derive(Clone)]
pub struct ProviderService {
    paths: AppPaths,
    transaction_service: TransactionService,
    backup_service: BackupService,
    secret_service: ProviderSecretService,
    auth_service: AuthService,
}

impl ProviderService {
    pub fn new(paths: AppPaths, app_version: impl Into<String>) -> Self {
        Self::with_file_ops(paths, app_version, Arc::new(StdFileOps))
    }

    pub fn with_file_ops(
        paths: AppPaths,
        app_version: impl Into<String>,
        file_ops: Arc<dyn FileOps>,
    ) -> Self {
        let backup_service = BackupService::new(paths.backups_dir.clone(), app_version);
        let transaction_service =
            TransactionService::with_file_ops(paths.clone(), backup_service.clone(), file_ops);
        Self {
            backup_service,
            secret_service: ProviderSecretService::new(paths.providers_file.clone()),
            auth_service: AuthService::new(paths.auth_file.clone()),
            paths,
            transaction_service,
        }
    }

    pub fn list_backups(&self) -> Result<Vec<BackupSummary>, AppError> {
        self.backup_service.list_backups()
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub async fn restore_backup(
        &self,
        directory_name: &str,
    ) -> Result<ProviderMutationOutcome, AppError> {
        self.transaction_service
            .restore_backup(directory_name)
            .await?;
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: "配置备份已恢复。".into(),
        })
    }

    pub fn list_providers(&self) -> Result<ProviderListState, AppError> {
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        Ok(self.list_state_from_disk(&disk))
    }

    pub fn get_api_key_for_edit(&self, provider_id: &str) -> Result<Option<String>, AppError> {
        let provider_id = config_service::validate_provider_id(provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Skip)?;
        if !disk
            .provider_configs
            .iter()
            .any(|provider| provider.id == provider_id)
        {
            return Err(provider_not_found(&provider_id));
        }
        Ok(configured_key(&disk.store, &provider_id))
    }

    pub async fn create_provider(
        &self,
        input: CreateProviderInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let validated = config_service::validate_provider_input(&ProviderInput {
            id: input.id,
            name: input.name,
            base_url: input.base_url,
            wire_api: input.wire_api,
            model: input.model,
        })?;
        let api_key = normalize_api_key(&input.api_key)?;
        let disk = self.read_consistent_state(AuthReadMode::Skip)?;
        let mut new_config = config_service::create_provider(&disk.config_source, &validated)?;
        let mut new_store = disk.store.clone();
        new_store.providers.insert(
            validated.id.clone(),
            ProviderSecret {
                api_key: api_key.clone(),
            },
        );

        let auth_change = if input.activate_after_save {
            new_config = config_service::select_provider(
                &new_config,
                &provider_config_from_validated(&validated),
            )?;
            FileChange::Write(render_auth_json(&api_key)?)
        } else {
            FileChange::Unchanged
        };
        let provider_bytes = serialize_store(&new_store)?;
        let activate = input.activate_after_save;
        let expected_files = input.expected_files;
        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::CreateProvider,
                    provider_id: Some(validated.id.clone()),
                    expected_files: Some(expected_files),
                    changes: FileChanges {
                        config: FileChange::Write(new_config.into_bytes()),
                        auth: auth_change,
                        providers: FileChange::Write(provider_bytes),
                    },
                },
                |paths| validate_provider_written(paths, &validated, Some(&api_key), activate),
            )
            .await?;

        let message = if activate {
            format!(
                "Provider「{}」已保存并启用。请重启 Codex 后生效。",
                validated.name
            )
        } else {
            format!("Provider「{}」已保存。", validated.name)
        };
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message,
        })
    }

    pub async fn update_provider(
        &self,
        input: UpdateProviderInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let validated = config_service::validate_provider_input(&ProviderInput {
            id: input.id,
            name: input.name,
            base_url: input.base_url,
            wire_api: input.wire_api,
            model: input.model,
        })?;
        let disk = self.read_consistent_state(AuthReadMode::Skip)?;
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(validated.id.as_str());
        let new_config =
            config_service::update_provider(&disk.config_source, &validated.id, &validated)?;
        let mut new_store = disk.store.clone();
        let (provider_change, effective_key) = match input.api_key_change {
            ApiKeyChange::Unchanged => (
                FileChange::Unchanged,
                configured_key(&new_store, &validated.id),
            ),
            ApiKeyChange::Set(api_key) => {
                let api_key = normalize_api_key(&api_key)?;
                new_store.providers.insert(
                    validated.id.clone(),
                    ProviderSecret {
                        api_key: api_key.clone(),
                    },
                );
                (
                    FileChange::Write(serialize_store(&new_store)?),
                    Some(api_key),
                )
            }
            ApiKeyChange::Clear => {
                new_store.providers.remove(&validated.id);
                (FileChange::Write(serialize_store(&new_store)?), None)
            }
        };

        let sync_active = is_active && input.sync_if_active;
        let (final_config, auth_change) = if sync_active {
            let api_key = effective_key
                .as_deref()
                .ok_or_else(provider_api_key_missing)?;
            (
                config_service::select_provider(
                    &new_config,
                    &provider_config_from_validated(&validated),
                )?,
                FileChange::Write(render_auth_json(api_key)?),
            )
        } else {
            (new_config, FileChange::Unchanged)
        };

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::UpdateProvider,
                    provider_id: Some(validated.id.clone()),
                    expected_files: Some(input.expected_files),
                    changes: FileChanges {
                        config: FileChange::Write(final_config.into_bytes()),
                        auth: auth_change,
                        providers: provider_change,
                    },
                },
                |paths| {
                    validate_provider_written(
                        paths,
                        &validated,
                        effective_key.as_deref(),
                        sync_active,
                    )
                },
            )
            .await?;

        let message = if is_active {
            format!(
                "Provider「{}」已更新。请重启 Codex 后生效。",
                validated.name
            )
        } else {
            format!("Provider「{}」已更新。", validated.name)
        };
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message,
        })
    }

    pub async fn delete_provider(
        &self,
        provider_id: &str,
        expected_files: FileSetFingerprint,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Skip)?;
        let provider = disk
            .provider_configs
            .iter()
            .find(|provider| provider.id == provider_id)
            .cloned()
            .ok_or_else(|| provider_not_found(&provider_id))?;
        let display_name = provider_display_name(&provider);
        let new_config = config_service::delete_provider(&disk.config_source, &provider_id)?;
        let mut new_store = disk.store.clone();
        new_store.providers.remove(&provider_id);

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::DeleteProvider,
                    provider_id: Some(provider_id.clone()),
                    expected_files: Some(expected_files),
                    changes: FileChanges {
                        config: FileChange::Write(new_config.into_bytes()),
                        auth: FileChange::Unchanged,
                        providers: FileChange::Write(serialize_store(&new_store)?),
                    },
                },
                |paths| validate_provider_deleted(paths, &provider_id),
            )
            .await?;

        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: format!("Provider「{display_name}」已删除。"),
        })
    }

    pub async fn switch_provider(&self, provider_id: &str) -> Result<SwitchOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Skip)?;
        let provider = disk
            .provider_configs
            .iter()
            .find(|provider| provider.id == provider_id)
            .cloned()
            .ok_or_else(|| provider_not_found(&provider_id))?;
        let validated = config_service::validate_provider_config(&provider)?;
        let api_key =
            configured_key(&disk.store, &provider_id).ok_or_else(provider_api_key_missing)?;
        let new_config = config_service::select_provider(&disk.config_source, &provider)?;

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::SwitchProvider,
                    provider_id: Some(provider_id.clone()),
                    expected_files: Some(disk.fingerprints),
                    changes: FileChanges {
                        config: FileChange::Write(new_config.into_bytes()),
                        auth: FileChange::Write(render_auth_json(&api_key)?),
                        providers: FileChange::Unchanged,
                    },
                },
                |paths| validate_provider_written(paths, &validated, Some(&api_key), true),
            )
            .await?;

        let refreshed = self.list_providers()?;
        Ok(SwitchOutcome {
            providers: refreshed.providers,
            active_provider_id: provider_id,
            message: format!(
                "已切换到「{}」。配置已写入，请重启 Codex 后生效。",
                validated.name
            ),
        })
    }

    pub async fn import_current_auth_key(
        &self,
        expected_provider_id: &str,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let expected_provider_id = config_service::validate_provider_id(expected_provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Strict)?;
        let active = config_service::current_provider_id(&disk.document)
            .ok_or_else(|| provider_not_found(&expected_provider_id))?;
        if active != expected_provider_id {
            return Err(AppError::new(
                "AUTH_IMPORT_PROVIDER_MISMATCH",
                "只能将当前 auth.json 密钥保存到当前 Provider。",
                "requested auth import provider does not match active provider",
            ));
        }
        if configured_key(&disk.store, &active).is_some() {
            return Err(AppError::new(
                "API_KEY_ALREADY_CONFIGURED",
                "当前 Provider 已经保存了 API Key。",
                "auth import requested for provider that already has a key",
            ));
        }
        let api_key = disk.auth_key.ok_or_else(|| {
            AppError::new(
                "AUTH_KEY_MISSING",
                "auth.json 中没有可导入的 OPENAI_API_KEY。",
                "auth import requested but auth.json has no key",
            )
        })?;
        let provider = disk
            .provider_configs
            .iter()
            .find(|provider| provider.id == active)
            .cloned()
            .ok_or_else(|| provider_not_found(&active))?;
        let mut new_store = disk.store;
        new_store.providers.insert(
            active.clone(),
            ProviderSecret {
                api_key: api_key.clone(),
            },
        );

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::UpdateProvider,
                    provider_id: Some(active.clone()),
                    expected_files: Some(disk.fingerprints),
                    changes: FileChanges {
                        config: FileChange::Unchanged,
                        auth: FileChange::Unchanged,
                        providers: FileChange::Write(serialize_store(&new_store)?),
                    },
                },
                |paths| validate_secret_only(paths, &active, &api_key),
            )
            .await?;

        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: format!(
                "已将当前 Codex API Key 保存到 Provider「{}」。",
                provider_display_name(&provider)
            ),
        })
    }

    fn read_consistent_state(&self, auth_mode: AuthReadMode) -> Result<DiskState, AppError> {
        for _ in 0..CONSISTENT_READ_ATTEMPTS {
            let before = self.current_fingerprints()?;
            let config_source = read_optional_utf8(&self.paths.config_file)?.unwrap_or_default();
            let document = config_service::parse_document(&config_source)?;
            let provider_configs = config_service::list_provider_configs(&document)?;
            let store = self.secret_service.load_or_create()?;
            let auth_key = match auth_mode {
                AuthReadMode::Skip => None,
                AuthReadMode::Lenient => self.auth_service.read_api_key().ok().flatten(),
                AuthReadMode::Strict => self.auth_service.read_api_key()?,
            };
            let after = self.current_fingerprints()?;
            if before == after {
                return Ok(DiskState {
                    config_source,
                    document,
                    provider_configs,
                    store,
                    auth_key,
                    fingerprints: after,
                });
            }
        }

        Err(external_modification_conflict())
    }

    fn current_fingerprints(&self) -> Result<FileSetFingerprint, AppError> {
        FileSetFingerprint::from_paths(
            &self.paths.config_file,
            &self.paths.auth_file,
            &self.paths.providers_file,
        )
    }

    fn list_state_from_disk(&self, disk: &DiskState) -> ProviderListState {
        let active_provider_id = config_service::current_provider_id(&disk.document);
        let providers = disk
            .provider_configs
            .iter()
            .map(|provider| {
                profile_from_config(
                    provider,
                    active_provider_id.as_deref(),
                    configured_key(&disk.store, &provider.id).is_some(),
                )
            })
            .collect();
        let current_auth_import_available = active_provider_id.as_deref().is_some_and(|active| {
            configured_key(&disk.store, active).is_none() && disk.auth_key.is_some()
        });

        ProviderListState {
            providers,
            active_provider_id,
            current_auth_import_available,
            fingerprints: disk.fingerprints.clone(),
        }
    }
}

#[derive(Clone, Copy)]
enum AuthReadMode {
    Skip,
    Lenient,
    Strict,
}

struct DiskState {
    config_source: String,
    document: toml_edit::DocumentMut,
    provider_configs: Vec<ProviderConfig>,
    store: ProviderSecretStore,
    auth_key: Option<String>,
    fingerprints: FileSetFingerprint,
}

fn profile_from_config(
    provider: &ProviderConfig,
    active_provider_id: Option<&str>,
    api_key_configured: bool,
) -> ProviderProfile {
    match config_service::validate_provider_config(provider) {
        Ok(validated) => ProviderProfile {
            id: validated.id,
            name: validated.name,
            base_url: validated.base_url,
            wire_api: WireApi::Responses,
            model: validated.model,
            api_key_configured,
            is_active: active_provider_id == Some(provider.id.as_str()),
            is_valid: true,
            validation_message: None,
        },
        Err(error) => ProviderProfile {
            id: provider.id.clone(),
            name: provider_display_name(provider),
            base_url: provider.base_url.clone().unwrap_or_default(),
            wire_api: WireApi::Responses,
            model: provider.model.clone(),
            api_key_configured,
            is_active: active_provider_id == Some(provider.id.as_str()),
            is_valid: false,
            validation_message: Some(error.public_message().to_owned()),
        },
    }
}

fn provider_config_from_validated(validated: &ValidatedProviderInput) -> ProviderConfig {
    ProviderConfig {
        id: validated.id.clone(),
        name: Some(validated.name.clone()),
        base_url: Some(validated.base_url.clone()),
        wire_api: Some(validated.wire_api.clone()),
        model: validated.model.clone(),
    }
}

fn configured_key(store: &ProviderSecretStore, provider_id: &str) -> Option<String> {
    store
        .providers
        .get(provider_id)
        .map(|secret| secret.api_key.clone())
        .filter(|api_key| !api_key.is_empty())
}

fn provider_display_name(provider: &ProviderConfig) -> String {
    provider
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(&provider.id)
        .to_owned()
}

fn validate_provider_written(
    paths: &AppPaths,
    expected: &ValidatedProviderInput,
    expected_key: Option<&str>,
    require_active_auth: bool,
) -> Result<(), AppError> {
    let source = read_required_utf8(&paths.config_file)?;
    let document = config_service::parse_document(&source)?;
    let provider = config_service::list_provider_configs(&document)?
        .into_iter()
        .find(|provider| provider.id == expected.id)
        .ok_or_else(|| post_write_validation_error("provider table is missing"))?;
    let actual = config_service::validate_provider_config(&provider)?;
    if &actual != expected {
        return Err(post_write_validation_error(
            "provider fields do not match expected values",
        ));
    }

    let store = ProviderSecretService::new(paths.providers_file.clone()).load_or_create()?;
    let actual_key = configured_key(&store, &expected.id);
    if actual_key.as_deref() != expected_key {
        return Err(post_write_validation_error(
            "provider secret state does not match expected value",
        ));
    }

    if require_active_auth {
        if config_service::current_provider_id(&document).as_deref() != Some(expected.id.as_str()) {
            return Err(post_write_validation_error(
                "top-level model_provider does not match expected provider",
            ));
        }
        let auth_key = AuthService::new(paths.auth_file.clone()).read_api_key()?;
        if auth_key.as_deref() != expected_key {
            return Err(post_write_validation_error(
                "auth.json key does not match provider secret",
            ));
        }
    }
    Ok(())
}

fn validate_provider_deleted(paths: &AppPaths, provider_id: &str) -> Result<(), AppError> {
    let source = read_required_utf8(&paths.config_file)?;
    let document = config_service::parse_document(&source)?;
    if config_service::list_provider_configs(&document)?
        .iter()
        .any(|provider| provider.id == provider_id)
    {
        return Err(post_write_validation_error(
            "deleted provider still exists in config.toml",
        ));
    }
    let store = ProviderSecretService::new(paths.providers_file.clone()).load_or_create()?;
    if store.providers.contains_key(provider_id) {
        return Err(post_write_validation_error(
            "deleted provider still exists in providers.json",
        ));
    }
    Ok(())
}

fn validate_secret_only(paths: &AppPaths, provider_id: &str, key: &str) -> Result<(), AppError> {
    let store = ProviderSecretService::new(paths.providers_file.clone()).load_or_create()?;
    if configured_key(&store, provider_id).as_deref() == Some(key) {
        Ok(())
    } else {
        Err(post_write_validation_error(
            "imported provider secret does not match auth key",
        ))
    }
}

fn read_optional_utf8(path: &std::path::Path) -> Result<Option<String>, AppError> {
    match fs::read(path) {
        Ok(bytes) => String::from_utf8(bytes).map(Some).map_err(|error| {
            AppError::new(
                "INVALID_UTF8_FILE",
                "配置文件不是有效的 UTF-8。",
                error.to_string(),
            )
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::from(error)),
    }
}

fn read_required_utf8(path: &std::path::Path) -> Result<String, AppError> {
    read_optional_utf8(path)?.ok_or_else(|| {
        AppError::new(
            "CONFIG_FILE_MISSING",
            "config.toml 不存在。",
            "required config file is missing during post-write validation",
        )
    })
}

fn provider_api_key_missing() -> AppError {
    AppError::new(
        "PROVIDER_API_KEY_MISSING",
        "该 Provider 尚未设置 API Key，无法启用。",
        "target provider has no configured API key",
    )
}

fn provider_not_found(provider_id: &str) -> AppError {
    AppError::new(
        "PROVIDER_NOT_FOUND",
        "指定的 Provider 不存在。",
        format!("provider not found: {provider_id}"),
    )
}

fn external_modification_conflict() -> AppError {
    AppError::new(
        "EXTERNAL_MODIFICATION_CONFLICT",
        "配置文件已被其他程序修改。请重新加载后再保存。",
        "files changed while ProviderService was reading a consistent snapshot",
    )
}

fn post_write_validation_error(detail: &str) -> AppError {
    AppError::new(
        "POST_WRITE_VALIDATION_FAILED",
        "配置写入后的验证失败。",
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::provider::{ApiKeyChange, CreateProviderInput, UpdateProviderInput};
    use std::fs;
    use std::sync::Arc;

    const MULTIPLE: &str = include_str!("../../../fixtures/config-multiple-providers.toml");
    const WITH_COMMENTS: &str = include_str!("../../../fixtures/config-with-comments.toml");
    const WITH_UNKNOWN: &str = include_str!("../../../fixtures/config-with-unknown-fields.toml");
    const AUTH_A: &str = include_str!("../../../fixtures/auth-api-key.json");
    const PROVIDERS_MULTIPLE: &str = include_str!("../../../fixtures/providers-multiple.json");
    const PROVIDERS_EMPTY: &str = include_str!("../../../fixtures/providers-empty.json");

    fn create_paths(directory: &tempfile::TempDir) -> AppPaths {
        let codex = directory.path().join("codex");
        let app_data = directory.path().join("app-data");
        fs::create_dir_all(&codex).unwrap();
        fs::create_dir_all(&app_data).unwrap();
        AppPaths::for_test(codex, app_data).unwrap()
    }

    fn write_state(paths: &AppPaths, config: &str, auth: &str, providers: &str) {
        fs::write(&paths.config_file, config).unwrap();
        fs::write(&paths.auth_file, auth).unwrap();
        fs::write(&paths.providers_file, providers).unwrap();
    }

    fn create_input(state: &ProviderListState, activate_after_save: bool) -> CreateProviderInput {
        CreateProviderInput {
            id: "provider-c".into(),
            name: "Provider C".into(),
            base_url: "https://provider-c.example.com/v1".into(),
            wire_api: "responses".into(),
            model: Some("test-model-c".into()),
            api_key: "test-key-c-not-real".into(),
            activate_after_save,
            expected_files: state.fingerprints.clone(),
        }
    }

    #[tokio::test]
    async fn list_merges_toml_and_key_status_without_returning_secrets() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths, "0.1.0");

        let state = service.list_providers().unwrap();
        let json = serde_json::to_string(&state).unwrap();

        assert_eq!(state.providers.len(), 2);
        assert!(state.providers[0].is_active);
        assert!(state.providers[0].api_key_configured);
        assert!(state.providers[1].api_key_configured);
        assert_eq!(state.active_provider_id.as_deref(), Some("provider-a"));
        assert!(!state.current_auth_import_available);
        assert!(!json.contains("test-key-a-not-real"));
        assert!(!json.contains("test-key-b-not-real"));
        assert!(!json.contains("\"apiKey\":"));
    }

    #[tokio::test]
    async fn invalid_existing_provider_is_returned_with_validation_message() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let invalid = r#"model_provider = "broken"

[model_providers.broken]
name = " "
base_url = "ftp://invalid.example.com"
wire_api = "chat_completions"
"#;
        write_state(&paths, invalid, AUTH_A, PROVIDERS_EMPTY);
        let service = ProviderService::new(paths, "0.1.0");

        let state = service.list_providers().unwrap();

        assert_eq!(state.providers.len(), 1);
        assert!(!state.providers[0].is_valid);
        assert!(state.providers[0].validation_message.is_some());
        assert!(!state.providers[0].api_key_configured);
    }

    #[test]
    fn secret_edit_interface_rejects_orphan_secret_entries() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let orphan = r#"{
  "version": 1,
  "providers": {
    "provider-a": { "apiKey": "test-key-a-not-real" },
    "orphan": { "apiKey": "test-key-b-not-real" }
  }
}
"#;
        write_state(&paths, MULTIPLE, AUTH_A, orphan);
        let service = ProviderService::new(paths, "0.1.0");

        let error = service.get_api_key_for_edit("orphan").unwrap_err();

        assert_eq!(error.code(), "PROVIDER_NOT_FOUND");
        assert!(!error.to_string().contains("test-key-b-not-real"));
    }

    #[tokio::test]
    async fn create_provider_preserves_config_and_saves_only_target_key() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let outcome = service
            .create_provider(create_input(&before, false))
            .await
            .unwrap();

        assert_eq!(outcome.message, "Provider「Provider C」已保存。");
        let config = fs::read_to_string(&paths.config_file).unwrap();
        assert!(config.contains("# This leading comment must survive every edit."));
        assert!(config.contains("[mcp_servers.sample]"));
        assert!(config.contains("[model_providers.provider-c]"));
        let store = ProviderSecretService::new(paths.providers_file.clone())
            .load_or_create()
            .unwrap();
        assert_eq!(store.providers.len(), 3);
        assert_eq!(
            store.providers.get("provider-c").unwrap().api_key,
            "test-key-c-not-real"
        );
        assert_eq!(fs::read_to_string(&paths.auth_file).unwrap(), AUTH_A);
    }

    #[tokio::test]
    async fn create_and_activate_writes_config_model_and_auth_in_one_transaction() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let outcome = service
            .create_provider(create_input(&before, true))
            .await
            .unwrap();

        assert_eq!(
            outcome.message,
            "Provider「Provider C」已保存并启用。请重启 Codex 后生效。"
        );
        let config = fs::read_to_string(&paths.config_file).unwrap();
        assert!(config.contains("model_provider = \"provider-c\""));
        assert!(config.contains("model = \"test-model-c\""));
        assert!(config.contains("cli_auth_credentials_store = \"file\""));
        assert!(
            fs::read_to_string(&paths.auth_file)
                .unwrap()
                .contains("test-key-c-not-real")
        );
    }

    #[tokio::test]
    async fn update_preserves_unknown_fields_and_unchanged_key() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_UNKNOWN, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let input = UpdateProviderInput {
            id: "provider-a".into(),
            name: "Updated Provider A".into(),
            base_url: "https://updated.example.com/v1".into(),
            wire_api: "responses".into(),
            model: None,
            api_key_change: ApiKeyChange::Unchanged,
            sync_if_active: false,
            expected_files: before.fingerprints,
        };

        let outcome = service.update_provider(input).await.unwrap();

        assert_eq!(
            outcome.message,
            "Provider「Updated Provider A」已更新。请重启 Codex 后生效。"
        );
        let config = fs::read_to_string(&paths.config_file).unwrap();
        assert!(config.contains("unknown_number = 42"));
        assert!(config.contains("[profiles.personal]"));
        let key = service.get_api_key_for_edit("provider-a").unwrap();
        assert_eq!(key.as_deref(), Some("test-key-a-not-real"));
    }

    #[tokio::test]
    async fn updating_active_key_with_sync_updates_auth_json() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let input = UpdateProviderInput {
            id: "provider-a".into(),
            name: "Provider A".into(),
            base_url: "https://provider-a.example.com/v2".into(),
            wire_api: "responses".into(),
            model: Some("updated-model".into()),
            api_key_change: ApiKeyChange::Set("test-key-a-updated-not-real".into()),
            sync_if_active: true,
            expected_files: before.fingerprints,
        };

        service.update_provider(input).await.unwrap();

        let auth = fs::read_to_string(&paths.auth_file).unwrap();
        assert!(auth.contains("test-key-a-updated-not-real"));
        let config = fs::read_to_string(&paths.config_file).unwrap();
        assert!(config.contains("base_url = \"https://provider-a.example.com/v2\""));
        assert!(config.contains("model = \"updated-model\""));
    }

    #[tokio::test]
    async fn clearing_non_current_key_preserves_other_keys_and_active_auth() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let input = UpdateProviderInput {
            id: "provider-b".into(),
            name: "Provider B".into(),
            base_url: "https://provider-b.example.com/v1".into(),
            wire_api: "responses".into(),
            model: Some("test-model-b".into()),
            api_key_change: ApiKeyChange::Clear,
            sync_if_active: false,
            expected_files: before.fingerprints,
        };

        let outcome = service.update_provider(input).await.unwrap();

        assert_eq!(outcome.message, "Provider「Provider B」已更新。");
        let store = ProviderSecretService::new(paths.providers_file.clone())
            .load_or_create()
            .unwrap();
        assert!(store.providers.contains_key("provider-a"));
        assert!(!store.providers.contains_key("provider-b"));
        assert_eq!(fs::read_to_string(&paths.auth_file).unwrap(), AUTH_A);
    }

    #[tokio::test]
    async fn delete_removes_only_non_current_provider_and_key() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let outcome = service
            .delete_provider("provider-b", before.fingerprints.clone())
            .await
            .unwrap();

        assert_eq!(outcome.message, "Provider「Provider B」已删除。");
        assert!(
            fs::read_to_string(&paths.config_file)
                .unwrap()
                .contains("[model_providers.provider-a]")
        );
        assert!(
            !fs::read_to_string(&paths.config_file)
                .unwrap()
                .contains("[model_providers.provider-b]")
        );
        assert!(
            !ProviderSecretService::new(paths.providers_file.clone())
                .is_configured("provider-b")
                .unwrap()
        );

        let refreshed = service.list_providers().unwrap();
        let error = service
            .delete_provider("provider-a", refreshed.fingerprints)
            .await
            .unwrap_err();
        assert_eq!(error.code(), "ACTIVE_PROVIDER_DELETE_FORBIDDEN");
    }

    #[tokio::test]
    async fn switch_updates_active_provider_model_and_auth() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");

        let outcome = service.switch_provider("provider-b").await.unwrap();

        assert_eq!(
            outcome.message,
            "已切换到「Provider B」。配置已写入，请重启 Codex 后生效。"
        );
        assert_eq!(outcome.active_provider_id, "provider-b");
        let config = fs::read_to_string(&paths.config_file).unwrap();
        assert!(config.contains("model_provider = \"provider-b\""));
        assert!(config.contains("model = \"test-model-b\""));
        assert!(
            fs::read_to_string(&paths.auth_file)
                .unwrap()
                .contains("test-key-b-not-real")
        );
    }

    #[tokio::test]
    async fn switch_rejects_missing_provider_or_key_without_modifying_files() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let only_a = r#"{
  "version": 1,
  "providers": {
    "provider-a": { "apiKey": "test-key-a-not-real" }
  }
}
"#;
        write_state(&paths, MULTIPLE, AUTH_A, only_a);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let original_config = fs::read(&paths.config_file).unwrap();

        let key_error = service.switch_provider("provider-b").await.unwrap_err();
        let missing_error = service.switch_provider("missing").await.unwrap_err();

        assert_eq!(key_error.code(), "PROVIDER_API_KEY_MISSING");
        assert_eq!(missing_error.code(), "PROVIDER_NOT_FOUND");
        assert_eq!(fs::read(&paths.config_file).unwrap(), original_config);
    }

    #[tokio::test]
    async fn imports_existing_auth_key_only_into_current_provider_after_confirmation() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_EMPTY);
        let service = ProviderService::new(paths.clone(), "0.1.0");

        let before = service.list_providers().unwrap();
        assert!(before.current_auth_import_available);
        let outcome = service.import_current_auth_key("provider-a").await.unwrap();

        assert_eq!(
            outcome.message,
            "已将当前 Codex API Key 保存到 Provider「Provider A」。"
        );
        let store = ProviderSecretService::new(paths.providers_file.clone())
            .load_or_create()
            .unwrap();
        assert_eq!(store.providers.len(), 1);
        assert!(store.providers.contains_key("provider-a"));
        assert!(!store.providers.contains_key("provider-b"));
    }

    #[tokio::test]
    async fn stale_edit_fingerprint_is_rejected_without_overwriting_external_change() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let state = service.list_providers().unwrap();
        fs::write(&paths.config_file, "model_provider = \"external\"\n").unwrap();

        let error = service
            .create_provider(create_input(&state, false))
            .await
            .unwrap_err();

        assert_eq!(error.code(), "EXTERNAL_MODIFICATION_CONFLICT");
        assert_eq!(
            fs::read_to_string(&paths.config_file).unwrap(),
            "model_provider = \"external\"\n"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn simultaneous_switches_leave_config_and_auth_consistent() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = Arc::new(ProviderService::new(paths.clone(), "0.1.0"));

        let left = {
            let service = service.clone();
            tokio::spawn(async move { service.switch_provider("provider-a").await })
        };
        let right = {
            let service = service.clone();
            tokio::spawn(async move { service.switch_provider("provider-b").await })
        };
        let _ = left.await.unwrap();
        let _ = right.await.unwrap();

        let state = service.list_providers().unwrap();
        let active = state.active_provider_id.unwrap();
        let auth = AuthService::new(paths.auth_file.clone())
            .read_api_key()
            .unwrap()
            .unwrap();
        let expected = ProviderSecretService::new(paths.providers_file.clone())
            .get_api_key_for_edit(&active)
            .unwrap()
            .unwrap();
        assert_eq!(auth, expected);
    }
}
