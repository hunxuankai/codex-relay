use crate::services::provider_service::ProviderAvailabilityTarget;
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

pub(crate) async fn probe_api(
    target: &ProviderAvailabilityTarget,
    proxy: Option<&str>,
    app_version: &str,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<ApiProbeReport, ApiProbeError> {
    if *cancel.borrow() {
        return Err(ApiProbeError::Cancelled);
    }
    let endpoint = responses_endpoint(&target.base_url)?;
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

    let send = client
        .post(endpoint)
        .bearer_auth(&target.api_key)
        .json(&payload)
        .send();
    tokio::pin!(send);
    let response = loop {
        tokio::select! {
            result = &mut send => break result.map_err(map_request_error)?,
            changed = cancel.changed() => {
                if changed.is_ok() && *cancel.borrow() {
                    return Err(ApiProbeError::Cancelled);
                }
                if changed.is_err() {
                    break send.await.map_err(map_request_error)?;
                }
            }
        }
    };
    let status = response.status();
    if !status.is_success() {
        return Err(map_http_status(status));
    }

    let body = read_bounded_body(response, cancel).await?;
    let document: Value =
        serde_json::from_slice(&body).map_err(|_| ApiProbeError::ResponseInvalid)?;
    let completed = document.get("status").and_then(Value::as_str) == Some("completed");
    let has_output = document.get("output").is_some_and(Value::is_array);
    if !completed || !has_output {
        return Err(ApiProbeError::ResponseInvalid);
    }

    Ok(ApiProbeReport {
        http_status: status.as_u16(),
    })
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

async fn read_bounded_body(
    mut response: reqwest::Response,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
) -> Result<Vec<u8>, ApiProbeError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        return Err(ApiProbeError::ResponseTooLarge);
    }

    let mut body = Vec::new();
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
            return Err(ApiProbeError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
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

    #[tokio::test]
    async fn classifies_http_failures_without_reading_provider_error_body() {
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
            assert_eq!(error, expected);
            let request = server.join().unwrap();
            assert!(!request.contains("secret body"));
        }
    }

    #[tokio::test]
    async fn rejects_non_responses_body_and_oversized_body() {
        let (base_url, server) = response_server(200, r#"{"choices":[]}"#.into());
        let (_cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
        assert_eq!(
            probe_api(&target(base_url), None, "0.1.0", &mut cancel)
                .await
                .unwrap_err(),
            ApiProbeError::ResponseInvalid
        );
        server.join().unwrap();

        let (base_url, server) = response_server(200, "x".repeat(MAX_RESPONSE_BYTES + 1));
        let (_cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
        assert_eq!(
            probe_api(&target(base_url), None, "0.1.0", &mut cancel)
                .await
                .unwrap_err(),
            ApiProbeError::ResponseTooLarge
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn does_not_follow_redirects() {
        let (base_url, server) =
            response_server(302, r#"{"location":"https://outside.test"}"#.into());
        let (_cancel_sender, mut cancel) = tokio::sync::watch::channel(false);
        assert_eq!(
            probe_api(&target(base_url), None, "0.1.0", &mut cancel)
                .await
                .unwrap_err(),
            ApiProbeError::Http(302)
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

        assert_eq!(
            request.await.unwrap().unwrap_err(),
            ApiProbeError::Cancelled
        );
        server.join().unwrap();
    }
}
