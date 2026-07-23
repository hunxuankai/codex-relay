use crate::infrastructure::codex_preflight::{
    MAX_PREFLIGHT_HEADER_BYTES, MAX_PREFLIGHT_REQUEST_BYTES, PreflightExpectation,
    validate_preflight_request,
};
use crate::infrastructure::provider_http::responses_endpoint;
use crate::infrastructure::rustls_provider::ensure_ring_crypto_provider;
use crate::services::provider_service::ProviderAvailabilityTarget;
use reqwest::{Client, Proxy, redirect::Policy};
use serde_json::Value;
use std::fmt;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

pub(crate) const GATEWAY_API_KEY: &str = "test-key-codex-gateway-not-real";
const GATEWAY_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_GATEWAY_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum GatewayError {
    #[error("兼容性转发层请求被拒绝")]
    RequestRejected,
    #[error("兼容性转发层检测到工具调用")]
    ToolCallBlocked,
    #[error("上游 Provider Responses 流无效")]
    UpstreamInvalid,
    #[error("无法连接上游 Provider")]
    UpstreamNetwork,
    #[error("兼容性转发层超时")]
    Timeout,
    #[error("兼容性转发层已取消")]
    Cancelled,
    #[error("兼容性转发层 I/O 失败")]
    Io,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GatewayOutcome {
    Passed,
    ToolCallBlocked,
    RequestRejected,
    UpstreamHttp(u16),
    UpstreamInvalid,
    Cancelled,
}

pub(crate) struct CodexCompatibilityGateway {
    base_url: String,
    shutdown: watch::Sender<bool>,
    outcome: Option<oneshot::Receiver<Result<GatewayOutcome, GatewayError>>>,
    task: Option<JoinHandle<()>>,
}

impl fmt::Debug for CodexCompatibilityGateway {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodexCompatibilityGateway")
            .field("base_url", &self.base_url)
            .field("running", &self.task.is_some())
            .finish()
    }
}

impl CodexCompatibilityGateway {
    pub(crate) async fn start(
        target: ProviderAvailabilityTarget,
        proxy: Option<&str>,
    ) -> Result<Self, GatewayError> {
        ensure_ring_crypto_provider().map_err(|_| GatewayError::UpstreamNetwork)?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| GatewayError::Io)?;
        let address = listener.local_addr().map_err(|_| GatewayError::Io)?;
        let mut client_builder = Client::builder()
            .redirect(Policy::none())
            .no_proxy()
            .timeout(GATEWAY_TIMEOUT);
        if let Some(proxy) = proxy {
            client_builder =
                client_builder.proxy(Proxy::all(proxy).map_err(|_| GatewayError::UpstreamNetwork)?);
        }
        let client = client_builder
            .build()
            .map_err(|_| GatewayError::UpstreamNetwork)?;
        let (shutdown, shutdown_receiver) = watch::channel(false);
        let (outcome_sender, outcome) = oneshot::channel();
        let task = tokio::spawn(run_gateway_server(
            listener,
            target,
            client,
            shutdown_receiver,
            outcome_sender,
        ));
        Ok(Self {
            base_url: format!("http://{address}"),
            shutdown,
            outcome: Some(outcome),
            task: Some(task),
        })
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) async fn wait(mut self, timeout: Duration) -> Result<GatewayOutcome, GatewayError> {
        let receiver = self.outcome.take().ok_or(GatewayError::Io)?;
        let result = match tokio::time::timeout(timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(GatewayError::Io),
            Err(_) => Err(GatewayError::Timeout),
        };
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            if matches!(result, Err(GatewayError::Timeout | GatewayError::Cancelled)) {
                task.abort();
            } else {
                let _ = task.await;
            }
        }
        result
    }
}

