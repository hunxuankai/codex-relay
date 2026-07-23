use crate::error::AppError;
use crate::infrastructure::atomic_file::atomic_write;
use crate::services::config_service::validate_provider_id;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const PROVIDER_SECRET_VERSION: u32 = 2;
const MAX_SECRET_ENTRY_NAME_LEN: usize = 100;
const MAX_API_KEY_LEN: usize = 16 * 1024;
const LEGACY_DEFAULT_ENTRY_ID: &str = "legacy-default";

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedApiKey {
    pub id: String,
    pub name: String,
    pub api_key: String,
}

impl fmt::Debug for NamedApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NamedApiKey")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("api_key_configured", &!self.api_key.is_empty())
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedNamedApiKeys {
    pub api_keys: Vec<NamedApiKey>,
    pub selected_api_key_id: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderSecretStore {
    pub version: u32,
    pub providers: BTreeMap<String, ProviderSecret>,
}

impl Default for ProviderSecretStore {
    fn default() -> Self {
        Self {
            version: PROVIDER_SECRET_VERSION,
            providers: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSecret {
    pub api_keys: Vec<NamedApiKey>,
    pub selected_api_key_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedProviderSecretStore {
    pub store: ProviderSecretStore,
    pub needs_upgrade: bool,
}

#[derive(Deserialize)]
struct StoreVersion {
    version: u32,
}

#[derive(Deserialize)]
struct LegacyProviderSecretStore {
    version: u32,
    providers: BTreeMap<String, LegacyProviderSecret>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyProviderSecret {
    api_key: String,
}

impl ProviderSecret {
    pub fn single_default(api_key: &str) -> Result<Self, AppError> {
        Self::single_named("默认密钥", api_key)
    }

    pub fn single_named(name: &str, api_key: &str) -> Result<Self, AppError> {
        let api_key = normalize_api_key(api_key)?;
        let id = Uuid::new_v4().to_string();
        Self::from_named_api_keys(
            vec![NamedApiKey {
                id: id.clone(),
                name: name.into(),
                api_key,
            }],
            &id,
        )
    }

    pub fn from_named_api_keys(
        api_keys: Vec<NamedApiKey>,
        selected_api_key_id: &str,
    ) -> Result<Self, AppError> {
        let normalized = normalize_named_api_keys(api_keys, selected_api_key_id)?;
        Ok(Self {
            api_keys: normalized.api_keys,
            selected_api_key_id: normalized.selected_api_key_id,
        })
    }

    pub fn selected_api_key(&self) -> Option<&str> {
        self.api_keys
            .iter()
            .find(|entry| entry.id == self.selected_api_key_id)
            .map(|entry| entry.api_key.as_str())
    }

    pub fn replace_selected_api_key(&mut self, api_key: &str) -> Result<(), AppError> {
        let api_key = normalize_api_key(api_key)?;
        let mut api_keys = self.api_keys.clone();
        let entry = api_keys
            .iter_mut()
            .find(|entry| entry.id == self.selected_api_key_id)
            .ok_or_else(|| {
                AppError::new(
                    "INVALID_SELECTED_API_KEY",
                    "当前 API Key 必须属于该 Provider 的密钥列表。",
                    "selected provider API key entry is missing",
                )
            })?;
        entry.api_key = api_key;
        *self = Self::from_named_api_keys(api_keys, &self.selected_api_key_id)?;
        Ok(())
    }
}

impl fmt::Debug for ProviderSecretStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let configured = self
            .providers
            .iter()
            .map(|(id, secret)| (id, secret.selected_api_key().is_some()))
            .collect::<BTreeMap<_, _>>();
        formatter
            .debug_struct("ProviderSecretStore")
            .field("version", &self.version)
            .field("providers_configured", &configured)
            .finish()
    }
}

impl fmt::Debug for ProviderSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderSecret")
            .field("api_key_count", &self.api_keys.len())
            .field("selected_api_key_id", &self.selected_api_key_id)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct ProviderSecretService {
    path: PathBuf,
}

impl ProviderSecretService {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_or_create(&self) -> Result<ProviderSecretStore, AppError> {
        Ok(self.load_versioned_or_create()?.store)
    }

    pub fn load_versioned_or_create(&self) -> Result<LoadedProviderSecretStore, AppError> {
        if !self.path.exists() {
            let store = ProviderSecretStore::default();
            self.save_store(&store)?;
            return Ok(LoadedProviderSecretStore {
                store,
                needs_upgrade: false,
            });
        }

        let bytes = fs::read(&self.path).map_err(AppError::from)?;
        match parse_store(&bytes) {
            Ok(loaded) => Ok(loaded),
            Err(error) => {
                self.back_up_corrupt_file()?;
                Err(error)
            }
        }
    }

    pub fn load_versioned(&self) -> Result<LoadedProviderSecretStore, AppError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(LoadedProviderSecretStore {
                    store: ProviderSecretStore::default(),
                    needs_upgrade: false,
                });
            }
            Err(error) => return Err(AppError::from(error)),
        };
        match parse_store(&bytes) {
            Ok(loaded) => Ok(loaded),
            Err(error) => {
                self.back_up_corrupt_file()?;
                Err(error)
            }
        }
    }

    /// 只读加载 Provider 密钥；测试和诊断边界不得因为缺少文件而创建或备份文件。
    pub fn load_read_only(&self) -> Result<ProviderSecretStore, AppError> {
        Ok(self.load_read_only_versioned()?.store)
    }

    pub fn load_read_only_versioned(&self) -> Result<LoadedProviderSecretStore, AppError> {
        match fs::read(&self.path) {
            Ok(bytes) => parse_store(&bytes).map_err(|error| {
                AppError::new(
                    error.code(),
                    "无法解析 providers.json。",
                    error.internal_detail().to_owned(),
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(LoadedProviderSecretStore {
                    store: ProviderSecretStore::default(),
                    needs_upgrade: false,
                })
            }
            Err(error) => Err(AppError::from(error)),
        }
    }

    pub fn is_configured(&self, provider_id: &str) -> Result<bool, AppError> {
        Ok(self
            .load_or_create()?
            .providers
            .get(provider_id)
            .is_some_and(|secret| secret.selected_api_key().is_some()))
    }

    pub fn get_api_key_for_edit(&self, provider_id: &str) -> Result<Option<String>, AppError> {
        Ok(self
            .load_or_create()?
            .providers
            .get(provider_id)
            .and_then(ProviderSecret::selected_api_key)
            .map(str::to_owned))
    }

    pub fn set_api_key(&self, provider_id: &str, api_key: &str) -> Result<(), AppError> {
        let provider_id = validate_provider_id(provider_id)?;
        let mut store = self.load_or_create()?;
        match store.providers.get_mut(&provider_id) {
            Some(secret) => secret.replace_selected_api_key(api_key)?,
            None => {
                store
                    .providers
                    .insert(provider_id, ProviderSecret::single_default(api_key)?);
            }
        }
        self.save_store(&store)
    }

    pub fn delete_api_key(&self, provider_id: &str) -> Result<(), AppError> {
        let provider_id = validate_provider_id(provider_id)?;
        let mut store = self.load_or_create()?;
        store.providers.remove(&provider_id);
        self.save_store(&store)
    }

    pub fn save_store(&self, store: &ProviderSecretStore) -> Result<(), AppError> {
        ensure_parent_exists(&self.path)?;
        let bytes = serialize_store(store)?;
        atomic_write(&self.path, &bytes, |candidate| {
            parse_store(candidate).map(|_| ())
        })
    }

    fn back_up_corrupt_file(&self) -> Result<PathBuf, AppError> {
        let parent = self.path.parent().ok_or_else(|| {
            AppError::new(
                "INVALID_PROVIDER_SECRET_PATH",
                "providers.json 路径无效。",
                format!("providers path has no parent: {}", self.path.display()),
            )
        })?;
        let file_name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("providers.json");
        let backup = parent.join(format!(
            "{file_name}.corrupt-{}-{}",
            Utc::now().format("%Y%m%d-%H%M%S"),
            Uuid::new_v4()
        ));
        fs::copy(&self.path, &backup).map_err(|error| {
            AppError::new(
                "CORRUPT_SECRET_BACKUP_FAILED",
                "providers.json 已损坏，并且无法创建损坏文件备份。",
                error.to_string(),
            )
        })?;
        Ok(backup)
    }
}

pub fn serialize_store(store: &ProviderSecretStore) -> Result<Vec<u8>, AppError> {
    if store.version != PROVIDER_SECRET_VERSION {
        return Err(AppError::new(
            "UNSUPPORTED_PROVIDER_SECRET_VERSION",
            "providers.json 版本不受支持。",
            format!("unsupported providers.json version: {}", store.version),
        ));
    }
    validate_store(store)?;
    let mut json = serde_json::to_string_pretty(store).map_err(AppError::from)?;
    json.push('\n');
    Ok(json.into_bytes())
}

pub fn parse_store(bytes: &[u8]) -> Result<LoadedProviderSecretStore, AppError> {
    let version = serde_json::from_slice::<StoreVersion>(bytes).map_err(|error| {
        AppError::new(
            "INVALID_PROVIDER_SECRETS",
            "无法解析 providers.json。损坏文件已备份，请重新设置 Provider 的 API Key。",
            error.to_string(),
        )
    })?;

    match version.version {
        1 => parse_legacy_store(bytes),
        PROVIDER_SECRET_VERSION => {
            let store = serde_json::from_slice::<ProviderSecretStore>(bytes).map_err(|error| {
                AppError::new(
                    "INVALID_PROVIDER_SECRETS",
                    "无法解析 providers.json。损坏文件已备份，请重新设置 Provider 的 API Key。",
                    error.to_string(),
                )
            })?;
            validate_store(&store)?;
            Ok(LoadedProviderSecretStore {
                store,
                needs_upgrade: false,
            })
        }
        unsupported => Err(AppError::new(
            "INVALID_PROVIDER_SECRETS",
            "providers.json 的版本不受支持。",
            format!("unsupported providers.json version: {unsupported}"),
        )),
    }
}

fn parse_legacy_store(bytes: &[u8]) -> Result<LoadedProviderSecretStore, AppError> {
    let legacy = serde_json::from_slice::<LegacyProviderSecretStore>(bytes).map_err(|error| {
        AppError::new(
            "INVALID_PROVIDER_SECRETS",
            "无法解析 providers.json。损坏文件已备份，请重新设置 Provider 的 API Key。",
            error.to_string(),
        )
    })?;
    if legacy.version != 1 {
        return Err(AppError::new(
            "INVALID_PROVIDER_SECRETS",
            "providers.json 的版本不受支持。",
            format!("legacy providers.json version is {}", legacy.version),
        ));
    }

    let mut providers = BTreeMap::new();
    for (provider_id, secret) in legacy.providers {
        let api_key = match normalize_api_key(&secret.api_key) {
            Ok(api_key) => api_key,
            Err(error) if error.code() == "EMPTY_API_KEY" => continue,
            Err(error) => return Err(error),
        };
        providers.insert(
            provider_id,
            ProviderSecret::from_named_api_keys(
                vec![NamedApiKey {
                    id: LEGACY_DEFAULT_ENTRY_ID.into(),
                    name: "默认密钥".into(),
                    api_key,
                }],
                LEGACY_DEFAULT_ENTRY_ID,
            )?,
        );
    }

    let store = ProviderSecretStore {
        version: PROVIDER_SECRET_VERSION,
        providers,
    };
    validate_store(&store)?;
    Ok(LoadedProviderSecretStore {
        store,
        needs_upgrade: true,
    })
}

fn validate_store(store: &ProviderSecretStore) -> Result<(), AppError> {
    if store.version != PROVIDER_SECRET_VERSION {
        return Err(AppError::new(
            "UNSUPPORTED_PROVIDER_SECRET_VERSION",
            "providers.json 版本不受支持。",
            format!("unsupported providers.json version: {}", store.version),
        ));
    }
    for (provider_id, secret) in &store.providers {
        validate_provider_id(provider_id)?;
        normalize_named_api_keys(secret.api_keys.clone(), &secret.selected_api_key_id)?;
    }
    Ok(())
}

pub fn normalize_api_key(api_key: &str) -> Result<String, AppError> {
    let normalized = api_key
        .trim_matches(|character| matches!(character, '\r' | '\n'))
        .to_owned();
    if normalized.is_empty() {
        return Err(AppError::new(
            "EMPTY_API_KEY",
            "API Key 不能为空。",
            "attempted to save an empty API key",
        ));
    }
    Ok(normalized)
}

pub fn normalize_named_api_keys(
    entries: Vec<NamedApiKey>,
    selected_api_key_id: &str,
) -> Result<NormalizedNamedApiKeys, AppError> {
    if entries.is_empty() {
        return Err(AppError::new(
            "PROVIDER_API_KEYS_REQUIRED",
            "Provider 必须至少保留一个 API Key。",
            "provider API key collection is empty",
        ));
    }

    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut values = BTreeSet::new();
    let mut normalized = Vec::with_capacity(entries.len());
    for entry in entries {
        let id = entry.id.trim();
        if id.is_empty()
            || (id != LEGACY_DEFAULT_ENTRY_ID && Uuid::parse_str(id).is_err())
            || !ids.insert(id.to_owned())
        {
            return Err(AppError::new(
                "INVALID_API_KEY_ID",
                "API Key 条目标识无效。",
                "provider API key entry id is empty, duplicated, or malformed",
            ));
        }

        let name = entry.name.trim();
        if name.is_empty() || name.chars().count() > MAX_SECRET_ENTRY_NAME_LEN {
            return Err(AppError::new(
                "INVALID_API_KEY_NAME",
                "API Key 名称不能为空且长度不能超过 100 个字符。",
                "provider API key entry name is empty or too long",
            ));
        }
        if !names.insert(name.to_lowercase()) {
            return Err(AppError::new(
                "DUPLICATE_API_KEY_NAME",
                "同一个 Provider 中的 API Key 名称不能重复。",
                "provider API key entry names are duplicated",
            ));
        }

        let api_key = normalize_api_key(&entry.api_key)?;
        if api_key.len() > MAX_API_KEY_LEN {
            return Err(AppError::new(
                "API_KEY_TOO_LONG",
                "API Key 长度超过允许范围。",
                "provider API key entry exceeds maximum length",
            ));
        }
        if !values.insert(api_key.clone()) {
            return Err(AppError::new(
                "DUPLICATE_API_KEY_VALUE",
                "同一个 Provider 中不能重复保存相同的 API Key。",
                "provider API key entry values are duplicated",
            ));
        }

        normalized.push(NamedApiKey {
            id: id.to_owned(),
            name: name.to_owned(),
            api_key,
        });
    }

    let selected_api_key_id = selected_api_key_id.trim();
    if !normalized
        .iter()
        .any(|entry| entry.id == selected_api_key_id)
    {
        return Err(AppError::new(
            "INVALID_SELECTED_API_KEY",
            "当前 API Key 必须属于该 Provider 的密钥列表。",
            "selected provider API key id is not in the entry collection",
        ));
    }

    Ok(NormalizedNamedApiKeys {
        api_keys: normalized,
        selected_api_key_id: selected_api_key_id.to_owned(),
    })
}

fn ensure_parent_exists(path: &Path) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "INVALID_FILE_PATH",
            "应用数据文件路径无效。",
            format!("file path has no parent: {}", path.display()),
        )
    })?;
    fs::create_dir_all(parent).map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn missing_store_is_created_with_versioned_empty_document() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("providers.json");
        let service = ProviderSecretService::new(path.clone());

        let store = service.load_or_create().unwrap();

        assert_eq!(store.version, 2);
        assert!(store.providers.is_empty());
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "{\n  \"version\": 2,\n  \"providers\": {}\n}\n"
        );
    }

    #[test]
    fn multiple_keys_can_be_updated_and_deleted_independently() {
        let directory = tempfile::tempdir().unwrap();
        let service = ProviderSecretService::new(directory.path().join("providers.json"));

        service
            .set_api_key("provider-a", "test-key-a-not-real\n")
            .unwrap();
        service
            .set_api_key("provider-b", "test-key-b-not-real")
            .unwrap();
        service
            .set_api_key("provider-a", "test-key-a-updated-not-real")
            .unwrap();

        assert_eq!(
            service
                .get_api_key_for_edit("provider-a")
                .unwrap()
                .as_deref(),
            Some("test-key-a-updated-not-real")
        );
        assert_eq!(
            service
                .get_api_key_for_edit("provider-b")
                .unwrap()
                .as_deref(),
            Some("test-key-b-not-real")
        );

        service.delete_api_key("provider-a").unwrap();
        assert!(!service.is_configured("provider-a").unwrap());
        assert!(service.is_configured("provider-b").unwrap());
    }

    #[test]
    fn empty_key_is_rejected_without_exposing_input() {
        let directory = tempfile::tempdir().unwrap();
        let service = ProviderSecretService::new(directory.path().join("providers.json"));

        let error = service.set_api_key("provider-a", "\r\n").unwrap_err();

        assert_eq!(error.code(), "EMPTY_API_KEY");
        assert!(!error.to_string().contains("provider-a"));
    }

    #[test]
    fn damaged_store_is_backed_up_and_never_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("providers.json");
        let invalid = include_str!("../../../../../fixtures/providers-invalid.json");
        fs::write(&path, invalid).unwrap();
        let service = ProviderSecretService::new(path.clone());

        let error = service.load_or_create().unwrap_err();

        assert_eq!(error.code(), "INVALID_PROVIDER_SECRETS");
        assert_eq!(fs::read_to_string(&path).unwrap(), invalid);
        let corrupt_copies = fs::read_dir(directory.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("providers.json.corrupt-")
            })
            .collect::<Vec<_>>();
        assert_eq!(corrupt_copies.len(), 1);
        assert_eq!(
            fs::read_to_string(corrupt_copies[0].path()).unwrap(),
            invalid
        );
    }

    #[test]
    fn debug_output_never_contains_api_keys() {
        let mut store = ProviderSecretStore::default();
        store.providers.insert(
            "provider-a".into(),
            ProviderSecret::single_default("test-key-a-not-real").unwrap(),
        );

        let output = format!("{store:?}");

        assert!(output.contains("provider-a"));
        assert!(!output.contains("test-key-a-not-real"));
    }

    #[test]
    fn named_api_keys_are_normalized_unique_ordered_and_selected_by_stable_id() {
        let first_id = "65c7650d-d20d-4dca-b445-8aa47fcbe92c";
        let second_id = "f8e62dc2-46df-4234-92d5-7d318d879ff7";
        let normalized = normalize_named_api_keys(
            vec![
                NamedApiKey {
                    id: first_id.into(),
                    name: "  主用密钥  ".into(),
                    api_key: "test-key-primary-not-real\n".into(),
                },
                NamedApiKey {
                    id: second_id.into(),
                    name: "备用密钥".into(),
                    api_key: "test-key-secondary-not-real".into(),
                },
            ],
            second_id,
        )
        .unwrap();

        assert_eq!(normalized.selected_api_key_id, second_id);
        assert_eq!(normalized.api_keys[0].id, first_id);
        assert_eq!(normalized.api_keys[0].name, "主用密钥");
        assert_eq!(normalized.api_keys[0].api_key, "test-key-primary-not-real");
        assert_eq!(normalized.api_keys[1].id, second_id);

        let duplicate_name = normalize_named_api_keys(
            vec![
                normalized.api_keys[0].clone(),
                NamedApiKey {
                    id: second_id.into(),
                    name: "主用密钥".into(),
                    api_key: "test-key-other-not-real".into(),
                },
            ],
            first_id,
        )
        .unwrap_err();
        assert_eq!(duplicate_name.code(), "DUPLICATE_API_KEY_NAME");

        let duplicate_value = normalize_named_api_keys(
            vec![
                normalized.api_keys[0].clone(),
                NamedApiKey {
                    id: second_id.into(),
                    name: "其他密钥".into(),
                    api_key: "test-key-primary-not-real".into(),
                },
            ],
            first_id,
        )
        .unwrap_err();
        assert_eq!(duplicate_value.code(), "DUPLICATE_API_KEY_VALUE");
    }

    #[test]
    fn version_one_store_is_normalized_to_version_two_without_losing_the_key() {
        let loaded = parse_store(
            br#"{
  "version": 1,
  "providers": {
    "provider-a": {
      "apiKey": "test-key-legacy-not-real"
    }
  }
}
"#,
        )
        .unwrap();

        assert!(loaded.needs_upgrade);
        assert_eq!(loaded.store.version, 2);
        let provider = &loaded.store.providers["provider-a"];
        assert_eq!(provider.selected_api_key_id, LEGACY_DEFAULT_ENTRY_ID);
        assert_eq!(provider.api_keys.len(), 1);
        assert_eq!(provider.api_keys[0].id, LEGACY_DEFAULT_ENTRY_ID);
        assert_eq!(provider.api_keys[0].name, "默认密钥");
        assert_eq!(provider.api_keys[0].api_key, "test-key-legacy-not-real");

        let rendered = String::from_utf8(serialize_store(&loaded.store).unwrap()).unwrap();
        assert!(rendered.contains("\"version\": 2"));
        assert!(rendered.contains("\"apiKeys\""));
        assert!(rendered.contains("\"selectedApiKeyId\""));
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert!(value["providers"]["provider-a"].get("apiKey").is_none());
    }
}
