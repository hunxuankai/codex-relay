use crate::app_state::ReleaseEventSink;
use crate::models::{
    ReleaseEvent, ReleaseLogEntry, ReleaseLogLevel, ReleaseLogPage, ReleaseLogSource,
    WorkflowRunStatus,
};
use chrono::{SecondsFormat, Utc};
use codex_relay_core::error::AppError;
use codex_relay_core::infrastructure::atomic_file::atomic_write;
use codex_relay_core::infrastructure::safe_log::redact;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const STATE_DIRECTORY: &str = "codex-relay-release-console";
const LOG_FILE: &str = "session.log.jsonl";
const LOG_SCHEMA_VERSION: u32 = 1;
const INCOMPLETE_TAIL_WARNING: &str = "发布日志末尾不完整，已保留此前有效记录。";
const CORRUPT_LOG_WARNING: &str = "发布日志包含损坏记录，已停止读取并保留此前有效记录。";
const ENTRY_TRUNCATED_SUFFIX: &str = "\n[单条大小上限已截断]";
const ENTRY_TRUNCATION_WARNING: &str = "单条日志消息超过大小上限，已在 UTF-8 边界截断。";
const PERSISTENCE_WARNING: &str = "发布日志无法持久化，当前窗口仍会显示日志，但重启后可能丢失。";
const RUN_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReleaseLogPolicy {
    pub max_bytes: u64,
    pub max_entries: u64,
    pub max_entry_bytes: usize,
    pub stream_chunk_bytes: usize,
    pub page_size: usize,
}

