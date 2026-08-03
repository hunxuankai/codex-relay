use codex_relay_release_console_lib::app_state::ReleaseEventSink;
use codex_relay_release_console_lib::models::{
    ReleaseEvent, ReleaseLogEntry, ReleaseLogLevel, ReleaseLogPage, ReleaseLogSource,
    ReleaseSession, ReleaseSessionSnapshot,
};
use codex_relay_release_console_lib::services::release_log::{
    ReleaseLogError, ReleaseLogPolicy, ReleaseLogRecorder, ReleaseLogStore, ReleaseProgressSink,
};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};

fn entry(session_id: &str, sequence: u64, message: &str) -> ReleaseLogEntry {
    ReleaseLogEntry {
        session_id: session_id.into(),
        sequence,
        timestamp: "2026-08-03T12:00:00Z".into(),
        step_id: "release-structure-tests".into(),
        source: ReleaseLogSource::Lifecycle,
        level: ReleaseLogLevel::Info,
        message: message.into(),
    }
}

#[test]
fn new_session_replaces_previous_session_logs() {
    let git_dir = tempfile::tempdir().unwrap();
    let store =
        ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), ReleaseLogPolicy::default());

    store.initialize("session-a").unwrap();
    store
        .append(entry("session-a", 1, "first session"))
        .unwrap();
    store.initialize("session-b").unwrap();
    store
        .append(entry("session-b", 1, "second session"))
        .unwrap();

    let page = store.load_page("session-b", None).unwrap();
    assert_eq!(page.entries, vec![entry("session-b", 1, "second session")]);
    assert_eq!(page.total_entries, 1);
    assert!(!page.truncated);

    let persisted = fs::read_to_string(
        git_dir
            .path()
            .join("codex-relay-release-console")
            .join("session.log.jsonl"),
    )
    .unwrap();
    assert!(!persisted.contains("session-a"));
    assert!(persisted.contains("session-b"));
}

#[test]
fn open_restores_sequence_and_pages_existing_session() {
    let git_dir = tempfile::tempdir().unwrap();
    let policy = ReleaseLogPolicy {
        page_size: 2,
        ..ReleaseLogPolicy::default()
    };
    let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    store.initialize("session-a").unwrap();
    for sequence in 1..=3 {
        store
            .append(entry("session-a", sequence, &format!("message {sequence}")))
            .unwrap();
    }

    let reopened = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    let state = reopened.open("session-a").unwrap();
    assert_eq!(state.last_sequence, 3);
    assert_eq!(state.total_entries, 3);
    assert!(state.total_bytes > 0);
    assert!(!state.truncated);
    assert_eq!(state.warning, None);

    reopened.append(entry("session-a", 4, "message 4")).unwrap();
    let latest = reopened.load_page("session-a", None).unwrap();
    assert_eq!(
        latest.entries,
        vec![
            entry("session-a", 3, "message 3"),
            entry("session-a", 4, "message 4"),
        ]
    );
    assert!(latest.has_earlier);
    assert_eq!(latest.next_before_sequence, Some(3));

    let earlier = reopened
        .load_page("session-a", latest.next_before_sequence)
        .unwrap();
    assert_eq!(
        earlier.entries,
        vec![
            entry("session-a", 1, "message 1"),
            entry("session-a", 2, "message 2"),
        ]
    );
    assert!(!earlier.has_earlier);
    assert_eq!(earlier.next_before_sequence, None);
}

#[test]
fn open_repairs_incomplete_tail_before_continuing() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    store
        .append(entry("session-a", 1, "complete entry"))
        .unwrap();
    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    OpenOptions::new()
        .append(true)
        .open(&log_path)
        .unwrap()
        .write_all(b"{\"schemaVersion\":1")
        .unwrap();

    let reopened = ReleaseLogStore::new(git_dir.path().to_path_buf());
    let state = reopened.open("session-a").unwrap();
    assert_eq!(state.last_sequence, 1);
    assert_eq!(
        state.warning.as_deref(),
        Some("发布日志末尾不完整，已保留此前有效记录。")
    );

    reopened
        .append(entry("session-a", 2, "continued entry"))
        .unwrap();
    let page = reopened.load_page("session-a", None).unwrap();
    assert_eq!(
        page.entries,
        vec![
            entry("session-a", 1, "complete entry"),
            entry("session-a", 2, "continued entry"),
        ]
    );
    let persisted = fs::read_to_string(log_path).unwrap();
    assert_eq!(persisted.lines().count(), 2);
    for line in persisted.lines() {
        serde_json::from_str::<serde_json::Value>(line).unwrap();
    }
}

