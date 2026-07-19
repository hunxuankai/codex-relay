use crate::error::AppError;
use crate::infrastructure::atomic_file::atomic_write;
use crate::services::config_service::validate_provider_id;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const PROVIDER_SECRET_VERSION: u32 = 1;

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
    pub api_key: String,
}

impl fmt::Debug for ProviderSecretStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let configured = self
            .providers
            .iter()
            .map(|(id, secret)| (id, !secret.api_key.is_empty()))
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
            .field("api_key_configured", &!self.api_key.is_empty())
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
        if !self.path.exists() {
            let store = ProviderSecretStore::default();
            self.save_store(&store)?;
            return Ok(store);
        }

        let bytes = fs::read(&self.path).map_err(AppError::from)?;
        match parse_store(&bytes) {
            Ok(store) => Ok(store),
            Err(error) => {
                self.back_up_corrupt_file()?;
                Err(error)
            }
        }
    }

    pub fn is_configured(&self, provider_id: &str) -> Result<bool, AppError> {
        Ok(self
            .load_or_create()?
            .providers
            .get(provider_id)
            .is_some_and(|secret| !secret.api_key.is_empty()))
    }

    pub fn get_api_key_for_edit(&self, provider_id: &str) -> Result<Option<String>, AppError> {
        Ok(self
            .load_or_create()?
            .providers
            .get(provider_id)
            .map(|secret| secret.api_key.clone())
            .filter(|key| !key.is_empty()))
    }

    pub fn set_api_key(&self, provider_id: &str, api_key: &str) -> Result<(), AppError> {
        let provider_id = validate_provider_id(provider_id)?;
        let api_key = normalize_api_key(api_key)?;

        let mut store = self.load_or_create()?;
        store
            .providers
            .insert(provider_id, ProviderSecret { api_key });
        self.save_store(&store)
    }

    pub fn clear_api_key(&self, provider_id: &str) -> Result<(), AppError> {
        self.delete_api_key(provider_id)
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
    let mut json = serde_json::to_string_pretty(store).map_err(AppError::from)?;
    json.push('\n');
    Ok(json.into_bytes())
}

fn parse_store(bytes: &[u8]) -> Result<ProviderSecretStore, AppError> {
    let store = serde_json::from_slice::<ProviderSecretStore>(bytes).map_err(|error| {
        AppError::new(
            "INVALID_PROVIDER_SECRETS",
            "无法解析 providers.json。损坏文件已备份，请重新设置 Provider 的 API Key。",
            error.to_string(),
        )
    })?;
    if store.version != PROVIDER_SECRET_VERSION {
        return Err(AppError::new(
            "INVALID_PROVIDER_SECRETS",
            "providers.json 的版本不受支持。",
            format!("unsupported providers.json version: {}", store.version),
        ));
    }
    Ok(store)
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

        assert_eq!(store.version, 1);
        assert!(store.providers.is_empty());
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "{\n  \"version\": 1,\n  \"providers\": {}\n}\n"
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
        let invalid = include_str!("../../../fixtures/providers-invalid.json");
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
            ProviderSecret {
                api_key: "test-key-a-not-real".into(),
            },
        );

        let output = format!("{store:?}");

        assert!(output.contains("provider-a"));
        assert!(!output.contains("test-key-a-not-real"));
    }
}
