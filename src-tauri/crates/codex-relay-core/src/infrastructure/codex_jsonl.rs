use serde_json::Value;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum CodexJsonlFailure {
    #[error("Codex JSONL 非法 UTF-8")]
    InvalidUtf8,
    #[error("Codex JSONL 为空或被截断")]
    Truncated,
    #[error("Codex JSONL 行不是有效 JSON")]
    InvalidJson,
    #[error("Codex JSONL 事件顺序无效")]
    ProtocolOrder,
    #[error("Codex JSONL 存在未知事件")]
    UnknownEvent,
    #[error("Codex JSONL 检测到工具调用")]
    ToolCall,
    #[error("Codex JSONL 检测到安全事件")]
    SecurityEvent,
    #[error("Codex JSONL 返回远端错误")]
    RemoteError,
    #[error("Codex JSONL 回合失败")]
    TurnFailed,
    #[error("Codex JSONL 包含安全警告")]
    SecurityWarning,
    #[error("Codex stderr 包含输出")]
    StderrWarning,
    #[error("Codex 进程退出码非零")]
    ExitFailure,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct CodexJsonlSummary {
    pub(crate) thread_started: bool,
    pub(crate) turn_started: bool,
    pub(crate) turn_completed: bool,
    pub(crate) agent_message_count: usize,
    pub(crate) reasoning_count: usize,
}

impl fmt::Debug for CodexJsonlSummary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodexJsonlSummary")
            .field("thread_started", &self.thread_started)
            .field("turn_started", &self.turn_started)
            .field("turn_completed", &self.turn_completed)
            .field("agent_message_count", &self.agent_message_count)
            .field("reasoning_count", &self.reasoning_count)
            .finish()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ParseState {
    Start,
    ThreadStarted,
    TurnStarted,
    Completed,
}

pub(crate) fn parse_codex_jsonl(bytes: &[u8]) -> Result<CodexJsonlSummary, CodexJsonlFailure> {
    if bytes.is_empty() || !bytes.ends_with(b"\n") {
        return Err(CodexJsonlFailure::Truncated);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| CodexJsonlFailure::InvalidUtf8)?;
    let mut state = ParseState::Start;
    let mut summary = CodexJsonlSummary {
        thread_started: false,
        turn_started: false,
        turn_completed: false,
        agent_message_count: 0,
        reasoning_count: 0,
    };

    for line in text.lines() {
        if line.trim().is_empty() {
            return Err(CodexJsonlFailure::InvalidJson);
        }
        let event: Value =
            serde_json::from_str(line).map_err(|_| CodexJsonlFailure::InvalidJson)?;
        let object = event.as_object().ok_or(CodexJsonlFailure::InvalidJson)?;
        let event_type = object
            .get("type")
            .and_then(Value::as_str)
            .ok_or(CodexJsonlFailure::InvalidJson)?;

        match event_type {
            "thread.started" => {
                if state != ParseState::Start
                    || object
                        .get("thread_id")
                        .and_then(Value::as_str)
                        .is_none_or(str::is_empty)
                {
                    return Err(CodexJsonlFailure::ProtocolOrder);
                }
                state = ParseState::ThreadStarted;
                summary.thread_started = true;
            }
            "turn.started" => {
                if state != ParseState::ThreadStarted {
                    return Err(CodexJsonlFailure::ProtocolOrder);
                }
                state = ParseState::TurnStarted;
                summary.turn_started = true;
            }
            "item.started" | "item.updated" | "item.completed" => {
                if state != ParseState::TurnStarted {
                    return Err(CodexJsonlFailure::ProtocolOrder);
                }
                let item = object
                    .get("item")
                    .and_then(Value::as_object)
                    .ok_or(CodexJsonlFailure::InvalidJson)?;
                let item_type = item
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or(CodexJsonlFailure::InvalidJson)?;
                if event_type != "item.completed" {
                    if is_tool_item(item_type) {
                        return Err(CodexJsonlFailure::ToolCall);
                    }
                    return Err(CodexJsonlFailure::ProtocolOrder);
                }
                match item_type {
                    "agent_message" => {
                        require_text(item)?;
                        summary.agent_message_count += 1;
                    }
                    "reasoning" => {
                        require_text(item)?;
                        summary.reasoning_count += 1;
                    }
                    "error" => return Err(CodexJsonlFailure::SecurityWarning),
                    item_type if is_tool_item(item_type) => {
                        return Err(CodexJsonlFailure::ToolCall);
                    }
                    _ => return Err(CodexJsonlFailure::UnknownEvent),
                }
            }
            "turn.completed" => {
                if state != ParseState::TurnStarted || !valid_usage(object.get("usage")) {
                    return Err(CodexJsonlFailure::ProtocolOrder);
                }
                state = ParseState::Completed;
                summary.turn_completed = true;
            }
            "turn.failed" => return Err(CodexJsonlFailure::TurnFailed),
            "error" => return Err(CodexJsonlFailure::RemoteError),
            _ if is_security_event(event_type) => return Err(CodexJsonlFailure::SecurityEvent),
            _ => return Err(CodexJsonlFailure::UnknownEvent),
        }
    }

    if state != ParseState::Completed {
        return Err(CodexJsonlFailure::Truncated);
    }
    Ok(summary)
}

fn require_text(item: &serde_json::Map<String, Value>) -> Result<(), CodexJsonlFailure> {
    item.get("text")
        .and_then(Value::as_str)
        .map(|_| ())
        .ok_or(CodexJsonlFailure::InvalidJson)
}

fn valid_usage(value: Option<&Value>) -> bool {
    let Some(usage) = value.and_then(Value::as_object) else {
        return false;
    };
    [
        "input_tokens",
        "cached_input_tokens",
        "output_tokens",
        "reasoning_output_tokens",
    ]
    .into_iter()
    .all(|field| usage.get(field).and_then(Value::as_i64).is_some())
}

fn is_tool_item(item_type: &str) -> bool {
    matches!(
        item_type,
        "command_execution"
            | "file_change"
            | "mcp_tool_call"
            | "collab_tool_call"
            | "web_search"
            | "todo_list"
            | "dynamic_tool_call"
            | "image_view"
            | "plugin_tool_call"
            | "function_call"
            | "function_call_output"
    ) || item_type.contains("tool")
        || item_type.contains("search")
        || item_type.contains("plugin")
        || item_type.contains("command")
        || item_type.contains("image")
}

fn is_security_event(event_type: &str) -> bool {
    ["permission", "hook", "mcp", "web", "plugin", "auth"]
        .into_iter()
        .any(|prefix| event_type.starts_with(prefix) || event_type.contains(&format!(".{prefix}")))
}

pub(crate) fn validate_codex_stderr(bytes: &[u8]) -> Result<(), CodexJsonlFailure> {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        Ok(())
    } else {
        Err(CodexJsonlFailure::StderrWarning)
    }
}

