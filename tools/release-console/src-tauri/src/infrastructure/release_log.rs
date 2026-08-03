use crate::infrastructure::process::{ProcessEventSink, ProcessStream};
use crate::models::{ReleaseLogLevel, ReleaseLogSource};
use crate::services::release_log::ReleaseLogRecorder;
use codex_relay_core::infrastructure::safe_log::{redact, redaction_safe_split_index};
use regex::Regex;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

const REDACTED: &str = "[REDACTED]";
const STREAM_SANITIZE_LOOKAHEAD_BYTES: usize = 256;

pub struct ReleaseLogSanitizer {
    repository_pattern: Regex,
    sensitive_values: Vec<String>,
}

pub struct ReleaseProcessLogSink {
    step_id: String,
    recorder: Arc<ReleaseLogRecorder>,
    sanitizer: ReleaseLogSanitizer,
    chunk_bytes: usize,
    state: Mutex<ProcessLogState>,
}

#[derive(Default)]
struct ProcessLogState {
    stdout: StreamBuffer,
    stderr: StreamBuffer,
    finished: bool,
}

#[derive(Default)]
struct StreamBuffer {
    undecoded: Vec<u8>,
    raw_text: String,
    safe_text: String,
}

impl ReleaseLogSanitizer {
    pub fn new<I>(repository_root: PathBuf, sensitive_values: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let escaped_root = regex::escape(
            repository_root
                .to_string_lossy()
                .trim_end_matches(['\\', '/']),
        );
        let flexible_root = escaped_root.replace(r"\\", r"[\\/]");
        let repository_pattern = Regex::new(&format!(r"(?i:{flexible_root})"))
            .expect("escaped repository paths form a valid regex");
        let mut sensitive_values = sensitive_values
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        sensitive_values.sort_by_key(|value| std::cmp::Reverse(value.len()));
        sensitive_values.dedup();
        Self {
            repository_pattern,
            sensitive_values,
        }
    }

    pub fn sanitize(&self, diagnostic: &str) -> String {
        let mut safe = strip_terminal_controls(diagnostic)
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        safe = self
            .repository_pattern
            .replace_all(&safe, "<repo>")
            .into_owned();
        for value in &self.sensitive_values {
            safe = safe.replace(value, REDACTED);
        }
        safe = high_confidence_token_regex()
            .replace_all(&safe, REDACTED)
            .into_owned();
        redact(&safe)
    }

    fn safe_split_index(&self, text: &str, preferred: usize) -> usize {
        let mut split = redaction_safe_split_index(text, preferred);
        for matched in self
            .repository_pattern
            .find_iter(text)
            .chain(high_confidence_token_regex().find_iter(text))
        {
            if matched.start() < preferred && matched.end() > preferred {
                split = split.min(matched.start());
            }
        }
        for value in &self.sensitive_values {
            let mut probe_length = value.len().min(STREAM_SANITIZE_LOOKAHEAD_BYTES);
            while probe_length > 0 && !value.is_char_boundary(probe_length) {
                probe_length -= 1;
            }
            if probe_length == 0 {
                continue;
            }
            let probe = &value[..probe_length];
            let mut search_from = 0;
            while let Some(found) = text[search_from..].find(probe) {
                let start = search_from + found;
                let end = start + value.len();
                if start < preferred && end > preferred {
                    split = split.min(start);
                }
                search_from = start + probe_length;
            }
        }
        if let Some(sequence_start) = terminal_sequence_crossing(text, split) {
            split = split.min(sequence_start);
        }
        split
    }
}

