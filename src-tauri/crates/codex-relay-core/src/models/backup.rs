use serde::{Deserialize, Serialize};

pub const BACKUP_METADATA_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BackupFileName {
    #[serde(rename = "config.toml")]
    Config,
    #[serde(rename = "auth.json")]
    Auth,
    #[serde(rename = "providers.json")]
    Providers,
    #[serde(rename = "provider-preferences.json")]
    Preferences,
    #[serde(rename = "metadata.json")]
    Metadata,
}

impl BackupFileName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Config => "config.toml",
            Self::Auth => "auth.json",
            Self::Providers => "providers.json",
            Self::Preferences => "provider-preferences.json",
            Self::Metadata => "metadata.json",
        }
    }

    pub fn existed_in(self, metadata: &BackupMetadata) -> bool {
        match self {
            Self::Config => metadata.config_existed,
            Self::Auth => metadata.auth_existed,
            Self::Providers => metadata.providers_existed,
            Self::Preferences => metadata.preferences_existed,
            Self::Metadata => true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupMetadata {
    pub schema_version: u32,
    pub transaction_id: String,
    pub created_at: String,
    pub operation: String,
    pub provider_id: Option<String>,
    pub config_existed: bool,
    pub auth_existed: bool,
    pub providers_existed: bool,
    pub preferences_existed: bool,
    pub app_version: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupCompatibility {
    Current,
    LegacyWithoutPreferences,
}

impl BackupMetadata {
    pub fn files(&self) -> Vec<BackupFileName> {
        let mut files = Vec::with_capacity(5);
        if self.config_existed {
            files.push(BackupFileName::Config);
        }
        if self.auth_existed {
            files.push(BackupFileName::Auth);
        }
        if self.providers_existed {
            files.push(BackupFileName::Providers);
        }
        if self.preferences_existed {
            files.push(BackupFileName::Preferences);
        }
        files.push(BackupFileName::Metadata);
        files
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub directory_name: String,
    pub metadata: BackupMetadata,
    pub files: Vec<BackupFileName>,
    pub compatibility: BackupCompatibility,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInventory {
    pub backups: Vec<BackupSummary>,
    pub unavailable_backups: Vec<UnavailableBackup>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnavailableBackup {
    pub directory_name: String,
    pub code: String,
    pub message: String,
    pub can_open_metadata: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_metadata_does_not_have_secret_fields() {
        let metadata = BackupMetadata {
            schema_version: BACKUP_METADATA_SCHEMA_VERSION,
            transaction_id: "tx-1".into(),
            created_at: "2026-07-20T22:00:00+08:00".into(),
            operation: "switch_provider".into(),
            provider_id: Some("provider-a".into()),
            config_existed: true,
            auth_existed: true,
            providers_existed: true,
            preferences_existed: true,
            app_version: "0.1.0".into(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(!json.contains("apiKey"));
        assert!(!json.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn backup_file_name_rejects_unknown_values() {
        let error = serde_json::from_str::<BackupFileName>("\"outside.txt\"").unwrap_err();

        assert!(error.to_string().contains("unknown variant"));
    }
}
