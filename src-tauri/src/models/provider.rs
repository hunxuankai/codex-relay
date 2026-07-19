use crate::infrastructure::file_fingerprint::FileSetFingerprint;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WireApi {
    #[default]
    Responses,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub wire_api: WireApi,
    pub model: Option<String>,
    pub api_key_configured: bool,
    pub is_active: bool,
    pub is_valid: bool,
    pub validation_message: Option<String>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", content = "value", rename_all = "camelCase")]
pub enum ApiKeyChange {
    Unchanged,
    Set(String),
    Clear,
}

impl fmt::Debug for ApiKeyChange {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unchanged => formatter.write_str("Unchanged"),
            Self::Set(value) => formatter
                .debug_struct("Set")
                .field("configured", &!value.is_empty())
                .finish(),
            Self::Clear => formatter.write_str("Clear"),
        }
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProviderInput {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub wire_api: String,
    pub model: Option<String>,
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
            .field("base_url", &self.base_url)
            .field("wire_api", &self.wire_api)
            .field("model", &self.model)
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
    pub base_url: String,
    pub wire_api: String,
    pub model: Option<String>,
    pub api_key_change: ApiKeyChange,
    pub sync_if_active: bool,
    pub expected_files: FileSetFingerprint,
}

impl fmt::Debug for UpdateProviderInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateProviderInput")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("base_url", &self.base_url)
            .field("wire_api", &self.wire_api)
            .field("model", &self.model)
            .field("api_key_change", &self.api_key_change)
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
            wire_api: WireApi::Responses,
            model: Some("test-model".into()),
            api_key_configured: true,
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
        };
        let create = CreateProviderInput {
            id: "provider-a".into(),
            name: "Provider A".into(),
            base_url: "https://provider-a.example.com/v1".into(),
            wire_api: "responses".into(),
            model: None,
            api_key: "test-key-a-not-real".into(),
            activate_after_save: false,
            expected_files: fingerprints,
        };

        assert!(!format!("{create:?}").contains("test-key-a-not-real"));
        assert!(
            !format!("{:?}", ApiKeyChange::Set("test-key-b-not-real".into()))
                .contains("test-key-b-not-real")
        );
    }
}