impl ReleaseProcessLogSink {
    pub fn new<I>(
        step_id: impl Into<String>,
        repository_root: PathBuf,
        sensitive_values: I,
        recorder: Arc<ReleaseLogRecorder>,
    ) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let chunk_bytes = recorder.stream_chunk_bytes().max(4);
        Self {
            step_id: step_id.into(),
            recorder,
            sanitizer: ReleaseLogSanitizer::new(repository_root, sensitive_values),
            chunk_bytes,
            state: Mutex::new(ProcessLogState::default()),
        }
    }

    pub fn finish(&self) {
        let (stdout, stderr) = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.finished {
                return;
            }
            state.finished = true;
            (
                finish_stream(&mut state.stdout, &self.sanitizer, self.chunk_bytes),
                finish_stream(&mut state.stderr, &self.sanitizer, self.chunk_bytes),
            )
        };
        self.record_messages(ReleaseLogSource::Stdout, stdout);
        self.record_messages(ReleaseLogSource::Stderr, stderr);
    }

    fn record_messages(&self, source: ReleaseLogSource, messages: Vec<String>) {
        for message in messages {
            if message.is_empty() {
                continue;
            }
            self.recorder
                .record(self.step_id.clone(), source, ReleaseLogLevel::Info, message);
        }
    }
}

impl ProcessEventSink for ReleaseProcessLogSink {
    fn on_output(&self, stream: ProcessStream, bytes: &[u8]) {
        let messages = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.finished {
                return;
            }
            let buffer = match stream {
                ProcessStream::Stdout => &mut state.stdout,
                ProcessStream::Stderr => &mut state.stderr,
            };
            let decoded = decode_incremental(&mut buffer.undecoded, bytes);
            buffer.raw_text.push_str(&decoded);
            prepare_public_messages(buffer, &self.sanitizer, self.chunk_bytes, false)
        };
        let source = match stream {
            ProcessStream::Stdout => ReleaseLogSource::Stdout,
            ProcessStream::Stderr => ReleaseLogSource::Stderr,
        };
        self.record_messages(source, messages);
    }
}

fn decode_incremental(undecoded: &mut Vec<u8>, bytes: &[u8]) -> String {
    undecoded.extend_from_slice(bytes);
    let owned = std::mem::take(undecoded);
    let mut remaining = owned.as_slice();
    let mut decoded = String::new();
    loop {
        match std::str::from_utf8(remaining) {
            Ok(text) => {
                decoded.push_str(text);
                return decoded;
            }
            Err(error) => {
                let valid_length = error.valid_up_to();
                decoded.push_str(
                    std::str::from_utf8(&remaining[..valid_length])
                        .expect("UTF-8 validator identified a valid prefix"),
                );
                remaining = &remaining[valid_length..];
                let Some(invalid_length) = error.error_len() else {
                    undecoded.extend_from_slice(remaining);
                    return decoded;
                };
                decoded.push('\u{fffd}');
                remaining = &remaining[invalid_length..];
            }
        }
    }
}

fn take_ready_messages(text: &mut String, chunk_bytes: usize, flush_tail: bool) -> Vec<String> {
    let mut messages = Vec::new();
    loop {
        let newline_end = text.find('\n').map(|index| index + 1);
        let mut end = match newline_end {
            Some(end) if end <= chunk_bytes => end,
            _ if text.len() >= chunk_bytes => chunk_bytes,
            _ => break,
        };
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        if end > 0
            && text.as_bytes().get(end - 1) == Some(&b'\r')
            && text.as_bytes().get(end) == Some(&b'\n')
        {
            end -= 1;
        }
        if end == 0 {
            break;
        }
        let remainder = text.split_off(end);
        messages.push(std::mem::replace(text, remainder));
    }
    if flush_tail && !text.is_empty() {
        messages.push(std::mem::take(text));
    }
    messages
}

fn finish_stream(
    buffer: &mut StreamBuffer,
    sanitizer: &ReleaseLogSanitizer,
    chunk_bytes: usize,
) -> Vec<String> {
    if !buffer.undecoded.is_empty() {
        buffer
            .raw_text
            .push_str(&String::from_utf8_lossy(&buffer.undecoded));
        buffer.undecoded.clear();
    }
    prepare_public_messages(buffer, sanitizer, chunk_bytes, true)
}

