use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthLevel {
    Normal,
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    pub id: String,
    pub label: String,
    pub level: HealthLevel,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub level: HealthLevel,
    pub checks: Vec<HealthCheck>,
    pub config_directory: String,
    pub current_provider: Option<String>,
    pub generated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_level_serializes_for_frontend() {
        assert_eq!(
            serde_json::to_string(&HealthLevel::Warning).unwrap(),
            "\"warning\""
        );
    }
}
