use crate::error::AppError;
use crate::infrastructure::file_fingerprint::FileSetFingerprint;
use crate::infrastructure::path_service::AppPaths;
use crate::models::backup::{BackupFileName, BackupInventory};
use crate::models::provider::{
    CreateProviderInput, ImportCurrentApiKeyInput, ModelCatalogItem, ProviderApiKeyManagementEntry,
    ProviderApiKeyManagementState, ProviderApiKeyStatus, ProviderApiKeySummary,
    ProviderBaseUrlStatus, ProviderBaseUrlSummary, ProviderListState, ProviderMutationOutcome,
    ProviderProfile, ReorderProvidersInput, SaveProviderApiKeysInput, SaveProviderBaseUrlsInput,
    SelectProviderApiKeyInput, SelectProviderBaseUrlInput, SwitchOutcome, UpdateProviderFastInput,
    UpdateProviderInput, UpdateProviderPreferenceInput, WireApi,
};
use crate::models::provider_availability::ProviderAvailabilityTarget;
use crate::models::transaction::TransactionOperation;
use crate::services::auth_service::{AuthService, render_auth_json};
use crate::services::backup_service::BackupService;
use crate::services::config_service::{
    self, ProviderConfig, ProviderInput, ValidatedProviderInput,
};
use crate::services::provider_preference_service::{
    NamedBaseUrl, ProviderPreference, ProviderPreferenceService, ProviderPreferenceStore,
    ProviderPrivatePreference, model_catalog, normalize_named_base_urls,
    serialize_store as serialize_preference_store,
};
use crate::services::provider_secret_service::{
    NamedApiKey, ProviderSecret, ProviderSecretService, ProviderSecretStore, normalize_api_key,
    normalize_named_api_keys, serialize_store,
};
use crate::services::transaction_service::{
    FileChange, FileChanges, FileOps, StdFileOps, TransactionRequest, TransactionService,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

const CONSISTENT_READ_ATTEMPTS: usize = 3;

#[derive(Clone)]
pub struct ProviderService {
    paths: AppPaths,
    transaction_service: TransactionService,
    backup_service: BackupService,
    secret_service: ProviderSecretService,
    preference_service: ProviderPreferenceService,
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
            preference_service: ProviderPreferenceService::new(
                paths.provider_preferences_file.clone(),
            ),
            auth_service: AuthService::new(paths.auth_file.clone()),
            paths,
            transaction_service,
        }
    }

    pub fn list_backups(&self) -> Result<BackupInventory, AppError> {
        self.backup_service.list_backups()
    }

    pub fn resolve_backup_file(
        &self,
        directory_name: &str,
        file_name: BackupFileName,
    ) -> Result<PathBuf, AppError> {
        self.backup_service
            .resolve_backup_file(directory_name, file_name)
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

    pub fn get_provider_api_keys_for_management(
        &self,
        provider_id: &str,
    ) -> Result<ProviderApiKeyManagementState, AppError> {
        let provider_id = config_service::validate_provider_id(provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        if !disk
            .provider_configs
            .iter()
            .any(|provider| provider.id == provider_id)
        {
            return Err(provider_not_found(&provider_id));
        }
        let secret = disk.store.providers.get(&provider_id);
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(provider_id.as_str());
        let (selected_api_key_id, api_key_status) =
            effective_api_key_selection(is_active, secret, disk.auth_key.as_deref());
        let entries = secret
            .map(|secret| {
                secret
                    .api_keys
                    .iter()
                    .map(|entry| ProviderApiKeyManagementEntry {
                        id: entry.id.clone(),
                        name: entry.name.clone(),
                        api_key: entry.api_key.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(ProviderApiKeyManagementState {
            provider_id,
            entries,
            selected_api_key_id,
            api_key_status,
            fingerprints: disk.fingerprints,
        })
    }

    pub(crate) fn resolve_availability_target(
        &self,
        provider_id: &str,
    ) -> Result<ProviderAvailabilityTarget, AppError> {
        let provider_id = config_service::validate_provider_id(provider_id)?;
        let disk = self.read_consistent_state_read_only(AuthReadMode::Strict)?;
        let provider = disk
            .provider_configs
            .iter()
            .find(|provider| provider.id == provider_id)
            .ok_or_else(|| provider_not_found(&provider_id))?;
        let validated = config_service::validate_provider_config(provider)?;
        let private_preference = disk
            .preference_store
            .providers
            .get(&provider_id)
            .ok_or_else(|| {
                AppError::new(
                    "PROVIDER_TEST_BASE_URL_UNMANAGED",
                    "当前 Base URL 尚未纳入 Relay 管理，无法测试。",
                    format!("availability target has no managed Base URL: {provider_id}"),
                )
            })?;
        if !private_preference
            .base_urls
            .iter()
            .any(|entry| entry.url == validated.base_url)
        {
            return Err(AppError::new(
                "PROVIDER_TEST_BASE_URL_UNMANAGED",
                "当前 Base URL 尚未纳入 Relay 管理，无法测试。",
                format!("availability target Base URL is unmanaged: {provider_id}"),
            ));
        }
        let model = private_preference
            .model_preference
            .as_ref()
            .map(|preference| preference.selected_model.trim())
            .filter(|model| !model.is_empty())
            .ok_or_else(|| {
                AppError::new(
                    "PROVIDER_TEST_MODEL_MISSING",
                    "该 Provider 尚未配置偏好模型，无法测试。",
                    format!("availability target has no selected model: {provider_id}"),
                )
            })?
            .to_owned();
        let secret = disk.store.providers.get(&provider_id);
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(provider_id.as_str());
        let (selected_api_key_id, api_key_status) =
            effective_api_key_selection(is_active, secret, disk.auth_key.as_deref());
        let api_key = match api_key_status {
            ProviderApiKeyStatus::Managed => {
                let selected_api_key_id = selected_api_key_id.expect("managed key has an id");
                secret
                    .and_then(|secret| {
                        secret
                            .api_keys
                            .iter()
                            .find(|entry| entry.id == selected_api_key_id)
                    })
                    .map(|entry| entry.api_key.clone())
                    .ok_or_else(|| {
                        AppError::new(
                            "PROVIDER_TEST_KEY_MISSING",
                            "该 Provider 尚未配置 API Key，无法测试。",
                            format!("availability target has no key: {provider_id}"),
                        )
                    })?
            }
            ProviderApiKeyStatus::External => {
                return Err(AppError::new(
                    "PROVIDER_TEST_KEY_UNMANAGED",
                    "当前 API Key 尚未纳入 Relay 管理，无法测试。",
                    format!("availability target auth key is unmanaged: {provider_id}"),
                ));
            }
            ProviderApiKeyStatus::Missing => {
                return Err(AppError::new(
                    "PROVIDER_TEST_KEY_MISSING",
                    "该 Provider 尚未配置 API Key，无法测试。",
                    format!("availability target has no key: {provider_id}"),
                ));
            }
        };

        Ok(ProviderAvailabilityTarget {
            provider_id,
            base_url: validated.base_url,
            model,
            api_key,
        })
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
        })?;
        let mut preference = ProviderPreference::from_models(&input.models)?;
        preference.set_fast(input.fast_enabled)?;
        let api_key = normalize_api_key(&input.api_key)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        let mut new_config = config_service::create_provider(&disk.config_source, &validated)?;
        let mut new_store = disk.store.clone();
        new_store.providers.insert(
            validated.id.clone(),
            ProviderSecret::single_named(&input.api_key_name, &api_key)?,
        );
        let mut new_preferences = disk.preference_store.clone();
        new_preferences.providers.insert(
            validated.id.clone(),
            ProviderPrivatePreference::with_initial_base_url(
                &input.base_url_name,
                &validated.base_url,
                Some(preference.clone()),
            )?,
        );
        let mut provider_order = ordered_provider_ids(
            &disk.preference_store.provider_order,
            &disk.provider_configs,
        );
        provider_order.push(validated.id.clone());
        new_preferences.provider_order = provider_order;

        let auth_change = if input.activate_after_save {
            new_config = config_service::select_provider_with_preference(
                &new_config,
                &validated.id,
                &preference.selected_model,
                &preference.reasoning_efforts[&preference.selected_model],
                preference.fast_enabled,
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
                        preferences: FileChange::Write(serialize_preference_store(
                            &new_preferences,
                        )?),
                    },
                },
                |paths| {
                    validate_provider_written(
                        paths,
                        &validated,
                        Some(&api_key),
                        Some(&preference),
                        activate,
                    )
                },
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
        let provider_id = config_service::validate_provider_id(&input.id)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        let existing = disk
            .provider_configs
            .iter()
            .find(|provider| provider.id == provider_id)
            .ok_or_else(|| provider_not_found(&provider_id))?;
        let validated = config_service::validate_provider_input(&ProviderInput {
            id: provider_id,
            name: input.name,
            base_url: existing.base_url.clone().unwrap_or_default(),
            wire_api: input.wire_api,
        })?;
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(validated.id.as_str());
        let new_config =
            config_service::update_provider(&disk.config_source, &validated.id, &validated)?;
        let mut new_preferences = disk.preference_store.clone();
        let mut fast_automatically_disabled = false;
        let selected_changed =
            if let Some(private_preference) = new_preferences.providers.get_mut(&validated.id) {
                if let Some(preference) = private_preference.model_preference.as_mut() {
                    let fast_was_enabled = preference.fast_enabled;
                    let selected_changed = preference.reconcile_models(&input.models)?;
                    fast_automatically_disabled = fast_was_enabled && !preference.fast_enabled;
                    selected_changed
                } else {
                    private_preference.model_preference =
                        Some(ProviderPreference::from_models(&input.models)?);
                    false
                }
            } else {
                new_preferences.providers.insert(
                    validated.id.clone(),
                    ProviderPrivatePreference {
                        base_urls: Vec::new(),
                        model_preference: Some(ProviderPreference::from_models(&input.models)?),
                    },
                );
                false
            };
        let preference = new_preferences
            .providers
            .get_mut(&validated.id)
            .expect("model preference container was initialized above")
            .model_preference
            .as_mut()
            .expect("model preference was initialized above");
        preference.set_fast(input.fast_enabled)?;
        let preference = preference.clone();
        let effective_key = configured_key(&disk.store, &validated.id);

        let sync_active = is_active && input.sync_if_active;
        let (final_config, auth_change) = if sync_active {
            let api_key = effective_key
                .as_deref()
                .ok_or_else(provider_api_key_missing)?;
            (
                config_service::select_provider_with_preference(
                    &new_config,
                    &validated.id,
                    &preference.selected_model,
                    &preference.reasoning_efforts[&preference.selected_model],
                    preference.fast_enabled,
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
                        providers: secret_upgrade_change(&disk)?,
                        preferences: FileChange::Write(serialize_preference_store(
                            &new_preferences,
                        )?),
                    },
                },
                |paths| {
                    validate_provider_written(
                        paths,
                        &validated,
                        effective_key.as_deref(),
                        Some(&preference),
                        sync_active,
                    )
                },
            )
            .await?;

        let fallback_note = if selected_changed {
            format!(" 当前偏好模型已改为 {}。", preference.selected_model)
        } else {
            String::new()
        };
        let fast_note = if fast_automatically_disabled {
            " Fast 已因当前模型不支持而自动关闭。"
        } else {
            ""
        };
        let message = if is_active {
            format!(
                "Provider「{}」已更新。{}{}请重启 Codex 后生效。",
                validated.name, fallback_note, fast_note
            )
        } else {
            format!(
                "Provider「{}」已更新。{}{}",
                validated.name, fallback_note, fast_note
            )
        };
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message,
        })
    }

    pub async fn save_provider_base_urls(
        &self,
        input: SaveProviderBaseUrlsInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(&input.provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        let provider = disk
            .provider_configs
            .iter()
            .find(|provider| provider.id == provider_id)
            .ok_or_else(|| provider_not_found(&provider_id))?;
        let validated = config_service::validate_provider_config(provider)?;
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(provider_id.as_str());

        let mut new_preferences = disk.preference_store.clone();
        let private_preference = new_preferences
            .providers
            .entry(provider_id.clone())
            .or_insert_with(|| ProviderPrivatePreference {
                base_urls: Vec::new(),
                model_preference: None,
            });
        let existing = private_preference.base_urls.clone();
        let selected_base_url_id = existing
            .iter()
            .find(|entry| entry.url == validated.base_url)
            .map(|entry| entry.id.clone());

        let mut updates = BTreeMap::new();
        let mut additions = Vec::new();
        for draft in input.entries {
            match draft.id {
                Some(id) => {
                    let id = id.trim().to_owned();
                    if !existing.iter().any(|entry| entry.id == id) {
                        return Err(AppError::new(
                            "UNKNOWN_BASE_URL_ID",
                            "Base URL 条目已变化，请重新加载后再保存。",
                            format!("unknown provider Base URL entry id: {id:?}"),
                        ));
                    }
                    if updates.insert(id, (draft.name, draft.url)).is_some() {
                        return Err(AppError::new(
                            "DUPLICATE_BASE_URL_ID",
                            "Base URL 草稿包含重复条目。",
                            "provider Base URL draft repeats an existing entry id",
                        ));
                    }
                }
                None => additions.push(NamedBaseUrl {
                    id: Uuid::new_v4().to_string(),
                    name: draft.name,
                    url: draft.url,
                }),
            }
        }

        if selected_base_url_id
            .as_ref()
            .is_some_and(|selected_id| !updates.contains_key(selected_id))
        {
            return Err(AppError::new(
                "SELECTED_BASE_URL_DELETE_FORBIDDEN",
                "当前 Base URL 不能直接删除，请先切换到其他地址。",
                "provider Base URL draft removes the selected entry",
            ));
        }

        let mut base_urls = Vec::with_capacity(updates.len() + additions.len());
        for entry in existing {
            if let Some((name, url)) = updates.remove(&entry.id) {
                base_urls.push(NamedBaseUrl {
                    id: entry.id,
                    name,
                    url,
                });
            }
        }
        base_urls.extend(additions);
        if base_urls.is_empty() {
            return Err(AppError::new(
                "LAST_BASE_URL_DELETE_FORBIDDEN",
                "Provider 必须至少保留一个 Base URL。",
                "provider Base URL draft removes the last entry",
            ));
        }
        let base_urls = normalize_named_base_urls(base_urls)?;
        let expected_base_url = selected_base_url_id
            .as_deref()
            .and_then(|selected_id| {
                base_urls
                    .iter()
                    .find(|entry| entry.id == selected_id)
                    .map(|entry| entry.url.clone())
            })
            .unwrap_or_else(|| validated.base_url.clone());
        let config_changed = expected_base_url != validated.base_url;
        let config_change = if config_changed {
            FileChange::Write(
                config_service::set_provider_base_url(
                    &disk.config_source,
                    &provider_id,
                    &expected_base_url,
                )?
                .into_bytes(),
            )
        } else {
            FileChange::Unchanged
        };
        private_preference.base_urls = base_urls.clone();

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::SaveProviderBaseUrls,
                    provider_id: Some(provider_id.clone()),
                    expected_files: Some(input.expected_files),
                    changes: FileChanges {
                        config: config_change,
                        auth: FileChange::Unchanged,
                        providers: secret_upgrade_change(&disk)?,
                        preferences: FileChange::Write(serialize_preference_store(
                            &new_preferences,
                        )?),
                    },
                },
                |paths| {
                    validate_base_urls_written(paths, &provider_id, &base_urls, &expected_base_url)
                },
            )
            .await?;

        let message = if config_changed && is_active {
            "Base URL 已保存并写入当前 Codex 配置，请重启 Codex 后生效。"
        } else if config_changed {
            "Base URL 已保存，将在应用此 Provider 时生效。"
        } else {
            "Base URL 已保存。"
        };
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: message.into(),
        })
    }

    pub async fn select_provider_base_url(
        &self,
        input: SelectProviderBaseUrlInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(&input.provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        let provider = disk
            .provider_configs
            .iter()
            .find(|provider| provider.id == provider_id)
            .ok_or_else(|| provider_not_found(&provider_id))?;
        let validated = config_service::validate_provider_config(provider)?;
        let private_preference = disk
            .preference_store
            .providers
            .get(&provider_id)
            .ok_or_else(|| {
                AppError::new(
                    "PROVIDER_BASE_URLS_MISSING",
                    "该 Provider 尚未保存命名 Base URL。",
                    "provider has no private Base URL preference",
                )
            })?;
        let selected = private_preference
            .base_urls
            .iter()
            .find(|entry| entry.id == input.base_url_id)
            .ok_or_else(|| {
                AppError::new(
                    "BASE_URL_NOT_FOUND",
                    "指定的 Base URL 不存在，请重新加载后再试。",
                    format!("provider Base URL id not found: {:?}", input.base_url_id),
                )
            })?;
        let config_changed = validated.base_url != selected.url;
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(provider_id.as_str());

        if config_changed || disk.preference_needs_upgrade || disk.secret_needs_upgrade {
            let config_change = if config_changed {
                FileChange::Write(
                    config_service::set_provider_base_url(
                        &disk.config_source,
                        &provider_id,
                        &selected.url,
                    )?
                    .into_bytes(),
                )
            } else {
                FileChange::Unchanged
            };
            let preference_change = if disk.preference_needs_upgrade {
                FileChange::Write(serialize_preference_store(&disk.preference_store)?)
            } else {
                FileChange::Unchanged
            };
            let expected_entries = private_preference.base_urls.clone();
            let expected_base_url = selected.url.clone();
            self.transaction_service
                .execute(
                    TransactionRequest {
                        operation: TransactionOperation::SelectProviderBaseUrl,
                        provider_id: Some(provider_id.clone()),
                        expected_files: Some(input.expected_files),
                        changes: FileChanges {
                            config: config_change,
                            auth: FileChange::Unchanged,
                            providers: secret_upgrade_change(&disk)?,
                            preferences: preference_change,
                        },
                    },
                    |paths| {
                        validate_base_urls_written(
                            paths,
                            &provider_id,
                            &expected_entries,
                            &expected_base_url,
                        )
                    },
                )
                .await?;
        }

        let message = if config_changed && is_active {
            "Base URL 已写入当前 Codex 配置，请重启 Codex 后生效。"
        } else if config_changed {
            "Base URL 已保存，将在应用此 Provider 时生效。"
        } else {
            "该 Base URL 已处于选中状态。"
        };
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: message.into(),
        })
    }

    pub async fn save_provider_api_keys(
        &self,
        input: SaveProviderApiKeysInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(&input.provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Strict)?;
        if !disk
            .provider_configs
            .iter()
            .any(|provider| provider.id == provider_id)
        {
            return Err(provider_not_found(&provider_id));
        }
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(provider_id.as_str());
        let existing_secret = disk.store.providers.get(&provider_id).cloned();
        let existing = existing_secret
            .as_ref()
            .map(|secret| secret.api_keys.clone())
            .unwrap_or_default();
        let active_managed_id = if is_active {
            disk.auth_key.as_deref().and_then(|auth_key| {
                existing
                    .iter()
                    .find(|entry| entry.api_key == auth_key)
                    .map(|entry| entry.id.clone())
            })
        } else {
            None
        };
        let protected_selected_id = active_managed_id.clone().or_else(|| {
            existing_secret
                .as_ref()
                .map(|secret| secret.selected_api_key_id.clone())
        });

        let mut updates = BTreeMap::new();
        let mut additions = Vec::new();
        for draft in input.entries {
            match draft.id {
                Some(id) => {
                    let id = id.trim().to_owned();
                    if !existing.iter().any(|entry| entry.id == id) {
                        return Err(AppError::new(
                            "UNKNOWN_API_KEY_ID",
                            "API Key 条目已变化，请重新加载后再保存。",
                            format!("unknown provider API key entry id: {id:?}"),
                        ));
                    }
                    if updates.insert(id, (draft.name, draft.api_key)).is_some() {
                        return Err(AppError::new(
                            "DUPLICATE_API_KEY_ID",
                            "API Key 草稿包含重复条目。",
                            "provider API key draft repeats an existing entry id",
                        ));
                    }
                }
                None => additions.push(NamedApiKey {
                    id: Uuid::new_v4().to_string(),
                    name: draft.name,
                    api_key: draft.api_key,
                }),
            }
        }

        if protected_selected_id
            .as_ref()
            .is_some_and(|selected_id| !updates.contains_key(selected_id))
        {
            return Err(AppError::new(
                "SELECTED_API_KEY_DELETE_FORBIDDEN",
                "当前 API Key 不能直接删除，请先切换到其他密钥。",
                "provider API key draft removes the selected entry",
            ));
        }

        let mut api_keys = Vec::with_capacity(updates.len() + additions.len());
        for entry in existing {
            if let Some((name, api_key)) = updates.remove(&entry.id) {
                api_keys.push(NamedApiKey {
                    id: entry.id,
                    name,
                    api_key,
                });
            }
        }
        api_keys.extend(additions);
        if api_keys.is_empty() {
            return Err(AppError::new(
                "LAST_API_KEY_DELETE_FORBIDDEN",
                "Provider 必须至少保留一个 API Key。",
                "provider API key draft removes the last entry",
            ));
        }
        let selected_api_key_id = protected_selected_id.unwrap_or_else(|| api_keys[0].id.clone());
        let normalized = normalize_named_api_keys(api_keys, &selected_api_key_id)?;
        let new_secret = ProviderSecret {
            api_keys: normalized.api_keys,
            selected_api_key_id: normalized.selected_api_key_id,
        };
        let expected_auth_key = active_managed_id
            .as_deref()
            .and_then(|selected_id| {
                new_secret
                    .api_keys
                    .iter()
                    .find(|entry| entry.id == selected_id)
                    .map(|entry| entry.api_key.clone())
            })
            .or_else(|| disk.auth_key.clone());
        let auth_changed = expected_auth_key != disk.auth_key;

        let mut new_store = disk.store.clone();
        new_store
            .providers
            .insert(provider_id.clone(), new_secret.clone());
        let preference_change = if disk.preference_needs_upgrade {
            FileChange::Write(serialize_preference_store(&disk.preference_store)?)
        } else {
            FileChange::Unchanged
        };
        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::SaveProviderApiKeys,
                    provider_id: Some(provider_id.clone()),
                    expected_files: Some(input.expected_files),
                    changes: FileChanges {
                        config: FileChange::Unchanged,
                        auth: if auth_changed {
                            match expected_auth_key.as_deref() {
                                Some(api_key) => FileChange::Write(render_auth_json(api_key)?),
                                None => FileChange::Delete,
                            }
                        } else {
                            FileChange::Unchanged
                        },
                        providers: FileChange::Write(serialize_store(&new_store)?),
                        preferences: preference_change,
                    },
                },
                |paths| {
                    validate_api_keys_written(
                        paths,
                        &provider_id,
                        &new_secret,
                        expected_auth_key.as_deref(),
                    )
                },
            )
            .await?;

        let message = if auth_changed && is_active {
            "API Key 已保存并写入当前 Codex 认证，请重启 Codex 后生效。"
        } else if is_active {
            "API Key 已保存。"
        } else {
            "API Key 已保存，将在应用此 Provider 时生效。"
        };
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: message.into(),
        })
    }

    pub async fn select_provider_api_key(
        &self,
        input: SelectProviderApiKeyInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(&input.provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Strict)?;
        if !disk
            .provider_configs
            .iter()
            .any(|provider| provider.id == provider_id)
        {
            return Err(provider_not_found(&provider_id));
        }
        let secret = disk
            .store
            .providers
            .get(&provider_id)
            .cloned()
            .ok_or_else(provider_api_key_missing)?;
        let selected = secret
            .api_keys
            .iter()
            .find(|entry| entry.id == input.api_key_id)
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "API_KEY_NOT_FOUND",
                    "指定的 API Key 不存在，请重新加载后再试。",
                    format!("provider API key id not found: {:?}", input.api_key_id),
                )
            })?;
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(provider_id.as_str());
        let selected_changed = secret.selected_api_key_id != selected.id;
        let expected_auth_key = if is_active {
            Some(selected.api_key.clone())
        } else {
            disk.auth_key.clone()
        };
        let auth_changed = expected_auth_key != disk.auth_key;

        if selected_changed
            || auth_changed
            || disk.secret_needs_upgrade
            || disk.preference_needs_upgrade
        {
            let new_secret =
                ProviderSecret::from_named_api_keys(secret.api_keys.clone(), &selected.id)?;
            let mut new_store = disk.store.clone();
            new_store
                .providers
                .insert(provider_id.clone(), new_secret.clone());
            self.transaction_service
                .execute(
                    TransactionRequest {
                        operation: TransactionOperation::SelectProviderApiKey,
                        provider_id: Some(provider_id.clone()),
                        expected_files: Some(input.expected_files),
                        changes: FileChanges {
                            config: FileChange::Unchanged,
                            auth: if auth_changed {
                                FileChange::Write(render_auth_json(&selected.api_key)?)
                            } else {
                                FileChange::Unchanged
                            },
                            providers: FileChange::Write(serialize_store(&new_store)?),
                            preferences: preference_upgrade_change(&disk)?,
                        },
                    },
                    |paths| {
                        validate_api_keys_written(
                            paths,
                            &provider_id,
                            &new_secret,
                            expected_auth_key.as_deref(),
                        )
                    },
                )
                .await?;
        }

        let message = if auth_changed && is_active {
            "API Key 已写入当前 Codex 认证，请重启 Codex 后生效。"
        } else if selected_changed && !is_active {
            "API Key 已保存，将在应用此 Provider 时生效。"
        } else {
            "该 API Key 已处于选中状态。"
        };
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: message.into(),
        })
    }

    pub async fn delete_provider(
        &self,
        provider_id: &str,
        expected_files: FileSetFingerprint,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
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
        let mut new_preferences = disk.preference_store.clone();
        new_preferences.providers.remove(&provider_id);
        new_preferences
            .provider_order
            .retain(|id| id != &provider_id);

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
                        preferences: FileChange::Write(serialize_preference_store(
                            &new_preferences,
                        )?),
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

    pub async fn reorder_providers(
        &self,
        input: ReorderProvidersInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        let provider_ids = validate_provider_order(&input.provider_ids, &disk.provider_configs)?;
        let mut new_preferences = disk.preference_store.clone();
        new_preferences.provider_order = provider_ids.clone();
        let expected_files = input.expected_files;

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::ReorderProviders,
                    provider_id: None,
                    expected_files: Some(expected_files),
                    changes: FileChanges {
                        config: FileChange::Unchanged,
                        auth: FileChange::Unchanged,
                        providers: FileChange::Unchanged,
                        preferences: FileChange::Write(serialize_preference_store(
                            &new_preferences,
                        )?),
                    },
                },
                |paths| validate_provider_order_written(paths, &provider_ids),
            )
            .await?;

        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: "Provider 顺序已保存。".into(),
        })
    }

    pub async fn switch_provider(&self, provider_id: &str) -> Result<SwitchOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        let provider = disk
            .provider_configs
            .iter()
            .find(|provider| provider.id == provider_id)
            .cloned()
            .ok_or_else(|| provider_not_found(&provider_id))?;
        let validated = config_service::validate_provider_config(&provider)?;
        let api_key =
            configured_key(&disk.store, &provider_id).ok_or_else(provider_api_key_missing)?;
        let private_preference = disk
            .preference_store
            .providers
            .get(&provider_id)
            .ok_or_else(provider_preference_missing)?;
        if !private_preference
            .base_urls
            .iter()
            .any(|entry| entry.url == validated.base_url)
        {
            return Err(AppError::new(
                "PROVIDER_BASE_URL_UNMANAGED",
                "当前 Base URL 尚未纳入 Relay 管理，无法应用该 Provider。",
                "target provider Base URL does not match a managed entry",
            ));
        }
        let preference = private_preference
            .model_preference
            .as_ref()
            .ok_or_else(provider_preference_missing)?;
        let new_config = config_service::select_provider_with_preference(
            &disk.config_source,
            &provider_id,
            &preference.selected_model,
            &preference.reasoning_efforts[&preference.selected_model],
            preference.fast_enabled,
        )?;

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::SwitchProvider,
                    provider_id: Some(provider_id.clone()),
                    expected_files: Some(disk.fingerprints),
                    changes: FileChanges {
                        config: FileChange::Write(new_config.into_bytes()),
                        auth: FileChange::Write(render_auth_json(&api_key)?),
                        providers: if disk.secret_needs_upgrade {
                            FileChange::Write(serialize_store(&disk.store)?)
                        } else {
                            FileChange::Unchanged
                        },
                        preferences: if disk.preference_needs_upgrade {
                            FileChange::Write(serialize_preference_store(&disk.preference_store)?)
                        } else {
                            FileChange::Unchanged
                        },
                    },
                },
                |paths| {
                    validate_provider_written(
                        paths,
                        &validated,
                        Some(&api_key),
                        Some(preference),
                        true,
                    )
                },
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

    pub async fn update_provider_preference(
        &self,
        input: UpdateProviderPreferenceInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(&input.provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        if !disk
            .provider_configs
            .iter()
            .any(|provider| provider.id == provider_id)
        {
            return Err(provider_not_found(&provider_id));
        }
        let mut new_preferences = disk.preference_store.clone();
        let private_preference = new_preferences
            .providers
            .get_mut(&provider_id)
            .ok_or_else(provider_preference_missing)?;
        let preference = private_preference
            .model_preference
            .as_mut()
            .ok_or_else(provider_preference_missing)?;
        let fast_automatically_disabled =
            preference.select(&input.model, &input.reasoning_effort)?;
        let expected_preference = preference.clone();
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(provider_id.as_str());
        let config_change = if is_active {
            FileChange::Write(
                config_service::select_provider_with_preference(
                    &disk.config_source,
                    &provider_id,
                    &expected_preference.selected_model,
                    &expected_preference.reasoning_efforts[&expected_preference.selected_model],
                    expected_preference.fast_enabled,
                )?
                .into_bytes(),
            )
        } else {
            FileChange::Unchanged
        };

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::UpdateProviderPreference,
                    provider_id: Some(provider_id.clone()),
                    expected_files: Some(input.expected_files),
                    changes: FileChanges {
                        config: config_change,
                        auth: FileChange::Unchanged,
                        providers: secret_upgrade_change(&disk)?,
                        preferences: FileChange::Write(serialize_preference_store(
                            &new_preferences,
                        )?),
                    },
                },
                |paths| {
                    validate_preference_written(
                        paths,
                        &provider_id,
                        &expected_preference,
                        is_active,
                    )
                },
            )
            .await?;

        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: match (is_active, fast_automatically_disabled) {
                (true, true) => "模型偏好已写入当前 Codex 配置。Fast 已因当前模型不支持而自动关闭。请重启 Codex 后生效。".into(),
                (true, false) => "模型偏好已写入当前 Codex 配置，请重启 Codex 后生效。".into(),
                (false, true) => "模型偏好已保存。Fast 已因当前模型不支持而自动关闭，将在应用此 Provider 时生效。".into(),
                (false, false) => "模型偏好已保存，将在应用此 Provider 时生效。".into(),
            },
        })
    }

    pub async fn update_provider_fast(
        &self,
        input: UpdateProviderFastInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let provider_id = config_service::validate_provider_id(&input.provider_id)?;
        let disk = self.read_consistent_state(AuthReadMode::Lenient)?;
        if !disk
            .provider_configs
            .iter()
            .any(|provider| provider.id == provider_id)
        {
            return Err(provider_not_found(&provider_id));
        }

        let mut new_preferences = disk.preference_store.clone();
        let preference = new_preferences
            .providers
            .get_mut(&provider_id)
            .ok_or_else(provider_preference_missing)?
            .model_preference
            .as_mut()
            .ok_or_else(provider_preference_missing)?;
        preference.set_fast(input.enabled)?;
        let expected_preference = preference.clone();
        let is_active = config_service::current_provider_id(&disk.document).as_deref()
            == Some(provider_id.as_str());
        let config_change = if is_active {
            FileChange::Write(
                config_service::select_provider_with_preference(
                    &disk.config_source,
                    &provider_id,
                    &expected_preference.selected_model,
                    &expected_preference.reasoning_efforts[&expected_preference.selected_model],
                    expected_preference.fast_enabled,
                )?
                .into_bytes(),
            )
        } else {
            FileChange::Unchanged
        };

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::UpdateProviderFast,
                    provider_id: Some(provider_id.clone()),
                    expected_files: Some(input.expected_files),
                    changes: FileChanges {
                        config: config_change,
                        auth: FileChange::Unchanged,
                        providers: secret_upgrade_change(&disk)?,
                        preferences: FileChange::Write(serialize_preference_store(
                            &new_preferences,
                        )?),
                    },
                },
                |paths| {
                    validate_preference_written(
                        paths,
                        &provider_id,
                        &expected_preference,
                        is_active,
                    )
                },
            )
            .await?;

        let state = if input.enabled { "开启" } else { "关闭" };
        Ok(ProviderMutationOutcome {
            providers: self.list_providers()?.providers,
            message: if is_active {
                format!("Fast 已{state}，已写入当前 Codex 配置，请重启 Codex 后生效。")
            } else {
                format!("Fast 偏好已{state}，将在应用此 Provider 时生效。")
            },
        })
    }

    pub async fn import_current_auth_key(
        &self,
        input: ImportCurrentApiKeyInput,
    ) -> Result<ProviderMutationOutcome, AppError> {
        let expected_provider_id = config_service::validate_provider_id(&input.provider_id)?;
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
        let preference_change = preference_upgrade_change(&disk)?;
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
        let mut api_keys = disk
            .store
            .providers
            .get(&active)
            .map(|secret| secret.api_keys.clone())
            .unwrap_or_default();
        let imported_id = Uuid::new_v4().to_string();
        api_keys.push(NamedApiKey {
            id: imported_id.clone(),
            name: input.name,
            api_key: api_key.clone(),
        });
        let imported = ProviderSecret::from_named_api_keys(api_keys, &imported_id)?;
        let mut new_store = disk.store;
        new_store.providers.insert(active.clone(), imported.clone());

        self.transaction_service
            .execute(
                TransactionRequest {
                    operation: TransactionOperation::ImportCurrentApiKey,
                    provider_id: Some(active.clone()),
                    expected_files: Some(input.expected_files),
                    changes: FileChanges {
                        config: FileChange::Unchanged,
                        auth: FileChange::Unchanged,
                        providers: FileChange::Write(serialize_store(&new_store)?),
                        preferences: preference_change,
                    },
                },
                |paths| validate_api_keys_written(paths, &active, &imported, Some(&api_key)),
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
        self.read_consistent_state_with_secret_mode(auth_mode, SecretReadMode::PreserveCorrupt)
    }

    fn read_consistent_state_read_only(
        &self,
        auth_mode: AuthReadMode,
    ) -> Result<DiskState, AppError> {
        self.read_consistent_state_with_secret_mode(auth_mode, SecretReadMode::ReadOnly)
    }

    fn read_consistent_state_with_secret_mode(
        &self,
        auth_mode: AuthReadMode,
        secret_mode: SecretReadMode,
    ) -> Result<DiskState, AppError> {
        for _ in 0..CONSISTENT_READ_ATTEMPTS {
            let before = self.current_fingerprints()?;
            let config_source = read_optional_utf8(&self.paths.config_file)?.unwrap_or_default();
            let document = config_service::parse_document(&config_source)?;
            let provider_configs = config_service::list_provider_configs(&document)?;
            let loaded_store = match secret_mode {
                SecretReadMode::PreserveCorrupt => self.secret_service.load_versioned()?,
                SecretReadMode::ReadOnly => self.secret_service.load_read_only_versioned()?,
            };
            let mut secret_needs_upgrade = loaded_store.needs_upgrade;
            let mut store = loaded_store.store;
            let mut loaded_preferences = self.preference_service.load_versioned()?;
            let preference_needs_upgrade = loaded_preferences.needs_upgrade;
            if loaded_preferences.needs_upgrade {
                loaded_preferences.store.providers.retain(|provider_id, _| {
                    provider_configs
                        .iter()
                        .any(|provider| provider.id == *provider_id)
                });
                for (provider_id, preference) in &mut loaded_preferences.store.providers {
                    let Some(base_url) = provider_configs
                        .iter()
                        .find(|provider| provider.id == *provider_id)
                        .and_then(|provider| provider.base_url.as_deref())
                    else {
                        continue;
                    };
                    preference.hydrate_legacy_base_url(base_url)?;
                }
            }
            let preference_store = loaded_preferences.store;
            let auth_key = match auth_mode {
                AuthReadMode::Lenient => self.auth_service.read_api_key().ok().flatten(),
                AuthReadMode::Strict => self.auth_service.read_api_key()?,
            };
            if let (Some(active_provider_id), Some(auth_key)) = (
                config_service::current_provider_id(&document),
                auth_key.as_deref(),
            ) && let Some(secret) = store.providers.get_mut(&active_provider_id)
            {
                let effective_id = secret
                    .api_keys
                    .iter()
                    .find(|entry| entry.api_key == auth_key)
                    .map(|entry| entry.id.clone());
                if let Some(effective_id) = effective_id
                    && secret.selected_api_key_id != effective_id
                {
                    secret.selected_api_key_id = effective_id;
                    secret_needs_upgrade = true;
                }
            }
            let after = self.current_fingerprints()?;
            if before == after {
                return Ok(DiskState {
                    config_source,
                    document,
                    provider_configs,
                    store,
                    secret_needs_upgrade,
                    preference_store,
                    preference_needs_upgrade,
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
            &self.paths.provider_preferences_file,
        )
    }

    fn list_state_from_disk(&self, disk: &DiskState) -> ProviderListState {
        let active_provider_id = config_service::current_provider_id(&disk.document);
        let ordered_configs = ordered_provider_configs(
            &disk.preference_store.provider_order,
            &disk.provider_configs,
        );
        let providers: Vec<ProviderProfile> = ordered_configs
            .into_iter()
            .map(|provider| {
                profile_from_config(
                    provider,
                    active_provider_id.as_deref(),
                    configured_model_preference(&disk.preference_store, &provider.id),
                    disk.preference_store.providers.get(&provider.id),
                    disk.store.providers.get(&provider.id),
                    disk.auth_key.as_deref(),
                )
            })
            .collect();
        let current_auth_import_available = providers.iter().any(|provider: &ProviderProfile| {
            provider.is_active && provider.api_key_status == ProviderApiKeyStatus::External
        });

        ProviderListState {
            providers,
            active_provider_id,
            current_auth_import_available,
            fingerprints: disk.fingerprints.clone(),
            model_catalog: model_catalog()
                .iter()
                .map(|entry| ModelCatalogItem {
                    id: entry.id.into(),
                    reasoning_efforts: entry
                        .reasoning_efforts
                        .iter()
                        .map(|effort| (*effort).into())
                        .collect(),
                    default_reasoning_effort: entry.default_reasoning_effort.into(),
                    supports_fast: entry.supports_fast,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy)]
enum AuthReadMode {
    Lenient,
    Strict,
}

#[derive(Clone, Copy)]
enum SecretReadMode {
    PreserveCorrupt,
    ReadOnly,
}

struct DiskState {
    config_source: String,
    document: toml_edit::DocumentMut,
    provider_configs: Vec<ProviderConfig>,
    store: ProviderSecretStore,
    secret_needs_upgrade: bool,
    preference_store: ProviderPreferenceStore,
    preference_needs_upgrade: bool,
    auth_key: Option<String>,
    fingerprints: FileSetFingerprint,
}

fn secret_upgrade_change(disk: &DiskState) -> Result<FileChange, AppError> {
    if disk.secret_needs_upgrade {
        Ok(FileChange::Write(serialize_store(&disk.store)?))
    } else {
        Ok(FileChange::Unchanged)
    }
}

fn preference_upgrade_change(disk: &DiskState) -> Result<FileChange, AppError> {
    if disk.preference_needs_upgrade {
        Ok(FileChange::Write(serialize_preference_store(
            &disk.preference_store,
        )?))
    } else {
        Ok(FileChange::Unchanged)
    }
}

fn profile_from_config(
    provider: &ProviderConfig,
    active_provider_id: Option<&str>,
    preference: Option<&ProviderPreference>,
    private_preference: Option<&ProviderPrivatePreference>,
    secret: Option<&ProviderSecret>,
    auth_key: Option<&str>,
) -> ProviderProfile {
    let is_active = active_provider_id == Some(provider.id.as_str());
    let projection =
        project_provider_selection(provider, is_active, private_preference, secret, auth_key);
    match config_service::validate_provider_config(provider) {
        Ok(validated) => build_provider_profile(
            ProviderProfileBasics {
                id: validated.id,
                name: validated.name,
                base_url: validated.base_url,
                is_valid: true,
                validation_message: None,
            },
            preference,
            projection,
            is_active,
        ),
        Err(error) => {
            let validation_message = error.public_message().to_owned();
            build_provider_profile(
                ProviderProfileBasics {
                    id: provider.id.clone(),
                    name: provider_display_name(provider),
                    base_url: provider.base_url.clone().unwrap_or_default(),
                    is_valid: false,
                    validation_message: Some(validation_message),
                },
                preference,
                projection,
                is_active,
            )
        }
    }
}

struct ProviderSelectionProjection {
    base_urls: Vec<ProviderBaseUrlSummary>,
    selected_base_url_id: Option<String>,
    base_url_status: ProviderBaseUrlStatus,
    api_keys: Vec<ProviderApiKeySummary>,
    selected_api_key_id: Option<String>,
    api_key_status: ProviderApiKeyStatus,
}

struct ProviderProfileBasics {
    id: String,
    name: String,
    base_url: String,
    is_valid: bool,
    validation_message: Option<String>,
}

fn project_provider_selection(
    provider: &ProviderConfig,
    is_active: bool,
    private_preference: Option<&ProviderPrivatePreference>,
    secret: Option<&ProviderSecret>,
    auth_key: Option<&str>,
) -> ProviderSelectionProjection {
    let base_urls = private_preference
        .map(|preference| {
            preference
                .base_urls
                .iter()
                .map(|entry| ProviderBaseUrlSummary {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                    url: entry.url.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    let normalized_base_url = provider
        .base_url
        .as_deref()
        .and_then(|base_url| config_service::normalize_base_url(base_url).ok());
    let selected_base_url_id = private_preference
        .and_then(|preference| {
            preference
                .base_urls
                .iter()
                .find(|entry| normalized_base_url.as_deref() == Some(entry.url.as_str()))
        })
        .map(|entry| entry.id.clone());
    let base_url_status = if selected_base_url_id.is_some() {
        ProviderBaseUrlStatus::Managed
    } else {
        ProviderBaseUrlStatus::External
    };

    let api_keys = secret
        .map(|secret| {
            secret
                .api_keys
                .iter()
                .map(|entry| ProviderApiKeySummary {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    let (selected_api_key_id, api_key_status) =
        effective_api_key_selection(is_active, secret, auth_key);

    ProviderSelectionProjection {
        base_urls,
        selected_base_url_id,
        base_url_status,
        api_keys,
        selected_api_key_id,
        api_key_status,
    }
}

fn effective_api_key_selection(
    is_active: bool,
    secret: Option<&ProviderSecret>,
    auth_key: Option<&str>,
) -> (Option<String>, ProviderApiKeyStatus) {
    if is_active {
        match auth_key {
            Some(auth_key) => match secret.and_then(|secret| {
                secret
                    .api_keys
                    .iter()
                    .find(|entry| entry.api_key == auth_key)
            }) {
                Some(entry) => (Some(entry.id.clone()), ProviderApiKeyStatus::Managed),
                None => (None, ProviderApiKeyStatus::External),
            },
            None => (None, ProviderApiKeyStatus::Missing),
        }
    } else {
        match secret.and_then(|secret| {
            secret
                .api_keys
                .iter()
                .find(|entry| entry.id == secret.selected_api_key_id)
        }) {
            Some(entry) => (Some(entry.id.clone()), ProviderApiKeyStatus::Managed),
            None => (None, ProviderApiKeyStatus::Missing),
        }
    }
}

fn build_provider_profile(
    basics: ProviderProfileBasics,
    preference: Option<&ProviderPreference>,
    projection: ProviderSelectionProjection,
    is_active: bool,
) -> ProviderProfile {
    let preference_configured = preference.is_some();
    let api_key_configured = !projection.api_keys.is_empty();
    let disabled_reason = if !basics.is_valid {
        basics.validation_message.clone()
    } else if projection.base_url_status != ProviderBaseUrlStatus::Managed {
        Some("当前 Base URL 尚未纳入 Relay 管理。".into())
    } else if projection.api_key_status == ProviderApiKeyStatus::External {
        Some("当前 API Key 尚未纳入 Relay 管理。".into())
    } else if projection.api_key_status == ProviderApiKeyStatus::Missing {
        Some("尚未配置 API Key。".into())
    } else if !preference_configured {
        Some("尚未配置可用模型。".into())
    } else {
        None
    };
    let configuration_complete = disabled_reason.is_none();

    ProviderProfile {
        id: basics.id,
        name: basics.name,
        base_url: basics.base_url,
        base_urls: projection.base_urls,
        selected_base_url_id: projection.selected_base_url_id,
        base_url_status: projection.base_url_status,
        api_keys: projection.api_keys,
        selected_api_key_id: projection.selected_api_key_id,
        api_key_status: projection.api_key_status,
        wire_api: WireApi::Responses,
        models: preference
            .map(|value| value.models.clone())
            .unwrap_or_default(),
        selected_model: preference.map(|value| value.selected_model.clone()),
        reasoning_efforts: preference
            .map(|value| value.reasoning_efforts.clone())
            .unwrap_or_default(),
        fast_enabled: preference.map(|value| value.fast_enabled).unwrap_or(false),
        preference_configured,
        api_key_configured,
        configuration_complete,
        disabled_reason,
        is_active,
        is_valid: basics.is_valid,
        validation_message: basics.validation_message,
    }
}

fn configured_key(store: &ProviderSecretStore, provider_id: &str) -> Option<String> {
    store
        .providers
        .get(provider_id)
        .and_then(ProviderSecret::selected_api_key)
        .map(str::to_owned)
}

fn ordered_provider_configs<'a>(
    provider_order: &[String],
    provider_configs: &'a [ProviderConfig],
) -> Vec<&'a ProviderConfig> {
    let mut seen = BTreeSet::new();
    let mut ordered = Vec::with_capacity(provider_configs.len());
    for provider_id in provider_order {
        if let Some(provider) = provider_configs
            .iter()
            .find(|provider| provider.id == *provider_id)
            && seen.insert(provider.id.as_str())
        {
            ordered.push(provider);
        }
    }
    for provider in provider_configs {
        if seen.insert(provider.id.as_str()) {
            ordered.push(provider);
        }
    }
    ordered
}

fn ordered_provider_ids(
    provider_order: &[String],
    provider_configs: &[ProviderConfig],
) -> Vec<String> {
    ordered_provider_configs(provider_order, provider_configs)
        .into_iter()
        .map(|provider| provider.id.clone())
        .collect()
}

fn validate_provider_order(
    requested: &[String],
    provider_configs: &[ProviderConfig],
) -> Result<Vec<String>, AppError> {
    let current_ids = provider_configs
        .iter()
        .map(|provider| provider.id.clone())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for provider_id in requested {
        let normalized = match config_service::validate_provider_id(provider_id) {
            Ok(normalized) => normalized,
            Err(_) => return Err(invalid_provider_order()),
        };
        if normalized != *provider_id || !seen.insert(provider_id.clone()) {
            return Err(invalid_provider_order());
        }
    }
    if seen.len() != current_ids.len() || seen != current_ids {
        return Err(invalid_provider_order());
    }
    Ok(requested.to_vec())
}

fn invalid_provider_order() -> AppError {
    AppError::new(
        "INVALID_PROVIDER_ORDER",
        "Provider 排序已变化，请刷新后重试。",
        "provider order is not an exact permutation of current providers",
    )
}

fn configured_model_preference<'a>(
    store: &'a ProviderPreferenceStore,
    provider_id: &str,
) -> Option<&'a ProviderPreference> {
    store
        .providers
        .get(provider_id)
        .and_then(|preference| preference.model_preference.as_ref())
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
    expected_preference: Option<&ProviderPreference>,
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

    let store = ProviderSecretService::new(paths.providers_file.clone()).load_read_only()?;
    let actual_key = configured_key(&store, &expected.id);
    if actual_key.as_deref() != expected_key {
        return Err(post_write_validation_error(
            "provider secret state does not match expected value",
        ));
    }

    if let Some(expected_preference) = expected_preference {
        validate_preference_written(
            paths,
            &expected.id,
            expected_preference,
            require_active_auth,
        )?;
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
    let store = ProviderSecretService::new(paths.providers_file.clone()).load_read_only()?;
    if store.providers.contains_key(provider_id) {
        return Err(post_write_validation_error(
            "deleted provider still exists in providers.json",
        ));
    }
    let preferences =
        ProviderPreferenceService::new(paths.provider_preferences_file.clone()).load()?;
    if preferences.providers.contains_key(provider_id) {
        return Err(post_write_validation_error(
            "deleted provider still exists in provider-preferences.json",
        ));
    }
    if preferences
        .provider_order
        .iter()
        .any(|id| id == provider_id)
    {
        return Err(post_write_validation_error(
            "deleted provider still exists in provider order",
        ));
    }
    Ok(())
}

fn validate_provider_order_written(
    paths: &AppPaths,
    expected_order: &[String],
) -> Result<(), AppError> {
    let preferences =
        ProviderPreferenceService::new(paths.provider_preferences_file.clone()).load()?;
    if preferences.provider_order != expected_order {
        return Err(post_write_validation_error(
            "provider order does not match expected values",
        ));
    }
    Ok(())
}

fn validate_preference_written(
    paths: &AppPaths,
    provider_id: &str,
    expected: &ProviderPreference,
    require_active_config: bool,
) -> Result<(), AppError> {
    let store = ProviderPreferenceService::new(paths.provider_preferences_file.clone()).load()?;
    if configured_model_preference(&store, provider_id) != Some(expected) {
        return Err(post_write_validation_error(
            "provider preference does not match expected value",
        ));
    }
    if require_active_config {
        let source = read_required_utf8(&paths.config_file)?;
        let document = config_service::parse_document(&source)?;
        let model = document.get("model").and_then(toml_edit::Item::as_str);
        let effort = document
            .get("model_reasoning_effort")
            .and_then(toml_edit::Item::as_str);
        let service_tier = document
            .get("service_tier")
            .and_then(toml_edit::Item::as_str);
        let fast_mode = document
            .get("features")
            .and_then(toml_edit::Item::as_table_like)
            .and_then(|features| features.get("fast_mode"))
            .and_then(toml_edit::Item::as_bool);
        if config_service::current_provider_id(&document).as_deref() != Some(provider_id)
            || model != Some(expected.selected_model.as_str())
            || effort
                != expected
                    .reasoning_efforts
                    .get(&expected.selected_model)
                    .map(String::as_str)
            || (expected.fast_enabled && (service_tier != Some("fast") || fast_mode != Some(true)))
            || (!expected.fast_enabled && service_tier.is_some())
        {
            return Err(post_write_validation_error(
                "top-level model preference does not match expected value",
            ));
        }
    }
    Ok(())
}

fn validate_base_urls_written(
    paths: &AppPaths,
    provider_id: &str,
    expected_entries: &[NamedBaseUrl],
    expected_base_url: &str,
) -> Result<(), AppError> {
    let store = ProviderPreferenceService::new(paths.provider_preferences_file.clone()).load()?;
    let actual_entries = store
        .providers
        .get(provider_id)
        .map(|preference| preference.base_urls.as_slice());
    if actual_entries != Some(expected_entries) {
        return Err(post_write_validation_error(
            "provider Base URL entries do not match expected values",
        ));
    }

    let source = read_required_utf8(&paths.config_file)?;
    let document = config_service::parse_document(&source)?;
    let provider = config_service::list_provider_configs(&document)?
        .into_iter()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| post_write_validation_error("provider table is missing"))?;
    let actual = config_service::validate_provider_config(&provider)?;
    if actual.base_url != expected_base_url {
        return Err(post_write_validation_error(
            "provider Base URL selection does not match expected value",
        ));
    }
    Ok(())
}

fn validate_api_keys_written(
    paths: &AppPaths,
    provider_id: &str,
    expected_secret: &ProviderSecret,
    expected_auth_key: Option<&str>,
) -> Result<(), AppError> {
    let store = ProviderSecretService::new(paths.providers_file.clone()).load_read_only()?;
    if store.providers.get(provider_id) != Some(expected_secret) {
        return Err(post_write_validation_error(
            "provider API Key entries do not match expected values",
        ));
    }
    let actual_auth_key = AuthService::new(paths.auth_file.clone()).read_api_key()?;
    if actual_auth_key.as_deref() != expected_auth_key {
        return Err(post_write_validation_error(
            "auth.json key does not match expected API Key state",
        ));
    }
    Ok(())
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

fn provider_preference_missing() -> AppError {
    AppError::new(
        "PROVIDER_PREFERENCE_MISSING",
        "该 Provider 尚未配置可用模型，无法启用。",
        "target provider has no configured model preference",
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
    use crate::models::provider::{
        CreateProviderInput, ImportCurrentApiKeyInput, ProviderApiKeyDraft, ProviderApiKeyStatus,
        ProviderBaseUrlDraft, ProviderBaseUrlStatus, ReorderProvidersInput,
        SaveProviderApiKeysInput, SaveProviderBaseUrlsInput, SelectProviderApiKeyInput,
        SelectProviderBaseUrlInput, UpdateProviderFastInput, UpdateProviderInput,
        UpdateProviderPreferenceInput,
    };
    use crate::services::transaction_service::{ManagedFileKind, WritePhase};
    use std::fs;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    const MULTIPLE: &str = include_str!("../../../../../fixtures/config-multiple-providers.toml");
    const WITH_COMMENTS: &str = include_str!("../../../../../fixtures/config-with-comments.toml");
    const WITH_UNKNOWN: &str =
        include_str!("../../../../../fixtures/config-with-unknown-fields.toml");
    const AUTH_A: &str = include_str!("../../../../../fixtures/auth-api-key.json");
    const PROVIDERS_MULTIPLE: &str =
        include_str!("../../../../../fixtures/providers-multiple.json");
    const PROVIDERS_EMPTY: &str = include_str!("../../../../../fixtures/providers-empty.json");
    const PREFERENCES_MULTIPLE: &str =
        include_str!("../../../../../fixtures/provider-preferences-multiple.json");
    const PREFERENCES_V2: &str =
        include_str!("../../../../../fixtures/provider-preferences-v2.json");

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
        fs::write(&paths.provider_preferences_file, PREFERENCES_MULTIPLE).unwrap();
    }

    struct FailPreferenceWriteOnce {
        inner: StdFileOps,
        should_fail: Mutex<bool>,
    }

    impl FailPreferenceWriteOnce {
        fn new() -> Self {
            Self {
                inner: StdFileOps,
                should_fail: Mutex::new(true),
            }
        }
    }

    impl FileOps for FailPreferenceWriteOnce {
        fn read_optional(&self, path: &Path) -> Result<Option<Vec<u8>>, AppError> {
            self.inner.read_optional(path)
        }

        fn write(
            &self,
            path: &Path,
            bytes: &[u8],
            kind: ManagedFileKind,
            phase: WritePhase,
        ) -> Result<(), AppError> {
            let fail = kind == ManagedFileKind::Preferences
                && phase == WritePhase::Forward
                && std::mem::take(&mut *self.should_fail.lock().unwrap());
            if fail {
                return Err(AppError::new(
                    "INJECTED_WRITE_FAILURE",
                    "注入的偏好写入失败。",
                    "injected provider preference write failure",
                ));
            }
            self.inner.write(path, bytes, kind, phase)
        }

        fn remove_if_exists(
            &self,
            path: &Path,
            kind: ManagedFileKind,
            phase: WritePhase,
        ) -> Result<(), AppError> {
            self.inner.remove_if_exists(path, kind, phase)
        }
    }

    fn create_input(state: &ProviderListState, activate_after_save: bool) -> CreateProviderInput {
        CreateProviderInput {
            id: "provider-c".into(),
            name: "Provider C".into(),
            base_url_name: "主用地址".into(),
            base_url: "https://provider-c.example.com/v1".into(),
            wire_api: "responses".into(),
            models: vec!["gpt-5.6-sol".into(), "gpt-5.4-mini".into()],
            fast_enabled: false,
            api_key_name: "主用密钥".into(),
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
        assert_eq!(state.providers[0].base_urls.len(), 1);
        assert_eq!(state.providers[0].base_urls[0].id, "legacy-default");
        assert_eq!(state.providers[0].base_urls[0].name, "默认地址");
        assert_eq!(
            state.providers[0].selected_base_url_id.as_deref(),
            Some("legacy-default")
        );
        assert_eq!(
            state.providers[0].base_url_status,
            ProviderBaseUrlStatus::Managed
        );
        assert_eq!(state.providers[0].api_keys.len(), 1);
        assert_eq!(state.providers[0].api_keys[0].id, "legacy-default");
        assert_eq!(state.providers[0].api_keys[0].name, "默认密钥");
        assert_eq!(
            state.providers[0].selected_api_key_id.as_deref(),
            Some("legacy-default")
        );
        assert_eq!(
            state.providers[0].api_key_status,
            ProviderApiKeyStatus::Managed
        );
        assert!(state.providers[0].configuration_complete);
        assert!(!state.providers[0].fast_enabled);
        assert!(
            state
                .model_catalog
                .iter()
                .find(|model| model.id == "gpt-5.6-sol")
                .unwrap()
                .supports_fast
        );
        assert!(
            !state
                .model_catalog
                .iter()
                .find(|model| model.id == "gpt-5.4-mini")
                .unwrap()
                .supports_fast
        );
        assert_eq!(state.active_provider_id.as_deref(), Some("provider-a"));
        assert!(!state.current_auth_import_available);
        assert!(!json.contains("test-key-a-not-real"));
        assert!(!json.contains("test-key-b-not-real"));
        assert!(!json.contains("\"apiKey\":"));
    }

    #[test]
    fn list_does_not_create_missing_private_stores() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");

        service.list_providers().unwrap();

        assert!(!paths.providers_file.exists());
        assert!(!paths.provider_preferences_file.exists());
    }

    #[tokio::test]
    async fn saving_external_base_url_does_not_create_missing_secret_store() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .save_provider_base_urls(SaveProviderBaseUrlsInput {
                provider_id: "provider-a".into(),
                entries: vec![ProviderBaseUrlDraft {
                    id: None,
                    name: "主用地址".into(),
                    url: "https://provider-a.example.com/v1".into(),
                }],
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert!(!paths.providers_file.exists());
        assert!(paths.provider_preferences_file.exists());
    }

    #[tokio::test]
    async fn regular_edit_does_not_import_an_external_base_url() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .update_provider(UpdateProviderInput {
                id: "provider-b".into(),
                name: "Provider B 已更新".into(),
                wire_api: "responses".into(),
                models: vec!["gpt-5.5".into()],
                fast_enabled: false,
                sync_if_active: false,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let preferences = ProviderPreferenceService::new(paths.provider_preferences_file.clone())
            .load()
            .unwrap();
        let preference = &preferences.providers["provider-b"];
        assert!(preference.base_urls.is_empty());
        assert!(preference.model_preference.is_some());
        assert!(!paths.providers_file.exists());

        let refreshed = service.list_providers().unwrap();
        let provider = refreshed
            .providers
            .iter()
            .find(|provider| provider.id == "provider-b")
            .unwrap();
        assert_eq!(provider.base_url_status, ProviderBaseUrlStatus::External);
        assert_eq!(provider.selected_base_url_id, None);
    }

    #[test]
    fn v2_projection_uses_config_and_active_auth_as_effective_selection() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(&paths.auth_file, AUTH_A).unwrap();
        fs::write(
            &paths.providers_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "当前密钥",
          "apiKey": "test-key-a-not-real"
        },
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "备用密钥",
          "apiKey": "test-key-a-backup-not-real"
        }
      ],
      "selectedApiKeyId": "f8e62dc2-46df-4234-92d5-7d318d879ff7"
    }
  }
}
"#,
        )
        .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "备用地址",
          "url": "https://provider-a-backup.example.com/v1"
        },
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "当前地址",
          "url": "https://provider-a.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": { "gpt-5.6-sol": "medium" }
      }
    }
  }
}
"#,
        )
        .unwrap();
        let service = ProviderService::new(paths, "0.1.0");

        let state = service.list_providers().unwrap();
        let provider = &state.providers[0];

        assert_eq!(
            provider.selected_base_url_id.as_deref(),
            Some("f8e62dc2-46df-4234-92d5-7d318d879ff7")
        );
        assert_eq!(provider.base_url_status, ProviderBaseUrlStatus::Managed);
        assert_eq!(
            provider.selected_api_key_id.as_deref(),
            Some("65c7650d-d20d-4dca-b445-8aa47fcbe92c")
        );
        assert_eq!(provider.api_key_status, ProviderApiKeyStatus::Managed);
        assert!(provider.configuration_complete);
    }

    #[test]
    fn unknown_external_values_and_missing_secret_are_projected_without_key_values() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(
            &paths.auth_file,
            "{\n  \"OPENAI_API_KEY\": \"test-key-external-not-real\"\n}\n",
        )
        .unwrap();
        fs::write(
            &paths.providers_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "已保存密钥",
          "apiKey": "test-key-a-not-real"
        }
      ],
      "selectedApiKeyId": "65c7650d-d20d-4dca-b445-8aa47fcbe92c"
    }
  }
}
"#,
        )
        .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "其他地址",
          "url": "https://provider-a-other.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": { "gpt-5.6-sol": "medium" }
      }
    },
    "provider-b": {
      "baseUrls": [
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "主用地址",
          "url": "https://provider-b.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.5"],
        "selectedModel": "gpt-5.5",
        "reasoningEfforts": { "gpt-5.5": "medium" }
      }
    }
  }
}
"#,
        )
        .unwrap();
        let service = ProviderService::new(paths, "0.1.0");

        let state = service.list_providers().unwrap();
        let active = &state.providers[0];
        let inactive = &state.providers[1];
        let json = serde_json::to_string(&state).unwrap();

        assert_eq!(active.base_url_status, ProviderBaseUrlStatus::External);
        assert_eq!(active.selected_base_url_id, None);
        assert_eq!(active.api_key_status, ProviderApiKeyStatus::External);
        assert_eq!(active.selected_api_key_id, None);
        assert!(!active.configuration_complete);
        assert_eq!(inactive.api_key_status, ProviderApiKeyStatus::Missing);
        assert!(!inactive.configuration_complete);
        assert!(!json.contains("test-key-external-not-real"));
        assert!(!json.contains("test-key-a-not-real"));
    }

    #[test]
    fn availability_target_uses_saved_key_and_selected_model_without_public_serialization() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths, "0.1.0");

        let target = service.resolve_availability_target("provider-a").unwrap();

        assert_eq!(target.provider_id, "provider-a");
        assert_eq!(target.base_url, "https://provider-a.example.com/v1");
        assert_eq!(target.model, "gpt-5.6-sol");
        assert_eq!(target.api_key, "test-key-a-not-real");
        assert!(!format!("{target:?}").contains("test-key-a-not-real"));
    }

    #[test]
    fn availability_target_rejects_missing_key_before_network_work() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_EMPTY);
        fs::remove_file(&paths.auth_file).unwrap();
        let service = ProviderService::new(paths, "0.1.0");

        let error = service
            .resolve_availability_target("provider-a")
            .unwrap_err();

        assert_eq!(error.code(), "PROVIDER_TEST_KEY_MISSING");
        assert!(!error.to_string().contains("test-key"));
    }

    #[test]
    fn availability_target_does_not_create_a_missing_secret_store() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(&paths.auth_file, AUTH_A).unwrap();
        fs::write(&paths.provider_preferences_file, PREFERENCES_MULTIPLE).unwrap();
        assert!(!paths.providers_file.exists());
        let service = ProviderService::new(paths.clone(), "0.1.0");

        let error = service
            .resolve_availability_target("provider-a")
            .unwrap_err();

        assert_eq!(error.code(), "PROVIDER_TEST_KEY_UNMANAGED");
        assert!(!paths.providers_file.exists());
    }

    #[test]
    fn availability_target_rejects_missing_model_preference_before_network_work() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        fs::write(
            &paths.provider_preferences_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "主用地址",
          "url": "https://provider-a.example.com/v1"
        }
      ]
    }
  }
}
"#,
        )
        .unwrap();
        let service = ProviderService::new(paths, "0.1.0");

        let error = service
            .resolve_availability_target("provider-a")
            .unwrap_err();

        assert_eq!(error.code(), "PROVIDER_TEST_MODEL_MISSING");
    }

    #[test]
    fn availability_target_rejects_unmanaged_url_and_external_active_key() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(&paths.auth_file, AUTH_A).unwrap();
        fs::write(&paths.providers_file, PROVIDERS_MULTIPLE).unwrap();
        fs::write(
            &paths.provider_preferences_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "其他地址",
          "url": "https://provider-a-other.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": { "gpt-5.6-sol": "medium" }
      }
    }
  }
}
"#,
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");

        let url_error = service
            .resolve_availability_target("provider-a")
            .unwrap_err();
        assert_eq!(url_error.code(), "PROVIDER_TEST_BASE_URL_UNMANAGED");

        fs::write(&paths.provider_preferences_file, PREFERENCES_MULTIPLE).unwrap();
        fs::write(
            &paths.auth_file,
            "{\n  \"OPENAI_API_KEY\": \"test-key-external-not-real\"\n}\n",
        )
        .unwrap();

        let key_error = service
            .resolve_availability_target("provider-a")
            .unwrap_err();
        assert_eq!(key_error.code(), "PROVIDER_TEST_KEY_UNMANAGED");
        assert!(!key_error.to_string().contains("test-key"));
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

        let error = service
            .get_provider_api_keys_for_management("orphan")
            .unwrap_err();

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
            store
                .providers
                .get("provider-c")
                .and_then(ProviderSecret::selected_api_key),
            Some("test-key-c-not-real")
        );
        let secret = &store.providers["provider-c"];
        assert_eq!(secret.api_keys[0].name, "主用密钥");
        assert_eq!(secret.selected_api_key_id, secret.api_keys[0].id);
        let preferences = ProviderPreferenceService::new(paths.provider_preferences_file.clone())
            .load()
            .unwrap();
        let preference = &preferences.providers["provider-c"];
        assert_eq!(preference.base_urls[0].name, "主用地址");
        assert_eq!(
            preferences.provider_order,
            ["provider-a", "provider-b", "provider-c"]
        );
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-not-real")
        );
    }

    #[tokio::test]
    async fn create_and_activate_writes_config_model_and_auth_in_one_transaction() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        preferences.provider_order = vec!["provider-b".into(), "provider-a".into()];
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
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
        assert!(config.contains("model = \"gpt-5.6-sol\""));
        assert!(config.contains("model_reasoning_effort = \"medium\""));
        assert!(config.contains("cli_auth_credentials_store = \"file\""));
        assert!(
            fs::read_to_string(&paths.auth_file)
                .unwrap()
                .contains("test-key-c-not-real")
        );
        assert_eq!(
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load()
                .unwrap()
                .provider_order,
            ["provider-b", "provider-a", "provider-c"]
        );
    }

    #[tokio::test]
    async fn create_and_activate_fast_projects_preference_and_global_config_atomically() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let mut input = create_input(&before, true);
        input.fast_enabled = true;

        service.create_provider(input).await.unwrap();

        let preferences = ProviderPreferenceService::new(paths.provider_preferences_file.clone())
            .load()
            .unwrap();
        assert!(
            preferences.providers["provider-c"]
                .model_preference
                .as_ref()
                .unwrap()
                .fast_enabled
        );
        let config = fs::read_to_string(&paths.config_file).unwrap();
        let document = config_service::parse_document(&config).unwrap();
        assert_eq!(
            document
                .get("service_tier")
                .and_then(toml_edit::Item::as_str),
            Some("fast")
        );
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-c-not-real")
        );
    }

    #[tokio::test]
    async fn create_fast_without_activation_only_saves_the_private_preference() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let mut input = create_input(&before, false);
        input.fast_enabled = true;

        service.create_provider(input).await.unwrap();

        let preferences = ProviderPreferenceService::new(paths.provider_preferences_file.clone())
            .load()
            .unwrap();
        assert!(
            preferences.providers["provider-c"]
                .model_preference
                .as_ref()
                .unwrap()
                .fast_enabled
        );
        let config = fs::read_to_string(&paths.config_file).unwrap();
        let document = config_service::parse_document(&config).unwrap();
        assert_eq!(
            config_service::current_provider_id(&document).as_deref(),
            Some("provider-a")
        );
        assert!(document.get("service_tier").is_none());
        assert!(document["features"].get("fast_mode").is_none());
    }

    #[tokio::test]
    async fn create_rejects_fast_for_an_unsupported_model_without_writing() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original = [
            fs::read(&paths.config_file).unwrap(),
            fs::read(&paths.auth_file).unwrap(),
            fs::read(&paths.providers_file).unwrap(),
            fs::read(&paths.provider_preferences_file).unwrap(),
        ];
        let mut input = create_input(&before, true);
        input.models = vec!["gpt-5.4-mini".into()];
        input.fast_enabled = true;

        let error = service.create_provider(input).await.unwrap_err();

        assert_eq!(error.code(), "MODEL_FAST_UNSUPPORTED");
        assert_eq!(fs::read(&paths.config_file).unwrap(), original[0]);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original[1]);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original[2]);
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original[3]
        );
        assert!(service.list_backups().unwrap().backups.is_empty());
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
            wire_api: "responses".into(),
            models: vec!["gpt-5.6-sol".into()],
            fast_enabled: false,
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
        assert!(config.contains("base_url = \"https://provider-a.example.com/v1\""));
        let keys = service
            .get_provider_api_keys_for_management("provider-a")
            .unwrap();
        assert_eq!(keys.entries.len(), 1);
        assert_eq!(keys.entries[0].api_key, "test-key-a-not-real");
        let preferences = ProviderPreferenceService::new(paths.provider_preferences_file.clone())
            .load_versioned()
            .unwrap();
        assert!(!preferences.needs_upgrade);
        assert_eq!(
            preferences.store.providers["provider-a"].base_urls[0].id,
            "legacy-default"
        );
        assert!(!preferences.store.providers.contains_key("provider-b"));
    }

    #[tokio::test]
    async fn updating_active_regular_fields_syncs_model_without_changing_url_or_auth() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let input = UpdateProviderInput {
            id: "provider-a".into(),
            name: "Provider A".into(),
            wire_api: "responses".into(),
            models: vec!["gpt-5.4-mini".into()],
            fast_enabled: false,
            sync_if_active: true,
            expected_files: before.fingerprints,
        };

        service.update_provider(input).await.unwrap();

        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-not-real")
        );
        let config = fs::read_to_string(&paths.config_file).unwrap();
        assert!(config.contains("base_url = \"https://provider-a.example.com/v1\""));
        assert!(config.contains("model = \"gpt-5.4-mini\""));
        assert!(config.contains("model_reasoning_effort = \"none\""));
    }

    #[tokio::test]
    async fn updating_active_provider_with_sync_projects_fast() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .update_provider(UpdateProviderInput {
                id: "provider-a".into(),
                name: "Provider A".into(),
                wire_api: "responses".into(),
                models: vec!["gpt-5.6-sol".into()],
                fast_enabled: true,
                sync_if_active: true,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let preferences = ProviderPreferenceService::new(paths.provider_preferences_file.clone())
            .load()
            .unwrap();
        assert!(
            preferences.providers["provider-a"]
                .model_preference
                .as_ref()
                .unwrap()
                .fast_enabled
        );
        let config = fs::read_to_string(&paths.config_file).unwrap();
        let document = config_service::parse_document(&config).unwrap();
        assert_eq!(
            document
                .get("service_tier")
                .and_then(toml_edit::Item::as_str),
            Some("fast")
        );
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn editing_to_an_unsupported_model_atomically_disables_fast_with_a_reason() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let fast_config = config_service::select_provider_with_preference(
            WITH_COMMENTS,
            "provider-a",
            "gpt-5.6-sol",
            "medium",
            true,
        )
        .unwrap();
        write_state(&paths, &fast_config, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        preferences
            .providers
            .get_mut("provider-a")
            .unwrap()
            .model_preference
            .as_mut()
            .unwrap()
            .set_fast(true)
            .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let outcome = service
            .update_provider(UpdateProviderInput {
                id: "provider-a".into(),
                name: "Provider A".into(),
                wire_api: "responses".into(),
                models: vec!["gpt-5.4-mini".into()],
                fast_enabled: false,
                sync_if_active: true,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert!(
            outcome
                .message
                .contains("Fast 已因当前模型不支持而自动关闭")
        );
        assert!(!outcome.providers[0].fast_enabled);
        let document =
            config_service::parse_document(&fs::read_to_string(&paths.config_file).unwrap())
                .unwrap();
        assert!(document.get("service_tier").is_none());
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn updating_active_provider_without_sync_only_saves_fast_preference() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .update_provider(UpdateProviderInput {
                id: "provider-a".into(),
                name: "Provider A".into(),
                wire_api: "responses".into(),
                models: vec!["gpt-5.6-sol".into()],
                fast_enabled: true,
                sync_if_active: false,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let refreshed = service.list_providers().unwrap();
        assert!(refreshed.providers[0].fast_enabled);
        let document =
            config_service::parse_document(&fs::read_to_string(&paths.config_file).unwrap())
                .unwrap();
        assert!(document.get("service_tier").is_none());
        assert!(document["features"].get("fast_mode").is_none());
    }

    #[tokio::test]
    async fn update_rejects_unsupported_fast_without_writing_any_managed_file() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original = [
            fs::read(&paths.config_file).unwrap(),
            fs::read(&paths.auth_file).unwrap(),
            fs::read(&paths.providers_file).unwrap(),
            fs::read(&paths.provider_preferences_file).unwrap(),
        ];

        let error = service
            .update_provider(UpdateProviderInput {
                id: "provider-a".into(),
                name: "Provider A".into(),
                wire_api: "responses".into(),
                models: vec!["gpt-5.4-mini".into()],
                fast_enabled: true,
                sync_if_active: true,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "MODEL_FAST_UNSUPPORTED");
        assert_eq!(fs::read(&paths.config_file).unwrap(), original[0]);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original[1]);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original[2]);
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original[3]
        );
        assert!(service.list_backups().unwrap().backups.is_empty());
    }

    #[tokio::test]
    async fn update_fast_for_active_provider_writes_preference_and_current_config() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let secrets = ProviderSecretService::new(paths.providers_file.clone())
            .load_versioned()
            .unwrap()
            .store;
        fs::write(&paths.providers_file, serialize_store(&secrets).unwrap()).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_auth = fs::read(&paths.auth_file).unwrap();
        let original_providers = fs::read(&paths.providers_file).unwrap();

        let outcome = service
            .update_provider_fast(UpdateProviderFastInput {
                provider_id: "provider-a".into(),
                enabled: true,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert!(outcome.message.contains("已写入当前 Codex 配置"));
        assert!(outcome.providers[0].fast_enabled);
        let document =
            config_service::parse_document(&fs::read_to_string(&paths.config_file).unwrap())
                .unwrap();
        assert_eq!(
            document
                .get("service_tier")
                .and_then(toml_edit::Item::as_str),
            Some("fast")
        );
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original_auth);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original_providers);
    }

    #[tokio::test]
    async fn update_fast_for_non_active_provider_only_writes_preference() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let secrets = ProviderSecretService::new(paths.providers_file.clone())
            .load_versioned()
            .unwrap()
            .store;
        fs::write(&paths.providers_file, serialize_store(&secrets).unwrap()).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_config = fs::read(&paths.config_file).unwrap();
        let original_auth = fs::read(&paths.auth_file).unwrap();
        let original_providers = fs::read(&paths.providers_file).unwrap();

        let outcome = service
            .update_provider_fast(UpdateProviderFastInput {
                provider_id: "provider-b".into(),
                enabled: true,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert!(outcome.message.contains("将在应用此 Provider 时生效"));
        let provider = outcome
            .providers
            .iter()
            .find(|provider| provider.id == "provider-b")
            .unwrap();
        assert!(provider.fast_enabled);
        assert_eq!(fs::read(&paths.config_file).unwrap(), original_config);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original_auth);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original_providers);
    }

    #[tokio::test]
    async fn update_fast_off_for_active_provider_removes_tier_but_keeps_feature_gate() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let fast_config = config_service::select_provider_with_preference(
            WITH_COMMENTS,
            "provider-a",
            "gpt-5.6-sol",
            "medium",
            true,
        )
        .unwrap();
        write_state(&paths, &fast_config, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        preferences
            .providers
            .get_mut("provider-a")
            .unwrap()
            .model_preference
            .as_mut()
            .unwrap()
            .set_fast(true)
            .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let outcome = service
            .update_provider_fast(UpdateProviderFastInput {
                provider_id: "provider-a".into(),
                enabled: false,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert!(!outcome.providers[0].fast_enabled);
        let document =
            config_service::parse_document(&fs::read_to_string(&paths.config_file).unwrap())
                .unwrap();
        assert!(document.get("service_tier").is_none());
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn update_fast_rejects_an_unsupported_selected_model_without_writing() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        preferences
            .providers
            .get_mut("provider-a")
            .unwrap()
            .model_preference
            .as_mut()
            .unwrap()
            .select("gpt-5.4-mini", "none")
            .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original = [
            fs::read(&paths.config_file).unwrap(),
            fs::read(&paths.auth_file).unwrap(),
            fs::read(&paths.providers_file).unwrap(),
            fs::read(&paths.provider_preferences_file).unwrap(),
        ];

        let error = service
            .update_provider_fast(UpdateProviderFastInput {
                provider_id: "provider-a".into(),
                enabled: true,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "MODEL_FAST_UNSUPPORTED");
        assert_eq!(fs::read(&paths.config_file).unwrap(), original[0]);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original[1]);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original[2]);
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original[3]
        );
        assert!(service.list_backups().unwrap().backups.is_empty());
    }

    #[tokio::test]
    async fn update_fast_rejects_stale_fingerprints_without_overwriting_external_changes() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_preferences = fs::read(&paths.provider_preferences_file).unwrap();
        let external_config = format!("{WITH_COMMENTS}\n# external change\n");
        fs::write(&paths.config_file, &external_config).unwrap();

        let error = service
            .update_provider_fast(UpdateProviderFastInput {
                provider_id: "provider-a".into(),
                enabled: true,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "EXTERNAL_MODIFICATION_CONFLICT");
        assert_eq!(
            fs::read_to_string(&paths.config_file).unwrap(),
            external_config
        );
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original_preferences
        );
        assert!(service.list_backups().unwrap().backups.is_empty());
    }

    #[tokio::test]
    async fn update_fast_preference_write_failure_restores_all_original_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let original = [
            fs::read(&paths.config_file).unwrap(),
            fs::read(&paths.auth_file).unwrap(),
            fs::read(&paths.providers_file).unwrap(),
            fs::read(&paths.provider_preferences_file).unwrap(),
        ];
        let service = ProviderService::with_file_ops(
            paths.clone(),
            "0.1.0",
            Arc::new(FailPreferenceWriteOnce::new()),
        );
        let before = service.list_providers().unwrap();

        let error = service
            .update_provider_fast(UpdateProviderFastInput {
                provider_id: "provider-a".into(),
                enabled: true,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "TRANSACTION_FAILED_ROLLED_BACK");
        assert!(error.public_message().contains("原配置已恢复"));
        assert_eq!(fs::read(&paths.config_file).unwrap(), original[0]);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original[1]);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original[2]);
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original[3]
        );
        assert!(!paths.app_data_dir.join("transaction.json").exists());
        assert_eq!(service.list_backups().unwrap().backups.len(), 1);
    }

    #[tokio::test]
    async fn updating_preference_for_fast_active_provider_keeps_fast_projection() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let fast_config = config_service::select_provider_with_preference(
            WITH_COMMENTS,
            "provider-a",
            "gpt-5.6-sol",
            "medium",
            true,
        )
        .unwrap();
        write_state(&paths, &fast_config, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        preferences
            .providers
            .get_mut("provider-a")
            .unwrap()
            .model_preference
            .as_mut()
            .unwrap()
            .set_fast(true)
            .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let outcome = service
            .update_provider_preference(UpdateProviderPreferenceInput {
                provider_id: "provider-a".into(),
                model: "gpt-5.6-sol".into(),
                reasoning_effort: "high".into(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert!(outcome.providers[0].fast_enabled);
        let document =
            config_service::parse_document(&fs::read_to_string(&paths.config_file).unwrap())
                .unwrap();
        assert_eq!(
            document
                .get("service_tier")
                .and_then(toml_edit::Item::as_str),
            Some("fast")
        );
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn selecting_unsupported_model_for_fast_active_provider_closes_fast_atomically() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let fast_config = config_service::select_provider_with_preference(
            WITH_COMMENTS,
            "provider-a",
            "gpt-5.6-sol",
            "medium",
            true,
        )
        .unwrap();
        write_state(&paths, &fast_config, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        preferences
            .providers
            .get_mut("provider-a")
            .unwrap()
            .model_preference
            .as_mut()
            .unwrap()
            .set_fast(true)
            .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let outcome = service
            .update_provider_preference(UpdateProviderPreferenceInput {
                provider_id: "provider-a".into(),
                model: "gpt-5.4-mini".into(),
                reasoning_effort: "none".into(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert!(
            outcome
                .message
                .contains("Fast 已因当前模型不支持而自动关闭")
        );
        assert!(!outcome.providers[0].fast_enabled);
        let document =
            config_service::parse_document(&fs::read_to_string(&paths.config_file).unwrap())
                .unwrap();
        assert!(document.get("service_tier").is_none());
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
    }

    #[tokio::test]
    async fn selecting_unsupported_model_for_non_active_provider_only_changes_preference() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, WITH_COMMENTS, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        let preference = preferences
            .providers
            .get_mut("provider-b")
            .unwrap()
            .model_preference
            .as_mut()
            .unwrap();
        preference
            .reconcile_models(&["gpt-5.5".into(), "gpt-5.4-mini".into()])
            .unwrap();
        preference.set_fast(true).unwrap();
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_config = fs::read(&paths.config_file).unwrap();

        let outcome = service
            .update_provider_preference(UpdateProviderPreferenceInput {
                provider_id: "provider-b".into(),
                model: "gpt-5.4-mini".into(),
                reasoning_effort: "none".into(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert!(
            outcome
                .message
                .contains("Fast 已因当前模型不支持而自动关闭")
        );
        let provider = outcome
            .providers
            .iter()
            .find(|provider| provider.id == "provider-b")
            .unwrap();
        assert!(!provider.fast_enabled);
        assert_eq!(fs::read(&paths.config_file).unwrap(), original_config);
    }

    #[tokio::test]
    async fn updating_non_current_regular_fields_preserves_all_keys_and_active_auth() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let input = UpdateProviderInput {
            id: "provider-b".into(),
            name: "Provider B".into(),
            wire_api: "responses".into(),
            models: vec!["gpt-5.5".into()],
            fast_enabled: false,
            sync_if_active: false,
            expected_files: before.fingerprints,
        };

        let outcome = service.update_provider(input).await.unwrap();

        assert_eq!(outcome.message, "Provider「Provider B」已更新。");
        let store = ProviderSecretService::new(paths.providers_file.clone())
            .load_or_create()
            .unwrap();
        assert!(store.providers.contains_key("provider-a"));
        assert_eq!(
            configured_key(&store, "provider-b").as_deref(),
            Some("test-key-b-not-real")
        );
        assert_eq!(fs::read_to_string(&paths.auth_file).unwrap(), AUTH_A);
    }

    #[tokio::test]
    async fn regular_edit_upgrades_the_legacy_secret_store() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .update_provider(UpdateProviderInput {
                id: "provider-b".into(),
                name: "Provider B".into(),
                wire_api: "responses".into(),
                models: vec!["gpt-5.5".into()],
                fast_enabled: false,
                sync_if_active: false,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let providers: serde_json::Value =
            serde_json::from_slice(&fs::read(&paths.providers_file).unwrap()).unwrap();
        assert_eq!(providers["version"], 2);
    }

    #[tokio::test]
    async fn api_key_selection_upgrades_the_legacy_preference_store() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .select_provider_api_key(SelectProviderApiKeyInput {
                provider_id: "provider-b".into(),
                api_key_id: "legacy-default".into(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let preferences: serde_json::Value =
            serde_json::from_slice(&fs::read(&paths.provider_preferences_file).unwrap()).unwrap();
        assert_eq!(preferences["version"], 3);
        assert_eq!(
            preferences["providers"]["provider-b"]["modelPreference"]["fastEnabled"],
            false
        );
    }

    #[tokio::test]
    async fn successful_fast_transaction_upgrades_v2_preferences_with_fast_disabled() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        fs::write(&paths.provider_preferences_file, PREFERENCES_V2).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        assert_eq!(
            fs::read_to_string(&paths.provider_preferences_file).unwrap(),
            PREFERENCES_V2
        );

        service
            .update_provider_fast(UpdateProviderFastInput {
                provider_id: "provider-b".into(),
                enabled: false,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let preferences: serde_json::Value =
            serde_json::from_slice(&fs::read(&paths.provider_preferences_file).unwrap()).unwrap();
        assert_eq!(preferences["version"], 3);
        assert_eq!(
            preferences["providers"]["provider-b"]["modelPreference"]["fastEnabled"],
            false
        );
    }

    #[tokio::test]
    async fn regular_edit_persists_the_active_auth_key_as_the_selected_entry() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(&paths.auth_file, AUTH_A).unwrap();
        fs::write(
            &paths.providers_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "当前认证",
          "apiKey": "test-key-a-not-real"
        },
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "旧预选",
          "apiKey": "test-key-a-backup-not-real"
        }
      ],
      "selectedApiKeyId": "f8e62dc2-46df-4234-92d5-7d318d879ff7"
    }
  }
}
"#,
        )
        .unwrap();
        fs::write(&paths.provider_preferences_file, PREFERENCES_MULTIPLE).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .update_provider(UpdateProviderInput {
                id: "provider-a".into(),
                name: "Provider A".into(),
                wire_api: "responses".into(),
                models: vec!["gpt-5.6-sol".into()],
                fast_enabled: false,
                sync_if_active: false,
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let store = ProviderSecretService::new(paths.providers_file.clone())
            .load_read_only()
            .unwrap();
        assert_eq!(
            store.providers["provider-a"].selected_api_key_id,
            "65c7650d-d20d-4dca-b445-8aa47fcbe92c"
        );
    }

    #[tokio::test]
    async fn base_url_batch_save_preserves_ids_and_order_and_syncs_selected_value() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        fs::write(
            &paths.provider_preferences_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "主用地址",
          "url": "https://provider-a.example.com/v1"
        },
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "待删除地址",
          "url": "https://provider-a-old.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": { "gpt-5.6-sol": "medium" }
      }
    }
  }
}
"#,
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .save_provider_base_urls(SaveProviderBaseUrlsInput {
                provider_id: "provider-a".into(),
                entries: vec![
                    ProviderBaseUrlDraft {
                        id: Some("65c7650d-d20d-4dca-b445-8aa47fcbe92c".into()),
                        name: "主用地址已改名".into(),
                        url: "https://provider-a-new.example.com/v1".into(),
                    },
                    ProviderBaseUrlDraft {
                        id: None,
                        name: "新增地址".into(),
                        url: "https://provider-a-backup.example.com/v1".into(),
                    },
                ],
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let preferences = ProviderPreferenceService::new(paths.provider_preferences_file.clone())
            .load()
            .unwrap();
        let entries = &preferences.providers["provider-a"].base_urls;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "65c7650d-d20d-4dca-b445-8aa47fcbe92c");
        assert_eq!(entries[0].name, "主用地址已改名");
        assert_eq!(entries[0].url, "https://provider-a-new.example.com/v1");
        assert_ne!(entries[1].id, "f8e62dc2-46df-4234-92d5-7d318d879ff7");
        assert_eq!(entries[1].name, "新增地址");
        let config = fs::read_to_string(&paths.config_file).unwrap();
        assert!(config.contains("base_url = \"https://provider-a-new.example.com/v1\""));
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-not-real")
        );
    }

    #[test]
    fn api_key_management_query_returns_only_target_full_keys_with_redacted_debug() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(&paths.auth_file, AUTH_A).unwrap();
        fs::write(
            &paths.providers_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "当前密钥",
          "apiKey": "test-key-a-not-real"
        },
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "备用密钥",
          "apiKey": "test-key-a-backup-not-real"
        }
      ],
      "selectedApiKeyId": "f8e62dc2-46df-4234-92d5-7d318d879ff7"
    },
    "provider-b": {
      "apiKeys": [
        {
          "id": "e2d4ae25-4dc8-4f24-9dd2-790f6f9e2da1",
          "name": "其他 Provider 密钥",
          "apiKey": "test-key-b-not-real"
        }
      ],
      "selectedApiKeyId": "e2d4ae25-4dc8-4f24-9dd2-790f6f9e2da1"
    }
  }
}
"#,
        )
        .unwrap();
        fs::write(&paths.provider_preferences_file, PREFERENCES_MULTIPLE).unwrap();
        let service = ProviderService::new(paths, "0.1.0");

        let management = service
            .get_provider_api_keys_for_management("provider-a")
            .unwrap();

        assert_eq!(management.provider_id, "provider-a");
        assert_eq!(management.entries.len(), 2);
        assert_eq!(management.entries[0].name, "当前密钥");
        assert_eq!(management.entries[0].api_key, "test-key-a-not-real");
        assert_eq!(
            management.selected_api_key_id.as_deref(),
            Some("65c7650d-d20d-4dca-b445-8aa47fcbe92c")
        );
        assert_eq!(management.api_key_status, ProviderApiKeyStatus::Managed);
        let debug = format!("{management:?}");
        assert!(!debug.contains("test-key-a-not-real"));
        assert!(!debug.contains("test-key-a-backup-not-real"));
        assert!(!debug.contains("test-key-b-not-real"));
    }

    #[tokio::test]
    async fn api_key_batch_save_preserves_ids_and_order_and_syncs_active_selected_value() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(&paths.auth_file, AUTH_A).unwrap();
        fs::write(
            &paths.providers_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "当前密钥",
          "apiKey": "test-key-a-not-real"
        },
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "待删除密钥",
          "apiKey": "test-key-a-old-not-real"
        }
      ],
      "selectedApiKeyId": "65c7650d-d20d-4dca-b445-8aa47fcbe92c"
    }
  }
}
"#,
        )
        .unwrap();
        fs::write(&paths.provider_preferences_file, PREFERENCES_MULTIPLE).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        service
            .save_provider_api_keys(SaveProviderApiKeysInput {
                provider_id: "provider-a".into(),
                entries: vec![
                    ProviderApiKeyDraft {
                        id: Some("65c7650d-d20d-4dca-b445-8aa47fcbe92c".into()),
                        name: "当前密钥已改名".into(),
                        api_key: "test-key-a-updated-not-real".into(),
                    },
                    ProviderApiKeyDraft {
                        id: None,
                        name: "新增密钥".into(),
                        api_key: "test-key-a-backup-not-real".into(),
                    },
                ],
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let store = ProviderSecretService::new(paths.providers_file.clone())
            .load_or_create()
            .unwrap();
        let secret = &store.providers["provider-a"];
        assert_eq!(secret.api_keys.len(), 2);
        assert_eq!(
            secret.api_keys[0].id,
            "65c7650d-d20d-4dca-b445-8aa47fcbe92c"
        );
        assert_eq!(secret.api_keys[0].name, "当前密钥已改名");
        assert_eq!(secret.api_keys[0].api_key, "test-key-a-updated-not-real");
        assert_ne!(
            secret.api_keys[1].id,
            "f8e62dc2-46df-4234-92d5-7d318d879ff7"
        );
        assert_eq!(secret.api_keys[1].name, "新增密钥");
        assert_eq!(secret.selected_api_key_id, secret.api_keys[0].id);
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-updated-not-real")
        );
        let list_json = serde_json::to_string(&service.list_providers().unwrap()).unwrap();
        assert!(!list_json.contains("test-key-a-updated-not-real"));
        assert!(!list_json.contains("test-key-a-backup-not-real"));
    }

    #[tokio::test]
    async fn api_key_batch_save_requires_switch_before_deleting_selected_entry() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_providers = fs::read(&paths.providers_file).unwrap();

        let error = service
            .save_provider_api_keys(SaveProviderApiKeysInput {
                provider_id: "provider-a".into(),
                entries: vec![ProviderApiKeyDraft {
                    id: None,
                    name: "替代密钥".into(),
                    api_key: "test-key-a-replacement-not-real".into(),
                }],
                expected_files: before.fingerprints,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "SELECTED_API_KEY_DELETE_FORBIDDEN");
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original_providers);
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-not-real")
        );
    }

    #[tokio::test]
    async fn api_key_selection_is_independent_for_current_and_non_current_providers() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(&paths.auth_file, AUTH_A).unwrap();
        fs::write(
            &paths.providers_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "主用密钥",
          "apiKey": "test-key-a-not-real"
        },
        {
          "id": "1c1d8839-188f-4e34-b9a8-4d56d43da2b0",
          "name": "备用密钥",
          "apiKey": "test-key-a-backup-not-real"
        }
      ],
      "selectedApiKeyId": "65c7650d-d20d-4dca-b445-8aa47fcbe92c"
    },
    "provider-b": {
      "apiKeys": [
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "主用密钥",
          "apiKey": "test-key-b-not-real"
        },
        {
          "id": "e2d4ae25-4dc8-4f24-9dd2-790f6f9e2da1",
          "name": "备用密钥",
          "apiKey": "test-key-b-backup-not-real"
        }
      ],
      "selectedApiKeyId": "f8e62dc2-46df-4234-92d5-7d318d879ff7"
    }
  }
}
"#,
        )
        .unwrap();
        fs::write(&paths.provider_preferences_file, PREFERENCES_MULTIPLE).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let inactive_outcome = service
            .select_provider_api_key(SelectProviderApiKeyInput {
                provider_id: "provider-b".into(),
                api_key_id: "e2d4ae25-4dc8-4f24-9dd2-790f6f9e2da1".into(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert_eq!(
            inactive_outcome.message,
            "API Key 已保存，将在应用此 Provider 时生效。"
        );
        let state = service.list_providers().unwrap();
        assert_eq!(state.active_provider_id.as_deref(), Some("provider-a"));
        assert_eq!(
            state.providers[1].selected_api_key_id.as_deref(),
            Some("e2d4ae25-4dc8-4f24-9dd2-790f6f9e2da1")
        );
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-not-real")
        );

        let refreshed = service.list_providers().unwrap();
        let active_outcome = service
            .select_provider_api_key(SelectProviderApiKeyInput {
                provider_id: "provider-a".into(),
                api_key_id: "1c1d8839-188f-4e34-b9a8-4d56d43da2b0".into(),
                expected_files: refreshed.fingerprints,
            })
            .await
            .unwrap();

        assert_eq!(
            active_outcome.message,
            "API Key 已写入当前 Codex 认证，请重启 Codex 后生效。"
        );
        let active_state = service.list_providers().unwrap();
        assert_eq!(
            active_state.active_provider_id.as_deref(),
            Some("provider-a")
        );
        assert_eq!(
            active_state.providers[0].selected_api_key_id.as_deref(),
            Some("1c1d8839-188f-4e34-b9a8-4d56d43da2b0")
        );
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-backup-not-real")
        );
    }

    #[tokio::test]
    async fn base_url_batch_save_rejects_deleting_last_entry_without_writing() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        fs::write(
            &paths.provider_preferences_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "未选中地址",
          "url": "https://provider-a-other.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": { "gpt-5.6-sol": "medium" }
      }
    }
  }
}
"#,
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_config = fs::read(&paths.config_file).unwrap();
        let original_auth = fs::read(&paths.auth_file).unwrap();
        let original_providers = fs::read(&paths.providers_file).unwrap();
        let original_preferences = fs::read(&paths.provider_preferences_file).unwrap();

        let error = service
            .save_provider_base_urls(SaveProviderBaseUrlsInput {
                provider_id: "provider-a".into(),
                entries: Vec::new(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "LAST_BASE_URL_DELETE_FORBIDDEN");
        assert_eq!(fs::read(&paths.config_file).unwrap(), original_config);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original_auth);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original_providers);
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original_preferences
        );
    }

    #[tokio::test]
    async fn base_url_batch_save_requires_switch_before_deleting_selected_entry() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_preferences = fs::read(&paths.provider_preferences_file).unwrap();

        let error = service
            .save_provider_base_urls(SaveProviderBaseUrlsInput {
                provider_id: "provider-a".into(),
                entries: vec![ProviderBaseUrlDraft {
                    id: None,
                    name: "替代地址".into(),
                    url: "https://provider-a-new.example.com/v1".into(),
                }],
                expected_files: before.fingerprints,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "SELECTED_BASE_URL_DELETE_FORBIDDEN");
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original_preferences
        );
    }

    #[tokio::test]
    async fn selecting_non_current_base_url_keeps_active_provider_and_auth_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        fs::write(
            &paths.provider_preferences_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "主用地址",
          "url": "https://provider-a.example.com/v1"
        },
        {
          "id": "1c1d8839-188f-4e34-b9a8-4d56d43da2b0",
          "name": "备用地址",
          "url": "https://provider-a-backup.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": { "gpt-5.6-sol": "medium" }
      }
    },
    "provider-b": {
      "baseUrls": [
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "主用地址",
          "url": "https://provider-b.example.com/v1"
        },
        {
          "id": "e2d4ae25-4dc8-4f24-9dd2-790f6f9e2da1",
          "name": "备用地址",
          "url": "https://provider-b-backup.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.5"],
        "selectedModel": "gpt-5.5",
        "reasoningEfforts": { "gpt-5.5": "medium" }
      }
    }
  }
}
"#,
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();

        let outcome = service
            .select_provider_base_url(SelectProviderBaseUrlInput {
                provider_id: "provider-b".into(),
                base_url_id: "e2d4ae25-4dc8-4f24-9dd2-790f6f9e2da1".into(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert_eq!(
            outcome.message,
            "Base URL 已保存，将在应用此 Provider 时生效。"
        );
        let state = service.list_providers().unwrap();
        assert_eq!(state.active_provider_id.as_deref(), Some("provider-a"));
        assert_eq!(
            state.providers[1].selected_base_url_id.as_deref(),
            Some("e2d4ae25-4dc8-4f24-9dd2-790f6f9e2da1")
        );
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-not-real")
        );

        let refreshed = service.list_providers().unwrap();
        let active_outcome = service
            .select_provider_base_url(SelectProviderBaseUrlInput {
                provider_id: "provider-a".into(),
                base_url_id: "1c1d8839-188f-4e34-b9a8-4d56d43da2b0".into(),
                expected_files: refreshed.fingerprints,
            })
            .await
            .unwrap();

        assert_eq!(
            active_outcome.message,
            "Base URL 已写入当前 Codex 配置，请重启 Codex 后生效。"
        );
        let active_state = service.list_providers().unwrap();
        assert_eq!(
            active_state.active_provider_id.as_deref(),
            Some("provider-a")
        );
        assert_eq!(
            active_state.providers[0].selected_base_url_id.as_deref(),
            Some("1c1d8839-188f-4e34-b9a8-4d56d43da2b0")
        );
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-a-not-real")
        );
    }

    #[tokio::test]
    async fn delete_removes_only_non_current_provider_and_key() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        preferences.provider_order = vec!["provider-b".into(), "provider-a".into()];
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
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
        assert!(
            !ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load()
                .unwrap()
                .providers
                .contains_key("provider-b")
        );
        assert_eq!(
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load()
                .unwrap()
                .provider_order,
            ["provider-a"]
        );

        let refreshed = service.list_providers().unwrap();
        let error = service
            .delete_provider("provider-a", refreshed.fingerprints)
            .await
            .unwrap_err();
        assert_eq!(error.code(), "ACTIVE_PROVIDER_DELETE_FORBIDDEN");
    }

    #[tokio::test]
    async fn reorder_persists_private_order_without_changing_codex_files() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_config = fs::read(&paths.config_file).unwrap();
        let original_auth = fs::read(&paths.auth_file).unwrap();
        let original_providers = fs::read(&paths.providers_file).unwrap();

        let outcome = service
            .reorder_providers(ReorderProvidersInput {
                provider_ids: vec!["provider-b".into(), "provider-a".into()],
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert_eq!(
            outcome
                .providers
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>(),
            ["provider-b", "provider-a"]
        );
        assert_eq!(fs::read(&paths.config_file).unwrap(), original_config);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original_auth);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original_providers);
        let preferences = ProviderPreferenceService::new(paths.provider_preferences_file.clone())
            .load()
            .unwrap();
        assert_eq!(preferences.provider_order, ["provider-b", "provider-a"]);
        assert_eq!(
            service
                .list_providers()
                .unwrap()
                .providers
                .iter()
                .map(|provider| provider.id.as_str())
                .collect::<Vec<_>>(),
            ["provider-b", "provider-a"]
        );
    }

    #[tokio::test]
    async fn invalid_reorders_are_rejected_without_writing_any_file() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_config = fs::read(&paths.config_file).unwrap();
        let original_auth = fs::read(&paths.auth_file).unwrap();
        let original_providers = fs::read(&paths.providers_file).unwrap();
        let original_preferences = fs::read(&paths.provider_preferences_file).unwrap();

        for provider_ids in [
            vec!["provider-a".into(), "provider-a".into()],
            vec!["provider-a".into()],
            vec!["provider-a".into(), "provider-c".into()],
            vec!["provider-a".into(), "provider/invalid".into()],
        ] {
            let error = service
                .reorder_providers(ReorderProvidersInput {
                    provider_ids,
                    expected_files: before.fingerprints.clone(),
                })
                .await
                .unwrap_err();

            assert_eq!(error.code(), "INVALID_PROVIDER_ORDER");
        }
        assert_eq!(fs::read(&paths.config_file).unwrap(), original_config);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original_auth);
        assert_eq!(fs::read(&paths.providers_file).unwrap(), original_providers);
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original_preferences
        );
    }

    #[tokio::test]
    async fn stale_reorder_fingerprint_does_not_overwrite_external_changes() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        let original_preferences = fs::read(&paths.provider_preferences_file).unwrap();
        fs::write(
            &paths.config_file,
            format!("{}\n# external change\n", MULTIPLE),
        )
        .unwrap();

        let error = service
            .reorder_providers(ReorderProvidersInput {
                provider_ids: vec!["provider-b".into(), "provider-a".into()],
                expected_files: before.fingerprints,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code(), "EXTERNAL_MODIFICATION_CONFLICT");
        assert_eq!(
            fs::read(&paths.provider_preferences_file).unwrap(),
            original_preferences
        );
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
    async fn switch_to_fast_provider_projects_tier_and_feature_gate() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        let mut preferences =
            ProviderPreferenceService::new(paths.provider_preferences_file.clone())
                .load_versioned()
                .unwrap()
                .store;
        preferences
            .providers
            .get_mut("provider-a")
            .unwrap()
            .hydrate_legacy_base_url("https://provider-a.example.com/v1")
            .unwrap();
        let provider_b = preferences.providers.get_mut("provider-b").unwrap();
        provider_b
            .hydrate_legacy_base_url("https://provider-b.example.com/v1")
            .unwrap();
        provider_b
            .model_preference
            .as_mut()
            .unwrap()
            .set_fast(true)
            .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            serialize_preference_store(&preferences).unwrap(),
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");

        let outcome = service.switch_provider("provider-b").await.unwrap();

        let provider = outcome
            .providers
            .iter()
            .find(|provider| provider.id == "provider-b")
            .unwrap();
        assert!(provider.fast_enabled);
        let document =
            config_service::parse_document(&fs::read_to_string(&paths.config_file).unwrap())
                .unwrap();
        assert_eq!(
            document
                .get("service_tier")
                .and_then(toml_edit::Item::as_str),
            Some("fast")
        );
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
        assert!(
            fs::read_to_string(&paths.auth_file)
                .unwrap()
                .contains("test-key-b-not-real")
        );

        service.switch_provider("provider-a").await.unwrap();

        let document =
            config_service::parse_document(&fs::read_to_string(&paths.config_file).unwrap())
                .unwrap();
        assert!(document.get("service_tier").is_none());
        assert_eq!(document["features"]["fast_mode"].as_bool(), Some(true));
        assert!(
            fs::read_to_string(&paths.auth_file)
                .unwrap()
                .contains("test-key-a-not-real")
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
    async fn switch_rejects_unmanaged_base_url_without_modifying_files() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_MULTIPLE);
        fs::write(
            &paths.provider_preferences_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "baseUrls": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "主用地址",
          "url": "https://provider-a.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.6-sol"],
        "selectedModel": "gpt-5.6-sol",
        "reasoningEfforts": { "gpt-5.6-sol": "medium" }
      }
    },
    "provider-b": {
      "baseUrls": [
        {
          "id": "f8e62dc2-46df-4234-92d5-7d318d879ff7",
          "name": "其他地址",
          "url": "https://provider-b-other.example.com/v1"
        }
      ],
      "modelPreference": {
        "models": ["gpt-5.5"],
        "selectedModel": "gpt-5.5",
        "reasoningEfforts": { "gpt-5.5": "medium" }
      }
    }
  }
}
"#,
        )
        .unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let original_config = fs::read(&paths.config_file).unwrap();
        let original_auth = fs::read(&paths.auth_file).unwrap();

        let error = service.switch_provider("provider-b").await.unwrap_err();

        assert_eq!(error.code(), "PROVIDER_BASE_URL_UNMANAGED");
        assert_eq!(fs::read(&paths.config_file).unwrap(), original_config);
        assert_eq!(fs::read(&paths.auth_file).unwrap(), original_auth);
    }

    #[tokio::test]
    async fn imports_existing_auth_key_only_into_current_provider_after_confirmation() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        write_state(&paths, MULTIPLE, AUTH_A, PROVIDERS_EMPTY);
        let service = ProviderService::new(paths.clone(), "0.1.0");

        let before = service.list_providers().unwrap();
        assert!(before.current_auth_import_available);
        let outcome = service
            .import_current_auth_key(ImportCurrentApiKeyInput {
                provider_id: "provider-a".into(),
                name: "外部密钥".into(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        assert_eq!(
            outcome.message,
            "已将当前 Codex API Key 保存到 Provider「Provider A」。"
        );
        let store = ProviderSecretService::new(paths.providers_file.clone())
            .load_or_create()
            .unwrap();
        assert_eq!(store.providers.len(), 1);
        assert!(store.providers.contains_key("provider-a"));
        assert_eq!(store.providers["provider-a"].api_keys[0].name, "外部密钥");
        assert!(!store.providers.contains_key("provider-b"));
    }

    #[tokio::test]
    async fn named_auth_import_appends_to_existing_keys_and_selects_the_imported_value() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, MULTIPLE).unwrap();
        fs::write(
            &paths.auth_file,
            "{\n  \"OPENAI_API_KEY\": \"test-key-external-not-real\"\n}\n",
        )
        .unwrap();
        fs::write(
            &paths.providers_file,
            r#"{
  "version": 2,
  "providers": {
    "provider-a": {
      "apiKeys": [
        {
          "id": "65c7650d-d20d-4dca-b445-8aa47fcbe92c",
          "name": "已保存密钥",
          "apiKey": "test-key-a-not-real"
        }
      ],
      "selectedApiKeyId": "65c7650d-d20d-4dca-b445-8aa47fcbe92c"
    }
  }
}
"#,
        )
        .unwrap();
        fs::write(&paths.provider_preferences_file, PREFERENCES_MULTIPLE).unwrap();
        let service = ProviderService::new(paths.clone(), "0.1.0");
        let before = service.list_providers().unwrap();
        assert!(before.current_auth_import_available);

        service
            .import_current_auth_key(ImportCurrentApiKeyInput {
                provider_id: "provider-a".into(),
                name: "外部密钥".into(),
                expected_files: before.fingerprints,
            })
            .await
            .unwrap();

        let store = ProviderSecretService::new(paths.providers_file.clone())
            .load_or_create()
            .unwrap();
        let secret = &store.providers["provider-a"];
        assert_eq!(secret.api_keys.len(), 2);
        assert_eq!(secret.api_keys[0].name, "已保存密钥");
        assert_eq!(secret.api_keys[1].name, "外部密钥");
        assert_eq!(secret.selected_api_key_id, secret.api_keys[1].id);
        assert_eq!(
            AuthService::new(paths.auth_file.clone())
                .read_api_key()
                .unwrap()
                .as_deref(),
            Some("test-key-external-not-real")
        );
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