fn prepare_public_messages(
    buffer: &mut StreamBuffer,
    sanitizer: &ReleaseLogSanitizer,
    chunk_bytes: usize,
    flush_tail: bool,
) -> Vec<String> {
    for raw in take_raw_segments(&mut buffer.raw_text, sanitizer, chunk_bytes, flush_tail) {
        buffer.safe_text.push_str(&sanitizer.sanitize(&raw));
    }
    take_ready_messages(&mut buffer.safe_text, chunk_bytes, flush_tail)
}

fn take_raw_segments(
    text: &mut String,
    sanitizer: &ReleaseLogSanitizer,
    chunk_bytes: usize,
    flush_tail: bool,
) -> Vec<String> {
    let mut segments = Vec::new();
    loop {
        if let Some(newline) = text.find('\n') {
            let remainder = text.split_off(newline + 1);
            segments.push(std::mem::replace(text, remainder));
            continue;
        }
        let streaming_threshold = chunk_bytes.saturating_add(STREAM_SANITIZE_LOOKAHEAD_BYTES);
        if text.len() >= streaming_threshold {
            let mut split = text.len().saturating_sub(STREAM_SANITIZE_LOOKAHEAD_BYTES);
            split = sanitizer.safe_split_index(text, split);
            while split > 0 && !text.is_char_boundary(split) {
                split -= 1;
            }
            if split > 0 {
                let remainder = text.split_off(split);
                segments.push(std::mem::replace(text, remainder));
            }
        }
        break;
    }
    if flush_tail && !text.is_empty() {
        segments.push(std::mem::take(text));
    }
    segments
}

fn high_confidence_token_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:test-key-[A-Za-z0-9_-]*-not-real|sk-(?:proj-)?[A-Za-z0-9_-]{8,})\b")
            .expect("valid high-confidence token regex")
    })
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum TerminalState {
    Text,
    Escape,
    ControlSequence,
    OperatingSystemCommand,
    OperatingSystemCommandEscape,
}

fn strip_terminal_controls(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut state = TerminalState::Text;
    for character in input.chars() {
        if state == TerminalState::Text
            && character != '\u{1b}'
            && (!character.is_control() || matches!(character, '\n' | '\r' | '\t'))
        {
            output.push(character);
        }
        state = advance_terminal_state(state, character);
    }
    output
}

fn terminal_sequence_crossing(text: &str, boundary: usize) -> Option<usize> {
    let mut state = TerminalState::Text;
    let mut sequence_start = None;
    for (index, character) in text.char_indices() {
        if index >= boundary {
            break;
        }
        if state == TerminalState::Text && character == '\u{1b}' {
            sequence_start = Some(index);
        }
        state = advance_terminal_state(state, character);
        if state == TerminalState::Text {
            sequence_start = None;
        }
    }
    (state != TerminalState::Text)
        .then_some(sequence_start)
        .flatten()
}

fn advance_terminal_state(state: TerminalState, character: char) -> TerminalState {
    match state {
        TerminalState::Text if character == '\u{1b}' => TerminalState::Escape,
        TerminalState::Text => TerminalState::Text,
        TerminalState::Escape if character == '[' => TerminalState::ControlSequence,
        TerminalState::Escape if character == ']' => TerminalState::OperatingSystemCommand,
        TerminalState::Escape => TerminalState::Text,
        TerminalState::ControlSequence if ('@'..='~').contains(&character) => TerminalState::Text,
        TerminalState::ControlSequence => TerminalState::ControlSequence,
        TerminalState::OperatingSystemCommand if character == '\u{7}' => TerminalState::Text,
        TerminalState::OperatingSystemCommand if character == '\u{1b}' => {
            TerminalState::OperatingSystemCommandEscape
        }
        TerminalState::OperatingSystemCommand => TerminalState::OperatingSystemCommand,
        TerminalState::OperatingSystemCommandEscape if character == '\\' => TerminalState::Text,
        TerminalState::OperatingSystemCommandEscape if character == '\u{1b}' => {
            TerminalState::OperatingSystemCommandEscape
        }
        TerminalState::OperatingSystemCommandEscape => TerminalState::OperatingSystemCommand,
    }
}