impl Default for ReleaseLogPolicy {
    fn default() -> Self {
        Self {
            max_bytes: 50 * 1024 * 1024,
            max_entries: 100_000,
            max_entry_bytes: 1024 * 1024,
            stream_chunk_bytes: 64 * 1024,
            page_size: 2_000,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReleaseLogError {
    #[error("无法读取发布日志")]
    ReadFailed,
    #[error("发布日志无效")]
    InvalidLog,
    #[error("无法保存发布日志")]
    WriteFailed,
    #[error("发布日志会话不匹配")]
    SessionMismatch,
    #[error("发布日志序号无效")]
    InvalidSequence,
    #[error("发布日志单条记录超过大小上限")]
    EntryTooLarge,
}

#[derive(Default)]
struct ReleaseLogState {
    session_id: Option<String>,
    last_sequence: u64,
    total_entries: u64,
    total_bytes: u64,
    truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseLogOpenState {
    pub last_sequence: u64,
    pub total_entries: u64,
    pub total_bytes: u64,
    pub truncated: bool,
    pub warning: Option<String>,
}

pub struct ReleaseLogStore {
    log_file: PathBuf,
    policy: ReleaseLogPolicy,
    state: Mutex<ReleaseLogState>,
}

struct ReleaseLogRecorderState {
    session_id: String,
    last_sequence: u64,
    max_entry_bytes: usize,
    stream_chunk_bytes: usize,
    store: Option<ReleaseLogStore>,
}

pub struct ReleaseLogRecorder {
    state: Mutex<ReleaseLogRecorderState>,
    events: Option<Arc<dyn ReleaseEventSink>>,
}

pub trait ReleaseProgressSink: Send + Sync {
    fn started(&self, step_id: &str, message: &str);
    fn log(&self, step_id: &str, level: ReleaseLogLevel, message: &str);
    fn completed(&self, step_id: &str, duration_millis: u64, message: &str);
    fn failed(&self, step_id: &str, code: &str, message: &str);
}

#[derive(Default)]
pub struct NoopReleaseProgressSink;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseRunProgressDecision {
    Changed,
    Heartbeat,
    Silent,
}

#[derive(Default)]
pub struct ReleaseRunProgressTracker {
    previous: Option<ReleaseRunProgressProjection>,
    last_emitted_at: Option<Duration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseRunProgressProjection {
    run_id: u64,
    status: String,
    conclusion: Option<String>,
    jobs: Vec<ReleaseRunJobProjection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseRunJobProjection {
    name: String,
    status: String,
    conclusion: Option<String>,
    steps: Vec<ReleaseRunStepProjection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReleaseRunStepProjection {
    number: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredReleaseLogEntry {
    schema_version: u32,
    entry: ReleaseLogEntry,
}

impl ReleaseLogStore {
    pub fn new(git_dir: PathBuf) -> Self {
        Self::with_policy(git_dir, ReleaseLogPolicy::default())
    }

    pub fn with_policy(git_dir: PathBuf, policy: ReleaseLogPolicy) -> Self {
        Self {
            log_file: git_dir.join(STATE_DIRECTORY).join(LOG_FILE),
            policy,
            state: Mutex::new(ReleaseLogState::default()),
        }
    }

    pub fn initialize(&self, session_id: &str) -> Result<(), ReleaseLogError> {
        if let Some(parent) = self.log_file.parent() {
            fs::create_dir_all(parent).map_err(|_| ReleaseLogError::WriteFailed)?;
        }
        atomic_write(&self.log_file, &[], validate_log_bytes)
            .map_err(|_| ReleaseLogError::WriteFailed)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| ReleaseLogError::WriteFailed)?;
        *state = ReleaseLogState {
            session_id: Some(session_id.to_string()),
            ..ReleaseLogState::default()
        };
        Ok(())
    }

    pub fn open(&self, session_id: &str) -> Result<ReleaseLogOpenState, ReleaseLogError> {
        let mut bytes = match fs::read(&self.log_file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
            Err(_) => return Err(ReleaseLogError::ReadFailed),
        };
        let mut warning = if !bytes.is_empty() && !bytes.ends_with(b"\n") {
            let valid_length = bytes
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |index| index + 1);
            bytes.truncate(valid_length);
            atomic_write(&self.log_file, &bytes, validate_log_bytes)
                .map_err(|_| ReleaseLogError::WriteFailed)?;
            Some(INCOMPLETE_TAIL_WARNING.to_string())
        } else {
            None
        };
        let entries = match parse_log_bytes(&bytes) {
            Ok(entries) => entries,
            Err(ReleaseLogError::InvalidLog) => {
                let (entries, valid_length) = parse_valid_log_prefix(&bytes);
                bytes.truncate(valid_length);
                atomic_write(&self.log_file, &bytes, validate_log_bytes)
                    .map_err(|_| ReleaseLogError::WriteFailed)?;
                warning = Some(CORRUPT_LOG_WARNING.to_string());
                entries
            }
            Err(error) => return Err(error),
        };
        if entries.iter().any(|entry| entry.session_id != session_id) {
            return Err(ReleaseLogError::SessionMismatch);
        }
        let last_sequence = entries.last().map_or(0, |entry| entry.sequence);
        let truncation_warning = entries
            .iter()
            .find(|entry| is_truncation_marker(entry))
            .map(|entry| entry.message.clone());
        let opened = ReleaseLogOpenState {
            last_sequence,
            total_entries: entries.len() as u64,
            total_bytes: bytes.len() as u64,
            truncated: truncation_warning.is_some(),
            warning: warning.or(truncation_warning),
        };
        let mut state = self.state.lock().map_err(|_| ReleaseLogError::ReadFailed)?;
        *state = ReleaseLogState {
            session_id: Some(session_id.to_string()),
            last_sequence: opened.last_sequence,
            total_entries: opened.total_entries,
            total_bytes: opened.total_bytes,
            truncated: opened.truncated,
        };
        Ok(opened)
    }

    pub fn append(
        &self,
        mut entry: ReleaseLogEntry,
    ) -> Result<Option<ReleaseLogPage>, ReleaseLogError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ReleaseLogError::WriteFailed)?;
        if state.session_id.as_deref() != Some(entry.session_id.as_str()) {
            return Err(ReleaseLogError::SessionMismatch);
        }
        if entry.sequence != state.last_sequence + 1 {
            return Err(ReleaseLogError::InvalidSequence);
        }
        entry.message = redact(&entry.message);

        let bytes = serialize_entry(&entry)?;
        if bytes.len() > self.policy.max_entry_bytes {
            return Err(ReleaseLogError::EntryTooLarge);
        }
        let needs_compaction = state.total_entries + 1 > self.policy.max_entries
            || state.total_bytes + bytes.len() as u64 > self.policy.max_bytes;
        if needs_compaction {
            let current_bytes =
                fs::read(&self.log_file).map_err(|_| ReleaseLogError::ReadFailed)?;
            let mut entries = parse_log_bytes(&current_bytes)?;
            let appended_sequence = entry.sequence;
            entries.push(entry);
            let compacted = compact_entries(entries, self.policy)?;
            let persisted = serialize_entries(&compacted)?;
            if persisted.len() as u64 > self.policy.max_bytes
                || compacted.len() as u64 > self.policy.max_entries
            {
                return Err(ReleaseLogError::WriteFailed);
            }
            atomic_write(&self.log_file, &persisted, validate_log_bytes)
                .map_err(|_| ReleaseLogError::WriteFailed)?;
            state.last_sequence = appended_sequence;
            state.total_entries = compacted.len() as u64;
            state.total_bytes = persisted.len() as u64;
            state.truncated = compacted.iter().any(is_truncation_marker);
            return Ok(Some(build_log_page(
                &compacted,
                None,
                state.total_bytes,
                self.policy.page_size,
                None,
            )));
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .map_err(|_| ReleaseLogError::WriteFailed)?;
        file.write_all(&bytes)
            .and_then(|()| file.flush())
            .map_err(|_| ReleaseLogError::WriteFailed)?;
        state.last_sequence += 1;
        state.total_entries += 1;
        state.total_bytes += bytes.len() as u64;
        Ok(None)
    }

    pub fn load_page(
        &self,
        session_id: &str,
        before_sequence: Option<u64>,
    ) -> Result<ReleaseLogPage, ReleaseLogError> {
        let bytes = match fs::read(&self.log_file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => Vec::new(),
            Err(_) => return Err(ReleaseLogError::ReadFailed),
        };
        let (all_entries, trusted_bytes, recovery_warning) = parse_log_bytes_for_page(&bytes)?;
        if all_entries
            .iter()
            .any(|entry| entry.session_id != session_id)
        {
            return Err(ReleaseLogError::SessionMismatch);
        }
        Ok(build_log_page(
            &all_entries,
            before_sequence,
            trusted_bytes as u64,
            self.policy.page_size,
            recovery_warning,
        ))
    }
}

impl ReleaseLogRecorder {
    pub fn new(
        session_id: impl Into<String>,
        store: ReleaseLogStore,
        last_sequence: u64,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Self {
        let max_entry_bytes = store.policy.max_entry_bytes;
        let stream_chunk_bytes = store.policy.stream_chunk_bytes;
        Self {
            state: Mutex::new(ReleaseLogRecorderState {
                session_id: session_id.into(),
                last_sequence,
                max_entry_bytes,
                stream_chunk_bytes,
                store: Some(store),
            }),
            events,
        }
    }

    pub fn volatile(
        session_id: impl Into<String>,
        last_sequence: u64,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Self {
        let policy = ReleaseLogPolicy::default();
        let recorder = Self {
            state: Mutex::new(ReleaseLogRecorderState {
                session_id: session_id.into(),
                last_sequence,
                max_entry_bytes: policy.max_entry_bytes,
                stream_chunk_bytes: policy.stream_chunk_bytes,
                store: None,
            }),
            events,
        };
        recorder.record(
            "releasePipeline",
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Warning,
            PERSISTENCE_WARNING,
        );
        recorder
    }

    pub fn record(
        &self,
        step_id: impl Into<String>,
        source: ReleaseLogSource,
        level: ReleaseLogLevel,
        message: impl AsRef<str>,
    ) -> ReleaseLogEntry {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.last_sequence += 1;
        let mut entry = ReleaseLogEntry {
            session_id: state.session_id.clone(),
            sequence: state.last_sequence,
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            step_id: step_id.into(),
            source,
            level,
            message: redact(message.as_ref()),
        };
        let was_truncated = truncate_entry_message(&mut entry, state.max_entry_bytes);
        let mut emitted = vec![entry.clone()];
        if was_truncated {
            state.last_sequence += 1;
            emitted.push(ReleaseLogEntry {
                session_id: state.session_id.clone(),
                sequence: state.last_sequence,
                timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                step_id: entry.step_id.clone(),
                source: ReleaseLogSource::Lifecycle,
                level: ReleaseLogLevel::Warning,
                message: ENTRY_TRUNCATION_WARNING.into(),
            });
        }
        let mut realtime_pages = vec![None; emitted.len()];
        let persistence_failed = if let Some(store) = state.store.as_ref() {
            let mut failed = false;
            for (index, emitted_entry) in emitted.iter().enumerate() {
                match store.append(emitted_entry.clone()) {
                    Ok(page) => realtime_pages[index] = page,
                    Err(_) => {
                        failed = true;
                        break;
                    }
                }
            }
            failed
        } else {
            false
        };
        if persistence_failed {
            state.store = None;
            state.last_sequence += 1;
            emitted.push(ReleaseLogEntry {
                session_id: state.session_id.clone(),
                sequence: state.last_sequence,
                timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
                step_id: entry.step_id.clone(),
                source: ReleaseLogSource::Lifecycle,
                level: ReleaseLogLevel::Warning,
                message: PERSISTENCE_WARNING.into(),
            });
            realtime_pages.push(None);
        }
        drop(state);

        for (emitted_entry, page) in emitted.into_iter().zip(realtime_pages) {
            self.emit_event(ReleaseEvent::StepLog {
                entry: emitted_entry,
                page,
            });
        }
        entry
    }

    pub fn stream_chunk_bytes(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .stream_chunk_bytes
    }

    fn emit_event(&self, event: ReleaseEvent) {
        if let Some(events) = &self.events {
            let _ = events.send(event);
        }
    }
}

impl ReleaseProgressSink for ReleaseLogRecorder {
    fn started(&self, step_id: &str, message: &str) {
        self.emit_event(ReleaseEvent::StepStarted {
            step_id: step_id.to_string(),
            started_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        });
        self.record(
            step_id,
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Info,
            message,
        );
    }

    fn log(&self, step_id: &str, level: ReleaseLogLevel, message: &str) {
        self.record(step_id, ReleaseLogSource::Lifecycle, level, message);
    }

    fn completed(&self, step_id: &str, duration_millis: u64, message: &str) {
        self.record(
            step_id,
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Info,
            message,
        );
        self.emit_event(ReleaseEvent::StepCompleted {
            step_id: step_id.to_string(),
            completed_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            duration_millis,
        });
    }

    fn failed(&self, step_id: &str, code: &str, message: &str) {
        self.record(
            step_id,
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Error,
            message,
        );
        self.emit_event(ReleaseEvent::StepFailed {
            step_id: step_id.to_string(),
            code: code.to_string(),
            message: redact(message),
        });
    }
}

impl ReleaseProgressSink for NoopReleaseProgressSink {
    fn started(&self, _step_id: &str, _message: &str) {}

    fn log(&self, _step_id: &str, _level: ReleaseLogLevel, _message: &str) {}

    fn completed(&self, _step_id: &str, _duration_millis: u64, _message: &str) {}

    fn failed(&self, _step_id: &str, _code: &str, _message: &str) {}
}

impl ReleaseRunProgressTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(
        &mut self,
        now: Duration,
        run: &WorkflowRunStatus,
    ) -> ReleaseRunProgressDecision {
        let projection = ReleaseRunProgressProjection::from(run);
        if self.previous.as_ref() != Some(&projection) {
            self.previous = Some(projection);
            self.last_emitted_at = Some(now);
            return ReleaseRunProgressDecision::Changed;
        }
        if self
            .last_emitted_at
            .is_none_or(|last| now.saturating_sub(last) >= RUN_HEARTBEAT_INTERVAL)
        {
            self.last_emitted_at = Some(now);
            return ReleaseRunProgressDecision::Heartbeat;
        }
        ReleaseRunProgressDecision::Silent
    }
}

impl From<&WorkflowRunStatus> for ReleaseRunProgressProjection {
    fn from(run: &WorkflowRunStatus) -> Self {
        Self {
            run_id: run.id,
            status: run.status.clone(),
            conclusion: run.conclusion.clone(),
            jobs: run
                .jobs
                .iter()
                .map(|job| ReleaseRunJobProjection {
                    name: job.name.clone(),
                    status: job.status.clone(),
                    conclusion: job.conclusion.clone(),
                    steps: job
                        .steps
                        .iter()
                        .map(|step| ReleaseRunStepProjection {
                            number: step.number,
                            name: step.name.clone(),
                            status: step.status.clone(),
                            conclusion: step.conclusion.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

pub fn format_run_progress(
    run: &WorkflowRunStatus,
    decision: ReleaseRunProgressDecision,
) -> String {
    let kind = match decision {
        ReleaseRunProgressDecision::Changed => "状态变化",
        ReleaseRunProgressDecision::Heartbeat => "心跳",
        ReleaseRunProgressDecision::Silent => "状态",
    };
    let mut message = format!("Run {} {kind}：status={}", run.id, run.status);
    if let Some(conclusion) = &run.conclusion {
        let _ = write!(message, ", conclusion={conclusion}");
    }
    for job in &run.jobs {
        let _ = write!(message, "；Job {}: status={}", job.name, job.status);
        if let Some(conclusion) = &job.conclusion {
            let _ = write!(message, ", conclusion={conclusion}");
        }
        for step in &job.steps {
            let _ = write!(
                message,
                "；Step #{} {}: status={}",
                step.number, step.name, step.status
            );
            if let Some(conclusion) = &step.conclusion {
                let _ = write!(message, ", conclusion={conclusion}");
            }
        }
    }
    message
}

fn truncate_entry_message(entry: &mut ReleaseLogEntry, max_entry_bytes: usize) -> bool {
    if serialize_entry(entry).is_ok_and(|bytes| bytes.len() <= max_entry_bytes) {
        return false;
    }

    let original = std::mem::take(&mut entry.message);
    let mut keep_bytes = original.len();
    loop {
        entry.message = format!("{}{}", &original[..keep_bytes], ENTRY_TRUNCATED_SUFFIX);
        let serialized_bytes = serialize_entry(entry).map_or(usize::MAX, |bytes| bytes.len());
        if serialized_bytes <= max_entry_bytes || keep_bytes == 0 {
            return true;
        }

        keep_bytes =
            keep_bytes.saturating_sub(serialized_bytes.saturating_sub(max_entry_bytes).max(1));
        while !original.is_char_boundary(keep_bytes) {
            keep_bytes -= 1;
        }
    }
}

fn parse_log_bytes(bytes: &[u8]) -> Result<Vec<ReleaseLogEntry>, ReleaseLogError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ReleaseLogError::InvalidLog)?;
    let mut entries = Vec::new();
    let mut last_sequence = 0;
    for line in text.lines() {
        let stored: StoredReleaseLogEntry =
            serde_json::from_str(line).map_err(|_| ReleaseLogError::InvalidLog)?;
        if stored.schema_version != LOG_SCHEMA_VERSION || stored.entry.sequence <= last_sequence {
            return Err(ReleaseLogError::InvalidLog);
        }
        last_sequence = stored.entry.sequence;
        entries.push(stored.entry);
    }
    Ok(entries)
}

fn parse_log_bytes_for_page(
    bytes: &[u8],
) -> Result<(Vec<ReleaseLogEntry>, usize, Option<String>), ReleaseLogError> {
    let has_incomplete_tail = !bytes.is_empty() && !bytes.ends_with(b"\n");
    let complete_length = if has_incomplete_tail {
        bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1)
    } else {
        bytes.len()
    };
    let complete = &bytes[..complete_length];
    match parse_log_bytes(complete) {
        Ok(entries) => Ok((
            entries,
            complete_length,
            has_incomplete_tail.then(|| INCOMPLETE_TAIL_WARNING.to_string()),
        )),
        Err(ReleaseLogError::InvalidLog) => {
            let (entries, valid_length) = parse_valid_log_prefix(complete);
            Ok((entries, valid_length, Some(CORRUPT_LOG_WARNING.to_string())))
        }
        Err(error) => Err(error),
    }
}

fn merge_warnings(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(first), Some(second)) if first != second => Some(format!("{first} {second}")),
        (Some(first), _) => Some(first),
        (None, second) => second,
    }
}

fn parse_valid_log_prefix(bytes: &[u8]) -> (Vec<ReleaseLogEntry>, usize) {
    let mut entries = Vec::new();
    let mut valid_length = 0;
    let mut last_sequence = 0;
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        if !line.ends_with(b"\n") {
            break;
        }
        let Ok(text) = std::str::from_utf8(&line[..line.len() - 1]) else {
            break;
        };
        let Ok(stored) = serde_json::from_str::<StoredReleaseLogEntry>(text) else {
            break;
        };
        if stored.schema_version != LOG_SCHEMA_VERSION || stored.entry.sequence <= last_sequence {
            break;
        }
        last_sequence = stored.entry.sequence;
        valid_length += line.len();
        entries.push(stored.entry);
    }
    (entries, valid_length)
}

fn serialize_entry(entry: &ReleaseLogEntry) -> Result<Vec<u8>, ReleaseLogError> {
    let mut bytes = serde_json::to_vec(&StoredReleaseLogEntry {
        schema_version: LOG_SCHEMA_VERSION,
        entry: entry.clone(),
    })
    .map_err(|_| ReleaseLogError::WriteFailed)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn serialize_entries(entries: &[ReleaseLogEntry]) -> Result<Vec<u8>, ReleaseLogError> {
    let mut bytes = Vec::new();
    for entry in entries {
        bytes.extend(serialize_entry(entry)?);
    }
    Ok(bytes)
}

fn is_priority(entry: &ReleaseLogEntry) -> bool {
    matches!(entry.source, crate::models::ReleaseLogSource::Lifecycle)
        || matches!(
            entry.level,
            crate::models::ReleaseLogLevel::Warning | crate::models::ReleaseLogLevel::Error
        )
}

fn is_truncation_marker(entry: &ReleaseLogEntry) -> bool {
    entry.level == crate::models::ReleaseLogLevel::Warning
        && entry.message.contains("早期普通输出已截断")
}

fn build_log_page(
    all_entries: &[ReleaseLogEntry],
    before_sequence: Option<u64>,
    total_bytes: u64,
    page_size: usize,
    recovery_warning: Option<String>,
) -> ReleaseLogPage {
    let eligible = all_entries
        .iter()
        .filter(|entry| before_sequence.is_none_or(|before| entry.sequence < before))
        .cloned()
        .collect::<Vec<_>>();
    let start = eligible.len().saturating_sub(page_size);
    let entries = eligible[start..].to_vec();
    let has_earlier = start > 0;
    let next_before_sequence = has_earlier.then(|| entries[0].sequence);
    let truncation_warning = all_entries
        .iter()
        .find(|entry| is_truncation_marker(entry))
        .map(|entry| entry.message.clone());
    ReleaseLogPage {
        entries,
        next_before_sequence,
        has_earlier,
        total_entries: all_entries.len() as u64,
        total_bytes,
        truncated: truncation_warning.is_some(),
        warning: merge_warnings(recovery_warning, truncation_warning),
    }
}

fn compact_entries(
    entries: Vec<ReleaseLogEntry>,
    policy: ReleaseLogPolicy,
) -> Result<Vec<ReleaseLogEntry>, ReleaseLogError> {
    let template = entries.last().ok_or(ReleaseLogError::InvalidLog)?;
    let marker_template = ReleaseLogEntry {
        session_id: template.session_id.clone(),
        sequence: template.sequence,
        timestamp: template.timestamp.clone(),
        step_id: template.step_id.clone(),
        source: crate::models::ReleaseLogSource::Lifecycle,
        level: crate::models::ReleaseLogLevel::Warning,
        message: "早期普通输出已截断".into(),
    };
    let marker_bytes = serialize_entry(&marker_template)?.len() as u64;
    let target_bytes = ((policy.max_bytes * 8) / 10).max(marker_bytes);
    let target_entries = ((policy.max_entries * 8) / 10).max(1) as usize;
    let entry_limit = if policy.max_entries > 1 {
        target_entries.min(policy.max_entries as usize - 1)
    } else {
        1
    };

    let newest_sequence = template.sequence;
    let mut candidates = entries
        .iter()
        .filter(|entry| !is_truncation_marker(entry))
        .cloned()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        retention_priority(right, newest_sequence)
            .cmp(&retention_priority(left, newest_sequence))
            .then_with(|| right.sequence.cmp(&left.sequence))
    });

    let mut retained = Vec::new();
    let mut retained_bytes = 0_u64;
    for candidate in candidates {
        if retained.len() >= entry_limit {
            break;
        }
        let candidate_bytes = serialize_entry(&candidate)?.len() as u64;
        let next_bytes = marker_bytes + retained_bytes + candidate_bytes;
        let is_newest = candidate.sequence == newest_sequence;
        if next_bytes <= target_bytes || (is_newest && next_bytes <= policy.max_bytes) {
            retained_bytes += candidate_bytes;
            retained.push(candidate);
        }
    }

    let retained_sequences = retained
        .iter()
        .map(|entry| entry.sequence)
        .collect::<std::collections::HashSet<_>>();
    let removed_max = entries
        .iter()
        .filter(|entry| !retained_sequences.contains(&entry.sequence))
        .map(|entry| entry.sequence)
        .max();
    if let Some(sequence) = removed_max
        && policy.max_entries > 1
    {
        retained.push(ReleaseLogEntry {
            sequence,
            ..marker_template
        });
    }
    retained.sort_by_key(|entry| entry.sequence);
    Ok(retained)
}

fn retention_priority(entry: &ReleaseLogEntry, newest_sequence: u64) -> u8 {
    if entry.sequence == newest_sequence {
        5
    } else if entry.level == crate::models::ReleaseLogLevel::Error {
        4
    } else if entry.level == crate::models::ReleaseLogLevel::Warning {
        3
    } else if is_priority(entry) {
        2
    } else {
        1
    }
}

fn validate_log_bytes(bytes: &[u8]) -> Result<(), AppError> {
    parse_log_bytes(bytes)
        .map(|_| ())
        .map_err(|error| AppError::new("RELEASE_LOG_INVALID", "发布日志无效。", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{ReleaseRunProgressDecision, ReleaseRunProgressTracker};
    use crate::models::{WorkflowJobStatus, WorkflowRunStatus, WorkflowStepStatus};
    use std::time::Duration;

    fn run_status(
        run_status: &str,
        run_conclusion: Option<&str>,
        job_status: &str,
        step_status: &str,
    ) -> WorkflowRunStatus {
        WorkflowRunStatus {
            id: 42,
            status: run_status.into(),
            conclusion: run_conclusion.map(str::to_string),
            head_sha: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            url: "https://github.com/hunxuankai/codex-relay/actions/runs/42".into(),
            jobs: vec![WorkflowJobStatus {
                name: "发布 Windows 更新".into(),
                status: job_status.into(),
                conclusion: None,
                started_at: None,
                completed_at: None,
                duration_millis: None,
                steps: vec![WorkflowStepStatus {
                    name: "运行检查".into(),
                    number: 3,
                    status: step_status.into(),
                    conclusion: None,
                    started_at: None,
                    completed_at: None,
                    duration_millis: None,
                }],
            }],
        }
    }

    #[test]
    fn run_progress_tracker_emits_changes_and_five_minute_heartbeats_only() {
        let mut tracker = ReleaseRunProgressTracker::new();
        let queued = run_status("queued", None, "queued", "pending");

        assert_eq!(
            tracker.observe(Duration::ZERO, &queued),
            ReleaseRunProgressDecision::Changed
        );
        assert_eq!(
            tracker.observe(Duration::from_secs(5), &queued),
            ReleaseRunProgressDecision::Silent
        );

        let running = run_status("in_progress", None, "in_progress", "in_progress");
        assert_eq!(
            tracker.observe(Duration::from_secs(10), &running),
            ReleaseRunProgressDecision::Changed
        );
        assert_eq!(
            tracker.observe(Duration::from_secs(309), &running),
            ReleaseRunProgressDecision::Silent
        );
        assert_eq!(
            tracker.observe(Duration::from_secs(310), &running),
            ReleaseRunProgressDecision::Heartbeat
        );
        assert_eq!(
            tracker.observe(Duration::from_secs(315), &running),
            ReleaseRunProgressDecision::Silent
        );

        let completed = run_status("completed", Some("failure"), "completed", "completed");
        assert_eq!(
            tracker.observe(Duration::from_secs(316), &completed),
            ReleaseRunProgressDecision::Changed
        );
    }
}
