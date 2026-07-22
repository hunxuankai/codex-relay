use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const PROVIDER_PREFERENCE_VERSION: u32 = 1;

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreference {
    pub models: Vec<String>,
    pub selected_model: String,
    pub reasoning_efforts: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderPreferenceStore {
    pub version: u32,
    pub providers: BTreeMap<String, ProviderPreference>,
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
    validate_store(store)?;
    let mut json = serde_json::to_string_pretty(store).map_err(AppError::from)?;
    json.push('\n');
    Ok(json.into_bytes())
}

pub fn parse_store(bytes: &[u8]) -> Result<ProviderPreferenceStore, AppError> {
    let store = serde_json::from_slice::<ProviderPreferenceStore>(bytes).map_err(|error| {
        AppError::new(
            "INVALID_PROVIDER_PREFERENCES",
            "无法解析 provider-preferences.json。",
            error.to_string(),
        )
    })?;
    validate_store(&store)?;
    Ok(store)
}

fn validate_store(store: &ProviderPreferenceStore) -> Result<(), AppError> {
    if store.version != PROVIDER_PREFERENCE_VERSION {
        return Err(AppError::new(
            "INVALID_PROVIDER_PREFERENCES",
            "provider-preferences.json 的版本不受支持。",
            format!("unsupported provider preference version: {}", store.version),
        ));
    }
    for preference in store.providers.values() {
        validate_preference(preference)?;
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
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(ProviderPreferenceStore::default());
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
            ProviderPreference::from_models(&["gpt-5.6-sol".into()]).unwrap(),
        );

        let bytes = serialize_store(&store).unwrap();
        let parsed = parse_store(&bytes).unwrap();

        assert_eq!(parsed, store);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"version\": 1"));
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
}