#[cfg(test)]
mod tests {
    use super::{ReleaseLogSanitizer, ReleaseProcessLogSink};
    use crate::infrastructure::process::{ProcessEventSink, ProcessStream};
    use crate::models::ReleaseLogSource;
    use crate::services::release_log::{ReleaseLogPolicy, ReleaseLogRecorder, ReleaseLogStore};
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn sanitizer_preserves_diagnostic_context_while_removing_sensitive_output() {
        let repository = PathBuf::from(r"D:\safe-temp\repository");
        let sanitizer = ReleaseLogSanitizer::new(
            repository.clone(),
            [
                "http://proxy.test:8080".to_string(),
                "known-sensitive-environment-value".to_string(),
            ],
        );
        let diagnostic = format!(
            concat!(
                "\u{1b}[31mFAILED\u{1b}[0m release_console::streams\r\n",
                "  at {}\\src\\release.rs:17:9\r\n",
                "  error: assertion failed\r\n",
                "  token=test-key-stream-not-real\r\n",
                "  proxy=http://proxy.test:8080\r\n",
                "  env=known-sensitive-environment-value\r\n",
                "  Authorization: Bearer test-key-bearer-not-real\r\n",
                "  url=https://example.test/run?api_key=test-key-query-not-real\r\n",
            ),
            repository.display()
        );

        let safe = sanitizer.sanitize(&diagnostic);

        assert!(safe.contains("FAILED release_console::streams"));
        assert!(safe.contains(r"<repo>\src\release.rs:17:9"));
        assert!(safe.contains("error: assertion failed"));
        assert!(!safe.contains('\u{1b}'));
        assert!(!safe.contains(repository.to_string_lossy().as_ref()));
        for secret in [
            "test-key-stream-not-real",
            "http://proxy.test:8080",
            "known-sensitive-environment-value",
            "test-key-bearer-not-real",
            "test-key-query-not-real",
        ] {
            assert!(!safe.contains(secret), "secret shape remained in output");
        }
        assert!(safe.matches("[REDACTED]").count() >= 5);
        assert!(!safe.contains('\r'));
    }

