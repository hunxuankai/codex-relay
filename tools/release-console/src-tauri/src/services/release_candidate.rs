use super::release_notes::{ReleaseNotesError, ReleaseNotesService};
use codex_relay_core::error::AppError;
use codex_relay_core::infrastructure::atomic_file::atomic_write;
use codex_relay_core::infrastructure::file_fingerprint::FileFingerprint;
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use toml_edit::{DocumentMut, Item, value};

const PACKAGE_JSON: &str = "package.json";
const PACKAGE_LOCK_JSON: &str = "package-lock.json";
const MAIN_CARGO_TOML: &str = "src-tauri/Cargo.toml";
const CORE_CARGO_TOML: &str = "src-tauri/crates/codex-relay-core/Cargo.toml";
const CARGO_LOCK: &str = "src-tauri/Cargo.lock";
const RELEASE_NOTES: &str = ".github/release-notes.md";
const STATE_DIRECTORY: &str = "codex-relay-release-console";
const TRANSACTION_MARKER: &str = "candidate-transaction.json";
const BACKUP_DIRECTORY: &str = "candidate-backup";

const RELEASE_FILE_PATHS: [&str; 6] = [
    PACKAGE_JSON,
    PACKAGE_LOCK_JSON,
    MAIN_CARGO_TOML,
    CORE_CARGO_TOML,
    CARGO_LOCK,
    RELEASE_NOTES,
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseFilePlan {
    pub relative_path: String,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    pub expected_fingerprint: FileFingerprint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseCandidatePlan {
    pub previous_version: String,
    pub target_version: String,
    pub files: Vec<ReleaseFilePlan>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateWritePhase {
    State,
    Forward,
    Rollback,
}

pub trait CandidateFileOps: Send + Sync {
    fn write(&self, path: &Path, bytes: &[u8], phase: CandidateWritePhase) -> Result<(), AppError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StdCandidateFileOps;

impl CandidateFileOps for StdCandidateFileOps {
    fn write(
        &self,
        path: &Path,
        bytes: &[u8],
        _phase: CandidateWritePhase,
    ) -> Result<(), AppError> {
        atomic_write(path, bytes, |candidate| {
            validate_exact_bytes(candidate, bytes)
        })
    }
}

impl ReleaseCandidatePlan {
    pub fn file(&self, relative_path: &str) -> Option<&ReleaseFilePlan> {
        self.files
            .iter()
            .find(|file| file.relative_path == relative_path)
    }

    pub fn has_changes(&self) -> bool {
        self.files.iter().any(|file| file.before != file.after)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReleaseCandidateError {
    #[error("无法读取发布候选文件")]
    FileRead,
    #[error("发布候选 JSON 无效")]
    InvalidJson,
    #[error("发布候选 TOML 无效")]
    InvalidToml,
    #[error("发布候选文件缺少必需字段")]
    MissingRequiredField,
    #[error("发布候选文件中的包身份不匹配")]
    PackageIdentityMismatch,
    #[error("Cargo.lock 中的本地包数量不正确")]
    CargoLockPackageMismatch,
    #[error("仓库版本高于目标版本")]
    RepositoryVersionAhead,
    #[error("发布候选文件已被外部修改")]
    SourceConflict,
    #[error("已有未完成的发布候选事务")]
    ActiveTransactionExists,
    #[error("无法写入发布候选恢复状态")]
    StateWriteFailed,
    #[error("无法读取发布候选恢复状态")]
    StateReadFailed,
    #[error("发布候选恢复状态无效")]
    StateInvalid,
    #[error("发布候选文件在恢复前已被外部修改")]
    RecoverySourceConflict,
    #[error("发布候选写入失败，原文件已恢复")]
    TransactionFailedRolledBack,
    #[error("发布候选写入失败，且未能完整恢复原文件")]
    RollbackIncomplete,
    #[error(transparent)]
    ReleaseNotes(#[from] ReleaseNotesError),
}

impl ReleaseCandidateError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::FileRead => "RELEASE_FILE_READ_FAILED",
            Self::InvalidJson => "RELEASE_FILE_JSON_INVALID",
            Self::InvalidToml => "RELEASE_FILE_TOML_INVALID",
            Self::MissingRequiredField => "RELEASE_FILE_REQUIRED_FIELD_MISSING",
            Self::PackageIdentityMismatch => "RELEASE_PACKAGE_IDENTITY_MISMATCH",
            Self::CargoLockPackageMismatch => "RELEASE_CARGO_LOCK_PACKAGE_MISMATCH",
            Self::RepositoryVersionAhead => "RELEASE_REPOSITORY_VERSION_AHEAD",
            Self::SourceConflict => "RELEASE_SOURCE_CONFLICT",
            Self::ActiveTransactionExists => "RELEASE_TRANSACTION_ALREADY_ACTIVE",
            Self::StateWriteFailed => "RELEASE_STATE_WRITE_FAILED",
            Self::StateReadFailed => "RELEASE_STATE_READ_FAILED",
            Self::StateInvalid => "RELEASE_STATE_INVALID",
            Self::RecoverySourceConflict => "RELEASE_RECOVERY_SOURCE_CONFLICT",
            Self::TransactionFailedRolledBack => "RELEASE_TRANSACTION_FAILED_ROLLED_BACK",
            Self::RollbackIncomplete => "RELEASE_ROLLBACK_INCOMPLETE",
            Self::ReleaseNotes(error) => error.code(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateTransactionMarker {
    schema_version: u32,
    previous_version: String,
    target_version: String,
    files: Vec<CandidateTransactionFile>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateTransactionFile {
    relative_path: String,
    backup_file: String,
    before_sha256: String,
    after_sha256: String,
}

pub struct ReleaseCandidateTransaction;

impl ReleaseCandidateTransaction {
    pub fn plan(
        repository_root: &Path,
        target_version: &str,
        release_notes: &str,
    ) -> Result<ReleaseCandidatePlan, ReleaseCandidateError> {
        let originals = RELEASE_FILE_PATHS
            .iter()
            .map(|relative_path| read_release_file(repository_root, relative_path))
            .collect::<Result<Vec<_>, _>>()?;

        let previous_version = package_version(&originals[0].1)?;
        build_plan(originals, &previous_version, target_version, release_notes)
    }

    pub fn plan_for_published_version(
        repository_root: &Path,
        published_version: &str,
        target_version: &str,
        release_notes: &str,
    ) -> Result<ReleaseCandidatePlan, ReleaseCandidateError> {
        let originals = RELEASE_FILE_PATHS
            .iter()
            .map(|relative_path| read_release_file(repository_root, relative_path))
            .collect::<Result<Vec<_>, _>>()?;
        build_plan(originals, published_version, target_version, release_notes)
    }

    pub fn apply(
        repository_root: &Path,
        git_dir: &Path,
        plan: &ReleaseCandidatePlan,
    ) -> Result<(), ReleaseCandidateError> {
        Self::apply_with_file_ops(repository_root, git_dir, plan, &StdCandidateFileOps)
    }

    pub fn apply_with_file_ops(
        repository_root: &Path,
        git_dir: &Path,
        plan: &ReleaseCandidatePlan,
        file_ops: &dyn CandidateFileOps,
    ) -> Result<(), ReleaseCandidateError> {
        ensure_no_active_transaction(git_dir)?;
        ensure_source_unchanged(repository_root, plan)?;

        prepare_recovery_state(git_dir, plan, file_ops)?;
        if let Err(error) = ensure_source_unchanged(repository_root, plan) {
            cleanup_recovery_state(git_dir).map_err(|_| ReleaseCandidateError::StateWriteFailed)?;
            return Err(error);
        }
        let mut written_indices = Vec::new();
        for (index, file) in plan.files.iter().enumerate() {
            let target = repository_root.join(&file.relative_path);
            if file_ops
                .write(&target, &file.after, CandidateWritePhase::Forward)
                .is_err()
            {
                return rollback_after_failure(
                    repository_root,
                    git_dir,
                    plan,
                    &written_indices,
                    file_ops,
                );
            }
            written_indices.push(index);
        }
        if written_indices.iter().any(|index| {
            let file = &plan.files[*index];
            fs::read(repository_root.join(&file.relative_path))
                .map(|bytes| bytes != file.after)
                .unwrap_or(true)
        }) {
            return rollback_after_failure(
                repository_root,
                git_dir,
                plan,
                &written_indices,
                file_ops,
            );
        }

        Ok(())
    }

    pub fn rollback_active(
        repository_root: &Path,
        git_dir: &Path,
    ) -> Result<(), ReleaseCandidateError> {
        let state_root = git_dir.join(STATE_DIRECTORY);
        let marker_bytes = fs::read(state_root.join(TRANSACTION_MARKER))
            .map_err(|_| ReleaseCandidateError::StateReadFailed)?;
        let marker: CandidateTransactionMarker = serde_json::from_slice(&marker_bytes)
            .map_err(|_| ReleaseCandidateError::StateInvalid)?;
        validate_marker(&marker)?;
        if marker.files.iter().any(|file| {
            fs::read(repository_root.join(&file.relative_path))
                .map(|bytes| sha256_hex(&bytes) != file.after_sha256)
                .unwrap_or(true)
        }) {
            return Err(ReleaseCandidateError::RecoverySourceConflict);
        }

        let backups = marker
            .files
            .iter()
            .map(|file| {
                fs::read(state_root.join(&file.backup_file))
                    .map_err(|_| ReleaseCandidateError::StateReadFailed)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if marker
            .files
            .iter()
            .zip(&backups)
            .any(|(file, before)| sha256_hex(before) != file.before_sha256)
        {
            return Err(ReleaseCandidateError::StateInvalid);
        }
        for (file, before) in marker.files.iter().zip(&backups) {
            StdCandidateFileOps
                .write(
                    &repository_root.join(&file.relative_path),
                    before,
                    CandidateWritePhase::Rollback,
                )
                .map_err(|_| ReleaseCandidateError::RollbackIncomplete)?;
        }
        for (file, before) in marker.files.iter().zip(&backups) {
            if fs::read(repository_root.join(&file.relative_path))
                .ok()
                .as_deref()
                != Some(before.as_slice())
            {
                return Err(ReleaseCandidateError::RollbackIncomplete);
            }
        }
        cleanup_recovery_state(git_dir).map_err(|_| ReleaseCandidateError::RollbackIncomplete)
    }

    pub fn finalize_active(
        repository_root: &Path,
        git_dir: &Path,
    ) -> Result<(), ReleaseCandidateError> {
        let state_root = git_dir.join(STATE_DIRECTORY);
        let marker_bytes = fs::read(state_root.join(TRANSACTION_MARKER))
            .map_err(|_| ReleaseCandidateError::StateReadFailed)?;
        let marker: CandidateTransactionMarker = serde_json::from_slice(&marker_bytes)
            .map_err(|_| ReleaseCandidateError::StateInvalid)?;
        validate_marker(&marker)?;
        if marker.files.iter().any(|file| {
            fs::read(repository_root.join(&file.relative_path))
                .map(|bytes| sha256_hex(&bytes) != file.after_sha256)
                .unwrap_or(true)
        }) {
            return Err(ReleaseCandidateError::RecoverySourceConflict);
        }
        cleanup_recovery_state(git_dir).map_err(|_| ReleaseCandidateError::StateWriteFailed)
    }
}

fn build_plan(
    originals: Vec<(&str, Vec<u8>, FileFingerprint)>,
    previous_version: &str,
    target_version: &str,
    release_notes: &str,
) -> Result<ReleaseCandidatePlan, ReleaseCandidateError> {
    let previous =
        Version::parse(previous_version).map_err(|_| ReleaseNotesError::InvalidVersion)?;
    let target = Version::parse(target_version).map_err(|_| ReleaseNotesError::InvalidVersion)?;
    if target <= previous {
        return Err(ReleaseNotesError::TargetVersionNotHigher.into());
    }
    let repository_version = package_version(&originals[0].1)?;
    let repository_version =
        Version::parse(&repository_version).map_err(|_| ReleaseNotesError::InvalidVersion)?;
    if repository_version > target {
        return Err(ReleaseCandidateError::RepositoryVersionAhead);
    }
    ReleaseNotesService::validate(previous_version, target_version, release_notes)?;
    let after = [
        update_package_json(&originals[0].1, target_version)?,
        update_package_lock(&originals[1].1, target_version)?,
        update_manifest(&originals[2].1, "codex-relay", target_version)?,
        update_manifest(&originals[3].1, "codex-relay-core", target_version)?,
        update_cargo_lock(&originals[4].1, target_version)?,
        release_notes.as_bytes().to_vec(),
    ];
    let files = originals
        .into_iter()
        .zip(after)
        .map(
            |((relative_path, before, expected_fingerprint), after)| ReleaseFilePlan {
                relative_path: relative_path.to_string(),
                before,
                after,
                expected_fingerprint,
            },
        )
        .collect();

    Ok(ReleaseCandidatePlan {
        previous_version: previous_version.to_string(),
        target_version: target_version.to_string(),
        files,
    })
}

fn ensure_source_unchanged(
    repository_root: &Path,
    plan: &ReleaseCandidatePlan,
) -> Result<(), ReleaseCandidateError> {
    for file in &plan.files {
        let current = FileFingerprint::from_path(&repository_root.join(&file.relative_path))
            .map_err(|_| ReleaseCandidateError::FileRead)?;
        if current != file.expected_fingerprint {
            return Err(ReleaseCandidateError::SourceConflict);
        }
    }
    Ok(())
}

fn ensure_no_active_transaction(git_dir: &Path) -> Result<(), ReleaseCandidateError> {
    let state_root = git_dir.join(STATE_DIRECTORY);
    if state_root.join(TRANSACTION_MARKER).exists() || state_root.join(BACKUP_DIRECTORY).exists() {
        Err(ReleaseCandidateError::ActiveTransactionExists)
    } else {
        Ok(())
    }
}

fn validate_marker(marker: &CandidateTransactionMarker) -> Result<(), ReleaseCandidateError> {
    if marker.schema_version != 1 || marker.files.len() != RELEASE_FILE_PATHS.len() {
        return Err(ReleaseCandidateError::StateInvalid);
    }
    for (index, (file, expected_path)) in marker.files.iter().zip(RELEASE_FILE_PATHS).enumerate() {
        if file.relative_path != expected_path
            || file.backup_file != format!("{BACKUP_DIRECTORY}/{index}.bin")
        {
            return Err(ReleaseCandidateError::StateInvalid);
        }
    }
    Ok(())
}

fn prepare_recovery_state(
    git_dir: &Path,
    plan: &ReleaseCandidatePlan,
    file_ops: &dyn CandidateFileOps,
) -> Result<(), ReleaseCandidateError> {
    let state_root = git_dir.join(STATE_DIRECTORY);
    let marker_path = state_root.join(TRANSACTION_MARKER);
    let backup_root = state_root.join(BACKUP_DIRECTORY);
    ensure_no_active_transaction(git_dir)?;

    fs::create_dir_all(&backup_root).map_err(|_| ReleaseCandidateError::StateWriteFailed)?;
    let files = plan
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let backup_file = format!("{BACKUP_DIRECTORY}/{index}.bin");
            file_ops
                .write(
                    &state_root.join(&backup_file),
                    &file.before,
                    CandidateWritePhase::State,
                )
                .map_err(|_| ReleaseCandidateError::StateWriteFailed)?;
            Ok(CandidateTransactionFile {
                relative_path: file.relative_path.clone(),
                backup_file,
                before_sha256: sha256_hex(&file.before),
                after_sha256: sha256_hex(&file.after),
            })
        })
        .collect::<Result<Vec<_>, ReleaseCandidateError>>()?;
    let marker = CandidateTransactionMarker {
        schema_version: 1,
        previous_version: plan.previous_version.clone(),
        target_version: plan.target_version.clone(),
        files,
    };
    let mut marker_bytes =
        serde_json::to_vec_pretty(&marker).map_err(|_| ReleaseCandidateError::StateWriteFailed)?;
    marker_bytes.push(b'\n');
    file_ops
        .write(&marker_path, &marker_bytes, CandidateWritePhase::State)
        .map_err(|_| ReleaseCandidateError::StateWriteFailed)
}

fn rollback_after_failure(
    repository_root: &Path,
    git_dir: &Path,
    plan: &ReleaseCandidatePlan,
    written_indices: &[usize],
    file_ops: &dyn CandidateFileOps,
) -> Result<(), ReleaseCandidateError> {
    for index in written_indices.iter().rev() {
        let file = &plan.files[*index];
        if file_ops
            .write(
                &repository_root.join(&file.relative_path),
                &file.before,
                CandidateWritePhase::Rollback,
            )
            .is_err()
        {
            return Err(ReleaseCandidateError::RollbackIncomplete);
        }
    }
    for index in written_indices {
        let file = &plan.files[*index];
        if fs::read(repository_root.join(&file.relative_path))
            .ok()
            .as_deref()
            != Some(file.before.as_slice())
        {
            return Err(ReleaseCandidateError::RollbackIncomplete);
        }
    }
    cleanup_recovery_state(git_dir).map_err(|_| ReleaseCandidateError::RollbackIncomplete)?;
    Err(ReleaseCandidateError::TransactionFailedRolledBack)
}

fn cleanup_recovery_state(git_dir: &Path) -> Result<(), std::io::Error> {
    let state_root = git_dir.join(STATE_DIRECTORY);
    let marker_path = state_root.join(TRANSACTION_MARKER);
    let backup_root = state_root.join(BACKUP_DIRECTORY);
    if marker_path.exists() {
        fs::remove_file(marker_path)?;
    }
    if backup_root.exists() {
        fs::remove_dir_all(backup_root)?;
    }
    if state_root.exists() {
        match fs::remove_dir(state_root) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::DirectoryNotEmpty => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn validate_exact_bytes(candidate: &[u8], expected: &[u8]) -> Result<(), AppError> {
    if candidate == expected {
        Ok(())
    } else {
        Err(AppError::new(
            "RELEASE_WRITE_VERIFICATION_FAILED",
            "发布候选写入验证失败。",
            "candidate bytes differ from expected bytes",
        ))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_release_file<'a>(
    repository_root: &Path,
    relative_path: &'a str,
) -> Result<(&'a str, Vec<u8>, FileFingerprint), ReleaseCandidateError> {
    let path = repository_root.join(relative_path);
    let before = fs::read(&path).map_err(|_| ReleaseCandidateError::FileRead)?;
    let expected_fingerprint =
        FileFingerprint::from_path(&path).map_err(|_| ReleaseCandidateError::FileRead)?;
    Ok((relative_path, before, expected_fingerprint))
}

fn package_version(bytes: &[u8]) -> Result<String, ReleaseCandidateError> {
    let document: JsonValue =
        serde_json::from_slice(bytes).map_err(|_| ReleaseCandidateError::InvalidJson)?;
    document
        .get("version")
        .and_then(JsonValue::as_str)
        .map(str::to_string)
        .ok_or(ReleaseCandidateError::MissingRequiredField)
}

fn update_package_json(
    bytes: &[u8],
    target_version: &str,
) -> Result<Vec<u8>, ReleaseCandidateError> {
    let mut document: JsonValue =
        serde_json::from_slice(bytes).map_err(|_| ReleaseCandidateError::InvalidJson)?;
    let root = document
        .as_object_mut()
        .ok_or(ReleaseCandidateError::InvalidJson)?;
    root.insert(
        "version".to_string(),
        JsonValue::String(target_version.to_string()),
    );
    render_json(&document)
}

fn update_package_lock(
    bytes: &[u8],
    target_version: &str,
) -> Result<Vec<u8>, ReleaseCandidateError> {
    let mut document: JsonValue =
        serde_json::from_slice(bytes).map_err(|_| ReleaseCandidateError::InvalidJson)?;
    let root = document
        .as_object_mut()
        .ok_or(ReleaseCandidateError::InvalidJson)?;
    root.insert(
        "version".to_string(),
        JsonValue::String(target_version.to_string()),
    );
    let package_root = root
        .get_mut("packages")
        .and_then(JsonValue::as_object_mut)
        .and_then(|packages| packages.get_mut(""))
        .and_then(JsonValue::as_object_mut)
        .ok_or(ReleaseCandidateError::MissingRequiredField)?;
    package_root.insert(
        "version".to_string(),
        JsonValue::String(target_version.to_string()),
    );
    render_json(&document)
}

fn render_json(document: &JsonValue) -> Result<Vec<u8>, ReleaseCandidateError> {
    let mut rendered =
        serde_json::to_string_pretty(document).map_err(|_| ReleaseCandidateError::InvalidJson)?;
    rendered.push('\n');
    Ok(rendered.into_bytes())
}

fn update_manifest(
    bytes: &[u8],
    expected_name: &str,
    target_version: &str,
) -> Result<Vec<u8>, ReleaseCandidateError> {
    let source = std::str::from_utf8(bytes).map_err(|_| ReleaseCandidateError::InvalidToml)?;
    let mut document = source
        .parse::<DocumentMut>()
        .map_err(|_| ReleaseCandidateError::InvalidToml)?;
    let package = document
        .get_mut("package")
        .and_then(Item::as_table_like_mut)
        .ok_or(ReleaseCandidateError::MissingRequiredField)?;
    if package.get("name").and_then(Item::as_str) != Some(expected_name) {
        return Err(ReleaseCandidateError::PackageIdentityMismatch);
    }
    package.insert("version", value(target_version));
    Ok(document.to_string().into_bytes())
}

fn update_cargo_lock(bytes: &[u8], target_version: &str) -> Result<Vec<u8>, ReleaseCandidateError> {
    let source = std::str::from_utf8(bytes).map_err(|_| ReleaseCandidateError::InvalidToml)?;
    let mut document = source
        .parse::<DocumentMut>()
        .map_err(|_| ReleaseCandidateError::InvalidToml)?;
    let packages = document
        .get_mut("package")
        .and_then(Item::as_array_of_tables_mut)
        .ok_or(ReleaseCandidateError::MissingRequiredField)?;

    for expected_name in ["codex-relay", "codex-relay-core"] {
        let matching = packages
            .iter_mut()
            .filter(|package| package.get("name").and_then(Item::as_str) == Some(expected_name))
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err(ReleaseCandidateError::CargoLockPackageMismatch);
        }
        matching.into_iter().next().expect("one package")["version"] = value(target_version);
    }

    Ok(document.to_string().into_bytes())
}
