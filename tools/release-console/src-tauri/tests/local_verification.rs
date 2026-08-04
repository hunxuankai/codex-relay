use codex_relay_release_console_lib::infrastructure::local_verification::ProcessLocalVerificationBackend;
use codex_relay_release_console_lib::infrastructure::process::filter_release_environment;
use codex_relay_release_console_lib::services::local_verification::{
    LocalArtifactService, LocalCommandEvidence, LocalExecutable, LocalVerificationBackend,
    LocalVerificationBackendError, LocalVerificationCommand, LocalVerificationError,
    LocalVerificationFailure, LocalVerificationProcessError, LocalVerificationService,
};
use codex_relay_release_console_lib::services::release_log::{ReleaseLogRecorder, ReleaseLogStore};
use std::ffi::OsString;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);
const WINDOWS_PROCESS_TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

fn find_on_path(file_name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH must be available for release checks");
    std::env::split_paths(&path)
        .map(|directory| directory.join(file_name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{file_name} must be available for release checks"))
}

fn set_environment(
    environment: &mut Vec<(OsString, OsString)>,
    name: &str,
    value: impl Into<OsString>,
) {
    let value = value.into();
    if let Some((_, existing)) = environment
        .iter_mut()
        .find(|(existing, _)| existing.to_string_lossy().eq_ignore_ascii_case(name))
    {
        *existing = value;
    } else {
        environment.push((OsString::from(name), value));
    }
}

struct RecordingBackend {
    commands: Mutex<Vec<LocalVerificationCommand>>,
}

impl LocalVerificationBackend for RecordingBackend {
    fn run<'a>(
        &'a self,
        _repository_path: &'a Path,
        command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        self.commands.lock().unwrap().push(command.clone());
        Box::pin(async move {
            Ok(LocalCommandEvidence {
                id: command.id.clone(),
                exit_code: 0,
                duration_millis: 10,
            })
        })
    }
}

#[test]
fn local_verification_runs_only_fixed_release_check_and_build_commands_in_order() {
    let backend = RecordingBackend {
        commands: Mutex::new(Vec::new()),
    };
    let service = LocalVerificationService::new();

    let evidence = tauri::async_runtime::block_on(
        service.run(&backend, Path::new(r"D:\safe-temp\repository")),
    )
    .unwrap();

    let commands = backend.commands.into_inner().unwrap();
    assert_eq!(
        commands
            .iter()
            .map(|command| command.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "release-structure-tests",
            "release-console-rust-tests",
            "full-project-check",
            "ordinary-build",
        ]
    );
    assert_eq!(commands[0].executable, LocalExecutable::Npm);
    assert_eq!(commands[0].args[0..3], ["exec", "--", "vitest"]);
    assert_eq!(commands[1].executable, LocalExecutable::Cargo);
    assert!(
        commands[1]
            .args
            .contains(&"codex-relay-release-console".into())
    );
    assert_eq!(commands[2].args, ["run", "check"]);
    assert_eq!(commands[3].args, ["run", "build"]);
    assert_eq!(
        evidence
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        commands
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>()
    );
}

struct NonZeroSecondCommandBackend {
    commands: Mutex<Vec<String>>,
}

impl LocalVerificationBackend for NonZeroSecondCommandBackend {
    fn run<'a>(
        &'a self,
        _repository_path: &'a Path,
        command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        let mut commands = self.commands.lock().unwrap();
        commands.push(command.id.clone());
        let exit_code = if commands.len() == 2 { 1 } else { 0 };
        Box::pin(async move {
            Ok(LocalCommandEvidence {
                id: command.id.clone(),
                exit_code,
                duration_millis: 10,
            })
        })
    }
}

#[test]
fn nonzero_command_stops_later_checks_and_preserves_failed_step_identity() {
    let backend = NonZeroSecondCommandBackend {
        commands: Mutex::new(Vec::new()),
    };
    let service = LocalVerificationService::new();

    let error = tauri::async_runtime::block_on(
        service.run(&backend, Path::new(r"D:\safe-temp\repository")),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        LocalVerificationError::CommandFailed {
            command_id,
            failure: LocalVerificationFailure::ExitCode(1),
        } if command_id == "release-console-rust-tests"
    ));
    assert_eq!(
        backend.commands.into_inner().unwrap(),
        ["release-structure-tests", "release-console-rust-tests"]
    );
}

struct BackendFailureBackend;

impl LocalVerificationBackend for BackendFailureBackend {
    fn run<'a>(
        &'a self,
        _repository_path: &'a Path,
        _command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async {
            Err(LocalVerificationBackendError::Process(
                LocalVerificationProcessError::Timeout,
            ))
        })
    }
}

