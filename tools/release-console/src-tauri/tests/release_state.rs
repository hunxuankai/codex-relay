use codex_relay_release_console_lib::models::{
    DraftAssetEvidence, DraftAuditEvidence, ReleaseFailureEvidence, ReleasePhase, ReleaseSession,
    WorkflowDispatch,
};
use codex_relay_release_console_lib::services::release_state::{
    ReleaseStateError, ReleaseStateStore, RepositorySessionLock,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

struct TempGitDir {
    path: PathBuf,
}

impl TempGitDir {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "codex-relay-release-state-{}-{id}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempGitDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn versioned_session_is_saved_atomically_and_loaded_with_phase() {
    let git_dir = TempGitDir::new();
    let store = ReleaseStateStore::new(git_dir.path.clone());
    let mut session = ReleaseSession::new("session-test-1", r"D:\safe-temp\repository", "0.5.0");
    session.phase = session
        .phase
        .transition_to(ReleasePhase::Inspected)
        .unwrap();
    session.phase = session.phase.transition_to(ReleasePhase::Planned).unwrap();

    store.save(&session).unwrap();
    let loaded = store.load().unwrap().unwrap();

    assert_eq!(loaded, session);
    let persisted: serde_json::Value = serde_json::from_slice(
        &fs::read(
            git_dir
                .path
                .join("codex-relay-release-console/session.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(persisted["schemaVersion"], 1);
    assert_eq!(persisted["session"]["phase"], "planned");
    assert_eq!(persisted["session"]["targetVersion"], "0.5.0");
    assert!(persisted["session"]["failure"].is_null());
}

#[test]
fn legacy_failed_session_without_failure_evidence_remains_readable() {
    let git_dir = TempGitDir::new();
    let state_file = git_dir
        .path
        .join("codex-relay-release-console/session.json");
    fs::create_dir_all(state_file.parent().unwrap()).unwrap();
    let legacy = br#"{
  "schemaVersion": 1,
  "session": {
    "id": "session-legacy-failed",
    "repositoryPath": "D:\\safe-temp\\repository",
    "targetVersion": "0.5.0",
    "phase": "failed",
    "candidateSha": null,
    "remoteMainSha": null
  }
}
"#;
    fs::write(&state_file, legacy).unwrap();

    let loaded = ReleaseStateStore::new(git_dir.path.clone())
        .load()
        .unwrap()
        .unwrap();

    assert_eq!(loaded.phase, ReleasePhase::Failed);
    assert_eq!(loaded.failure, None);
    assert_eq!(fs::read(state_file).unwrap(), legacy);
}

#[test]
fn nonfailed_session_with_failure_evidence_is_rejected_without_rewriting_file() {
    let git_dir = TempGitDir::new();
    let state_file = git_dir
        .path
        .join("codex-relay-release-console/session.json");
    fs::create_dir_all(state_file.parent().unwrap()).unwrap();
    let invalid = br#"{
  "schemaVersion": 1,
  "session": {
    "id": "session-invalid-failure",
    "repositoryPath": "D:\\safe-temp\\repository",
    "targetVersion": "0.5.0",
    "phase": "planned",
    "candidateSha": null,
    "remoteMainSha": null,
    "failure": {
      "phase": "planned",
      "stepId": "plan",
      "code": "RELEASE_PLAN_FAILED"
    }
  }
}
"#;
    fs::write(&state_file, invalid).unwrap();

    let error = ReleaseStateStore::new(git_dir.path.clone())
        .load()
        .unwrap_err();

    assert!(matches!(error, ReleaseStateError::InvalidState));
    assert_eq!(fs::read(state_file).unwrap(), invalid);
}

#[test]
fn semantically_corrupted_pushed_state_is_rejected_without_rewriting_file() {
    let git_dir = TempGitDir::new();
    let state_file = git_dir
        .path
        .join("codex-relay-release-console/session.json");
    fs::create_dir_all(state_file.parent().unwrap()).unwrap();
    let corrupted = br#"{
  "schemaVersion": 1,
  "session": {
    "id": "session-test-1",
    "repositoryPath": "D:\\safe-temp\\repository",
    "targetVersion": "0.5.0",
    "phase": "pushed",
    "candidateSha": null,
    "remoteMainSha": null
  }
}
"#;
    fs::write(&state_file, corrupted).unwrap();
    let store = ReleaseStateStore::new(git_dir.path.clone());

    let error = store.load().unwrap_err();

    assert!(matches!(error, ReleaseStateError::InvalidState));
    assert_eq!(fs::read(state_file).unwrap(), corrupted);
}

#[test]
fn repository_lock_allows_only_one_active_process_and_releases_on_drop() {
    let git_dir = TempGitDir::new();

    let first = RepositorySessionLock::acquire(&git_dir.path).unwrap();
    let second = RepositorySessionLock::acquire(&git_dir.path).unwrap_err();
    assert!(matches!(second, ReleaseStateError::SessionLocked));

    drop(first);
    RepositorySessionLock::acquire(&git_dir.path).unwrap();
}

#[test]
fn active_session_blocks_reinitialization_until_it_reaches_a_terminal_phase() {
    let git_dir = TempGitDir::new();
    let store = ReleaseStateStore::new(git_dir.path.clone());
    let first = ReleaseSession::new("session-first", r"D:\safe-temp\repository", "0.5.0");
    let second = ReleaseSession::new("session-second", r"D:\safe-temp\repository", "0.6.0");

    store.initialize(&first).unwrap();
    let error = store.initialize(&second).unwrap_err();
    assert!(matches!(error, ReleaseStateError::ActiveSessionExists));
    assert_eq!(store.load().unwrap().unwrap(), first);

    let mut terminal = first;
    terminal.phase = ReleasePhase::Failed;
    store.save(&terminal).unwrap();
    store.initialize(&second).unwrap();
    let initialized = store.load().unwrap().unwrap();
    assert_eq!(initialized, second);
    assert_eq!(initialized.failure, None);
}

#[test]
fn phase_advance_is_persisted_and_invalid_skip_leaves_last_valid_state() {
    let git_dir = TempGitDir::new();
    let store = ReleaseStateStore::new(git_dir.path.clone());
    let mut session = ReleaseSession::new("session-test-1", r"D:\safe-temp\repository", "0.5.0");

    store
        .advance(&mut session, ReleasePhase::Inspected)
        .unwrap();
    store.advance(&mut session, ReleasePhase::Planned).unwrap();
    let error = store
        .advance(&mut session, ReleasePhase::LocalBuild)
        .unwrap_err();

    assert!(matches!(error, ReleaseStateError::InvalidTransition));
    assert_eq!(session.phase, ReleasePhase::Planned);
    assert_eq!(store.load().unwrap().unwrap().phase, ReleasePhase::Planned);
}

#[test]
fn failure_checkpoint_is_saved_atomically_with_the_terminal_phase() {
    let git_dir = TempGitDir::new();
    let store = ReleaseStateStore::new(git_dir.path.clone());
    let mut session =
        ReleaseSession::new("session-local-failure", r"D:\safe-temp\repository", "0.5.0");
    session.phase = ReleasePhase::LocalChecks;
    store.save(&session).unwrap();

    store
        .fail(
            &mut session,
            "full-project-check",
            "RELEASE_LOCAL_VERIFICATION_FAILED",
        )
        .unwrap();

    let expected = ReleaseFailureEvidence {
        phase: ReleasePhase::LocalChecks,
        step_id: "full-project-check".into(),
        code: "RELEASE_LOCAL_VERIFICATION_FAILED".into(),
    };
    assert_eq!(session.phase, ReleasePhase::Failed);
    assert_eq!(session.failure.as_ref(), Some(&expected));
    assert_eq!(store.load().unwrap().unwrap(), session);
}

#[test]
fn remote_release_checkpoints_round_trip_for_restart_recovery() {
    let git_dir = TempGitDir::new();
    let store = ReleaseStateStore::new(git_dir.path.clone());
    let mut session =
        ReleaseSession::new("session-test-remote", r"D:\safe-temp\repository", "0.5.0");
    session.phase = ReleasePhase::AwaitingPublishApproval;
    session.candidate_sha = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
    session.remote_main_sha = session.candidate_sha.clone();
    session.workflow = Some(WorkflowDispatch {
        run_id: 123,
        url: "https://github.com/hunxuankai/codex-relay/actions/runs/123".into(),
    });
    session.draft = Some(DraftAuditEvidence {
        release_id: 42,
        tag_name: "v0.5.0".into(),
        target_commit_sha: session.candidate_sha.clone().unwrap(),
        assets: vec![DraftAssetEvidence {
            id: 501,
            name: "Codex.Relay_0.5.0_x64-setup.exe".into(),
            size: 9,
            sha256: "9c0d294c05fc1d88d698034609bb81c0c69196327594e4c69d2915c80fd9850c".into(),
        }],
        manifest_version: "0.5.0".into(),
        manifest_notes: "最终说明".into(),
        signature: "signature-test-not-real".into(),
    });

    store.save(&session).unwrap();

    assert_eq!(store.load().unwrap().unwrap(), session);
}