#[test]
fn load_page_reads_the_valid_prefix_without_rewriting_an_untrusted_tail() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    store
        .append(entry("session-a", 1, "complete entry"))
        .unwrap();
    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    let valid = fs::read(&log_path).unwrap();

    let mut incomplete = valid.clone();
    incomplete.extend_from_slice(b"{\"schemaVersion\":1");
    fs::write(&log_path, &incomplete).unwrap();

    let incomplete_page = ReleaseLogStore::new(git_dir.path().to_path_buf())
        .load_page("session-a", None)
        .expect("an incomplete tail must not hide the valid page");
    assert_eq!(
        incomplete_page.entries,
        vec![entry("session-a", 1, "complete entry")]
    );
    assert_eq!(
        incomplete_page.warning.as_deref(),
        Some("发布日志末尾不完整，已保留此前有效记录。")
    );
    assert_eq!(fs::read(&log_path).unwrap(), incomplete);

    let mut corrupt = valid;
    corrupt.extend_from_slice(b"{not-json}\n");
    fs::write(&log_path, &corrupt).unwrap();

    let corrupt_page = ReleaseLogStore::new(git_dir.path().to_path_buf())
        .load_page("session-a", None)
        .expect("a corrupt suffix must not hide the valid page");
    assert_eq!(
        corrupt_page.entries,
        vec![entry("session-a", 1, "complete entry")]
    );
    assert_eq!(
        corrupt_page.warning.as_deref(),
        Some("发布日志包含损坏记录，已停止读取并保留此前有效记录。")
    );
    assert_eq!(fs::read(&log_path).unwrap(), corrupt);
}

#[test]
fn open_discards_corrupt_record_and_untrusted_suffix() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    for sequence in 1..=3 {
        store
            .append(entry("session-a", sequence, &format!("message {sequence}")))
            .unwrap();
    }
    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    let original = fs::read_to_string(&log_path).unwrap();
    let lines = original.lines().collect::<Vec<_>>();
    fs::write(
        &log_path,
        format!("{}\n{{not-json}}\n{}\n", lines[0], lines[2]),
    )
    .unwrap();

    let reopened = ReleaseLogStore::new(git_dir.path().to_path_buf());
    let state = reopened.open("session-a").unwrap();
    assert_eq!(state.last_sequence, 1);
    assert_eq!(state.total_entries, 1);
    assert_eq!(
        state.warning.as_deref(),
        Some("发布日志包含损坏记录，已停止读取并保留此前有效记录。")
    );

    reopened
        .append(entry("session-a", 2, "continued safely"))
        .unwrap();
    let page = reopened.load_page("session-a", None).unwrap();
    assert_eq!(
        page.entries,
        vec![
            entry("session-a", 1, "message 1"),
            entry("session-a", 2, "continued safely"),
        ]
    );
    assert!(!fs::read_to_string(log_path).unwrap().contains("message 3"));
}

#[test]
fn open_stops_at_non_increasing_sequence() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    for sequence in 1..=3 {
        store
            .append(entry("session-a", sequence, &format!("message {sequence}")))
            .unwrap();
    }
    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    let original = fs::read_to_string(&log_path).unwrap();
    let mut lines = original.lines().map(str::to_string).collect::<Vec<_>>();
    let mut duplicate = serde_json::from_str::<serde_json::Value>(&lines[1]).unwrap();
    duplicate["entry"]["sequence"] = serde_json::json!(1);
    lines[1] = serde_json::to_string(&duplicate).unwrap();
    fs::write(&log_path, format!("{}\n", lines.join("\n"))).unwrap();

    let reopened = ReleaseLogStore::new(git_dir.path().to_path_buf());
    let state = reopened.open("session-a").unwrap();
    assert_eq!(state.last_sequence, 1);
    assert_eq!(state.total_entries, 1);
    assert_eq!(
        state.warning.as_deref(),
        Some("发布日志包含损坏记录，已停止读取并保留此前有效记录。")
    );
    reopened
        .append(entry("session-a", 2, "continued after sequence repair"))
        .unwrap();
}

