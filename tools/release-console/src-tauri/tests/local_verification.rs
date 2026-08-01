use codex_relay_release_console_lib::infrastructure::local_verification::ProcessLocalVerificationBackend;
use codex_relay_release_console_lib::infrastructure::process::filter_release_environment;
use codex_relay_release_console_lib::services::local_verification::{
    LocalArtifactService, LocalCommandEvidence, LocalExecutable, LocalVerificationBackend,
    LocalVerificationBackendError, LocalVerificationCommand, LocalVerificationError,
    LocalVerificationService,
};
use std::ffi::OsString;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

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
            exit_code,
        } if command_id == "release-console-rust-tests" && exit_code == Some(1)
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
        Box::pin(async { Err(LocalVerificationBackendError::Failed) })
    }
}

#[test]
fn backend_failure_preserves_command_identity_without_inventing_an_exit_code() {
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
            exit_code: None,
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
    let path = std::env::var_os("PATH").expect("PATH must be available for release checks");
    let find_on_path = |file_name: &str| {
        std::env::split_paths(&path)
            .map(|directory| directory.join(file_name))
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| panic!("{file_name} must be available for release checks"))
    };
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
