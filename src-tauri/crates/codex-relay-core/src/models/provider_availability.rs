use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone)]
pub(crate) struct ProviderAvailabilityTarget {
    pub(crate) provider_id: String,
    pub(crate) base_url: String,
    pub(crate) model: String,
    pub(crate) api_key: String,
}

impl fmt::Debug for ProviderAvailabilityTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderAvailabilityTarget")
            .field("provider_id", &self.provider_id)
            .field("base_url_configured", &!self.base_url.is_empty())
            .field("model", &self.model)
            .field("api_key_configured", &!self.api_key.is_empty())
            .finish()
    }
}

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
    #[serde(default)]
    pub trace: Option<ProviderAvailabilityTrace>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAvailabilityTrace {
    pub request: ProviderAvailabilityRequestTrace,
    pub response: Option<ProviderAvailabilityResponseTrace>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAvailabilityRequestTrace {
    pub method: String,
    pub url: String,
    pub body: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAvailabilityResponseTrace {
    pub status: u16,
    pub body: String,
    pub body_truncated: bool,
}

impl fmt::Debug for ProviderAvailabilityTrace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderAvailabilityTrace")
            .field("request_method", &self.request.method)
            .field("request_url_configured", &!self.request.url.is_empty())
            .field("request_body_bytes", &self.request.body.len())
            .field(
                "response_status",
                &self.response.as_ref().map(|response| response.status),
            )
            .field(
                "response_body_bytes",
                &self.response.as_ref().map(|response| response.body.len()),
            )
            .field(
                "response_body_truncated",
                &self
                    .response
                    .as_ref()
                    .is_some_and(|response| response.body_truncated),
            )
            .finish()
    }
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
            trace: Some(ProviderAvailabilityTrace {
                request: ProviderAvailabilityRequestTrace {
                    method: "POST".into(),
                    url: "https://provider.example.test/v1/responses".into(),
                    body: r#"{"model":"gpt-5.6-sol","stream":false}"#.into(),
                },
                response: Some(ProviderAvailabilityResponseTrace {
                    status: 200,
                    body: r#"{"status":"completed"}"#.into(),
                    body_truncated: false,
                }),
            }),
        };

        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains(r#""providerId":"provider-a""#));
        assert!(json.contains(r#""kind":"api""#));
        assert!(json.contains(r#""status":"passed""#));
        assert!(json.contains(r#""durationMs":42"#));
        assert!(json.contains(r#""bodyTruncated":false"#));
        assert!(!json.contains("apiKey"));
        let debug = format!("{result:?}");
        assert!(!debug.contains("completed"));
        assert!(!debug.contains("provider.example.test"));
        assert!(!debug.contains("test-key-a-not-real"));
    }

    #[test]
    fn availability_target_debug_reports_key_presence_without_exposing_it() {
        let target = ProviderAvailabilityTarget {
            provider_id: "provider-a".into(),
            base_url: "https://provider.example.test/v1".into(),
            model: "gpt-5.6-sol".into(),
            api_key: "test-key-target-not-real".into(),
        };

        let debug = format!("{target:?}");

        assert!(debug.contains("api_key_configured: true"));
        assert!(!debug.contains("provider.example.test"));
        assert!(!debug.contains("test-key-target-not-real"));
    }
}