fn bounded_policy() -> ReleaseLogPolicy {
    ReleaseLogPolicy {
        max_bytes: 8 * 1024,
        max_entries: 3,
        max_entry_bytes: 512,
        stream_chunk_bytes: 8,
        page_size: 20,
    }
}

#[test]
fn entry_limit_compacts_old_output_and_persists_truncation_warning() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), bounded_policy());
    store.initialize("session-a").unwrap();
    store
        .append(entry("session-a", 1, "stage started"))
        .unwrap();
    let mut stdout_entry = entry("session-a", 2, "old stdout");
    stdout_entry.source = ReleaseLogSource::Stdout;
    store.append(stdout_entry).unwrap();
    let mut stderr_entry = entry("session-a", 3, "old stderr");
    stderr_entry.source = ReleaseLogSource::Stderr;
    store.append(stderr_entry).unwrap();
    let mut failure = entry("session-a", 4, "latest failure");
    failure.level = ReleaseLogLevel::Error;
    store.append(failure).unwrap();

    let page = store.load_page("session-a", None).unwrap();
    assert!(page.truncated);
    assert!(page.total_entries <= 3);
    assert!(
        page.entries.iter().any(|item| {
            item.level == ReleaseLogLevel::Error && item.message == "latest failure"
        })
    );
    assert!(
        page.entries.iter().any(|item| {
            item.level == ReleaseLogLevel::Warning && item.message.contains("截断")
        })
    );
}

#[test]
fn open_restores_persisted_truncation_state() {
    let git_dir = tempfile::tempdir().unwrap();
    let policy = bounded_policy();
    let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    store.initialize("session-a").unwrap();
    for sequence in 1..=4 {
        let mut output = entry("session-a", sequence, &format!("bounded output {sequence}"));
        output.source = ReleaseLogSource::Stdout;
        store.append(output).unwrap();
    }

    let reopened = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    let state = reopened.open("session-a").unwrap();

    assert!(state.truncated);
    assert!(
        state
            .warning
            .as_deref()
            .is_some_and(|warning| warning.contains("早期普通输出已截断"))
    );
}

#[test]
fn byte_limit_keeps_file_bounded_and_retains_latest_error() {
    let git_dir = tempfile::tempdir().unwrap();
    let policy = ReleaseLogPolicy {
        max_bytes: 900,
        max_entries: 100,
        max_entry_bytes: 512,
        stream_chunk_bytes: 64,
        page_size: 100,
    };
    let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    store.initialize("session-a").unwrap();
    for sequence in 1..=5 {
        let mut output = entry(
            "session-a",
            sequence,
            &format!("old output {sequence} {}", "x".repeat(120)),
        );
        output.source = ReleaseLogSource::Stdout;
        store.append(output).unwrap();
    }
    let mut failure = entry("session-a", 6, "bounded latest failure");
    failure.level = ReleaseLogLevel::Error;
    store.append(failure).unwrap();

    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    assert!(fs::metadata(log_path).unwrap().len() <= policy.max_bytes);
    let page = store.load_page("session-a", None).unwrap();
    assert!(page.truncated);
    assert!(page.entries.iter().any(|item| {
        item.level == ReleaseLogLevel::Error && item.message == "bounded latest failure"
    }));
    assert!(
        page.entries
            .iter()
            .any(|item| item.level == ReleaseLogLevel::Warning)
    );
}