impl Drop for CodexCompatibilityGateway {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn run_gateway_server(
    listener: TcpListener,
    target: ProviderAvailabilityTarget,
    client: Client,
    mut shutdown: watch::Receiver<bool>,
    outcome_sender: oneshot::Sender<Result<GatewayOutcome, GatewayError>>,
) {
    let result = match tokio::select! {
        accepted = listener.accept() => accepted.map(|(stream, _)| stream).map_err(|_| GatewayError::Io),
        changed = shutdown.changed() => {
            if changed.is_ok() && *shutdown.borrow() { Err(GatewayError::Cancelled) } else { Err(GatewayError::Io) }
        }
    } {
        Ok(mut stream) => {
            handle_gateway_connection(&mut stream, &target, &client, &mut shutdown).await
        }
        Err(error) => Err(error),
    };
    let _ = outcome_sender.send(result);
}

async fn handle_gateway_connection(
    stream: &mut TcpStream,
    target: &ProviderAvailabilityTarget,
    client: &Client,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<GatewayOutcome, GatewayError> {
    let request = read_request(stream).await?;
    let expectation = PreflightExpectation::new(target.model.clone()).with_api_key(GATEWAY_API_KEY);
    if validate_preflight_request(
        &request.method,
        &request.path,
        &request.headers,
        &request.body,
        &expectation,
    )
    .is_err()
    {
        write_generic_response(stream, "400 Bad Request", "兼容性测试请求不符合安全契约").await?;
        return Ok(GatewayOutcome::RequestRejected);
    }

    let endpoint =
        responses_endpoint(&target.base_url).map_err(|_| GatewayError::RequestRejected)?;
    let request_future = client
        .post(endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .bearer_auth(&target.api_key)
        .body(request.body)
        .send();
    let response = tokio::select! {
        response = request_future => response.map_err(|_| GatewayError::UpstreamNetwork)?,
        changed = shutdown.changed() => {
            if changed.is_ok() && *shutdown.borrow() { return Ok(GatewayOutcome::Cancelled); }
            return Err(GatewayError::Cancelled);
        }
    };
    if !response.status().is_success() {
        let status = response.status().as_u16();
        write_generic_response(stream, status_line(status), "Provider 返回了错误").await?;
        return Ok(GatewayOutcome::UpstreamHttp(status));
    }
    if !response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/event-stream"))
    {
        write_generic_response(stream, "502 Bad Gateway", "Provider Responses 流格式无效").await?;
        return Ok(GatewayOutcome::UpstreamInvalid);
    }
    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        )
        .await
        .map_err(|_| GatewayError::Io)?;

    let mut response = response;
    let mut scanner = SseScanner::default();
    let mut total_bytes = 0usize;
    loop {
        let chunk = tokio::select! {
            chunk = response.chunk() => chunk.map_err(|_| GatewayError::UpstreamNetwork)?,
            changed = shutdown.changed() => {
                if changed.is_ok() && *shutdown.borrow() { return Ok(GatewayOutcome::Cancelled); }
                return Err(GatewayError::Cancelled);
            }
        };
        let Some(chunk) = chunk else { break };
        total_bytes = total_bytes.saturating_add(chunk.len());
        if total_bytes > MAX_GATEWAY_RESPONSE_BYTES {
            write_generic_response(stream, "502 Bad Gateway", "Provider 响应超过安全大小上限")
                .await?;
            return Ok(GatewayOutcome::UpstreamInvalid);
        }
        let events = match scanner.push_chunk(&chunk) {
            Ok(events) => events,
            Err(GatewayError::ToolCallBlocked) => {
                // Do not forward the triggering SSE event to Codex.
                write_generic_response(stream, "502 Bad Gateway", "兼容性测试检测到工具调用")
                    .await?;
                return Ok(GatewayOutcome::ToolCallBlocked);
            }
            Err(_) => {
                write_generic_response(stream, "502 Bad Gateway", "Provider Responses 流格式无效")
                    .await?;
                return Ok(GatewayOutcome::UpstreamInvalid);
            }
        };
        for event in events {
            stream
                .write_all(&event)
                .await
                .map_err(|_| GatewayError::Io)?;
        }
    }
    scanner
        .finish()
        .map_err(|_| GatewayError::UpstreamInvalid)?;
    stream.flush().await.map_err(|_| GatewayError::Io)?;
    let _ = stream.shutdown().await;
    Ok(GatewayOutcome::Passed)
}

#[derive(Debug)]
struct GatewayRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

async fn read_request(stream: &mut TcpStream) -> Result<GatewayRequest, GatewayError> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|_| GatewayError::Io)?;
        if read == 0 {
            return Err(GatewayError::RequestRejected);
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() > MAX_PREFLIGHT_HEADER_BYTES {
            return Err(GatewayError::RequestRejected);
        }
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let header_text =
        std::str::from_utf8(&bytes[..header_end]).map_err(|_| GatewayError::RequestRejected)?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or(GatewayError::RequestRejected)?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or(GatewayError::RequestRejected)?
        .to_owned();
    let path = parts
        .next()
        .ok_or(GatewayError::RequestRejected)?
        .to_owned();
    if parts.next() != Some("HTTP/1.1") || parts.next().is_some() {
        return Err(GatewayError::RequestRejected);
    }
    let mut headers = Vec::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or(GatewayError::RequestRejected)?;
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .ok_or(GatewayError::RequestRejected)?;
    if length > MAX_PREFLIGHT_REQUEST_BYTES {
        return Err(GatewayError::RequestRejected);
    }
    let total = header_end
        .checked_add(length)
        .ok_or(GatewayError::RequestRejected)?;
    while bytes.len() < total {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|_| GatewayError::Io)?;
        if read == 0 {
            return Err(GatewayError::RequestRejected);
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() > total {
            return Err(GatewayError::RequestRejected);
        }
    }
    Ok(GatewayRequest {
        method,
        path,
        headers,
        body: bytes[header_end..total].to_vec(),
    })
}

async fn write_generic_response(
    stream: &mut TcpStream,
    status: &str,
    message: &str,
) -> Result<(), GatewayError> {
    let body = format!(r#"{{"error":"{message}"}}"#);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|_| GatewayError::Io)?;
    stream.flush().await.map_err(|_| GatewayError::Io)?;
    let _ = stream.shutdown().await;
    Ok(())
}

fn status_line(status: u16) -> &'static str {
    match status {
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        429 => "429 Too Many Requests",
        500 => "500 Internal Server Error",
        502 => "502 Bad Gateway",
        503 => "503 Service Unavailable",
        _ => "502 Bad Gateway",
    }
}

#[derive(Default)]
pub(crate) struct SseScanner {
    pending: Vec<u8>,
    completed: bool,
}

impl SseScanner {
    pub(crate) fn push_chunk(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, GatewayError> {
        self.pending.extend_from_slice(chunk);
        if self.pending.len() > MAX_GATEWAY_RESPONSE_BYTES {
            return Err(GatewayError::UpstreamInvalid);
        }
        let mut events = Vec::new();
        while let Some((end, delimiter_len)) = find_sse_event_end(&self.pending) {
            let event = self
                .pending
                .drain(..end + delimiter_len)
                .collect::<Vec<_>>();
            inspect_sse_event(&event, &mut self.completed)?;
            events.push(event);
        }
        Ok(events)
    }

    pub(crate) fn finish(&self) -> Result<(), GatewayError> {
        if !self.pending.is_empty() || !self.completed {
            Err(GatewayError::UpstreamInvalid)
        } else {
            Ok(())
        }
    }
}

fn find_sse_event_end(bytes: &[u8]) -> Option<(usize, usize)> {
    let lf = bytes.windows(2).position(|window| window == b"\n\n");
    let crlf = bytes.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(lf), Some(crlf)) if crlf < lf => Some((crlf, 4)),
        (Some(lf), _) => Some((lf, 2)),
        (None, Some(crlf)) => Some((crlf, 4)),
        (None, None) => None,
    }
}