#[test]
fn backend_failure_preserves_command_identity_and_safe_process_classification() {
    let service = LocalVerificationService::new();

    let error = tauri::async_runtime::block_on(service.run(
        &BackendFailureBackend,
        Path::new(r"D:\safe-temp\repository"),
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        LocalVerificationError::CommandFailed {
            command_id,
            failure: LocalVerificationFailure::Process(
                LocalVerificationProcessError::Timeout
            ),
        } if command_id == "release-structure-tests"
    ));
}

#[test]
fn ordinary_build_artifacts_are_enumerated_with_actual_size_time_and_sha256() {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let repository = std::env::temp_dir().join(format!(
        "codex-relay-local-artifacts-{}-{id}",
        std::process::id()
    ));
    let release_dir = repository.join("src-tauri/target/release");
    let nsis_dir = release_dir.join("bundle/nsis");
    fs::create_dir_all(&nsis_dir).unwrap();
    fs::write(release_dir.join("CodexRelay.exe"), b"main-exe").unwrap();
    fs::write(
        nsis_dir.join("Codex.Relay_0.5.0_x64-setup.exe"),
        b"nsis-exe",
    )
    .unwrap();

    let artifacts = LocalArtifactService::enumerate(&repository).unwrap();

    assert_eq!(artifacts.len(), 2);
    assert_eq!(
        artifacts[0].relative_path,
        PathBuf::from("src-tauri/target/release/CodexRelay.exe")
    );
    assert_eq!(artifacts[0].size, 8);
    assert_eq!(
        artifacts[0].sha256,
        "13c8c796e1ad956551d152aa7adea00037facecd23643e55e3bc8372a0e82263"
    );
    assert!(artifacts[0].modified_unix_millis > 0);
    assert_eq!(artifacts[1].size, 8);
    assert_eq!(
        artifacts[1].sha256,
        "476ed4128d233f64455e9ecb5703ff397832c981b2d91e42cf85f56c10176b65"
    );

    fs::remove_dir_all(repository).unwrap();
}

#[test]
fn process_backend_builds_direct_invocation_with_filtered_environment_and_no_shell() {
    let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
    let backend = ProcessLocalVerificationBackend::new(
        PathBuf::from(r"D:\tools\npm.cmd"),
        PathBuf::from(r"D:\tools\cargo.exe"),
        filter_release_environment([
            (OsString::from("PATH"), OsString::from(r"D:\safe-bin")),
            (
                OsString::from("TAURI_SIGNING_PRIVATE_KEY"),
                OsString::from("test-private-key-not-real"),
            ),
            (
                OsString::from("GH_TOKEN"),
                OsString::from("github_pat_test-token-not-real"),
            ),
            (
                OsString::from("CODEX_HOME"),
                OsString::from(r"D:\unsafe-codex-home"),
            ),
        ]),
        cancel,
    );
    let command = LocalVerificationCommand {
        id: "ordinary-build".into(),
        executable: LocalExecutable::Npm,
        args: vec!["run".into(), "build".into()],
    };

    let invocation = backend.invocation_for(Path::new(r"D:\safe-temp\repository"), &command);

    assert_eq!(invocation.executable, PathBuf::from(r"D:\tools\npm.cmd"));
    assert_eq!(invocation.args, ["run", "build"]);
    assert_eq!(
        invocation.workdir,
        PathBuf::from(r"D:\safe-temp\repository")
    );
    assert_eq!(
        invocation.env,
        [(OsString::from("PATH"), OsString::from(r"D:\safe-bin"))]
    );
    let debug = format!("{invocation:?}");
    assert!(!debug.contains("cmd.exe"));
    assert!(!debug.contains("powershell"));
    assert!(!debug.contains("test-private-key-not-real"));
    assert!(!debug.contains("test-token-not-real"));
    assert!(!debug.contains("unsafe-codex-home"));
}

