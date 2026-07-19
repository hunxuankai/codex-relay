use codex_relay_lib::error::AppError;
use codex_relay_lib::infrastructure::path_service::{PathMode, resolve_paths};
use codex_relay_lib::models::provider::CreateProviderInput;
use codex_relay_lib::services::provider_service::ProviderService;
use codex_relay_lib::services::transaction_service::{
    FileOps, ManagedFileKind, StdFileOps, WritePhase,
};
use serial_test::serial;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct EnvGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn set(entries: &[(&'static str, Option<&Path>)]) -> Self {
        let saved = entries
            .iter()
            .map(|(name, _)| (*name, std::env::var_os(name)))
            .collect();
        for (name, value) in entries {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, value) in self.saved.drain(..) {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
}

#[derive(Default)]
struct AuditingFileOps {
    inner: StdFileOps,
    paths: Mutex<Vec<PathBuf>>,
}

impl AuditingFileOps {
    fn record(&self, path: &Path) {
        self.paths.lock().unwrap().push(path.to_path_buf());
    }

    fn observed(&self) -> Vec<PathBuf> {
        self.paths.lock().unwrap().clone()
    }
}

impl FileOps for AuditingFileOps {
    fn read_optional(&self, path: &Path) -> Result<Option<Vec<u8>>, AppError> {
        self.record(path);
        self.inner.read_optional(path)
    }

    fn write(
        &self,
        path: &Path,
        bytes: &[u8],
        kind: ManagedFileKind,
        phase: WritePhase,
    ) -> Result<(), AppError> {
        self.record(path);
        self.inner.write(path, bytes, kind, phase)
    }

    fn remove_if_exists(
        &self,
        path: &Path,
        kind: ManagedFileKind,
        phase: WritePhase,
    ) -> Result<(), AppError> {
        self.record(path);
        self.inner.remove_if_exists(path, kind, phase)
    }
}

fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, directory: &Path, snapshot: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if entry.file_type().unwrap().is_dir() {
                snapshot.insert(relative, None);
                visit(root, &path, snapshot);
            } else {
                snapshot.insert(relative, Some(fs::read(&path).unwrap()));
            }
        }
    }

    let mut snapshot = BTreeMap::new();
    visit(root, root, &mut snapshot);
    snapshot
}

#[test]
#[serial]
fn test_mode_refuses_to_fall_back_to_real_user_directories() {
    let _guard = EnvGuard::set(&[
        ("CODEX_RELAY_CODEX_HOME", None),
        ("CODEX_RELAY_APP_DATA_DIR", None),
        ("CODEX_HOME", None),
    ]);

    let error = resolve_paths(PathMode::Test).unwrap_err();

    assert_eq!(error.code(), "TEST_PATH_OVERRIDE_REQUIRED");
}

#[tokio::test]
#[serial]
async fn audited_provider_and_backup_workflow_ignores_default_path_sentinels() {
    let directory = tempfile::tempdir().unwrap();
    let codex = directory.path().join("codex");
    let app_data = directory.path().join("app-data");
    let sentinel_profile = directory.path().join("sentinel-profile");
    let sentinel_local = directory.path().join("sentinel-local");
    let real_codex_sentinel = sentinel_profile.join(".codex");
    let real_app_data_sentinel = sentinel_local.join("CodexRelay");
    fs::create_dir_all(&real_codex_sentinel).unwrap();
    fs::create_dir_all(real_app_data_sentinel.join("backups")).unwrap();
    fs::write(
        real_codex_sentinel.join("config.toml"),
        "this is deliberately invalid toml [[[",
    )
    .unwrap();
    fs::write(
        real_codex_sentinel.join("auth.json"),
        "{ deliberately invalid json",
    )
    .unwrap();
    fs::write(
        real_app_data_sentinel.join("providers.json"),
        "{ deliberately invalid json",
    )
    .unwrap();
    fs::write(
        real_app_data_sentinel
            .join("backups")
            .join("never-touch.txt"),
        "sentinel",
    )
    .unwrap();
    let sentinel_profile_before = snapshot_tree(&sentinel_profile);
    let sentinel_local_before = snapshot_tree(&sentinel_local);
    let _guard = EnvGuard::set(&[
        ("CODEX_RELAY_CODEX_HOME", Some(&codex)),
        ("CODEX_RELAY_APP_DATA_DIR", Some(&app_data)),
        ("CODEX_HOME", None),
        ("USERPROFILE", Some(&sentinel_profile)),
        ("LOCALAPPDATA", Some(&sentinel_local)),
    ]);
    let paths = resolve_paths(PathMode::Test).unwrap();
    fs::create_dir_all(&paths.codex_home).unwrap();
    fs::create_dir_all(&paths.app_data_dir).unwrap();
    fs::write(
        &paths.config_file,
        "model_provider = \"provider-a\"\n\n[model_providers.provider-a]\nname = \"Provider A\"\nbase_url = \"https://provider-a.example.test/v1\"\nwire_api = \"responses\"\n",
    )
    .unwrap();
    fs::write(
        &paths.auth_file,
        "{\"OPENAI_API_KEY\":\"test-key-a-not-real\"}\n",
    )
    .unwrap();
    fs::write(
        &paths.providers_file,
        "{\"version\":1,\"providers\":{\"provider-a\":{\"apiKey\":\"test-key-a-not-real\"}}}\n",
    )
    .unwrap();

    let audit = Arc::new(AuditingFileOps::default());
    let service = ProviderService::with_file_ops(paths.clone(), "0.1.0", audit.clone());
    assert_eq!(service.paths(), &paths);
    let state = service.list_providers().unwrap();
    service
        .create_provider(CreateProviderInput {
            id: "provider-b".into(),
            name: "Provider B".into(),
            base_url: "https://provider-b.example.test/v1".into(),
            wire_api: "responses".into(),
            model: None,
            api_key: "test-key-b-not-real".into(),
            activate_after_save: false,
            expected_files: state.fingerprints,
        })
        .await
        .unwrap();
    assert_eq!(service.list_backups().unwrap().len(), 1);

    let observed = audit.observed();
    assert!(!observed.is_empty());
    assert!(
        observed
            .iter()
            .all(|path| path.starts_with(&codex) || path.starts_with(&app_data))
    );

    assert!(
        observed
            .iter()
            .all(|path| !path.starts_with(&real_codex_sentinel))
    );
    assert!(
        observed
            .iter()
            .all(|path| !path.starts_with(&real_app_data_sentinel))
    );
    assert_eq!(snapshot_tree(&sentinel_profile), sentinel_profile_before);
    assert_eq!(snapshot_tree(&sentinel_local), sentinel_local_before);
}
