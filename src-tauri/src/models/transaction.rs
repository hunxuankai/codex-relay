use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionOperation {
    CreateProvider,
    UpdateProvider,
    DeleteProvider,
    SwitchProvider,
    RestoreBackup,
    SyncCurrentProvider,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigTransaction {
    pub id: String,
    pub operation: TransactionOperation,
    pub provider_id: Option<String>,
    pub started_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_serializes_provider_id_in_camel_case() {
        let transaction = ConfigTransaction {
            id: "tx-1".into(),
            operation: TransactionOperation::SwitchProvider,
            provider_id: Some("provider-a".into()),
            started_at: "2026-07-20T22:00:00+08:00".into(),
        };

        let json = serde_json::to_string(&transaction).unwrap();
        assert!(json.contains("providerId"));
        assert!(json.contains("switch_provider"));
    }
}