#[test]
fn process_backend_persists_safe_output_before_the_command_completes() {
    tauri::async_runtime::block_on(async {
        let repository = tempfile::tempdir().unwrap();
        let git_dir = tempfile::tempdir().unwrap();
        let stream_script = repository.path().join("stream-output.ps1");
        let release_file = repository.path().join("release-stream");
        fs::write(
            &stream_script,
            r#"param([string]$ReleaseFile)
$ErrorActionPreference = 'Stop'
[Console]::Out.Write("first`n")
[Console]::Out.Flush()
$deadline = [DateTime]::UtcNow.AddSeconds(20)
while (-not (Test-Path -LiteralPath $ReleaseFile)) {
    if ([DateTime]::UtcNow -ge $deadline) {
        throw 'STREAM_RELEASE_TIMEOUT'
    }
    Start-Sleep -Milliseconds 20
}
[Console]::Out.Write("second-tail")
"#,
        )
        .unwrap();
        let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
        store.initialize("session-a").unwrap();
        let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
        let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
        let powershell = find_on_path("powershell.exe");
        let backend = ProcessLocalVerificationBackend::with_recorder(
            powershell.clone(),
            powershell,
            filter_release_environment(std::env::vars_os()),
            cancel,
            recorder,
        );
        let command = LocalVerificationCommand {
            id: "release-console-rust-tests".into(),
            executable: LocalExecutable::Npm,
            args: vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-File".into(),
                stream_script.to_string_lossy().into_owned(),
                release_file.to_string_lossy().into_owned(),
            ],
        };
        let reader = ReleaseLogStore::new(git_dir.path().to_path_buf());
        let run = backend.run(repository.path(), &command);
        tokio::pin!(run);
        let deadline = tokio::time::sleep(WINDOWS_PROCESS_TEST_TIMEOUT);
        tokio::pin!(deadline);

        loop {
            tokio::select! {
                result = &mut run => panic!("command completed before the first streamed log: {result:?}"),
                _ = tokio::time::sleep(std::time::Duration::from_millis(20)) => {
                    let page = reader.load_page("session-a", None).unwrap();
                    if page.entries.iter().any(|entry| entry.message == "first\n") {
                        break;
                    }
                }
                _ = &mut deadline => panic!("first streamed log did not arrive before the deadline"),
            }
        }

        fs::write(&release_file, b"release").unwrap();
        let evidence = tokio::time::timeout(WINDOWS_PROCESS_TEST_TIMEOUT, &mut run)
            .await
            .expect("command should complete after the delayed output")
            .unwrap();
        assert_eq!(evidence.exit_code, 0);
        let page = reader.load_page("session-a", None).unwrap();
        assert_eq!(
            page.entries
                .iter()
                .map(|entry| entry.message.as_str())
                .collect::<String>(),
            "first\nsecond-tail"
        );
    });
}

struct FirstCommandProcessBackend {
    process: ProcessLocalVerificationBackend,
    calls: AtomicU64,
}

impl LocalVerificationBackend for FirstCommandProcessBackend {
    fn run<'a>(
        &'a self,
        repository_path: &'a Path,
        command: &'a LocalVerificationCommand,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<LocalCommandEvidence, LocalVerificationBackendError>>
                + Send
                + 'a,
        >,
    > {
        if self.calls.fetch_add(1, Ordering::Relaxed) == 0 {
            self.process.run(repository_path, command)
        } else {
            Box::pin(async move {
                Ok(LocalCommandEvidence {
                    id: command.id.clone(),
                    exit_code: 0,
                    duration_millis: 0,
                })
            })
        }
    }
}

#[test]
fn filtered_process_backend_runs_release_structure_tests_without_encoding_sensitive_failure() {
    let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
    let backend = FirstCommandProcessBackend {
        process: ProcessLocalVerificationBackend::new(
            find_on_path("npm.cmd"),
            find_on_path("cargo.exe"),
            filter_release_environment(std::env::vars_os()),
            cancel,
        ),
        calls: AtomicU64::new(0),
    };
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();

    let evidence =
        tauri::async_runtime::block_on(LocalVerificationService::new().run(&backend, &repository))
            .unwrap();

    assert_eq!(evidence[0].id, "release-structure-tests");
    assert_eq!(evidence[0].exit_code, 0);
}

#[test]
#[ignore = "runs the complete project check through the production process backend"]
fn filtered_process_backend_runs_full_project_check_without_backend_failure() {
    let safe_root = tempfile::tempdir().unwrap();
    let profile = safe_root.path().join("profile");
    let app_data = safe_root.path().join("appdata");
    let local_app_data = safe_root.path().join("localappdata");
    let npm_cache = safe_root.path().join("npm-cache");
    let temp = safe_root.path().join("temp");
    for directory in [&profile, &app_data, &local_app_data, &npm_cache, &temp] {
        fs::create_dir_all(directory).unwrap();
    }
    let mut environment = filter_release_environment(std::env::vars_os());
    for (name, value) in [
        ("USERPROFILE", profile.as_os_str()),
        ("APPDATA", app_data.as_os_str()),
        ("LOCALAPPDATA", local_app_data.as_os_str()),
        ("NPM_CONFIG_CACHE", npm_cache.as_os_str()),
        ("TEMP", temp.as_os_str()),
        ("TMP", temp.as_os_str()),
    ] {
        set_environment(&mut environment, name, value);
    }
    let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
    let backend = ProcessLocalVerificationBackend::new(
        find_on_path("npm.cmd"),
        find_on_path("cargo.exe"),
        environment,
        cancel,
    );
    let command = LocalVerificationCommand {
        id: "full-project-check".into(),
        executable: LocalExecutable::Npm,
        args: vec!["run".into(), "check".into()],
    };
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();

    let evidence = tauri::async_runtime::block_on(backend.run(&repository, &command)).unwrap();

    assert_eq!(evidence.id, "full-project-check");
    assert_eq!(evidence.exit_code, 0);
}