fn inspect_sse_event(event: &[u8], completed: &mut bool) -> Result<(), GatewayError> {
    let text = std::str::from_utf8(event).map_err(|_| GatewayError::UpstreamInvalid)?;
    let data = text
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() || data == "[DONE]" {
        return Ok(());
    }
    let value: Value = serde_json::from_str(&data).map_err(|_| GatewayError::UpstreamInvalid)?;
    if value_contains_tool_event(&value) {
        return Err(GatewayError::ToolCallBlocked);
    }
    if value.get("type").and_then(Value::as_str) == Some("response.completed") {
        *completed = true;
    }
    Ok(())
}

fn value_contains_tool_event(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().any(value_contains_tool_event),
        Value::Object(object) => object.iter().any(|(key, value)| {
            (key == "type" && value.as_str().is_some_and(is_tool_event_type))
                || value_contains_tool_event(value)
        }),
        _ => false,
    }
}

fn is_tool_event_type(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "function_call",
        "custom_tool_call",
        "computer_call",
        "web_search_call",
        "file_search_call",
        "code_interpreter_call",
        "image_generation_call",
        "local_shell_call",
        "mcp_tool",
        "plugin_call",
        "tool_call",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::provider_service::ProviderAvailabilityTarget;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn target(base_url: String) -> ProviderAvailabilityTarget {
        ProviderAvailabilityTarget {
            provider_id: "provider-a".into(),
            base_url,
            model: "gpt-5.6-sol".into(),
            api_key: "test-key-upstream-not-real".into(),
        }
    }

    fn request_body() -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "model": "gpt-5.6-sol",
            "input": [],
            "tools": [
                {"type":"function","name":"update_plan"},
                {"type":"function","name":"view_image"}
            ],
            "stream": true
        }))
        .unwrap()
    }

    fn send_gateway_request(base_url: &str, body: &[u8]) -> String {
        let endpoint = base_url.trim_start_matches("http://");
        let mut stream = std::net::TcpStream::connect(endpoint).unwrap();
        write!(
            stream,
            "POST /v1/responses HTTP/1.1\r\nHost: {endpoint}\r\nAuthorization: Bearer {GATEWAY_API_KEY}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
        stream.flush().unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn sse_scanner_accepts_completion_but_blocks_function_and_image_calls() {
        let mut scanner = SseScanner::default();
        scanner
            .push_chunk(
                b"event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{}}\n\n",
            )
            .unwrap();
        assert!(scanner.finish().is_ok());

        let mut scanner = SseScanner::default();
        assert_eq!(
            scanner
                .push_chunk(
                    b"event: response.output_item.added\ndata: {\"type\":\"response.output_item.added\",\"item\":{\"type\":\"function_call\"}}\n\n",
                )
                .unwrap_err(),
            GatewayError::ToolCallBlocked
        );
        let mut scanner = SseScanner::default();
        assert_eq!(
            scanner
                .push_chunk(
                    b"event: response.output_item.done\ndata: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"image_generation_call\"}}\n\n",
                )
                .unwrap_err(),
            GatewayError::ToolCallBlocked
        );
    }

    #[tokio::test]
    async fn gateway_forwards_safe_sse_and_injects_only_upstream_key() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = thread::spawn(move || {
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
            let request_text = String::from_utf8(request).unwrap();
            let lower_request = request_text.to_ascii_lowercase();
            assert!(lower_request.contains("authorization: bearer test-key-upstream-not-real"));
            assert!(!lower_request.contains(&GATEWAY_API_KEY.to_ascii_lowercase()));
            let body = concat!(
                "event: response.created\n",
                "data: {\"type\":\"response.created\",\"response\":{\"id\":\"r1\"}}\n\n",
                "event: response.completed\n",
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r1\"}}\n\n"
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let gateway =
            CodexCompatibilityGateway::start(target(format!("http://{address}/v1")), None)
                .await
                .unwrap();
        let gateway_url = gateway.base_url().to_owned();
        let body = request_body();
        let response =
            tokio::task::spawn_blocking(move || send_gateway_request(&gateway_url, &body))
                .await
                .unwrap();
        let outcome = gateway
            .wait(std::time::Duration::from_secs(5))
            .await
            .unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("response.completed"));
        assert_eq!(outcome, GatewayOutcome::Passed);
        upstream.join().unwrap();
    }

    #[tokio::test]
    async fn gateway_blocks_upstream_tool_events_before_codex_receives_them() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let body = "event: response.output_item.added\ndata: {\"type\":\"response.output_item.added\",\"item\":{\"type\":\"function_call\"}}\n\n";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let gateway =
            CodexCompatibilityGateway::start(target(format!("http://{address}/v1")), None)
                .await
                .unwrap();
        let gateway_url = gateway.base_url().to_owned();
        let body = request_body();
        let response =
            tokio::task::spawn_blocking(move || send_gateway_request(&gateway_url, &body))
                .await
                .unwrap();
        let outcome = gateway
            .wait(std::time::Duration::from_secs(5))
            .await
            .unwrap();

        assert!(!response.contains("function_call"));
        assert_eq!(outcome, GatewayOutcome::ToolCallBlocked);
        upstream.join().unwrap();
    }
}
