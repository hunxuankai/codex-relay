use crate::infrastructure::file_fingerprint::FileSetFingerprint;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WireApi {
    #[default]
    Responses,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBaseUrlSummary {
    pub id: String,
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderApiKeySummary {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderApiKeyManagementEntry {
    pub id: String,
    pub name: String,
    pub api_key: String,
}

impl fmt::Debug for ProviderApiKeyManagementEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderApiKeyManagementEntry")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("api_key_configured", &!self.api_key.is_empty())
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderBaseUrlStatus {
    Managed,
    External,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderApiKeyStatus {
    Managed,
    External,
    Missing,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderApiKeyManagementState {
    pub provider_id: String,
    pub entries: Vec<ProviderApiKeyManagementEntry>,
    pub selected_api_key_id: Option<String>,
    pub api_key_status: ProviderApiKeyStatus,
    pub fingerprints: FileSetFingerprint,
}

impl fmt::Debug for ProviderApiKeyManagementState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderApiKeyManagementState")
            .field("provider_id", &self.provider_id)
            .field("entries", &self.entries)
            .field("selected_api_key_id", &self.selected_api_key_id)
            .field("api_key_status", &self.api_key_status)
            .field("fingerprints", &self.fingerprints)
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub base_urls: Vec<ProviderBaseUrlSummary>,
    pub selected_base_url_id: Option<String>,
    pub base_url_status: ProviderBaseUrlStatus,
    pub api_keys: Vec<ProviderApiKeySummary>,
    pub selected_api_key_id: Option<String>,
    pub api_key_status: ProviderApiKeyStatus,
    pub wire_api: WireApi,
    pub models: Vec<String>,
    pub selected_model: Option<String>,
    pub reasoning_efforts: BTreeMap<String, String>,
    pub fast_enabled: bool,
    pub preference_configured: bool,
    pub api_key_configured: bool,
    pub configuration_complete: bool,
    pub disabled_reason: Option<String>,
    pub is_active: bool,
    pub is_valid: bool,
    pub validation_message: Option<String>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProviderInput {
    pub id: String,
    pub name: String,
    pub base_url_name: String,
    pub base_url: String,
    pub wire_api: String,
    pub models: Vec<String>,
    pub fast_enabled: bool,
    pub api_key_name: String,
    pub api_key: String,
    pub activate_after_save: bool,
    pub expected_files: FileSetFingerprint,
}

impl fmt::Debug for CreateProviderInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreateProviderInput")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("base_url_name", &self.base_url_name)
            .field("base_url", &self.base_url)
            .field("wire_api", &self.wire_api)
            .field("models", &self.models)
            .field("fast_enabled", &self.fast_enabled)
            .field("api_key_name", &self.api_key_name)
            .field("api_key_configured", &!self.api_key.is_empty())
            .field("activate_after_save", &self.activate_after_save)
            .field("expected_files", &self.expected_files)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProviderInput {
    pub id: String,
    pub name: String,
    pub wire_api: String,
    pub models: Vec<String>,
    pub fast_enabled: bool,
    pub sync_if_active: bool,
    pub expected_files: FileSetFingerprint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBaseUrlDraft {
    pub id: Option<String>,
    pub name: String,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderBaseUrlsInput {
    pub provider_id: String,
    pub entries: Vec<ProviderBaseUrlDraft>,
    pub expected_files: FileSetFingerprint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectProviderBaseUrlInput {
    pub provider_id: String,
    pub base_url_id: String,
    pub expected_files: FileSetFingerprint,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderApiKeyDraft {
    pub id: Option<String>,
    pub name: String,
    pub api_key: String,
}

impl fmt::Debug for ProviderApiKeyDraft {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderApiKeyDraft")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("api_key_configured", &!self.api_key.is_empty())
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderApiKeysInput {
    pub provider_id: String,
    pub entries: Vec<ProviderApiKeyDraft>,
    pub expected_files: FileSetFingerprint,
}

impl fmt::Debug for SaveProviderApiKeysInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveProviderApiKeysInput")
            .field("provider_id", &self.provider_id)
            .field("entries", &self.entries)
            .field("expected_files", &self.expected_files)
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectProviderApiKeyInput {
    pub provider_id: String,
    pub api_key_id: String,
    pub expected_files: FileSetFingerprint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportCurrentApiKeyInput {
    pub provider_id: String,
    pub name: String,
    pub expected_files: FileSetFingerprint,
}

impl fmt::Debug for UpdateProviderInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateProviderInput")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("wire_api", &self.wire_api)
            .field("models", &self.models)
            .field("fast_enabled", &self.fast_enabled)
            .field("sync_if_active", &self.sync_if_active)
            .field("expected_files", &self.expected_files)
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderListState {
    pub providers: Vec<ProviderProfile>,
    pub active_provider_id: Option<String>,
    pub current_auth_import_available: bool,
    pub fingerprints: FileSetFingerprint,
    pub model_catalog: Vec<ModelCatalogItem>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogItem {
    pub id: String,
    pub reasoning_efforts: Vec<String>,
    pub default_reasoning_effort: String,
    pub supports_fast: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProviderPreferenceInput {
    pub provider_id: String,
    pub model: String,
    pub reasoning_effort: String,
    pub expected_files: FileSetFingerprint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProviderFastInput {
    pub provider_id: String,
    pub enabled: bool,
    pub expected_files: FileSetFingerprint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderProvidersInput {
    pub provider_ids: Vec<String>,
    pub expected_files: FileSetFingerprint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMutationOutcome {
    pub providers: Vec<ProviderProfile>,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchOutcome {
    pub providers: Vec<ProviderProfile>,
    pub active_provider_id: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_profile_serialization_excludes_api_key() {
        let profile = ProviderProfile {
            id: "provider-a".into(),
            name: "Provider A".into(),
            base_url: "https://provider-a.example.com/v1".into(),
            base_urls: vec![ProviderBaseUrlSummary {
                id: "legacy-default".into(),
                name: "默认地址".into(),
                url: "https://provider-a.example.com/v1".into(),
            }],
            selected_base_url_id: Some("legacy-default".into()),
            base_url_status: ProviderBaseUrlStatus::Managed,
            api_keys: vec![ProviderApiKeySummary {
                id: "legacy-default".into(),
                name: "默认密钥".into(),
            }],
            selected_api_key_id: Some("legacy-default".into()),
            api_key_status: ProviderApiKeyStatus::Managed,
            wire_api: WireApi::Responses,
            models: vec!["gpt-5.6-sol".into()],
            selected_model: Some("gpt-5.6-sol".into()),
            reasoning_efforts: BTreeMap::from([("gpt-5.6-sol".into(), "medium".into())]),
            fast_enabled: false,
            preference_configured: true,
            api_key_configured: true,
            configuration_complete: true,
            disabled_reason: None,
            is_active: true,
            is_valid: true,
            validation_message: None,
        };

        let json = serde_json::to_string(&profile).unwrap();

        assert!(json.contains("apiKeyConfigured"));
        assert!(json.contains("isActive"));
        assert!(json.contains("isValid"));
        assert!(!json.contains("\"apiKey\":"));
        assert!(!json.contains("test-key-a-not-real"));
    }

    #[test]
    fn provider_mutation_debug_output_redacts_keys() {
        let fingerprints = FileSetFingerprint {
            config: crate::infrastructure::file_fingerprint::FileFingerprint::missing(),
            auth: crate::infrastructure::file_fingerprint::FileFingerprint::missing(),
            providers: crate::infrastructure::file_fingerprint::FileFingerprint::missing(),
            preferences: crate::infrastructure::file_fingerprint::FileFingerprint::missing(),
        };
        let create = CreateProviderInput {
            id: "provider-a".into(),
            name: "Provider A".into(),
            base_url_name: "主用地址".into(),
            base_url: "https://provider-a.example.com/v1".into(),
            wire_api: "responses".into(),
            models: vec!["gpt-5.6-sol".into()],
            fast_enabled: false,
            api_key_name: "主用密钥".into(),
            api_key: "test-key-a-not-real".into(),
            activate_after_save: false,
            expected_files: fingerprints.clone(),
        };
        let save_keys = SaveProviderApiKeysInput {
            provider_id: "provider-a".into(),
            entries: vec![ProviderApiKeyDraft {
                id: None,
                name: "主用密钥".into(),
                api_key: "test-key-b-not-real".into(),
            }],
            expected_files: fingerprints,
        };

        assert!(!format!("{create:?}").contains("test-key-a-not-real"));
        assert!(!format!("{save_keys:?}").contains("test-key-b-not-real"));
    }
}
