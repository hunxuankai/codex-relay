use serde_json::Value;
use std::fmt;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(crate) const PREFLIGHT_API_KEY: &str = "test-key-codex-preflight-not-real";
pub(crate) const MAX_PREFLIGHT_REQUEST_BYTES: usize = 256 * 1024;
pub(crate) const MAX_PREFLIGHT_HEADER_BYTES: usize = 32 * 1024;
const ALLOWED_TOOLS: [&str; 2] = ["update_plan", "view_image"];

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct PreflightExpectation {
    model: String,
    api_key: String,
}

impl PreflightExpectation {
    pub(crate) fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            api_key: PREFLIGHT_API_KEY.to_owned(),
        }
    }

    pub(crate) fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = api_key.into();
        self
    }
}

impl fmt::Debug for PreflightExpectation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreflightExpectation")
            .field("model", &self.model)
            .field("api_key_configured", &!self.api_key.is_empty())
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CodexPreflightReport {
    pub(crate) model: String,
    pub(crate) tool_names: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum CodexPreflightFailure {
    #[error("回环请求方法不匹配")]
    Method,
    #[error("回环请求端点不匹配")]
    Endpoint,
    #[error("回环请求认证不匹配")]
    Authorization,
    #[error("回环请求过大")]
    RequestTooLarge,
    #[error("回环请求格式无效")]
    RequestShape,
    #[error("回环请求模型不匹配")]
    Model,
    #[error("回环请求工具集合不匹配")]
    Tools,
    #[error("回环服务器超时")]
    Timeout,
    #[error("回环服务器启动或读取失败")]
    Io,
}

pub(crate) fn validate_preflight_request(
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
    expectation: &PreflightExpectation,
) -> Result<CodexPreflightReport, CodexPreflightFailure> {
    if method != "POST" {
        return Err(CodexPreflightFailure::Method);
    }
    if path != "/v1/responses" {
        return Err(CodexPreflightFailure::Endpoint);
    }
    if body.len() > MAX_PREFLIGHT_REQUEST_BYTES {
        return Err(CodexPreflightFailure::RequestTooLarge);
    }
    let authorization =
        header_value(headers, "authorization").ok_or(CodexPreflightFailure::Authorization)?;
    if authorization != format!("Bearer {}", expectation.api_key) {
        return Err(CodexPreflightFailure::Authorization);
    }
    if !header_value(headers, "content-type")
        .is_some_and(|value| value.split(';').next() == Some("application/json"))
    {
        return Err(CodexPreflightFailure::RequestShape);
    }

    let document: Value =
        serde_json::from_slice(body).map_err(|_| CodexPreflightFailure::RequestShape)?;
    if document.get("model").and_then(Value::as_str) != Some(expectation.model.as_str()) {
        return Err(CodexPreflightFailure::Model);
    }
    if document.get("stream").and_then(Value::as_bool) != Some(true)
        || !document.get("input").is_some_and(Value::is_array)
    {
        return Err(CodexPreflightFailure::RequestShape);
    }

    let tools = document
        .get("tools")
        .and_then(Value::as_array)
        .ok_or(CodexPreflightFailure::Tools)?;
    let mut names = Vec::with_capacity(tools.len());
    for tool in tools {
        if tool.get("type").and_then(Value::as_str) != Some("function") {
            return Err(CodexPreflightFailure::Tools);
        }
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .ok_or(CodexPreflightFailure::Tools)?;
        names.push(name.to_owned());
    }
    names.sort();
    let mut allowed = ALLOWED_TOOLS.map(str::to_owned).to_vec();
    allowed.sort();
    if names != allowed {
        return Err(CodexPreflightFailure::Tools);
    }

    Ok(CodexPreflightReport {
        model: expectation.model.clone(),
        tool_names: names,
    })
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

pub(crate) struct CodexPreflightServer {
    base_url: String,
    shutdown: Arc<AtomicBool>,
    result: Receiver<Result<CodexPreflightReport, CodexPreflightFailure>>,
    thread: Option<JoinHandle<()>>,
}

impl CodexPreflightServer {
    pub(crate) fn start(expectation: PreflightExpectation) -> Result<Self, CodexPreflightFailure> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| CodexPreflightFailure::Io)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| CodexPreflightFailure::Io)?;
        let address = listener
            .local_addr()
            .map_err(|_| CodexPreflightFailure::Io)?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let (sender, result) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || {
            while !thread_shutdown.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let outcome = handle_tcp_connection(&mut stream, &expectation);
                        if matches!(outcome, Err(CodexPreflightFailure::Io)) {
                            continue;
                        }
                        let _ = sender.send(outcome);
                        return;
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => {
                        let _ = sender.send(Err(CodexPreflightFailure::Io));
                        return;
                    }
                }
            }
        });
        Ok(Self {
            base_url: format!("http://{address}"),
            shutdown,
            result,
            thread: Some(thread),
        })
    }

    pub(crate) fn provider_base_url(&self) -> String {
        format!("{}/v1", self.base_url)
    }

    pub(crate) fn wait(
        mut self,
        timeout: Duration,
    ) -> Result<CodexPreflightReport, CodexPreflightFailure> {
        let result = match self.result.recv_timeout(timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(CodexPreflightFailure::Timeout),
            Err(RecvTimeoutError::Disconnected) => Err(CodexPreflightFailure::Io),
        };
        self.shutdown.store(true, Ordering::Release);
        self.join_thread();
        result
    }

    fn join_thread(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for CodexPreflightServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        self.join_thread();
    }
}

