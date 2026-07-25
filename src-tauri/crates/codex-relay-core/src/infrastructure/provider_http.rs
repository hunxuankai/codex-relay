use crate::infrastructure::rustls_provider::ensure_ring_crypto_provider;
use crate::infrastructure::safe_log::redact;
use crate::models::provider_availability::{
    ProviderAvailabilityRequestTrace, ProviderAvailabilityResponseTrace,
    ProviderAvailabilityTarget, ProviderAvailabilityTrace,
};
use reqwest::{Client, Proxy, StatusCode, redirect::Policy};
use serde_json::Value;
use std::time::Duration;
use url::Url;

const API_TIMEOUT: Duration = Duration::from_secs(30);
const API_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApiProbeReport {
    pub(crate) http_status: u16,
    pub(crate) trace: ProviderAvailabilityTrace,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApiProbeFailure {
    pub(crate) error: ApiProbeError,
    pub(crate) trace: Option<ProviderAvailabilityTrace>,
    pub(crate) http_status: Option<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ApiProbeError {
    InvalidEndpoint,
    RequestBuild,
    Network,
    Tls,
    Timeout,
    Auth,
    EndpointOrModelNotFound,
    RateLimited,
    Provider,
    Http(u16),
    ResponseTooLarge,
    ResponseInvalid,
    Cancelled,
}

impl From<ApiProbeError> for ApiProbeFailure {
    fn from(error: ApiProbeError) -> Self {
        Self {
            error,
            trace: None,
            http_status: None,
        }
    }
}

impl ApiProbeFailure {
    fn with_trace(error: ApiProbeError, trace: ProviderAvailabilityTrace) -> Self {
        let http_status = trace.response.as_ref().map(|response| response.status);
        Self {
            error,
            trace: Some(trace),
            http_status,
        }
    }
}

pub(crate) async fn probe_api(
    target: &ProviderAvailabilityTarget,
    proxy: Option<&str>,
    app_version: &str,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<ApiProbeReport, ApiProbeFailure> {
    if *cancel.borrow() {
        return Err(ApiProbeError::Cancelled.into());
    }
    let endpoint = responses_endpoint(&target.base_url)?;
    ensure_ring_crypto_provider().map_err(|_| ApiProbeError::RequestBuild)?;
    let mut builder = Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(API_CONNECT_TIMEOUT)
        .timeout(API_TIMEOUT)
        .user_agent(format!("CodexRelay/{app_version}"));
    if let Some(proxy) = proxy {
        builder = builder.proxy(Proxy::all(proxy).map_err(|_| ApiProbeError::RequestBuild)?);
    }
    let client = builder.build().map_err(|_| ApiProbeError::RequestBuild)?;
    let payload = serde_json::json!({
        "model": target.model,
        "input": "Reply with exactly OK.",
        "max_output_tokens": 16,
        "stream": false,
    });
    let request_body = serde_json::to_string(&payload).map_err(|_| ApiProbeError::RequestBuild)?;
    let request_trace = ProviderAvailabilityRequestTrace {
        method: "POST".into(),
        url: sanitize_trace_url(&endpoint),
        body: request_body,
    };

    let send = client
        .post(endpoint.clone())
        .bearer_auth(&target.api_key)
        .json(&payload)
        .send();
    tokio::pin!(send);
    let response = loop {
        tokio::select! {
            result = &mut send => break result.map_err(|error| {
                ApiProbeFailure::with_trace(
                    map_request_error(error),
                    ProviderAvailabilityTrace {
                        request: request_trace.clone(),
                        response: None,
                    },
                )
            })?,
            changed = cancel.changed() => {
                if changed.is_ok() && *cancel.borrow() {
                    return Err(ApiProbeFailure::with_trace(
                        ApiProbeError::Cancelled,
                        ProviderAvailabilityTrace {
                            request: request_trace.clone(),
                            response: None,
                        },
                    ));
                }
                if changed.is_err() {
                    break send.await.map_err(|error| {
                        ApiProbeFailure::with_trace(
                            map_request_error(error),
                            ProviderAvailabilityTrace {
                                request: request_trace.clone(),
                                response: None,
                            },
                        )
                    })?;
                }
            }
        }
    };
    let status = response.status();
    let body = read_bounded_body(response, cancel).await.map_err(|error| {
        ApiProbeFailure::with_trace(
            error,
            ProviderAvailabilityTrace {
                request: request_trace.clone(),
                response: Some(ProviderAvailabilityResponseTrace {
                    status: status.as_u16(),
                    body: String::new(),
                    body_truncated: false,
                }),
            },
        )
    })?;
    let (response_body, response_body_truncated) =
        sanitize_trace_body(&body.bytes, &target.api_key);
    let response_trace = ProviderAvailabilityResponseTrace {
        status: status.as_u16(),
        body: response_body,
        body_truncated: body.truncated || response_body_truncated,
    };
    let trace = ProviderAvailabilityTrace {
        request: request_trace,
        response: Some(response_trace),
    };
    if body.truncated {
        return Err(ApiProbeFailure::with_trace(
            ApiProbeError::ResponseTooLarge,
            trace,
        ));
    }
    if !status.is_success() {
        return Err(ApiProbeFailure::with_trace(map_http_status(status), trace));
    }

    let document: Value = serde_json::from_slice(&body.bytes)
        .map_err(|_| ApiProbeFailure::with_trace(ApiProbeError::ResponseInvalid, trace.clone()))?;
    let completed = document.get("status").and_then(Value::as_str) == Some("completed");
    let has_output = document.get("output").is_some_and(Value::is_array);
    if !completed || !has_output {
        return Err(ApiProbeFailure::with_trace(
            ApiProbeError::ResponseInvalid,
            trace,
        ));
    }

    Ok(ApiProbeReport {
        http_status: status.as_u16(),
        trace,
    })
}

fn sanitize_trace_body(body: &[u8], api_key: &str) -> (String, bool) {
    let body = String::from_utf8_lossy(body);
    let replaced = if api_key.is_empty() {
        body.into_owned()
    } else {
        body.replace(api_key, "[API Key 已隐藏]")
    };
    let mut sanitized = redact(&replaced);
    if sanitized.len() <= MAX_RESPONSE_BYTES {
        return (sanitized, false);
    }
    let mut end = MAX_RESPONSE_BYTES;
    while !sanitized.is_char_boundary(end) {
        end -= 1;
    }
    sanitized.truncate(end);
    (sanitized, true)
}

fn sanitize_trace_url(endpoint: &Url) -> String {
    let mut sanitized = endpoint.clone();
    let _ = sanitized.set_username("");
    let _ = sanitized.set_password(None);
    redact(sanitized.as_str())
}

pub(crate) fn responses_endpoint(base_url: &str) -> Result<Url, ApiProbeError> {
    let mut url = Url::parse(base_url).map_err(|_| ApiProbeError::InvalidEndpoint)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(ApiProbeError::InvalidEndpoint);
    }
    if url.fragment().is_some() {
        return Err(ApiProbeError::InvalidEndpoint);
    }
    let path = url.path().trim_end_matches('/');
    let next_path = if path.is_empty() {
        "/responses".to_owned()
    } else if path.ends_with("/responses") {
        path.to_owned()
    } else {
        format!("{path}/responses")
    };
    url.set_path(&next_path);
    Ok(url)
}

struct BoundedBody {
    bytes: Vec<u8>,
    truncated: bool,
}

async fn read_bounded_body(
    mut response: reqwest::Response,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<BoundedBody, ApiProbeError> {
    let declared_too_large = response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64);
    let mut body = Vec::with_capacity(
        response
            .content_length()
            .map(|length| (length as usize).min(MAX_RESPONSE_BYTES))
            .unwrap_or_default(),
    );
    loop {
        let chunk = tokio::select! {
            result = response.chunk() => result.map_err(map_request_error)?,
            changed = cancel.changed() => {
                if changed.is_ok() && *cancel.borrow() {
                    return Err(ApiProbeError::Cancelled);
                }
                if changed.is_err() {
                    response.chunk().await.map_err(map_request_error)?
                } else {
                    continue;
                }
            }
        };
        let Some(chunk) = chunk else {
            break;
        };
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            let remaining = MAX_RESPONSE_BYTES.saturating_sub(body.len());
            body.extend_from_slice(&chunk[..remaining]);
            return Ok(BoundedBody {
                bytes: body,
                truncated: true,
            });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(BoundedBody {
        bytes: body,
        truncated: declared_too_large,
    })
}

fn map_request_error(error: reqwest::Error) -> ApiProbeError {
    if error.is_timeout() {
        return ApiProbeError::Timeout;
    }
    if error.is_connect() {
        let detail = error.to_string().to_ascii_lowercase();
        if detail.contains("certificate") || detail.contains("tls") {
            return ApiProbeError::Tls;
        }
        return ApiProbeError::Network;
    }
    if error.is_builder() {
        return ApiProbeError::RequestBuild;
    }
    ApiProbeError::Network
}

fn map_http_status(status: StatusCode) -> ApiProbeError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ApiProbeError::Auth,
        StatusCode::NOT_FOUND => ApiProbeError::EndpointOrModelNotFound,
        StatusCode::TOO_MANY_REQUESTS => ApiProbeError::RateLimited,
        status if status.is_server_error() => ApiProbeError::Provider,
        status => ApiProbeError::Http(status.as_u16()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread::{self, JoinHandle};

    fn target(base_url: String) -> ProviderAvailabilityTarget {
        ProviderAvailabilityTarget {
            provider_id: "provider-a".into(),
            base_url,
            model: "gpt-5.6-sol".into(),
            api_key: "test-key-api-not-real".into(),
        }
    }

    fn response_server(status: u16, body: String) -> (String, JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = stream.read(&mut buffer).unwrap();
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8(request).unwrap()
        });
        (format!("http://{address}/v1"), handle)
    }

    #[tokio::test]
    async fn sends_minimal_responses_request_with_bearer_auth() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = stream.read(&mut buffer).unwrap();
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("POST /v1/responses HTTP/1.1"));
            let lower_request = request.to_ascii_lowercase();
            assert!(lower_request.contains("authorization: bearer test-key-api-not-real"));
            assert!(lower_request.contains("content-type: application/json"));
            assert!(!lower_request.contains("\"tools\""));
            let body = r#"{"id":"resp_test","status":"completed","output":[]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let target = ProviderAvailabilityTarget {
            provider_id: "provider-a".into(),
            base_url: format!("http://{address}/v1"),
            model: "gpt-5.6-sol".into(),
            api_key: "test-key-api-not-real".into(),
        };

        let (_cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
        let report = probe_api(&target, None, "0.1.0", &mut cancel)
            .await
            .unwrap();

        assert_eq!(report.http_status, 200);
        assert_eq!(report.trace.request.method, "POST");
        assert_eq!(
            report.trace.request.url,
            format!("http://{address}/v1/responses")
        );
        let request_body: Value = serde_json::from_str(&report.trace.request.body).unwrap();
        assert_eq!(request_body["model"], "gpt-5.6-sol");
        assert_eq!(request_body["stream"], false);
        let response = report.trace.response.expect("successful response trace");
        assert_eq!(response.status, 200);
        assert!(response.body.contains("resp_test"));
        assert!(!response.body.contains("test-key-api-not-real"));
        assert!(!response.body_truncated);
        server.join().unwrap();
    }

    #[test]
    fn responses_endpoint_preserves_base_path_and_rejects_fragments() {
        assert_eq!(
            responses_endpoint("https://provider.example.test/v1/")
                .unwrap()
                .as_str(),
            "https://provider.example.test/v1/responses"
        );
        assert_eq!(
            responses_endpoint("https://provider.example.test/v1/responses")
                .unwrap()
                .as_str(),
            "https://provider.example.test/v1/responses"
        );
        assert_eq!(
            responses_endpoint("https://provider.example.test/v1/responses/")
                .unwrap()
                .as_str(),
            "https://provider.example.test/v1/responses"
        );
        assert_eq!(
            responses_endpoint("https://provider.example.test/v1#secret").unwrap_err(),
            ApiProbeError::InvalidEndpoint
        );
    }

    #[test]
    fn trace_url_omits_userinfo_and_query_secrets() {
        let endpoint = Url::parse(
            "https://test-key-user-not-real:test-key-password-not-real@provider.example.test/v1/responses?api_key=test-key-query-not-real&region=cn",
        )
        .unwrap();

        let trace_url = sanitize_trace_url(&endpoint);

        assert!(trace_url.contains("provider.example.test/v1/responses"));
        assert!(trace_url.contains("region=cn"));
        assert!(!trace_url.contains("test-key-user-not-real"));
        assert!(!trace_url.contains("test-key-password-not-real"));
        assert!(!trace_url.contains("test-key-query-not-real"));
    }

    #[test]
    fn final_trace_body_remains_within_utf8_limit_after_sanitizing() {
        let raw = vec![0xff; MAX_RESPONSE_BYTES];

        let (body, truncated) = sanitize_trace_body(&raw, "test-key-body-not-real");

        assert!(truncated);
        assert!(body.len() <= MAX_RESPONSE_BYTES);
        assert!(body.is_char_boundary(body.len()));
    }

    #[tokio::test]
    async fn classifies_http_failures_and_captures_provider_error_body() {
        for (status, expected) in [
            (401, ApiProbeError::Auth),
            (429, ApiProbeError::RateLimited),
            (500, ApiProbeError::Provider),
            (418, ApiProbeError::Http(418)),
        ] {
            let (base_url, server) =
                response_server(status, r#"{"error":{"message":"secret body"}}"#.into());
            let (_cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
            let error = probe_api(&target(base_url), None, "0.1.0", &mut cancel)
                .await
                .unwrap_err();
            assert_eq!(error.error, expected);
            let trace = error.trace.expect("HTTP error trace");
            let response = trace.response.expect("HTTP error response");
            assert_eq!(response.status, status);
            assert!(response.body.contains("secret body"));
            assert!(!response.body.contains("test-key-api-not-real"));
            let request = server.join().unwrap();
            assert!(!request.contains("secret body"));
        }
    }

    #[tokio::test]
    async fn rejects_non_responses_body_and_oversized_body() {
        let (base_url, server) = response_server(200, r#"{"choices":[]}"#.into());
        let (_cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
        let error = probe_api(&target(base_url), None, "0.1.0", &mut cancel)
            .await
            .unwrap_err();
        assert_eq!(error.error, ApiProbeError::ResponseInvalid);
        assert!(
            error
                .trace
                .as_ref()
                .and_then(|trace| trace.response.as_ref())
                .is_some_and(|response| response.body.contains("choices"))
        );
        server.join().unwrap();

        let (base_url, server) = response_server(200, "x".repeat(MAX_RESPONSE_BYTES + 1));
        let (_cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
        let error = probe_api(&target(base_url), None, "0.1.0", &mut cancel)
            .await
            .unwrap_err();
        assert_eq!(error.error, ApiProbeError::ResponseTooLarge);
        assert!(
            error
                .trace
                .as_ref()
                .and_then(|trace| trace.response.as_ref())
                .is_some_and(|response| response.body_truncated)
        );
        let response = error
            .trace
            .as_ref()
            .and_then(|trace| trace.response.as_ref())
            .expect("oversized response trace");
        assert_eq!(response.body.len(), MAX_RESPONSE_BYTES);
        assert!(response.body.chars().all(|character| character == 'x'));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn does_not_follow_redirects() {
        let (base_url, server) =
            response_server(302, r#"{"location":"https://outside.test"}"#.into());
        let (_cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
        let error = probe_api(&target(base_url), None, "0.1.0", &mut cancel)
            .await
            .unwrap_err();
        assert_eq!(error.error, ApiProbeError::Http(302));
        assert_eq!(
            error
                .trace
                .as_ref()
                .and_then(|trace| trace.response.as_ref())
                .map(|response| response.status),
            Some(302)
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn cancellation_stops_a_hanging_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            thread::sleep(Duration::from_secs(1));
        });
        let target = target(format!("http://{address}/v1"));
        let (cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
        let request =
            tokio::spawn(async move { probe_api(&target, None, "0.1.0", &mut cancel).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel_sender.send(true).unwrap();

        let error = request.await.unwrap().unwrap_err();
        assert_eq!(error.error, ApiProbeError::Cancelled);
        assert!(
            error
                .trace
                .as_ref()
                .is_some_and(|trace| trace.response.is_none())
        );
        server.join().unwrap();
    }
}
