use codex_relay_core::error::AppError;
use codex_relay_release_console_lib::services::release_candidate::{
    CandidateFileOps, CandidateWritePhase, ReleaseCandidateTransaction, StdCandidateFileOps,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);
const VALID_RELEASE_NOTES: &str = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";

struct TempRepository {
    root: PathBuf,
}

impl TempRepository {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-relay-release-console-{}-{id}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(root.join("src-tauri/crates/codex-relay-core")).unwrap();
        fs::create_dir_all(root.join(".github")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn write(&self, relative_path: &str, contents: &str) {
        let path = self.root.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents.as_bytes()).unwrap();
    }

    fn read(&self, relative_path: &str) -> Vec<u8> {
        fs::read(self.root.join(relative_path)).unwrap()
    }
}

impl Drop for TempRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn create_repository_fixture() -> TempRepository {
    let repository = TempRepository::new();
    repository.write(
        "package.json",
        r#"{
  "name": "codex-relay",
  "version": "0.4.0",
  "private": true,
  "releaseConsoleUnknown": { "keep": true }
}
"#,
    );
    repository.write(
        "package-lock.json",
        r#"{
  "name": "codex-relay",
  "version": "0.4.0",
  "lockfileVersion": 3,
  "packages": {
    "": {
      "name": "codex-relay",
      "version": "0.4.0",
      "unknown": "keep"
    },
    "tools/release-console": {
      "name": "@codex-relay/release-console",
      "version": "0.1.0"
    }
  },
  "unknownTopLevel": 7
}
"#,
    );
    repository.write(
        "src-tauri/Cargo.toml",
        r#"# 主程序注释必须保留
[package]
name = "codex-relay"
version = "0.4.0"
edition = "2024"

[package.metadata.release-console]
keep = true
"#,
    );
    repository.write(
        "src-tauri/crates/codex-relay-core/Cargo.toml",
        r#"# 核心包注释必须保留
[package]
name = "codex-relay-core"
version = "0.4.0"
edition = "2024"

[dependencies]
serde = "1"
"#,
    );
    repository.write(
        "src-tauri/Cargo.lock",
        r#"version = 4

[[package]]
name = "codex-relay"
version = "0.4.0"
dependencies = ["codex-relay-core"]

[[package]]
name = "codex-relay-core"
version = "0.4.0"

[[package]]
name = "unrelated"
version = "9.9.9"
"#,
    );
    repository.write(".github/release-notes.md", "旧发布说明\n");
    repository
}

#[test]
fn plan_updates_six_release_files_without_writing_and_preserves_unknown_content() {
    let repository = create_repository_fixture();
    let originals = [
        "package.json",
        "package-lock.json",
        "src-tauri/Cargo.toml",
        "src-tauri/crates/codex-relay-core/Cargo.toml",
        "src-tauri/Cargo.lock",
        ".github/release-notes.md",
    ]
    .map(|path| (path, repository.read(path)));
    let notes = VALID_RELEASE_NOTES;

    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();

    assert_eq!(plan.previous_version, "0.4.0");
    assert_eq!(plan.target_version, "0.5.0");
    assert_eq!(
        plan.files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>(),
        originals.iter().map(|(path, _)| *path).collect::<Vec<_>>()
    );

    let package: serde_json::Value =
        serde_json::from_slice(&plan.file("package.json").unwrap().after).unwrap();
    assert_eq!(package["version"], "0.5.0");
    assert_eq!(package["releaseConsoleUnknown"]["keep"], true);

    let package_lock: serde_json::Value =
        serde_json::from_slice(&plan.file("package-lock.json").unwrap().after).unwrap();
    assert_eq!(package_lock["version"], "0.5.0");
    assert_eq!(package_lock["packages"][""]["version"], "0.5.0");
    assert_eq!(
        package_lock["packages"]["tools/release-console"]["version"],
        "0.1.0"
    );
    assert_eq!(package_lock["unknownTopLevel"], 7);

    let main_manifest =
        String::from_utf8(plan.file("src-tauri/Cargo.toml").unwrap().after.clone()).unwrap();
    assert!(main_manifest.contains("# 主程序注释必须保留"));
    assert!(main_manifest.contains("version = \"0.5.0\""));
    assert!(main_manifest.contains("[package.metadata.release-console]"));

    let core_manifest = String::from_utf8(
        plan.file("src-tauri/crates/codex-relay-core/Cargo.toml")
            .unwrap()
            .after
            .clone(),
    )
    .unwrap();
    assert!(core_manifest.contains("# 核心包注释必须保留"));
    assert!(core_manifest.contains("version = \"0.5.0\""));
    assert!(core_manifest.contains("serde = \"1\""));

    let cargo_lock =
        String::from_utf8(plan.file("src-tauri/Cargo.lock").unwrap().after.clone()).unwrap();
    assert_eq!(cargo_lock.matches("version = \"0.5.0\"").count(), 2);
    assert!(cargo_lock.contains("name = \"unrelated\"\nversion = \"9.9.9\""));
    assert_eq!(
        plan.file(".github/release-notes.md").unwrap().after,
        notes.as_bytes()
    );

    for (path, original) in originals {
        assert_eq!(repository.read(path), original);
    }
}