    #[test]
    fn process_sink_decodes_split_utf8_and_flushes_each_stream_tail_once() {
        let git_dir = tempfile::tempdir().unwrap();
        let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let sink = ReleaseProcessLogSink::new(
            "release-console-rust-tests",
            PathBuf::from(r"D:\safe-temp\repository"),
            Vec::<String>::new(),
            recorder,
        );
        let stdout = "first 中\r\nstdout tail".as_bytes();
        let split = stdout.iter().position(|byte| *byte >= 0x80).unwrap() + 1;

        sink.on_output(ProcessStream::Stdout, &stdout[..split]);
        sink.on_output(ProcessStream::Stdout, &stdout[split..]);
        sink.on_output(ProcessStream::Stderr, b"stderr tail");
        sink.finish();
        sink.finish();

        let reader = ReleaseLogStore::new(git_dir.path().to_path_buf());
        let entries = reader.load_page("session-a", None).unwrap().entries;
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.sequence)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        let stdout_text = entries
            .iter()
            .filter(|entry| entry.source == ReleaseLogSource::Stdout)
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        let stderr_text = entries
            .iter()
            .filter(|entry| entry.source == ReleaseLogSource::Stderr)
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        assert_eq!(stdout_text, "first 中\nstdout tail");
        assert_eq!(stderr_text, "stderr tail");
        assert!(!stdout_text.contains('\u{fffd}'));
    }

    #[test]
    fn process_sink_streams_long_unterminated_output_in_bounded_chunks() {
        let git_dir = tempfile::tempdir().unwrap();
        let policy = ReleaseLogPolicy {
            max_entry_bytes: 512,
            stream_chunk_bytes: 8,
            ..ReleaseLogPolicy::default()
        };
        let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let sink = ReleaseProcessLogSink::new(
            "full-project-check",
            PathBuf::from(r"D:\safe-temp\repository"),
            Vec::<String>::new(),
            recorder,
        );
        let diagnostic = "a".repeat(300);

        sink.on_output(ProcessStream::Stdout, diagnostic.as_bytes());

        let reader = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        let streamed = reader.load_page("session-a", None).unwrap().entries;
        assert!(!streamed.is_empty());
        let streamed_text = streamed
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        assert!(diagnostic.starts_with(&streamed_text));
        assert_eq!(streamed_text.len() % policy.stream_chunk_bytes, 0);

        sink.finish();
        let entries = reader.load_page("session-a", None).unwrap().entries;
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.message.as_str())
                .collect::<String>(),
            diagnostic.as_str()
        );
        assert!(
            entries
                .iter()
                .all(|entry| entry.message.len() <= policy.stream_chunk_bytes)
        );
        assert_eq!(entries.last().unwrap().sequence, entries.len() as u64);
    }

    #[test]
    fn process_sink_sanitizes_a_logical_line_before_splitting_public_entries() {
        let git_dir = tempfile::tempdir().unwrap();
        let policy = ReleaseLogPolicy {
            max_entry_bytes: 512,
            stream_chunk_bytes: 64,
            ..ReleaseLogPolicy::default()
        };
        let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let repository = PathBuf::from(r"D:\safe-temp\repository");
        let proxy = "http://proxy.test:8080";
        let sink = ReleaseProcessLogSink::new(
            "full-project-check",
            repository.clone(),
            [proxy.to_string()],
            recorder,
        );
        let diagnostic = format!(
            "\u{1b}[31merror\u{1b}[0m at {}\\src\\main.rs:9:4: assertion failed; token=test-key-stream-not-real; proxy={proxy}\r\n",
            repository.display()
        );

        for bytes in diagnostic.as_bytes().chunks(7) {
            sink.on_output(ProcessStream::Stderr, bytes);
        }
        sink.finish();

        let reader = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        let entries = reader.load_page("session-a", None).unwrap().entries;
        let public_text = entries
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        assert!(public_text.contains("error at <repo>"));
        assert!(public_text.contains("src\\main.rs:9:4: assertion failed"));
        assert!(!public_text.contains(repository.to_string_lossy().as_ref()));
        assert!(!public_text.contains("test-key-stream-not-real"));
        assert!(!public_text.contains(proxy));
        assert!(!public_text.contains('\u{1b}'));
        assert!(!public_text.contains("[31m"));
        assert!(
            entries
                .iter()
                .all(|entry| entry.message.len() <= policy.stream_chunk_bytes)
        );
    }

    #[test]
    fn process_sink_retains_a_security_tail_across_forced_long_line_flushes() {
        let git_dir = tempfile::tempdir().unwrap();
        let policy = ReleaseLogPolicy {
            max_entry_bytes: 512,
            stream_chunk_bytes: 64,
            ..ReleaseLogPolicy::default()
        };
        let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let sink = ReleaseProcessLogSink::new(
            "full-project-check",
            PathBuf::from(r"D:\safe-temp\repository"),
            Vec::<String>::new(),
            recorder,
        );
        let first = format!("{} te", "x".repeat(317));
        assert_eq!(first.len(), 320);

        sink.on_output(ProcessStream::Stdout, first.as_bytes());
        sink.on_output(ProcessStream::Stdout, b"st-key-stream-not-real");
        sink.finish();

        let reader = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        let public_text = reader
            .load_page("session-a", None)
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        assert!(!public_text.contains("test-key-stream-not-real"));
        assert!(public_text.contains("[REDACTED]"));
    }

    #[test]
    fn process_sink_does_not_leak_a_long_bearer_continuation_across_forced_flushes() {
        let git_dir = tempfile::tempdir().unwrap();
        let policy = ReleaseLogPolicy {
            max_entry_bytes: 512,
            stream_chunk_bytes: 64,
            ..ReleaseLogPolicy::default()
        };
        let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let sink = ReleaseProcessLogSink::new(
            "full-project-check",
            PathBuf::from(r"D:\safe-temp\repository"),
            Vec::<String>::new(),
            recorder,
        );
        let token = "AbCdEf0123456789".repeat(32);
        let diagnostic = format!("Authorization: Bearer {token}");
        let first_flush = 320;

        sink.on_output(ProcessStream::Stderr, &diagnostic.as_bytes()[..first_flush]);
        sink.on_output(ProcessStream::Stderr, &diagnostic.as_bytes()[first_flush..]);
        sink.finish();

        let reader = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        let public_text = reader
            .load_page("session-a", None)
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        assert!(public_text.contains("[REDACTED]"));
        assert!(!public_text.contains(&token[128..256]));
    }

    #[test]
    fn process_sink_does_not_split_a_long_known_sensitive_value() {
        let git_dir = tempfile::tempdir().unwrap();
        let policy = ReleaseLogPolicy {
            max_entry_bytes: 512,
            stream_chunk_bytes: 64,
            ..ReleaseLogPolicy::default()
        };
        let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let secret = format!("proxy-secret-{}", "z".repeat(400));
        let sink = ReleaseProcessLogSink::new(
            "full-project-check",
            PathBuf::from(r"D:\safe-temp\repository"),
            [secret.clone()],
            recorder,
        );
        let diagnostic = format!("proxy={secret}");
        let first_flush = 320;

        sink.on_output(ProcessStream::Stdout, &diagnostic.as_bytes()[..first_flush]);
        sink.on_output(ProcessStream::Stdout, &diagnostic.as_bytes()[first_flush..]);
        sink.finish();

        let reader = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        let public_text = reader
            .load_page("session-a", None)
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        assert!(public_text.contains("[REDACTED]"));
        assert!(!public_text.contains(&secret[128..256]));
    }

    #[test]
    fn process_sink_does_not_split_a_sensitive_match_at_the_security_tail_boundary() {
        let git_dir = tempfile::tempdir().unwrap();
        let policy = ReleaseLogPolicy {
            max_entry_bytes: 512,
            stream_chunk_bytes: 64,
            ..ReleaseLogPolicy::default()
        };
        let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let sink = ReleaseProcessLogSink::new(
            "full-project-check",
            PathBuf::from(r"D:\safe-temp\repository"),
            Vec::<String>::new(),
            recorder,
        );
        let secret = "test-key-stream-not-real";
        let mut diagnostic = format!("{} {secret} ", "x".repeat(61));
        diagnostic.push_str(&"y".repeat(320 - diagnostic.len()));
        assert_eq!(diagnostic.len(), 320);

        sink.on_output(ProcessStream::Stdout, diagnostic.as_bytes());
        sink.finish();

        let reader = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        let public_text = reader
            .load_page("session-a", None)
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        assert!(!public_text.contains(secret));
        assert!(public_text.contains("[REDACTED]"));
    }

    #[test]
    fn process_sink_does_not_split_an_ansi_sequence_at_the_security_tail_boundary() {
        let git_dir = tempfile::tempdir().unwrap();
        let policy = ReleaseLogPolicy {
            max_entry_bytes: 512,
            stream_chunk_bytes: 64,
            ..ReleaseLogPolicy::default()
        };
        let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let sink = ReleaseProcessLogSink::new(
            "full-project-check",
            PathBuf::from(r"D:\safe-temp\repository"),
            Vec::<String>::new(),
            recorder,
        );
        let mut diagnostic = format!("{}\u{1b}[31mERROR\u{1b}[0m", "x".repeat(63));
        diagnostic.push_str(&"y".repeat(320 - diagnostic.len()));

        sink.on_output(ProcessStream::Stderr, diagnostic.as_bytes());
        sink.finish();

        let reader = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
        let public_text = reader
            .load_page("session-a", None)
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<String>();
        assert!(public_text.contains("ERROR"));
        assert!(!public_text.contains('\u{1b}'));
        assert!(!public_text.contains("[31m"));
        assert!(!public_text.contains("[0m"));
    }
}