fn handle_tcp_connection(
    stream: &mut TcpStream,
    expectation: &PreflightExpectation,
) -> Result<CodexPreflightReport, CodexPreflightFailure> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| CodexPreflightFailure::Io)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| CodexPreflightFailure::Io)?;
    let outcome = handle_connection(stream, expectation);
    let _ = stream.shutdown(Shutdown::Write);
    outcome
}

fn handle_connection<Stream: Read + Write>(
    stream: &mut Stream,
    expectation: &PreflightExpectation,
) -> Result<CodexPreflightReport, CodexPreflightFailure> {
    let parsed = read_http_request(stream).and_then(|request| {
        validate_preflight_request(
            &request.method,
            &request.path,
            &request.headers,
            &request.body,
            expectation,
        )
    });
    match parsed {
        Ok(report) => {
            let _ = write_response(stream, "200 OK", "text/event-stream", preflight_sse_body());
            Ok(report)
        }
        Err(CodexPreflightFailure::Io) => Err(CodexPreflightFailure::Io),
        Err(failure) => {
            let _ = write_response(
                stream,
                "400 Bad Request",
                "application/json",
                r#"{"error":"preflight rejected"}"#,
            );
            Err(failure)
        }
    }
}

struct ParsedHttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut impl Read) -> Result<ParsedHttpRequest, CodexPreflightFailure> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let read = stream
            .read(&mut buffer)
            .map_err(|_| CodexPreflightFailure::Io)?;
        if read == 0 {
            return Err(CodexPreflightFailure::RequestShape);
        }
        request.extend_from_slice(&buffer[..read]);
        if request.len() > MAX_PREFLIGHT_HEADER_BYTES {
            return Err(CodexPreflightFailure::RequestTooLarge);
        }
        if let Some(position) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            break position + 4;
        }
    };

    let header_text = std::str::from_utf8(&request[..header_end])
        .map_err(|_| CodexPreflightFailure::RequestShape)?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or(CodexPreflightFailure::RequestShape)?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or(CodexPreflightFailure::RequestShape)?
        .to_owned();
    let path = request_parts
        .next()
        .ok_or(CodexPreflightFailure::RequestShape)?
        .to_owned();
    if request_parts.next() != Some("HTTP/1.1") || request_parts.next().is_some() {
        return Err(CodexPreflightFailure::RequestShape);
    }
    let mut headers = Vec::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or(CodexPreflightFailure::RequestShape)?;
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    let content_length = header_value(&headers, "content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or(CodexPreflightFailure::RequestShape)?;
    if content_length > MAX_PREFLIGHT_REQUEST_BYTES {
        return Err(CodexPreflightFailure::RequestTooLarge);
    }
    let total_length = header_end
        .checked_add(content_length)
        .ok_or(CodexPreflightFailure::RequestTooLarge)?;
    while request.len() < total_length {
        let read = stream
            .read(&mut buffer)
            .map_err(|_| CodexPreflightFailure::Io)?;
        if read == 0 {
            return Err(CodexPreflightFailure::RequestShape);
        }
        request.extend_from_slice(&buffer[..read]);
        if request.len() > total_length {
            return Err(CodexPreflightFailure::RequestShape);
        }
    }

    Ok(ParsedHttpRequest {
        method,
        path,
        headers,
        body: request[header_end..total_length].to_vec(),
    })
}