#[test]
fn store_redacts_secret_shapes_and_debug_omits_message_body() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    let unsafe_message = "Authorization: Bearer test-key-store-not-real\nerror: compilation failed";
    let unsafe_entry = entry("session-a", 1, unsafe_message);
    assert!(!format!("{unsafe_entry:?}").contains("compilation failed"));

    store.append(unsafe_entry).unwrap();

    let persisted = fs::read_to_string(
        git_dir
            .path()
            .join("codex-relay-release-console")
            .join("session.log.jsonl"),
    )
    .unwrap();
    assert!(!persisted.contains("test-key-store-not-real"));
    assert!(persisted.contains("[REDACTED]"));
    assert!(persisted.contains("compilation failed"));
    let page = store.load_page("session-a", None).unwrap();
    assert!(!page.entries[0].message.contains("test-key-store-not-real"));
    assert!(page.entries[0].message.contains("compilation failed"));
}

#[test]
fn oversized_serialized_entry_is_rejected_without_changing_file() {
    let git_dir = tempfile::tempdir().unwrap();
    let policy = ReleaseLogPolicy {
        max_entry_bytes: 256,
        ..ReleaseLogPolicy::default()
    };
    let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    store.initialize("session-a").unwrap();
    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    let before = fs::read(&log_path).unwrap();

    let error = store
        .append(entry("session-a", 1, &"x".repeat(2_000)))
        .unwrap_err();

    assert!(matches!(error, ReleaseLogError::EntryTooLarge));
    assert_eq!(fs::read(log_path).unwrap(), before);
    assert_eq!(store.load_page("session-a", None).unwrap().total_entries, 0);
}

#[test]
fn append_reports_log_read_failures_instead_of_treating_them_as_empty() {
    let git_dir = tempfile::tempdir().unwrap();
    let policy = ReleaseLogPolicy {
        max_entries: 0,
        ..ReleaseLogPolicy::default()
    };
    let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    store.initialize("session-a").unwrap();
    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    fs::remove_file(&log_path).unwrap();
    fs::create_dir(&log_path).unwrap();

    let error = store
        .append(entry("session-a", 1, "diagnostic"))
        .unwrap_err();

    assert!(matches!(error, ReleaseLogError::ReadFailed));
}

#[derive(Default)]
struct MemoryEventSink {
    events: Mutex<Vec<ReleaseEvent>>,
}

impl ReleaseEventSink for MemoryEventSink {
    fn send(&self, event: ReleaseEvent) -> Result<(), String> {
        self.events.lock().unwrap().push(event);
        Ok(())
    }
}

#[test]
fn recorder_assigns_sequence_and_mirrors_persisted_entries_to_events() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    let events = Arc::new(MemoryEventSink::default());
    let recorder = ReleaseLogRecorder::new(
        "session-a",
        store,
        0,
        Some(events.clone() as Arc<dyn ReleaseEventSink>),
    );

    let first = recorder.record(
        "release-structure-tests",
        ReleaseLogSource::Stdout,
        ReleaseLogLevel::Info,
        "first diagnostic",
    );
    let second = recorder.record(
        "release-structure-tests",
        ReleaseLogSource::Stderr,
        ReleaseLogLevel::Error,
        "second diagnostic",
    );

    assert_eq!((first.sequence, second.sequence), (1, 2));
    let reader = ReleaseLogStore::new(git_dir.path().to_path_buf());
    let persisted = reader.load_page("session-a", None).unwrap().entries;
    assert_eq!(persisted, vec![first.clone(), second.clone()]);
    assert_eq!(
        events.events.lock().unwrap().as_slice(),
        [
            ReleaseEvent::StepLog {
                entry: first,
                page: None,
            },
            ReleaseEvent::StepLog {
                entry: second,
                page: None,
            },
        ]
    );
}

