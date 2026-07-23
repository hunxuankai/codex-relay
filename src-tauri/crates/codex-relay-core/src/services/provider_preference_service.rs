use crate::error::AppError;
use crate::services::config_service::{normalize_base_url, validate_provider_id};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const PROVIDER_PREFERENCE_VERSION: u32 = 2;
const MAX_BASE_URL_ENTRY_NAME_LEN: usize = 100;
const LEGACY_DEFAULT_ENTRY_ID: &str = "legacy-default";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedBaseUrl {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelCatalogEntry {
    pub id: &'static str,
    pub reasoning_efforts: &'static [&'static str],
    pub default_reasoning_effort: &'static str,
}

const STANDARD_EFFORTS: &[&str] = &["none", "low", "medium", "high", "xhigh"];
const GPT_56_EFFORTS: &[&str] = &["none", "low", "medium", "high", "xhigh", "max"];

const MODEL_CATALOG: &[ModelCatalogEntry] = &[
    ModelCatalogEntry {
        id: "gpt-5.6-sol",
        reasoning_efforts: GPT_56_EFFORTS,
        default_reasoning_effort: "medium",
    },
    ModelCatalogEntry {
        id: "gpt-5.6-terra",
        reasoning_efforts: GPT_56_EFFORTS,
        default_reasoning_effort: "medium",
    },
    ModelCatalogEntry {
        id: "gpt-5.6-luna",
        reasoning_efforts: GPT_56_EFFORTS,
        default_reasoning_effort: "medium",
    },
    ModelCatalogEntry {
        id: "gpt-5.5",
        reasoning_efforts: STANDARD_EFFORTS,
        default_reasoning_effort: "medium",
    },
    ModelCatalogEntry {
        id: "gpt-5.4",
        reasoning_efforts: STANDARD_EFFORTS,
        default_reasoning_effort: "none",
    },
    ModelCatalogEntry {
        id: "gpt-5.4-mini",
        reasoning_efforts: STANDARD_EFFORTS,
        default_reasoning_effort: "none",
    },
];

pub fn model_catalog() -> &'static [ModelCatalogEntry] {
    MODEL_CATALOG
}