pub(crate) fn validate_codex_exit(exit_code: Option<i32>) -> Result<(), CodexJsonlFailure> {
    (exit_code == Some(0))
        .then_some(())
        .ok_or(CodexJsonlFailure::ExitFailure)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUCCESS: &str = concat!(
        "{\"type\":\"thread.started\",\"thread_id\":\"0199a213-81c0-7800-8aa1-bbab2a035a53\"}\n",
        "{\"type\":\"turn.started\"}\n",
        "{\"type\":\"item.completed\",\"item\":{\"id\":\"item_0\",\"type\":\"reasoning\",\"text\":\"safe\"}}\n",
        "{\"type\":\"item.completed\",\"item\":{\"id\":\"item_1\",\"type\":\"agent_message\",\"text\":\"CODEX_RELAY_OK\"}}\n",
        "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":10,\"cached_input_tokens\":0,\"output_tokens\":2,\"reasoning_output_tokens\":0}}\n"
    );

    #[test]
    fn accepts_the_verified_exec_jsonl_contract_without_retaining_message_text() {
        let summary = parse_codex_jsonl(SUCCESS.as_bytes()).unwrap();

        assert!(summary.turn_completed);
        assert_eq!(summary.agent_message_count, 1);
        assert_eq!(summary.reasoning_count, 1);
        assert!(!format!("{summary:?}").contains("CODEX_RELAY_OK"));
    }

    #[test]
    fn remote_failures_and_error_items_fail_closed_without_reflecting_messages() {
        for (line, expected) in [
            (
                "{\"type\":\"turn.failed\",\"error\":{\"message\":\"secret upstream body\"}}\n",
                CodexJsonlFailure::TurnFailed,
            ),
            (
                "{\"type\":\"error\",\"message\":\"secret upstream body\"}\n",
                CodexJsonlFailure::RemoteError,
            ),
            (
                "{\"type\":\"item.completed\",\"item\":{\"id\":\"item_0\",\"type\":\"error\",\"message\":\"unsafe config fallback\"}}\n",
                CodexJsonlFailure::SecurityWarning,
            ),
        ] {
            let stream = format!(
                "{{\"type\":\"thread.started\",\"thread_id\":\"thread\"}}\n{{\"type\":\"turn.started\"}}\n{line}"
            );
            let error = parse_codex_jsonl(stream.as_bytes()).unwrap_err();
            assert_eq!(error, expected);
            assert!(!format!("{error:?}").contains("secret upstream body"));
        }
    }

    #[test]
    fn every_known_or_future_tool_item_is_rejected() {
        for item_type in [
            "command_execution",
            "file_change",
            "mcp_tool_call",
            "collab_tool_call",
            "web_search",
            "todo_list",
            "dynamic_tool_call",
            "image_view",
            "plugin_tool_call",
        ] {
            let stream = format!(
                "{{\"type\":\"thread.started\",\"thread_id\":\"thread\"}}\n{{\"type\":\"turn.started\"}}\n{{\"type\":\"item.started\",\"item\":{{\"id\":\"item_0\",\"type\":\"{item_type}\"}}}}\n"
            );
            assert_eq!(
                parse_codex_jsonl(stream.as_bytes()).unwrap_err(),
                CodexJsonlFailure::ToolCall
            );
        }
    }

    #[test]
    fn permission_hook_mcp_web_and_plugin_top_level_events_are_rejected() {
        for event_type in [
            "permission.requested",
            "hook.started",
            "mcp.call.started",
            "web_search.started",
            "plugin.called",
        ] {
            let stream = format!(
                "{{\"type\":\"thread.started\",\"thread_id\":\"thread\"}}\n{{\"type\":\"turn.started\"}}\n{{\"type\":\"{event_type}\"}}\n"
            );
            assert_eq!(
                parse_codex_jsonl(stream.as_bytes()).unwrap_err(),
                CodexJsonlFailure::SecurityEvent
            );
        }
    }

    #[test]
    fn unknown_non_json_truncated_and_out_of_order_streams_are_rejected() {
        assert_eq!(
            parse_codex_jsonl(b"not-json\n").unwrap_err(),
            CodexJsonlFailure::InvalidJson
        );
        assert_eq!(
            parse_codex_jsonl(b"{\"type\":\"thread.started\",\"thread_id\":\"thread\"}")
                .unwrap_err(),
            CodexJsonlFailure::Truncated
        );
        assert_eq!(
            parse_codex_jsonl(
                b"{\"type\":\"thread.started\",\"thread_id\":\"thread\"}\n{\"type\":\"turn.started\"}\n{\"type\":\"future.event\"}\n"
            )
            .unwrap_err(),
            CodexJsonlFailure::UnknownEvent
        );
        assert_eq!(
            parse_codex_jsonl(
                b"{\"type\":\"turn.started\"}\n{\"type\":\"thread.started\",\"thread_id\":\"thread\"}\n"
            )
            .unwrap_err(),
            CodexJsonlFailure::ProtocolOrder
        );
    }

    #[test]
    fn stderr_must_be_empty_and_exit_code_must_be_zero() {
        assert!(validate_codex_stderr(b" \r\n\t").is_ok());
        assert_eq!(
            validate_codex_stderr(b"warning: config fallback").unwrap_err(),
            CodexJsonlFailure::StderrWarning
        );
        assert!(validate_codex_exit(Some(0)).is_ok());
        assert_eq!(
            validate_codex_exit(Some(1)).unwrap_err(),
            CodexJsonlFailure::ExitFailure
        );
        assert_eq!(
            validate_codex_exit(None).unwrap_err(),
            CodexJsonlFailure::ExitFailure
        );
    }
}
