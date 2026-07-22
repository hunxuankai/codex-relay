use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderTestKind {
    Api,
    Codex,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderTestStatus {
    Passed,
    Failed,
    Unsupported,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAvailabilityResult {
    pub provider_id: String,
    pub kind: ProviderTestKind,
    pub status: ProviderTestStatus,
    pub code: String,
    pub message: String,
    pub model: String,
    pub duration_ms: u64,
    pub tested_at: String,
    pub http_status: Option<u16>,
    pub codex_version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn availability_result_serializes_stable_safe_fields() {
        let result = ProviderAvailabilityResult {
            provider_id: "provider-a".into(),
            kind: ProviderTestKind::Api,
            status: ProviderTestStatus::Passed,
            code: "API_TEST_PASSED".into(),
            message: "API 可用性测试通过。".into(),
            model: "gpt-5.6-sol".into(),
            duration_ms: 42,
            tested_at: "2026-07-23T12:00:00Z".into(),
            http_status: Some(200),
            codex_version: None,
        };

        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains(r#""providerId":"provider-a""#));
        assert!(json.contains(r#""kind":"api""#));
        assert!(json.contains(r#""status":"passed""#));
        assert!(json.contains(r#""durationMs":42"#));
        assert!(!json.contains("apiKey"));
        assert!(!format!("{result:?}").contains("test-key-a-not-real"));
    }
}