pub fn normalize_named_base_urls(
    entries: Vec<NamedBaseUrl>,
) -> Result<Vec<NamedBaseUrl>, AppError> {
    if entries.is_empty() {
        return Err(AppError::new(
            "PROVIDER_BASE_URLS_REQUIRED",
            "Provider 必须至少保留一个 Base URL。",
            "provider Base URL collection is empty",
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
                "INVALID_BASE_URL_ID",
                "Base URL 条目标识无效。",
                "provider Base URL entry id is empty, duplicated, or malformed",
            ));
        }

        let name = entry.name.trim();
        if name.is_empty() || name.chars().count() > MAX_BASE_URL_ENTRY_NAME_LEN {
            return Err(AppError::new(
                "INVALID_BASE_URL_NAME",
                "Base URL 名称不能为空且长度不能超过 100 个字符。",
                "provider Base URL entry name is empty or too long",
            ));
        }
        if !names.insert(name.to_lowercase()) {
            return Err(AppError::new(
                "DUPLICATE_BASE_URL_NAME",
                "同一个 Provider 中的 Base URL 名称不能重复。",
                "provider Base URL entry names are duplicated",
            ));
        }

        let url = normalize_base_url(&entry.url)?;
        if !values.insert(url.clone()) {
            return Err(AppError::new(
                "DUPLICATE_BASE_URL_VALUE",
                "同一个 Provider 中不能重复保存相同的 Base URL。",
                "provider Base URL entry values are duplicated",
            ));
        }

        normalized.push(NamedBaseUrl {
            id: id.to_owned(),
            name: name.to_owned(),
            url,
        });
    }

    Ok(normalized)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreference {
    pub models: Vec<String>,
    pub selected_model: String,
    pub reasoning_efforts: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPrivatePreference {
    pub base_urls: Vec<NamedBaseUrl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preference: Option<ProviderPreference>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderPreferenceStore {
    pub version: u32,
    pub providers: BTreeMap<String, ProviderPrivatePreference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedProviderPreferenceStore {
    pub store: ProviderPreferenceStore,
    pub needs_upgrade: bool,
}

#[derive(Deserialize)]
struct StoreVersion {
    version: u32,
}

#[derive(Deserialize)]
struct LegacyProviderPreferenceStore {
    version: u32,
    providers: BTreeMap<String, ProviderPreference>,
}

impl Default for ProviderPreferenceStore {
    fn default() -> Self {
        Self {
            version: PROVIDER_PREFERENCE_VERSION,
            providers: BTreeMap::new(),
        }
    }
}

impl ProviderPreference {
    pub fn from_models(models: &[String]) -> Result<Self, AppError> {
        let selected_model = models.first().cloned().ok_or_else(|| {
            AppError::new(
                "PROVIDER_MODELS_REQUIRED",
                "请至少选择一个可用模型。",
                "provider preference model list is empty",
            )
        })?;
        let mut reasoning_efforts = BTreeMap::new();
        for model in models {
            let entry = MODEL_CATALOG
                .iter()
                .find(|entry| entry.id == model)
                .ok_or_else(|| unknown_model(model))?;
            reasoning_efforts.insert(model.clone(), entry.default_reasoning_effort.into());
        }
        Ok(Self {
            models: models.to_vec(),
            selected_model,
            reasoning_efforts,
        })
    }

    pub fn reconcile_models(&mut self, models: &[String]) -> Result<bool, AppError> {
        let defaults = Self::from_models(models)?;
        let selected_changed = !models.contains(&self.selected_model);
        let selected_model = if selected_changed {
            defaults.selected_model
        } else {
            self.selected_model.clone()
        };
        let reasoning_efforts = models
            .iter()
            .map(|model| {
                let effort = self
                    .reasoning_efforts
                    .get(model)
                    .cloned()
                    .unwrap_or_else(|| defaults.reasoning_efforts[model].clone());
                (model.clone(), effort)
            })
            .collect();
        self.models = models.to_vec();
        self.selected_model = selected_model;
        self.reasoning_efforts = reasoning_efforts;
        validate_preference(self)?;
        Ok(selected_changed)
    }

    pub fn select(&mut self, model: &str, reasoning_effort: &str) -> Result<(), AppError> {
        if !self.models.iter().any(|allowed| allowed == model) {
            return Err(AppError::new(
                "MODEL_NOT_ALLOWED_FOR_PROVIDER",
                "该模型不在 Provider 的可用模型列表中。",
                format!("model {model} is not allowed for provider"),
            ));
        }
        self.selected_model = model.to_owned();
        self.reasoning_efforts
            .insert(model.to_owned(), reasoning_effort.to_owned());
        validate_preference(self)
    }
}

impl ProviderPrivatePreference {
    pub fn with_initial_base_url(
        base_url_name: &str,
        base_url: &str,
        model_preference: Option<ProviderPreference>,
    ) -> Result<Self, AppError> {
        let base_urls = normalize_named_base_urls(vec![NamedBaseUrl {
            id: Uuid::new_v4().to_string(),
            name: base_url_name.into(),
            url: base_url.into(),
        }])?;
        if let Some(preference) = &model_preference {
            validate_preference(preference)?;
        }
        Ok(Self {
            base_urls,
            model_preference,
        })
    }

    pub fn hydrate_legacy_base_url(&mut self, base_url: &str) -> Result<(), AppError> {
        if self.base_urls.is_empty() {
            self.base_urls = normalize_named_base_urls(vec![NamedBaseUrl {
                id: LEGACY_DEFAULT_ENTRY_ID.into(),
                name: "默认地址".into(),
                url: base_url.into(),
            }])?;
        }
        Ok(())
    }
}

pub fn validate_preference(preference: &ProviderPreference) -> Result<(), AppError> {
    if preference.models.is_empty() {
        return Err(AppError::new(
            "PROVIDER_MODELS_REQUIRED",
            "请至少选择一个可用模型。",
            "provider preference model list is empty",
        ));
    }
    let model_set = preference.models.iter().collect::<BTreeSet<_>>();
    if model_set.len() != preference.models.len() {
        return Err(AppError::new(
            "DUPLICATE_PROVIDER_MODEL",
            "Provider 可用模型不能重复。",
            "provider preference contains duplicate models",
        ));
    }
    if !model_set.contains(&preference.selected_model) {
        return Err(AppError::new(
            "INVALID_SELECTED_MODEL",
            "当前偏好模型必须属于可用模型。",
            "selected provider model is not in the allowed model list",
        ));
    }
    let effort_keys = preference.reasoning_efforts.keys().collect::<BTreeSet<_>>();
    if effort_keys != model_set {
        return Err(AppError::new(
            "INVALID_REASONING_EFFORT_MAP",
            "每个可用模型都必须配置一个推理强度。",
            "reasoning effort keys do not match provider models",
        ));
    }
    for model in &preference.models {
        let entry = MODEL_CATALOG
            .iter()
            .find(|entry| entry.id == model)
            .ok_or_else(|| unknown_model(model))?;
        let effort = preference
            .reasoning_efforts
            .get(model)
            .expect("keys checked");
        if !entry.reasoning_efforts.contains(&effort.as_str()) {
            return Err(AppError::new(
                "INVALID_MODEL_REASONING_EFFORT",
                "所选模型不支持该推理强度。",
                format!("unsupported reasoning effort {effort} for model {model}"),
            ));
        }
    }
    Ok(())
}

fn unknown_model(model: &str) -> AppError {
    AppError::new(
        "UNKNOWN_PROVIDER_MODEL",
        "Provider 包含当前版本不支持的模型。",
        format!("unknown provider model: {model}"),
    )
}

pub fn serialize_store(store: &ProviderPreferenceStore) -> Result<Vec<u8>, AppError> {
    let normalized = normalize_store(store.clone())?;
    let mut json = serde_json::to_string_pretty(&normalized).map_err(AppError::from)?;
    json.push('\n');
    Ok(json.into_bytes())
}

pub fn parse_store(bytes: &[u8]) -> Result<LoadedProviderPreferenceStore, AppError> {
    let version = serde_json::from_slice::<StoreVersion>(bytes).map_err(|error| {
        AppError::new(
            "INVALID_PROVIDER_PREFERENCES",
            "无法解析 provider-preferences.json。",
            error.to_string(),
        )
    })?;

    match version.version {
        1 => parse_legacy_store(bytes),
        PROVIDER_PREFERENCE_VERSION => {
            let store =
                serde_json::from_slice::<ProviderPreferenceStore>(bytes).map_err(|error| {
                    AppError::new(
                        "INVALID_PROVIDER_PREFERENCES",
                        "无法解析 provider-preferences.json。",
                        error.to_string(),
                    )
                })?;
            Ok(LoadedProviderPreferenceStore {
                store: normalize_store(store)?,
                needs_upgrade: false,
            })
        }
        unsupported => Err(AppError::new(
            "INVALID_PROVIDER_PREFERENCES",
            "provider-preferences.json 的版本不受支持。",
            format!("unsupported provider preference version: {unsupported}"),
        )),
    }
}

fn parse_legacy_store(bytes: &[u8]) -> Result<LoadedProviderPreferenceStore, AppError> {
    let legacy =
        serde_json::from_slice::<LegacyProviderPreferenceStore>(bytes).map_err(|error| {
            AppError::new(
                "INVALID_PROVIDER_PREFERENCES",
                "无法解析 provider-preferences.json。",
                error.to_string(),
            )
        })?;
    if legacy.version != 1 {
        return Err(AppError::new(
            "INVALID_PROVIDER_PREFERENCES",
            "provider-preferences.json 的版本不受支持。",
            format!("legacy provider preference version is {}", legacy.version),
        ));
    }

    let mut providers = BTreeMap::new();
    for (provider_id, preference) in legacy.providers {
        validate_provider_store_id(&provider_id)?;
        validate_preference(&preference)?;
        providers.insert(
            provider_id,
            ProviderPrivatePreference {
                base_urls: Vec::new(),
                model_preference: Some(preference),
            },
        );
    }
    Ok(LoadedProviderPreferenceStore {
        store: ProviderPreferenceStore {
            version: PROVIDER_PREFERENCE_VERSION,
            providers,
        },
        needs_upgrade: true,
    })
}

fn normalize_store(
    mut store: ProviderPreferenceStore,
) -> Result<ProviderPreferenceStore, AppError> {
    if store.version != PROVIDER_PREFERENCE_VERSION {
        return Err(AppError::new(
            "INVALID_PROVIDER_PREFERENCES",
            "provider-preferences.json 的版本不受支持。",
            format!("unsupported provider preference version: {}", store.version),
        ));
    }
    for (provider_id, preference) in &mut store.providers {
        validate_provider_store_id(provider_id)?;
        if !preference.base_urls.is_empty() {
            preference.base_urls =
                normalize_named_base_urls(std::mem::take(&mut preference.base_urls))?;
        }
        if let Some(model_preference) = &preference.model_preference {
            validate_preference(model_preference)?;
        }
    }
    Ok(store)
}

fn validate_provider_store_id(provider_id: &str) -> Result<(), AppError> {
    if validate_provider_id(provider_id)? != provider_id {
        return Err(AppError::new(
            "INVALID_PROVIDER_PREFERENCES",
            "provider-preferences.json 包含无效的 Provider ID。",
            format!("provider preference id is not normalized: {provider_id:?}"),
        ));
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ProviderPreferenceService {
    path: PathBuf,
}

impl ProviderPreferenceService {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<ProviderPreferenceStore, AppError> {
        Ok(self.load_versioned()?.store)
    }

    pub fn load_versioned(&self) -> Result<LoadedProviderPreferenceStore, AppError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(LoadedProviderPreferenceStore {
                    store: ProviderPreferenceStore::default(),
                    needs_upgrade: false,
                });
            }
            Err(error) => return Err(AppError::from(error)),
        };
        parse_store(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_catalog_exposes_verified_reasoning_efforts_and_defaults() {
        let catalog = model_catalog();

        assert_eq!(catalog.len(), 6);
        let sol = catalog
            .iter()
            .find(|model| model.id == "gpt-5.6-sol")
            .unwrap();
        assert_eq!(
            sol.reasoning_efforts,
            ["none", "low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(sol.default_reasoning_effort, "medium");

        let mini = catalog
            .iter()
            .find(|model| model.id == "gpt-5.4-mini")
            .unwrap();
        assert_eq!(
            mini.reasoning_efforts,
            ["none", "low", "medium", "high", "xhigh"]
        );
        assert_eq!(mini.default_reasoning_effort, "none");
        assert!(!catalog.iter().any(|model| model.id == "gpt-5.6"));
    }

    #[test]
    fn new_preference_uses_first_model_and_catalog_defaults() {
        let preference =
            ProviderPreference::from_models(&["gpt-5.6-sol".into(), "gpt-5.4-mini".into()])
                .unwrap();

        assert_eq!(preference.models, ["gpt-5.6-sol", "gpt-5.4-mini"]);
        assert_eq!(preference.selected_model, "gpt-5.6-sol");
        assert_eq!(
            preference
                .reasoning_efforts
                .get("gpt-5.6-sol")
                .map(String::as_str),
            Some("medium")
        );
        assert_eq!(
            preference
                .reasoning_efforts
                .get("gpt-5.4-mini")
                .map(String::as_str),
            Some("none")
        );
    }

    #[test]
    fn strict_validation_rejects_unknown_models_and_invalid_efforts() {
        let unknown = ProviderPreference::from_models(&["gpt-unknown".into()]).unwrap_err();
        assert_eq!(unknown.code(), "UNKNOWN_PROVIDER_MODEL");

        let mut invalid = ProviderPreference::from_models(&["gpt-5.4-mini".into()]).unwrap();
        invalid
            .reasoning_efforts
            .insert("gpt-5.4-mini".into(), "max".into());

        let error = validate_preference(&invalid).unwrap_err();

        assert_eq!(error.code(), "INVALID_MODEL_REASONING_EFFORT");
    }

    #[test]
    fn store_round_trip_is_versioned_and_stable() {
        let mut store = ProviderPreferenceStore::default();
        store.providers.insert(
            "provider-a".into(),
            ProviderPrivatePreference::with_initial_base_url(
                "默认地址",
                "https://provider-a.example.test/v1",
                Some(ProviderPreference::from_models(&["gpt-5.6-sol".into()]).unwrap()),
            )
            .unwrap(),
        );

        let bytes = serialize_store(&store).unwrap();
        let parsed = parse_store(&bytes).unwrap();

        assert_eq!(parsed.store, store);
        assert!(!parsed.needs_upgrade);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"version\": 2"));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn missing_preference_file_loads_empty_without_creating_assumptions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("provider-preferences.json");
        let service = ProviderPreferenceService::new(path.clone());

        let store = service.load().unwrap();

        assert!(store.providers.is_empty());
        assert!(!path.exists());
        assert!(fs::read_dir(directory.path()).unwrap().next().is_none());
    }

    #[test]
    fn removing_selected_model_falls_back_to_first_remaining_and_preserves_effort() {
        let mut preference = ProviderPreference::from_models(&[
            "gpt-5.6-sol".into(),
            "gpt-5.4-mini".into(),
            "gpt-5.5".into(),
        ])
        .unwrap();
        preference
            .reasoning_efforts
            .insert("gpt-5.4-mini".into(), "high".into());

        let selected_changed = preference
            .reconcile_models(&["gpt-5.4-mini".into(), "gpt-5.5".into()])
            .unwrap();

        assert!(selected_changed);
        assert_eq!(preference.selected_model, "gpt-5.4-mini");
        assert_eq!(
            preference
                .reasoning_efforts
                .get("gpt-5.4-mini")
                .map(String::as_str),
            Some("high")
        );
        assert!(!preference.reasoning_efforts.contains_key("gpt-5.6-sol"));
    }

    #[test]
    fn selecting_models_remembers_each_models_reasoning_effort() {
        let mut preference =
            ProviderPreference::from_models(&["gpt-5.6-sol".into(), "gpt-5.4-mini".into()])
                .unwrap();

        preference.select("gpt-5.6-sol", "high").unwrap();
        preference.select("gpt-5.4-mini", "low").unwrap();
        preference.select("gpt-5.6-sol", "high").unwrap();

        assert_eq!(preference.selected_model, "gpt-5.6-sol");
        assert_eq!(preference.reasoning_efforts["gpt-5.6-sol"], "high");
        assert_eq!(preference.reasoning_efforts["gpt-5.4-mini"], "low");
    }

    #[test]
    fn named_base_urls_are_normalized_unique_and_keep_insertion_order() {
        let first_id = "65c7650d-d20d-4dca-b445-8aa47fcbe92c";
        let second_id = "f8e62dc2-46df-4234-92d5-7d318d879ff7";
        let normalized = normalize_named_base_urls(vec![
            NamedBaseUrl {
                id: first_id.into(),
                name: "  主用地址  ".into(),
                url: " https://provider-a.example.test/v1 ".into(),
            },
            NamedBaseUrl {
                id: second_id.into(),
                name: "备用地址".into(),
                url: "https://provider-b.example.test/v1".into(),
            },
        ])
        .unwrap();

        assert_eq!(normalized[0].id, first_id);
        assert_eq!(normalized[0].name, "主用地址");
        assert_eq!(normalized[0].url, "https://provider-a.example.test/v1");
        assert_eq!(normalized[1].id, second_id);

        let duplicate_name = normalize_named_base_urls(vec![
            normalized[0].clone(),
            NamedBaseUrl {
                id: second_id.into(),
                name: "主用地址".into(),
                url: "https://provider-c.example.test/v1".into(),
            },
        ])
        .unwrap_err();
        assert_eq!(duplicate_name.code(), "DUPLICATE_BASE_URL_NAME");

        let duplicate_value = normalize_named_base_urls(vec![
            normalized[0].clone(),
            NamedBaseUrl {
                id: second_id.into(),
                name: "其他地址".into(),
                url: "https://provider-a.example.test/v1".into(),
            },
        ])
        .unwrap_err();
        assert_eq!(duplicate_value.code(), "DUPLICATE_BASE_URL_VALUE");
    }

    #[test]
    fn preference_store_v1_is_loaded_for_upgrade_and_v2_round_trips() {
        let legacy = br#"{
  "version": 1,
  "providers": {
    "provider-a": {
      "models": ["gpt-5.6-sol"],
      "selectedModel": "gpt-5.6-sol",
      "reasoningEfforts": { "gpt-5.6-sol": "high" }
    }
  }
}
"#;

        let loaded = parse_store(legacy).unwrap();

        assert!(loaded.needs_upgrade);
        assert_eq!(loaded.store.version, 2);
        let migrated = &loaded.store.providers["provider-a"];
        assert!(migrated.base_urls.is_empty());
        assert_eq!(
            migrated
                .model_preference
                .as_ref()
                .map(|preference| preference.selected_model.as_str()),
            Some("gpt-5.6-sol")
        );

        let mut store = ProviderPreferenceStore::default();
        store.providers.insert(
            "provider-a".into(),
            ProviderPrivatePreference {
                base_urls: normalize_named_base_urls(vec![NamedBaseUrl {
                    id: "65c7650d-d20d-4dca-b445-8aa47fcbe92c".into(),
                    name: "主用地址".into(),
                    url: "https://provider-a.example.test/v1".into(),
                }])
                .unwrap(),
                model_preference: Some(
                    ProviderPreference::from_models(&["gpt-5.6-sol".into()]).unwrap(),
                ),
            },
        );

        let bytes = serialize_store(&store).unwrap();
        let reparsed = parse_store(&bytes).unwrap();

        assert!(!reparsed.needs_upgrade);
        assert_eq!(reparsed.store, store);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"version\": 2"));
        assert!(text.contains("\"baseUrls\""));
        assert!(text.contains("\"modelPreference\""));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn legacy_load_is_read_only_and_unknown_or_malformed_versions_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("provider-preferences.json");
        let legacy = br#"{
  "version": 1,
  "providers": {
    "provider-a": {
      "models": ["gpt-5.6-sol"],
      "selectedModel": "gpt-5.6-sol",
      "reasoningEfforts": { "gpt-5.6-sol": "medium" }
    }
  }
}
"#;
        fs::write(&path, legacy).unwrap();
        let service = ProviderPreferenceService::new(path.clone());

        let loaded = service.load_versioned().unwrap();

        assert!(loaded.needs_upgrade);
        assert_eq!(fs::read(&path).unwrap(), legacy);

        let unknown = br#"{"version":3,"providers":{}}"#;
        fs::write(&path, unknown).unwrap();
        let unknown_error = service.load_versioned().unwrap_err();
        assert_eq!(unknown_error.code(), "INVALID_PROVIDER_PREFERENCES");
        assert_eq!(fs::read(&path).unwrap(), unknown);

        let malformed = br#"{"version":2,"providers":"broken"}"#;
        fs::write(&path, malformed).unwrap();
        let malformed_error = service.load_versioned().unwrap_err();
        assert_eq!(malformed_error.code(), "INVALID_PROVIDER_PREFERENCES");
        assert_eq!(fs::read(&path).unwrap(), malformed);
    }
}