#[test]
fn recorder_projects_the_authoritative_page_when_storage_compacts() {
    let git_dir = tempfile::tempdir().unwrap();
    let policy = bounded_policy();
    let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    store.initialize("session-a").unwrap();
    let events = Arc::new(MemoryEventSink::default());
    let recorder = ReleaseLogRecorder::new(
        "session-a",
        store,
        0,
        Some(events.clone() as Arc<dyn ReleaseEventSink>),
    );

    for sequence in 1..=3 {
        recorder.record(
            "release-structure-tests",
            ReleaseLogSource::Stdout,
            ReleaseLogLevel::Info,
            format!("ordinary output {sequence}"),
        );
    }
    recorder.record(
        "release-structure-tests",
        ReleaseLogSource::Stderr,
        ReleaseLogLevel::Error,
        "latest failure",
    );

    let compacted_page = events
        .events
        .lock()
        .unwrap()
        .iter()
        .find_map(|event| match event {
            ReleaseEvent::StepLog {
                page: Some(page), ..
            } => Some(page.clone()),
            _ => None,
        })
        .expect("compaction should be projected to the realtime log reducer");
    assert!(compacted_page.truncated);
    assert!(compacted_page.total_entries <= policy.max_entries);
    assert!(
        compacted_page
            .entries
            .iter()
            .any(|entry| entry.message.contains("早期普通输出已截断"))
    );
    assert!(
        compacted_page
            .entries
            .iter()
            .any(|entry| entry.level == ReleaseLogLevel::Error && entry.message == "latest failure")
    );
}

#[test]
fn recorder_switches_to_volatile_logging_and_warns_once_after_write_failure() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    let events = Arc::new(MemoryEventSink::default());
    let recorder = ReleaseLogRecorder::new(
        "session-a",
        store,
        0,
        Some(events.clone() as Arc<dyn ReleaseEventSink>),
    );
    fs::remove_file(&log_path).unwrap();
    fs::create_dir(&log_path).unwrap();

    recorder.record(
        "release-structure-tests",
        ReleaseLogSource::Stdout,
        ReleaseLogLevel::Info,
        "first volatile diagnostic",
    );
    recorder.record(
        "release-structure-tests",
        ReleaseLogSource::Stderr,
        ReleaseLogLevel::Error,
        "second volatile diagnostic",
    );

    let entries = events
        .events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|event| match event {
            ReleaseEvent::StepLog { entry, .. } => Some(entry.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.level == ReleaseLogLevel::Warning)
            .count(),
        1
    );
    assert!(entries[1].message.contains("重启后可能丢失"));
    assert_eq!(entries[2].message, "second volatile diagnostic");
}

#[test]
fn recorder_truncates_oversized_utf8_message_and_persists_a_warning() {
    let git_dir = tempfile::tempdir().unwrap();
    let policy = ReleaseLogPolicy {
        max_entry_bytes: 512,
        ..ReleaseLogPolicy::default()
    };
    let store = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    store.initialize("session-a").unwrap();
    let events = Arc::new(MemoryEventSink::default());
    let recorder = ReleaseLogRecorder::new(
        "session-a",
        store,
        0,
        Some(events.clone() as Arc<dyn ReleaseEventSink>),
    );
    let oversized = "诊断输出".repeat(1_000);

    let recorded = recorder.record(
        "release-console-rust-tests",
        ReleaseLogSource::Stderr,
        ReleaseLogLevel::Error,
        &oversized,
    );

    assert!(recorded.message.len() < oversized.len());
    assert!(recorded.message.starts_with("诊断输出"));
    assert!(recorded.message.contains("单条大小上限"));
    let reader = ReleaseLogStore::with_policy(git_dir.path().to_path_buf(), policy);
    let persisted = reader.load_page("session-a", None).unwrap().entries;
    assert_eq!(persisted.len(), 2);
    assert_eq!(persisted[0], recorded);
    assert_eq!(persisted[1].level, ReleaseLogLevel::Warning);
    assert!(persisted[1].message.contains("UTF-8"));
    let log_path = git_dir
        .path()
        .join("codex-relay-release-console")
        .join("session.log.jsonl");
    for line in fs::read(log_path)
        .unwrap()
        .split_inclusive(|byte| *byte == b'\n')
    {
        assert!(line.len() <= policy.max_entry_bytes);
    }
    assert_eq!(events.events.lock().unwrap().len(), 2);
}