#[test]
fn apply_rejects_fingerprint_conflict_before_writing_or_creating_marker() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();
    let untouched = plan
        .files
        .iter()
        .skip(1)
        .map(|file| (file.relative_path.clone(), file.before.clone()))
        .collect::<Vec<_>>();
    repository.write(
        "package.json",
        "{\n  \"name\": \"codex-relay\",\n  \"version\": \"0.4.0\",\n  \"external\": true\n}\n",
    );

    let error = ReleaseCandidateTransaction::apply(
        repository.path(),
        &repository.path().join(".git"),
        &plan,
    )
    .unwrap_err();

    assert_eq!(error.code(), "RELEASE_SOURCE_CONFLICT");
    assert!(
        String::from_utf8(repository.read("package.json"))
            .unwrap()
            .contains("\"external\": true")
    );
    for (path, before) in untouched {
        assert_eq!(repository.read(&path), before);
    }
    assert!(
        !repository
            .path()
            .join(".git/codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

#[test]
fn apply_writes_exact_candidate_and_persists_recovery_marker_and_backups() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();

    ReleaseCandidateTransaction::apply(repository.path(), &repository.path().join(".git"), &plan)
        .unwrap();

    for file in &plan.files {
        assert_eq!(repository.read(&file.relative_path), file.after);
    }

    let state_root = repository.path().join(".git/codex-relay-release-console");
    let marker: serde_json::Value =
        serde_json::from_slice(&fs::read(state_root.join("candidate-transaction.json")).unwrap())
            .unwrap();
    assert_eq!(marker["schemaVersion"], 1);
    assert_eq!(marker["previousVersion"], "0.4.0");
    assert_eq!(marker["targetVersion"], "0.5.0");
    assert_eq!(marker["files"].as_array().unwrap().len(), 6);

    for (index, file) in plan.files.iter().enumerate() {
        assert_eq!(
            marker["files"][index]["relativePath"],
            file.relative_path.as_str()
        );
        assert_eq!(
            fs::read(state_root.join(format!("candidate-backup/{index}.bin"))).unwrap(),
            file.before
        );
    }
}

struct FailPackageLockWrite {
    inner: StdCandidateFileOps,
    failed: Mutex<bool>,
}

impl CandidateFileOps for FailPackageLockWrite {
    fn write(&self, path: &Path, bytes: &[u8], phase: CandidateWritePhase) -> Result<(), AppError> {
        if phase == CandidateWritePhase::Forward
            && path.ends_with("package-lock.json")
            && !*self.failed.lock().unwrap()
        {
            *self.failed.lock().unwrap() = true;
            return Err(AppError::new(
                "INJECTED_RELEASE_WRITE_FAILURE",
                "注入发布候选写入失败。",
                "package-lock forward write failed",
            ));
        }
        self.inner.write(path, bytes, phase)
    }
}

#[test]
fn forward_write_failure_restores_exact_original_bytes_and_removes_marker() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();
    let file_ops = FailPackageLockWrite {
        inner: StdCandidateFileOps,
        failed: Mutex::new(false),
    };

    let error = ReleaseCandidateTransaction::apply_with_file_ops(
        repository.path(),
        &repository.path().join(".git"),
        &plan,
        &file_ops,
    )
    .unwrap_err();

    assert_eq!(error.code(), "RELEASE_TRANSACTION_FAILED_ROLLED_BACK");
    assert!(error.to_string().contains("原文件已恢复"));
    for file in &plan.files {
        assert_eq!(repository.read(&file.relative_path), file.before);
    }
    assert!(
        !repository
            .path()
            .join(".git/codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

struct CorruptPackageLockAfterWrite {
    inner: StdCandidateFileOps,
}

impl CandidateFileOps for CorruptPackageLockAfterWrite {
    fn write(&self, path: &Path, bytes: &[u8], phase: CandidateWritePhase) -> Result<(), AppError> {
        self.inner.write(path, bytes, phase)?;
        if phase == CandidateWritePhase::Forward && path.ends_with("package-lock.json") {
            fs::write(path, [0xff, 0xfe]).map_err(AppError::from)?;
        }
        Ok(())
    }
}

#[test]
fn post_write_verification_failure_restores_all_original_bytes() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();

    let error = ReleaseCandidateTransaction::apply_with_file_ops(
        repository.path(),
        &repository.path().join(".git"),
        &plan,
        &CorruptPackageLockAfterWrite {
            inner: StdCandidateFileOps,
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), "RELEASE_TRANSACTION_FAILED_ROLLED_BACK");
    for file in &plan.files {
        assert_eq!(repository.read(&file.relative_path), file.before);
    }
}

struct FailForwardAndRollback {
    inner: StdCandidateFileOps,
}

impl CandidateFileOps for FailForwardAndRollback {
    fn write(&self, path: &Path, bytes: &[u8], phase: CandidateWritePhase) -> Result<(), AppError> {
        if (phase == CandidateWritePhase::Forward && path.ends_with("package-lock.json"))
            || (phase == CandidateWritePhase::Rollback && path.ends_with("package.json"))
        {
            return Err(AppError::new(
                "INJECTED_RELEASE_ROLLBACK_FAILURE",
                "注入发布候选回滚失败。",
                "injected forward or rollback failure",
            ));
        }
        self.inner.write(path, bytes, phase)
    }
}

#[test]
fn rollback_failure_is_reported_truthfully_and_keeps_recovery_marker() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();

    let error = ReleaseCandidateTransaction::apply_with_file_ops(
        repository.path(),
        &repository.path().join(".git"),
        &plan,
        &FailForwardAndRollback {
            inner: StdCandidateFileOps,
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), "RELEASE_ROLLBACK_INCOMPLETE");
    assert!(!error.to_string().contains("原文件已恢复"));
    assert_eq!(repository.read("package.json"), plan.files[0].after);
    assert!(
        repository
            .path()
            .join(".git/codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

#[test]
fn active_marker_can_restore_candidate_after_process_restart() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();
    let git_dir = repository.path().join(".git");
    ReleaseCandidateTransaction::apply(repository.path(), &git_dir, &plan).unwrap();

    ReleaseCandidateTransaction::rollback_active(repository.path(), &git_dir).unwrap();

    for file in &plan.files {
        assert_eq!(repository.read(&file.relative_path), file.before);
    }
    assert!(!git_dir.join("codex-relay-release-console").exists());
}

#[test]
fn recovery_refuses_external_source_changes_without_overwriting_any_file() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();
    let git_dir = repository.path().join(".git");
    ReleaseCandidateTransaction::apply(repository.path(), &git_dir, &plan).unwrap();
    repository.write(
        "package.json",
        "{\n  \"name\": \"codex-relay\",\n  \"version\": \"0.5.0\",\n  \"externalAfterApply\": true\n}\n",
    );
    let current = plan
        .files
        .iter()
        .map(|file| {
            (
                file.relative_path.clone(),
                repository.read(&file.relative_path),
            )
        })
        .collect::<Vec<_>>();

    let error =
        ReleaseCandidateTransaction::rollback_active(repository.path(), &git_dir).unwrap_err();

    assert_eq!(error.code(), "RELEASE_RECOVERY_SOURCE_CONFLICT");
    for (path, bytes) in current {
        assert_eq!(repository.read(&path), bytes);
    }
    assert!(
        git_dir
            .join("codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

#[test]
fn recovery_rejects_corrupted_backup_before_overwriting_source_files() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();
    let git_dir = repository.path().join(".git");
    ReleaseCandidateTransaction::apply(repository.path(), &git_dir, &plan).unwrap();
    fs::write(
        git_dir.join("codex-relay-release-console/candidate-backup/0.bin"),
        b"corrupted backup",
    )
    .unwrap();
    let current = plan
        .files
        .iter()
        .map(|file| {
            (
                file.relative_path.clone(),
                repository.read(&file.relative_path),
            )
        })
        .collect::<Vec<_>>();

    let error =
        ReleaseCandidateTransaction::rollback_active(repository.path(), &git_dir).unwrap_err();

    assert_eq!(error.code(), "RELEASE_STATE_INVALID");
    for (path, bytes) in current {
        assert_eq!(repository.read(&path), bytes);
    }
    assert!(
        git_dir
            .join("codex-relay-release-console/candidate-transaction.json")
            .exists()
    );
}

struct MutateSourceAfterMarker {
    inner: StdCandidateFileOps,
    package_json: PathBuf,
}

impl CandidateFileOps for MutateSourceAfterMarker {
    fn write(&self, path: &Path, bytes: &[u8], phase: CandidateWritePhase) -> Result<(), AppError> {
        self.inner.write(path, bytes, phase)?;
        if phase == CandidateWritePhase::State && path.ends_with("candidate-transaction.json") {
            fs::write(
                &self.package_json,
                b"{\n  \"name\": \"codex-relay\",\n  \"version\": \"0.4.0\",\n  \"externalDuringTransaction\": true\n}\n",
            )
            .map_err(AppError::from)?;
        }
        Ok(())
    }
}

#[test]
fn source_is_rechecked_after_marker_and_before_first_candidate_write() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();
    let git_dir = repository.path().join(".git");

    let error = ReleaseCandidateTransaction::apply_with_file_ops(
        repository.path(),
        &git_dir,
        &plan,
        &MutateSourceAfterMarker {
            inner: StdCandidateFileOps,
            package_json: repository.path().join("package.json"),
        },
    )
    .unwrap_err();

    assert_eq!(error.code(), "RELEASE_SOURCE_CONFLICT");
    assert!(
        String::from_utf8(repository.read("package.json"))
            .unwrap()
            .contains("\"externalDuringTransaction\": true")
    );
    for file in plan.files.iter().skip(1) {
        assert_eq!(repository.read(&file.relative_path), file.before);
    }
    assert!(!git_dir.join("codex-relay-release-console").exists());
}

#[test]
fn second_apply_is_rejected_as_an_active_transaction_without_changing_candidate() {
    let repository = create_repository_fixture();
    let notes = VALID_RELEASE_NOTES;
    let plan = ReleaseCandidateTransaction::plan(repository.path(), "0.5.0", notes).unwrap();
    let git_dir = repository.path().join(".git");
    ReleaseCandidateTransaction::apply(repository.path(), &git_dir, &plan).unwrap();
    let candidate = plan
        .files
        .iter()
        .map(|file| {
            (
                file.relative_path.clone(),
                repository.read(&file.relative_path),
            )
        })
        .collect::<Vec<_>>();

    let error = ReleaseCandidateTransaction::apply(repository.path(), &git_dir, &plan).unwrap_err();

    assert_eq!(error.code(), "RELEASE_TRANSACTION_ALREADY_ACTIVE");
    for (path, bytes) in candidate {
        assert_eq!(repository.read(&path), bytes);
    }
}
