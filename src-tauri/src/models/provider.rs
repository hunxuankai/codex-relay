use serde::{Deserialize, Serialize};

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
}
