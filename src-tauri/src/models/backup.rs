use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupMetadata {
    pub transaction_id: String,
    pub created_at: String,
    pub operation: String,
    pub provider_id: Option<String>,
    pub config_existed: bool,
    pub auth_existed: bool,
    pub providers_existed: bool,
    pub app_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub directory_name: String,
    pub metadata: BackupMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_metadata_does_not_have_secret_fields() {
        let metadata = BackupMetadata {
            transaction_id: "tx-1".into(),
            created_at: "2026-07-20T22:00:00+08:00".into(),
            operation: "switch_provider".into(),
            provider_id: Some("provider-a".into()),
            config_existed: true,
            auth_existed: true,
            providers_existed: true,
            app_version: "0.1.0".into(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(!json.contains("apiKey"));
        assert!(!json.contains("OPENAI_API_KEY"));
    }
}