#[test]
fn recorder_progress_emits_timeline_events_around_persisted_lifecycle_logs() {
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    let events = Arc::new(MemoryEventSink::default());
    let recorder = ReleaseLogRecorder::new(
        "session-a",
        store,
        0,
        Some(events.clone() as Arc<dyn ReleaseEventSink>),
    );
    let progress: &dyn ReleaseProgressSink = &recorder;

    progress.started("candidate", "开始应用发布候选。");
    progress.log("candidate", ReleaseLogLevel::Warning, "候选检查需要关注。");
    progress.completed("candidate", 120, "发布候选已应用。");
    progress.failed("commitPush", "RELEASE_PUSH_FAILED", "候选提交或推送失败。");

    let reader = ReleaseLogStore::new(git_dir.path().to_path_buf());
    let persisted = reader.load_page("session-a", None).unwrap().entries;
    assert_eq!(persisted.len(), 4);
    assert!(
        persisted
            .iter()
            .all(|entry| entry.source == ReleaseLogSource::Lifecycle)
    );
    assert_eq!(persisted[1].level, ReleaseLogLevel::Warning);
    assert_eq!(persisted[3].level, ReleaseLogLevel::Error);

    let emitted = events.events.lock().unwrap();
    assert!(
        matches!(emitted[0], ReleaseEvent::StepStarted { ref step_id, .. } if step_id == "candidate")
    );
    assert!(matches!(emitted[1], ReleaseEvent::StepLog { ref entry, .. } if entry.sequence == 1));
    assert!(matches!(emitted[3], ReleaseEvent::StepLog { ref entry, .. } if entry.sequence == 3));
    assert!(
        matches!(emitted[4], ReleaseEvent::StepCompleted { ref step_id, duration_millis: 120, .. } if step_id == "candidate")
    );
    assert!(matches!(emitted[5], ReleaseEvent::StepLog { ref entry, .. } if entry.sequence == 4));
    assert!(
        matches!(emitted[6], ReleaseEvent::StepFailed { ref step_id, ref code, .. } if step_id == "commitPush" && code == "RELEASE_PUSH_FAILED")
    );
}

#[test]
fn snapshot_page_and_step_log_use_the_camel_case_contract() {
    let log_entry = entry("session-a", 7, "safe diagnostic");
    let page = ReleaseLogPage {
        entries: vec![log_entry.clone()],
        next_before_sequence: Some(7),
        has_earlier: true,
        total_entries: 42,
        total_bytes: 4_096,
        truncated: true,
        warning: Some("早期日志已截断".into()),
    };
    let snapshot = ReleaseSessionSnapshot {
        session: ReleaseSession::new("session-a", "D:/safe-repository", "0.5.0"),
        logs: page.clone(),
    };

    let snapshot_json = serde_json::to_value(snapshot).unwrap();
    assert_eq!(snapshot_json["logs"]["nextBeforeSequence"], 7);
    assert_eq!(snapshot_json["logs"]["hasEarlier"], true);
    assert_eq!(snapshot_json["logs"]["totalEntries"], 42);
    assert_eq!(snapshot_json["logs"]["totalBytes"], 4_096);
    assert_eq!(
        snapshot_json["logs"]["entries"][0]["sessionId"],
        "session-a"
    );

    let event_json = serde_json::to_value(ReleaseEvent::StepLog {
        entry: log_entry,
        page: None,
    })
    .unwrap();
    assert_eq!(event_json["kind"], "stepLog");
    assert_eq!(event_json["entry"]["sequence"], 7);
    assert_eq!(event_json["entry"]["source"], "lifecycle");
    assert_eq!(event_json["entry"]["level"], "info");
    assert_eq!(event_json["entry"]["message"], "safe diagnostic");
    assert!(event_json.get("page").is_none());
    assert!(event_json.get("stepId").is_none());

    let compacted_event_json = serde_json::to_value(ReleaseEvent::StepLog {
        entry: page.entries[0].clone(),
        page: Some(page),
    })
    .unwrap();
    assert_eq!(compacted_event_json["page"]["totalEntries"], 42);
    assert_eq!(compacted_event_json["page"]["totalBytes"], 4_096);
    assert_eq!(compacted_event_json["page"]["truncated"], true);
}