fn preflight_sse_body() -> &'static str {
    concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-codex-relay-preflight\"}}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-codex-relay-preflight\",\"usage\":{\"input_tokens\":0,\"input_tokens_details\":null,\"output_tokens\":0,\"output_tokens_details\":null,\"total_tokens\":0}}}\n\n"
    )
}

fn write_response(
    stream: &mut impl Write,
    status: &str,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    struct TestStream {
        request: Cursor<Vec<u8>>,
        response: Vec<u8>,
    }

    impl Read for TestStream {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            self.request.read(buffer)
        }
    }

    impl Write for TestStream {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.response.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct FailingReadStream {
        response: Vec<u8>,
    }

    impl Read for FailingReadStream {
        fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(ErrorKind::ConnectionAborted))
        }
    }

    impl Write for FailingReadStream {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.response.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn valid_body() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "model": "gpt-5.6-sol",
            "input": [{"role": "user", "content": [{"type": "input_text", "text": "probe"}]}],
            "tools": [
                {"type": "function", "name": "update_plan"},
                {"type": "function", "name": "view_image"}
            ],
            "stream": true
        }))
        .unwrap()
    }

    fn headers(body: &[u8]) -> Vec<(String, String)> {
        vec![
            (
                "authorization".into(),
                format!("Bearer {PREFLIGHT_API_KEY}"),
            ),
            ("content-length".into(), body.len().to_string()),
            ("content-type".into(), "application/json".into()),
        ]
    }

    #[test]
    fn accepts_only_the_verified_model_auth_and_tool_surface() {
        let body = valid_body();
        let expectation = PreflightExpectation::new("gpt-5.6-sol");

        let report = validate_preflight_request(
            "POST",
            "/v1/responses",
            &headers(&body),
            &body,
            &expectation,
        )
        .unwrap();

        assert_eq!(report.tool_names, vec!["update_plan", "view_image"]);
        assert_eq!(report.model, "gpt-5.6-sol");
    }

    #[test]
    fn rejects_tool_model_auth_and_shape_drift_fail_closed() {
        let expectation = PreflightExpectation::new("gpt-5.6-sol");
        let body = valid_body();
        let mut wrong_auth = headers(&body);
        wrong_auth[0].1 = "Bearer test-key-wrong-not-real".into();
        assert_eq!(
            validate_preflight_request("POST", "/v1/responses", &wrong_auth, &body, &expectation)
                .unwrap_err(),
            CodexPreflightFailure::Authorization
        );

        for (document, expected) in [
            (
                json!({"model":"wrong-model","input":[],"tools":[{"type":"function","name":"update_plan"},{"type":"function","name":"view_image"}],"stream":true}),
                CodexPreflightFailure::Model,
            ),
            (
                json!({"model":"gpt-5.6-sol","input":[],"tools":[{"type":"function","name":"update_plan"}],"stream":true}),
                CodexPreflightFailure::Tools,
            ),
            (
                json!({"model":"gpt-5.6-sol","input":[],"tools":[{"type":"function","name":"update_plan"},{"type":"function","name":"view_image"},{"type":"function","name":"shell_command"}],"stream":true}),
                CodexPreflightFailure::Tools,
            ),
            (
                json!({"model":"gpt-5.6-sol","input":[],"tools":[{"type":"function","name":"update_plan"},{"type":"function","name":"update_plan"}],"stream":true}),
                CodexPreflightFailure::Tools,
            ),
            (
                json!({"model":"gpt-5.6-sol","input":[],"tools":[{"type":"function","name":"update_plan"},{"type":"function","name":"view_image"}],"stream":false}),
                CodexPreflightFailure::RequestShape,
            ),
        ] {
            let body = serde_json::to_vec(&document).unwrap();
            assert_eq!(
                validate_preflight_request(
                    "POST",
                    "/v1/responses",
                    &headers(&body),
                    &body,
                    &expectation
                )
                .unwrap_err(),
                expected
            );
        }
    }

    #[test]
    fn rejects_oversized_or_wrong_endpoint_requests() {
        let expectation = PreflightExpectation::new("gpt-5.6-sol");
        let oversized = vec![b'x'; MAX_PREFLIGHT_REQUEST_BYTES + 1];
        assert_eq!(
            validate_preflight_request(
                "POST",
                "/v1/responses",
                &headers(&oversized),
                &oversized,
                &expectation
            )
            .unwrap_err(),
            CodexPreflightFailure::RequestTooLarge
        );
        let body = valid_body();
        assert_eq!(
            validate_preflight_request(
                "GET",
                "/v1/responses",
                &headers(&body),
                &body,
                &expectation
            )
            .unwrap_err(),
            CodexPreflightFailure::Method
        );
        assert_eq!(
            validate_preflight_request("POST", "/unexpected", &headers(&body), &body, &expectation)
                .unwrap_err(),
            CodexPreflightFailure::Endpoint
        );
    }

    #[test]
    fn connection_handler_captures_one_request_and_returns_minimal_sse() {
        let body = valid_body();
        let mut request = format!(
            "POST /v1/responses HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {PREFLIGHT_API_KEY}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        request.extend_from_slice(&body);
        let mut stream = TestStream {
            request: Cursor::new(request),
            response: Vec::new(),
        };

        let report =
            handle_connection(&mut stream, &PreflightExpectation::new("gpt-5.6-sol")).unwrap();
        let response = String::from_utf8(stream.response).unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("event: response.created"));
        assert!(response.contains("event: response.completed"));
        assert_eq!(report.tool_names, vec!["update_plan", "view_image"]);
    }

    #[test]
    fn transport_io_is_not_rewritten_as_bad_request() {
        let mut stream = FailingReadStream {
            response: Vec::new(),
        };

        let failure =
            handle_connection(&mut stream, &PreflightExpectation::new("gpt-5.6-sol")).unwrap_err();

        assert_eq!(failure, CodexPreflightFailure::Io);
        assert!(stream.response.is_empty());
    }

    #[test]
    fn complete_invalid_request_remains_fail_closed_with_bad_request() {
        let body = valid_body();
        let mut request = format!(
            "GET /v1/responses HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {PREFLIGHT_API_KEY}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        request.extend_from_slice(&body);
        let mut stream = TestStream {
            request: Cursor::new(request),
            response: Vec::new(),
        };

        let failure =
            handle_connection(&mut stream, &PreflightExpectation::new("gpt-5.6-sol")).unwrap_err();
        let response = String::from_utf8(stream.response).unwrap();

        assert_eq!(failure, CodexPreflightFailure::Method);
        assert!(response.starts_with("HTTP/1.1 400 Bad Request"));
    }
}
