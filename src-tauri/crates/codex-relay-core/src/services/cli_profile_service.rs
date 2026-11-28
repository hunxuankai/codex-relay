use crate::error::AppError;
use crate::infrastructure::path_service::AppPaths;
use crate::infrastructure::safe_log::redact;
use crate::infrastructure::user_environment::{
    UserEnvironment, UserValue, desktop_environment, open_environment_settings,
};
use crate::models::cli_profile::{
    ApplyProfileProviderKeyInput, CliProfile, CliProfileList, KeyRestriction, ProfileKeyEdit,
    ProfileKeySaved, ProfileKeyStatus, SaveProfileKeyInput, VariablePresence,
};
use crate::models::provider::ProviderApiKeySummary;
use crate::services::cli_profile_catalog::{CliProfileCatalog, ProfileConfig};
use crate::services::provider_secret_service::{
    NamedApiKey, ProviderSecretService, ProviderSecretStore,
};
use crate::services::transaction_service::{TransactionService, profile_key_conflict};
use sha2::{Digest, Sha256};
use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;
use zeroize::Zeroizing;

const EDIT_TTL: Duration = Duration::from_secs(300);
const MAX_KEY_LENGTH: usize = 4096;

pub struct CliProfileService {
    paths: AppPaths,
    environment: Arc<dyn UserEnvironment>,
    transactions: TransactionService,
    pending_edit: Mutex<Option<EditBaseline>>,
}

struct EditBaseline {
    token: String,
    profile_name: String,
    provider_id: String,
    variable: String,
    opened: Instant,
    catalog_fingerprint: [u8; 32],
    variable_fingerprint: [u8; 32],
    provider_key_fingerprints: BTreeMap<String, [u8; 32]>,
}

impl CliProfileService {
    pub(super) fn new(transactions: TransactionService) -> Self {
        let environment = desktop_environment(transactions.paths());
        Self::with_environment(transactions, environment)
    }

    pub fn open_settings(&self) -> Result<(), AppError> {
        open_environment_settings()
    }

    fn with_environment(
        transactions: TransactionService,
        environment: Arc<dyn UserEnvironment>,
    ) -> Self {
        Self {
            paths: transactions.paths().clone(),
            environment,
            transactions,
            pending_edit: Mutex::new(None),
        }
    }

    pub fn prepare_key_edit(&self, profile_name: &str) -> Result<ProfileKeyEdit, AppError> {
        let catalog = CliProfileCatalog::read(&self.paths)?;
        let profile = catalog
            .profiles
            .iter()
            .find(|item| item.name == profile_name);
        let Some(profile) = profile.filter(|item| item.problem.is_none()) else {
            return Err(key_read_only());
        };
        let variable = profile.variable.as_deref().ok_or_else(key_read_only)?;
        let provider_id = profile.provider_id.clone().ok_or_else(key_read_only)?;
        let current = self.environment.user_value(variable);
        if key_restriction(
            &*self.environment,
            variable,
            &shared_references(&catalog, profile, variable),
            &current,
        )
        .is_some()
        {
            return Err(key_read_only());
        }
        let current = current?;
        let mut provider_key_fingerprints = BTreeMap::new();
        let provider_keys = if profile.provider_defined_in_base {
            self.read_provider_keys(&provider_id)
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| valid_profile_key(&entry.api_key))
                .map(|entry| {
                    provider_key_fingerprints
                        .insert(entry.id.clone(), provider_key_fingerprint(&entry));
                    ProviderApiKeySummary {
                        id: entry.id,
                        name: entry.name,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let edit = ProfileKeyEdit {
            profile_name: profile_name.to_owned(),
            provider_id,
            host: profile.host.clone(),
            variable: variable.to_owned(),
            token: Uuid::new_v4().to_string(),
            replaces_value: !matches!(current, UserValue::Missing),
            relay_process_has_value: self
                .environment
                .process_has_value(variable)
                .unwrap_or(false),
            provider_keys,
        };
        *self.pending_edit.lock().map_err(|_| expired_edit())? = Some(EditBaseline {
            token: edit.token.clone(),
            profile_name: edit.profile_name.clone(),
            provider_id: edit.provider_id.clone(),
            variable: edit.variable.clone(),
            opened: Instant::now(),
            catalog_fingerprint: catalog.fingerprint,
            variable_fingerprint: current.fingerprint(),
            provider_key_fingerprints,
        });
        Ok(edit)
    }

    fn read_provider_secret_store(&self) -> Result<ProviderSecretStore, AppError> {
        ProviderSecretService::new(self.paths.providers_file.clone()).load_read_only()
    }

    fn read_provider_keys(&self, provider_id: &str) -> Result<Vec<NamedApiKey>, AppError> {
        let mut store = self.read_provider_secret_store()?;
        Ok(store
            .providers
            .remove(provider_id)
            .map(|secret| secret.api_keys)
            .unwrap_or_default())
    }

    pub async fn save_key(&self, input: SaveProfileKeyInput) -> Result<ProfileKeySaved, AppError> {
        let key = Zeroizing::new(input.new_key);
        let baseline = self.take_edit(&input.profile_name, &input.token)?;
        if !valid_profile_key(&key) {
            return Err(AppError::new(
                "INVALID_PROFILE_KEY",
                "Key 不能为空、包含控制字符或首尾空白，且不能超过 4096 字节。",
                "profile key input rejected",
            ));
        }
        let replacement = UserValue::text(&key);
        drop(key);
        self.write_key(&baseline, replacement, || Ok(())).await
    }

    pub async fn apply_provider_key(
        &self,
        input: ApplyProfileProviderKeyInput,
    ) -> Result<ProfileKeySaved, AppError> {
        let baseline = self.take_edit(&input.profile_name, &input.token)?;
        let expected = baseline
            .provider_key_fingerprints
            .get(&input.api_key_id)
            .ok_or_else(provider_key_unavailable)?;
        let read_key = || {
            self.read_provider_keys(&baseline.provider_id)
                .map_err(|_| provider_key_unavailable())?
                .into_iter()
                .find(|entry| {
                    entry.id == input.api_key_id && provider_key_fingerprint(entry) == *expected
                })
                .map(|entry| Zeroizing::new(entry.api_key))
                .ok_or_else(provider_key_unavailable)
        };
        let key = read_key()?;
        let replacement = UserValue::text(&key);
        drop(key);
        self.write_key(&baseline, replacement, || read_key().map(|_| ()))
            .await
    }

    fn take_edit(&self, profile_name: &str, token: &str) -> Result<EditBaseline, AppError> {
        let mut pending = self.pending_edit.lock().map_err(|_| expired_edit())?;
        if !pending.as_ref().is_some_and(|edit| {
            edit.token == token
                && edit.profile_name == profile_name
                && edit.opened.elapsed() < EDIT_TTL
        }) {
            return Err(expired_edit());
        }
        pending.take().ok_or_else(expired_edit)
    }

    async fn write_key(
        &self,
        baseline: &EditBaseline,
        replacement: UserValue,
        confirm_source: impl Fn() -> Result<(), AppError> + Send + Sync,
    ) -> Result<ProfileKeySaved, AppError> {
        self.transactions
            .write_profile_environment(
                &*self.environment,
                &baseline.variable,
                baseline.variable_fingerprint,
                &replacement,
                || {
                    if CliProfileCatalog::read(&self.paths)?.fingerprint
                        != baseline.catalog_fingerprint
                        || self.environment.system_has_value(&baseline.variable)?
                    {
                        return Err(profile_key_conflict());
                    }
                    confirm_source()
                },
            )
            .await?;
        let environment = Arc::clone(&self.environment);
        let windows_notified = tokio::task::spawn_blocking(move || environment.notify_changed())
            .await
            .unwrap_or(false);
        Ok(ProfileKeySaved { windows_notified })
    }

    pub fn list(&self) -> Result<CliProfileList, AppError> {
        let catalog = CliProfileCatalog::read(&self.paths)?;
        let provider_secrets = OnceCell::new();
        let profiles = catalog
            .profiles
            .iter()
            .map(|profile| CliProfile {
                name: profile.name.clone(),
                command: profile.command.clone(),
                provider_id: profile.provider_id.clone(),
                host: profile.host.clone(),
                variable: profile.variable.clone(),
                inherits_default: profile.inherits_default,
                problem: profile.problem,
                key_status: self.key_status(&catalog, profile, &provider_secrets),
            })
            .collect();
        Ok(CliProfileList {
            codex_home: redact(&self.paths.codex_home.display().to_string()),
            profiles,
            pending_write: self.transactions.profile_key_recovery_pending(),
        })
    }

    fn key_status(
        &self,
        catalog: &CliProfileCatalog,
        profile: &ProfileConfig,
        provider_secrets: &OnceCell<Option<ProviderSecretStore>>,
    ) -> Option<ProfileKeyStatus> {
        if profile.problem.is_some() {
            return None;
        }
        let variable = profile.variable.as_deref()?;
        let user_value = self.environment.user_value(variable);
        let shared_with = shared_references(catalog, profile, variable);
        let restriction = key_restriction(&*self.environment, variable, &shared_with, &user_value);
        let user = match &user_value {
            Ok(UserValue::Text(bytes)) if bytes.len() > 2 => VariablePresence::Set,
            Ok(UserValue::Text(_) | UserValue::Missing) => VariablePresence::NotDetected,
            Ok(UserValue::Unsupported) | Err(_) => VariablePresence::Unavailable,
        };
        let matched_user_key_name = if user == VariablePresence::Set
            && profile.provider_defined_in_base
            && let Some(provider_id) = profile.provider_id.as_deref()
            && let Ok(value) = &user_value
        {
            provider_secrets
                .get_or_init(|| self.read_provider_secret_store().ok())
                .as_ref()
                .and_then(|store| store.providers.get(provider_id))
                .and_then(|secret| {
                    secret
                        .api_keys
                        .iter()
                        .find(|entry| value == &UserValue::text(&entry.api_key))
                })
                .map(|entry| entry.name.clone())
        } else {
            None
        };
        Some(ProfileKeyStatus {
            user,
            relay_process: match self.environment.process_has_value(variable) {
                Ok(true) => VariablePresence::Set,
                Ok(false) => VariablePresence::NotDetected,
                Err(_) => VariablePresence::Unavailable,
            },
            matched_user_key_name,
            restriction,
            shared_with,
        })
    }
}

fn provider_key_fingerprint(entry: &NamedApiKey) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(entry.name.len().to_le_bytes());
    digest.update(entry.name.as_bytes());
    digest.update(entry.api_key.as_bytes());
    digest.finalize().into()
}

fn provider_key_unavailable() -> AppError {
    AppError::new(
        "PROFILE_PROVIDER_KEY_UNAVAILABLE",
        "所选供应商 Key 已变更或不可用，请重新打开设置后重试。",
        "named provider key is not available for this profile edit",
    )
}

fn valid_profile_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= MAX_KEY_LENGTH
        && key.trim() == key
        && !key.chars().any(char::is_control)
}

fn key_read_only() -> AppError {
    AppError::new(
        "PROFILE_KEY_READ_ONLY",
        "当前 Profile 的 Key 不适合直接编辑，请刷新后查看原因。",
        "profile key target is not writable",
    )
}

fn expired_edit() -> AppError {
    AppError::new(
        "PROFILE_KEY_EDIT_EXPIRED",
        "编辑已失效，请重新打开 Key 设置。",
        "profile environment edit token is stale",
    )
}

fn shared_references(
    catalog: &CliProfileCatalog,
    profile: &ProfileConfig,
    variable: &str,
) -> Vec<String> {
    let mut references = Vec::new();
    if catalog
        .default_variable
        .as_deref()
        .is_some_and(|active| active.eq_ignore_ascii_case(variable))
    {
        references.push("默认供应商".to_owned());
    }
    references.extend(
        catalog
            .profiles
            .iter()
            .filter(|peer| {
                peer.name != profile.name
                    && peer
                        .variable
                        .as_deref()
                        .is_some_and(|other| other.eq_ignore_ascii_case(variable))
            })
            .map(|peer| peer.name.clone()),
    );
    references
}

fn key_restriction(
    environment: &dyn UserEnvironment,
    variable: &str,
    shared_with: &[String],
    user_value: &Result<UserValue, AppError>,
) -> Option<KeyRestriction> {
    if !shared_with.is_empty() {
        return Some(KeyRestriction::SharedVariable);
    }
    let name = variable.to_ascii_uppercase();
    if !(name
        .strip_suffix("_API_KEY")
        .or_else(|| name.strip_suffix("_TOKEN"))
        .is_some_and(|prefix| prefix.bytes().any(|byte| byte.is_ascii_alphanumeric())))
        || ["OPENAI_", "CODEX_", "GITHUB_", "GH_"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
    {
        return Some(KeyRestriction::NotDedicated);
    }
    match environment.system_has_value(variable) {
        Ok(true) => return Some(KeyRestriction::SystemVariable),
        Ok(false) => {}
        Err(_) => return Some(KeyRestriction::EnvironmentUnavailable),
    }
    match user_value {
        Ok(UserValue::Missing | UserValue::Text(_)) => None,
        Ok(UserValue::Unsupported) => Some(KeyRestriction::UnsupportedValue),
        Err(_) => Some(KeyRestriction::EnvironmentUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::user_environment::{UserEnvironmentTransaction, unavailable};
    use crate::models::cli_profile::{KeyRestriction, VariablePresence};
    use crate::services::backup_service::BackupService;
    use crate::services::provider_secret_service::ProviderSecret;
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;

    const PRIMARY_KEY_ID: &str = "11111111-1111-4111-8111-111111111111";
    const BACKUP_KEY_ID: &str = "22222222-2222-4222-8222-222222222222";

    #[derive(Default)]
    struct FakeEnvironment {
        users: Mutex<BTreeMap<String, UserValue>>,
        machine: BTreeSet<String>,
        process: BTreeSet<String>,
        failure: Mutex<Option<SaveFault>>,
        fail_next_read: Mutex<bool>,
        during_stage: Mutex<Option<Box<dyn FnOnce() + Send>>>,
        notified: Mutex<bool>,
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum SaveFault {
        Stage,
        Commit,
        ReadBack,
        CompetingWriter,
        Abort,
    }

    struct FakeTransaction<'store> {
        environment: &'store FakeEnvironment,
        staged: Option<(String, UserValue)>,
    }

    impl UserEnvironment for FakeEnvironment {
        fn user_value(&self, name: &str) -> Result<UserValue, AppError> {
            if name == "UNKNOWN_API_KEY"
                || std::mem::take(&mut *self.fail_next_read.lock().unwrap())
            {
                return Err(unavailable());
            }
            Ok(self
                .users
                .lock()
                .unwrap()
                .get(&name.to_ascii_uppercase())
                .cloned()
                .unwrap_or(UserValue::Missing))
        }

        fn system_has_value(&self, name: &str) -> Result<bool, AppError> {
            Ok(self.machine.contains(&name.to_ascii_uppercase()))
        }

        fn process_has_value(&self, name: &str) -> Result<bool, AppError> {
            Ok(self.process.contains(&name.to_ascii_uppercase()))
        }

        fn begin(&self) -> Result<Box<dyn UserEnvironmentTransaction + '_>, AppError> {
            Ok(Box::new(FakeTransaction {
                environment: self,
                staged: None,
            }))
        }

        fn notify_changed(&self) -> bool {
            *self.notified.lock().unwrap() = true;
            true
        }
    }

    impl UserEnvironmentTransaction for FakeTransaction<'_> {
        fn read(&self, name: &str) -> Result<UserValue, AppError> {
            if let Some((target, staged)) = &self.staged
                && target.eq_ignore_ascii_case(name)
            {
                return Ok(staged.clone());
            }
            self.environment.user_value(name)
        }

        fn stage(&mut self, name: &str, value: &UserValue) -> Result<(), AppError> {
            self.staged = Some((name.to_ascii_uppercase(), value.clone()));
            if let Some(change) = self.environment.during_stage.lock().unwrap().take() {
                change();
            }
            if matches!(
                *self.environment.failure.lock().unwrap(),
                Some(SaveFault::Stage | SaveFault::Abort)
            ) {
                return Err(unavailable());
            }
            Ok(())
        }

        fn commit(self: Box<Self>) -> Result<(), AppError> {
            let fault = self.environment.failure.lock().unwrap().take();
            if fault == Some(SaveFault::Commit) {
                return Err(unavailable());
            }
            let (name, value) = self.staged.unwrap();
            let mut users = self.environment.users.lock().unwrap();
            if fault == Some(SaveFault::CompetingWriter) {
                users.insert(name, text("test-key-external-not-real"));
            } else if value == UserValue::Missing {
                users.remove(&name);
            } else {
                users.insert(name, value);
            }
            if fault == Some(SaveFault::ReadBack) {
                *self.environment.fail_next_read.lock().unwrap() = true;
            }
            Ok(())
        }

        fn roll_back(self: Box<Self>) -> Result<(), AppError> {
            if self.environment.failure.lock().unwrap().take() == Some(SaveFault::Abort) {
                return Err(unavailable());
            }
            Ok(())
        }
    }

    fn temporary_paths() -> (tempfile::TempDir, AppPaths) {
        let temporary = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(temporary.path().join("home"), temporary.path().join("app"))
            .unwrap();
        fs::create_dir(&paths.codex_home).unwrap();
        (temporary, paths)
    }

    fn write_profile(paths: &AppPaths, name: &str, variable: &str) {
        fs::write(
            paths.codex_home.join(format!("{name}.config.toml")),
            format!("model_provider = 'vendor'\n[model_providers.vendor]\nbase_url = 'https://vendor.test'\nenv_key = '{variable}'\n"),
        ).unwrap();
    }

    fn service_at(paths: AppPaths, environment: Arc<FakeEnvironment>) -> CliProfileService {
        let backup = BackupService::new(paths.backups_dir.clone(), "0.0.0");
        CliProfileService::with_environment(TransactionService::new(paths, backup), environment)
    }

    fn text(value: &str) -> UserValue {
        UserValue::text(value)
    }

    fn provider_key_fixture() -> ProviderSecretStore {
        let mut store = ProviderSecretStore::default();
        store.providers.insert(
            "vendor".to_owned(),
            ProviderSecret::from_named_api_keys(
                vec![
                    NamedApiKey {
                        id: PRIMARY_KEY_ID.to_owned(),
                        name: "主用".to_owned(),
                        api_key: "test-key-primary-not-real".to_owned(),
                    },
                    NamedApiKey {
                        id: BACKUP_KEY_ID.to_owned(),
                        name: "备用".to_owned(),
                        api_key: "test-key-backup-not-real".to_owned(),
                    },
                ],
                PRIMARY_KEY_ID,
            )
            .unwrap(),
        );
        store.providers.insert(
            "other".to_owned(),
            ProviderSecret::single_named("其他", "test-key-other-provider-not-real").unwrap(),
        );
        store
    }

    fn write_default_provider_fixture(paths: &AppPaths) {
        let config = r#"model_provider = "other"
[model_providers.other]
name = "Shared name"
base_url = "https://other.test"
[model_providers.vendor]
name = "Shared name"
base_url = "https://vendor.test"
"#;
        fs::write(&paths.config_file, config).unwrap();
    }

    #[test]
    fn exposes_only_named_key_summaries_for_an_exact_default_provider_id() {
        let (_temporary, paths) = temporary_paths();
        write_profile(&paths, "fixture", "VENDOR_API_KEY");
        write_default_provider_fixture(&paths);
        let service = service_at(paths.clone(), Arc::new(FakeEnvironment::default()));
        let secrets = ProviderSecretService::new(paths.providers_file.clone());
        let empty = serde_json::to_value(service.prepare_key_edit("fixture").unwrap()).unwrap();
        let store_created = paths.providers_file.exists();
        secrets.save_store(&provider_key_fixture()).unwrap();
        let edit = serde_json::to_value(service.prepare_key_edit("fixture").unwrap()).unwrap();
        assert_eq!(
            edit["providerKeys"],
            json!([{ "id": PRIMARY_KEY_ID, "name": "主用" }, { "id": BACKUP_KEY_ID, "name": "备用" }])
        );
        assert_eq!(empty["providerKeys"], json!([]));
        assert!(!store_created);

        let original = fs::read_to_string(&paths.config_file).unwrap();
        fs::write(&paths.config_file, original.replace(".vendor]", ".Vendor]")).unwrap();
        let edit = serde_json::to_value(service.prepare_key_edit("fixture").unwrap()).unwrap();
        assert_eq!(edit["providerKeys"], json!([]));

        fs::write(&paths.config_file, original).unwrap();
        let malformed = b"not a secret store fixture";
        fs::write(&paths.providers_file, malformed).unwrap();
        let edit = serde_json::to_value(service.prepare_key_edit("fixture").unwrap()).unwrap();
        assert_eq!(edit["providerKeys"], json!([]));
        assert_eq!(fs::read(&paths.providers_file).unwrap(), malformed);
        assert!(!paths.backups_dir.exists());
    }

    #[test]
    fn lists_the_key_name_from_the_windows_user_value_only_for_the_same_provider() {
        let (_temporary, paths) = temporary_paths();
        write_default_provider_fixture(&paths);
        for (name, variable) in [
            ("fixture", "VENDOR_API_KEY"),
            ("shared-peer", "vendor_api_key"),
            ("other-provider", "OTHER_API_KEY"),
            ("not-exact", "DIFFERENT_API_KEY"),
            ("process-only", "PROCESS_API_KEY"),
        ] {
            write_profile(&paths, name, variable);
        }
        ProviderSecretService::new(paths.providers_file.clone())
            .save_store(&provider_key_fixture())
            .unwrap();
        let environment = Arc::new(FakeEnvironment {
            users: Mutex::new(BTreeMap::from([
                (
                    "VENDOR_API_KEY".to_owned(),
                    text("test-key-backup-not-real"),
                ),
                (
                    "OTHER_API_KEY".to_owned(),
                    text("test-key-other-provider-not-real"),
                ),
                (
                    "DIFFERENT_API_KEY".to_owned(),
                    text("test-key-backup-NOT-real"),
                ),
            ])),
            process: BTreeSet::from(["PROCESS_API_KEY".to_owned()]),
            ..Default::default()
        });
        let service = service_at(paths, environment);
        let list = serde_json::to_value(service.list().unwrap()).unwrap();
        let status = |name| {
            &list["profiles"]
                .as_array()
                .unwrap()
                .iter()
                .find(|profile| profile["name"] == name)
                .unwrap()["keyStatus"]
        };

        assert_eq!(status("fixture")["matchedUserKeyName"], json!("备用"));
        assert_eq!(status("fixture")["restriction"], json!("sharedVariable"));
        assert_eq!(status("shared-peer")["matchedUserKeyName"], json!("备用"));
        for name in ["other-provider", "not-exact", "process-only"] {
            assert_eq!(status(name).get("matchedUserKeyName"), Some(&json!(null)));
        }
        assert_eq!(status("process-only")["relayProcess"], json!("set"));
        let response = list.to_string();
        assert!(!response.contains("test-key-"));
        assert!(!response.contains(BACKUP_KEY_ID));
    }

    #[test]
    fn lists_no_key_name_when_the_saved_source_is_missing_or_unusable() {
        let (_temporary, paths) = temporary_paths();
        write_default_provider_fixture(&paths);
        write_profile(&paths, "fixture", "VENDOR_API_KEY");
        let environment = Arc::new(FakeEnvironment {
            users: Mutex::new(BTreeMap::from([(
                "VENDOR_API_KEY".to_owned(),
                text("test-key-backup-not-real"),
            )])),
            ..Default::default()
        });
        let service = service_at(paths.clone(), environment);
        let status = || {
            let list = serde_json::to_value(service.list().unwrap()).unwrap();
            list["profiles"][0]["keyStatus"].clone()
        };

        let missing_store = status();
        assert_eq!(missing_store["user"], json!("set"));
        assert_eq!(missing_store.get("matchedUserKeyName"), Some(&json!(null)));
        assert!(!paths.app_data_dir.exists());

        fs::create_dir_all(&paths.app_data_dir).unwrap();
        let malformed = b"not a secret store fixture";
        fs::write(&paths.providers_file, malformed).unwrap();
        assert_eq!(status().get("matchedUserKeyName"), Some(&json!(null)));
        assert_eq!(fs::read(&paths.providers_file).unwrap(), malformed);
        assert_eq!(fs::read_dir(&paths.app_data_dir).unwrap().count(), 1);

        ProviderSecretService::new(paths.providers_file.clone())
            .save_store(&provider_key_fixture())
            .unwrap();
        let original = fs::read_to_string(&paths.config_file).unwrap();
        fs::write(&paths.config_file, original.replace(".vendor]", ".Vendor]")).unwrap();
        assert_eq!(status().get("matchedUserKeyName"), Some(&json!(null)));
    }

    #[test]
    fn reports_scopes_and_edit_restrictions_without_serializing_values() {
        let (_temporary, paths) = temporary_paths();
        for (name, variable) in [
            ("set", "VENDOR_API_KEY"),
            ("other", "OTHER_TOKEN"),
            ("shared", "SHARED_TOKEN"),
            ("shared-peer", "shared_token"),
            ("machine", "MACHINE_API_KEY"),
            ("generic", "OPENAI_API_KEY"),
            ("unsupported", "UNSUPPORTED_TOKEN"),
            ("unavailable", "UNKNOWN_API_KEY"),
        ] {
            write_profile(&paths, name, variable);
        }
        let environment = FakeEnvironment {
            users: Mutex::new(BTreeMap::from([
                (
                    "VENDOR_API_KEY".to_owned(),
                    text("test-key-profile-not-real"),
                ),
                ("UNSUPPORTED_TOKEN".to_owned(), UserValue::Unsupported),
            ])),
            machine: BTreeSet::from(["MACHINE_API_KEY".to_owned()]),
            process: BTreeSet::from(["OTHER_TOKEN".to_owned()]),
            ..Default::default()
        };
        let list = service_at(paths.clone(), Arc::new(environment))
            .list()
            .unwrap();
        assert_eq!(list.profiles.len(), 8);
        let status = |name| {
            list.profiles
                .iter()
                .find(|profile| profile.name == name)
                .unwrap()
                .key_status
                .as_ref()
                .unwrap()
        };
        assert_eq!(status("set").user, VariablePresence::Set);
        assert_eq!(status("set").relay_process, VariablePresence::NotDetected);
        assert_eq!(status("other").user, VariablePresence::NotDetected);
        assert_eq!(status("other").relay_process, VariablePresence::Set);
        assert_eq!(status("set").restriction, None);
        assert_eq!(
            status("shared").restriction,
            Some(KeyRestriction::SharedVariable)
        );
        assert_eq!(status("shared").shared_with, ["shared-peer"]);
        assert_eq!(
            status("machine").restriction,
            Some(KeyRestriction::SystemVariable)
        );
        assert_eq!(
            status("generic").restriction,
            Some(KeyRestriction::NotDedicated)
        );
        assert_eq!(
            status("unsupported").restriction,
            Some(KeyRestriction::UnsupportedValue)
        );
        assert_eq!(status("unavailable").user, VariablePresence::Unavailable);
        assert_eq!(
            status("unavailable").restriction,
            Some(KeyRestriction::EnvironmentUnavailable)
        );
        assert!(!serde_json::to_string(&list).unwrap().contains("test-key-"));
        assert!(!paths.app_data_dir.exists());
    }

    async fn save(
        service: &CliProfileService,
        edit: ProfileKeyEdit,
    ) -> Result<ProfileKeySaved, AppError> {
        service
            .save_key(SaveProfileKeyInput {
                token: edit.token,
                profile_name: edit.profile_name,
                new_key: "test-key-replacement-not-real".to_owned(),
            })
            .await
    }

    async fn apply_saved_key(
        service: &CliProfileService,
        edit: &ProfileKeyEdit,
        api_key_id: &str,
    ) -> Result<ProfileKeySaved, AppError> {
        service
            .apply_provider_key(ApplyProfileProviderKeyInput {
                profile_name: edit.profile_name.clone(),
                token: edit.token.clone(),
                api_key_id: api_key_id.to_owned(),
            })
            .await
    }

    #[tokio::test]
    async fn applies_the_named_key_to_only_the_profile_variable_without_selecting_a_default_key() {
        let (_temporary, paths) = temporary_paths();
        write_default_provider_fixture(&paths);
        write_profile(&paths, "fixture", "VENDOR_API_KEY");
        write_profile(&paths, "peer", "OTHER_API_KEY");
        ProviderSecretService::new(paths.providers_file.clone())
            .save_store(&provider_key_fixture())
            .unwrap();
        fs::write(&paths.auth_file, b"auth fixture stays unchanged").unwrap();
        let unchanged = [
            &paths.config_file,
            &paths.auth_file,
            &paths.providers_file,
            &paths.provider_preferences_file,
            &paths.codex_home.join("fixture.config.toml"),
            &paths.codex_home.join("peer.config.toml"),
        ];
        let before = unchanged.map(|path| fs::read(path).ok());
        let environment = Arc::new(FakeEnvironment::default());
        environment
            .users
            .lock()
            .unwrap()
            .insert("OTHER_API_KEY".to_owned(), text("test-key-peer-not-real"));
        let service = service_at(paths.clone(), environment.clone());
        let edit = service.prepare_key_edit("fixture").unwrap();
        assert!(
            apply_saved_key(&service, &edit, BACKUP_KEY_ID)
                .await
                .unwrap()
                .windows_notified
        );
        assert!(
            environment.user_value("VENDOR_API_KEY").unwrap() == text("test-key-backup-not-real")
        );
        assert!(environment.user_value("OTHER_API_KEY").unwrap() == text("test-key-peer-not-real"));
        assert!(before == unchanged.map(|path| fs::read(path).ok()));
        let list = service.list().unwrap();
        assert!(!list.pending_write);
        let listed = serde_json::to_value(list).unwrap();
        assert_eq!(listed["profiles"][0]["name"], json!("fixture"));
        assert_eq!(
            listed["profiles"][0]["keyStatus"]["matchedUserKeyName"],
            json!("备用")
        );
        assert_eq!(
            apply_saved_key(&service, &edit, PRIMARY_KEY_ID)
                .await
                .unwrap_err()
                .code(),
            "PROFILE_KEY_EDIT_EXPIRED"
        );
    }

    #[tokio::test]
    async fn rejects_other_provider_entries_and_deleted_renamed_or_replaced_keys() {
        let (_temporary, paths) = temporary_paths();
        write_default_provider_fixture(&paths);
        write_profile(&paths, "fixture", "VENDOR_API_KEY");
        let secrets = ProviderSecretService::new(paths.providers_file.clone());
        let store = provider_key_fixture();
        secrets.save_store(&store).unwrap();
        let environment = Arc::new(FakeEnvironment::default());
        let service = service_at(paths.clone(), environment.clone());
        let edit = service.prepare_key_edit("fixture").unwrap();
        let other_id = &store.providers["other"].selected_api_key_id;
        assert_eq!(
            apply_saved_key(&service, &edit, other_id)
                .await
                .unwrap_err()
                .code(),
            "PROFILE_PROVIDER_KEY_UNAVAILABLE"
        );

        let mut deleted = store.clone();
        deleted.providers.get_mut("vendor").unwrap().api_keys.pop();
        let mut renamed = store.clone();
        renamed.providers.get_mut("vendor").unwrap().api_keys[1].name = "新名称".to_owned();
        let mut replaced = store.clone();
        replaced.providers.get_mut("vendor").unwrap().api_keys[1].api_key =
            "test-key-rotated-not-real".to_owned();
        for changed in [deleted, renamed, replaced] {
            secrets.save_store(&store).unwrap();
            let edit = service.prepare_key_edit("fixture").unwrap();
            secrets.save_store(&changed).unwrap();
            assert_eq!(
                apply_saved_key(&service, &edit, BACKUP_KEY_ID)
                    .await
                    .unwrap_err()
                    .code(),
                "PROFILE_PROVIDER_KEY_UNAVAILABLE"
            );
        }
        assert!(environment.user_value("VENDOR_API_KEY").unwrap() == UserValue::Missing);
        assert!(!service.list().unwrap().pending_write);
    }

    #[tokio::test]
    async fn cancels_a_staged_profile_write_if_the_named_key_changes_before_commit() {
        let (_temporary, paths) = temporary_paths();
        write_default_provider_fixture(&paths);
        write_profile(&paths, "fixture", "VENDOR_API_KEY");
        let secrets = ProviderSecretService::new(paths.providers_file.clone());
        let mut store = provider_key_fixture();
        secrets.save_store(&store).unwrap();
        let environment = Arc::new(FakeEnvironment::default());
        let original = text("test-key-before-stage-not-real");
        environment
            .users
            .lock()
            .unwrap()
            .insert("VENDOR_API_KEY".to_owned(), original.clone());
        let service = service_at(paths, environment.clone());
        let edit = service.prepare_key_edit("fixture").unwrap();
        *environment.during_stage.lock().unwrap() = Some(Box::new(move || {
            store.providers.get_mut("vendor").unwrap().api_keys[1].api_key =
                "test-key-during-stage-not-real".to_owned();
            secrets.save_store(&store).unwrap();
        }));

        assert_eq!(
            apply_saved_key(&service, &edit, BACKUP_KEY_ID)
                .await
                .unwrap_err()
                .code(),
            "PROFILE_PROVIDER_KEY_UNAVAILABLE"
        );
        assert!(environment.user_value("VENDOR_API_KEY").unwrap() == original);
        assert!(!*environment.notified.lock().unwrap());
        assert!(!service.list().unwrap().pending_write);
    }

    #[tokio::test]
    async fn sets_and_replaces_only_the_variable_bound_to_this_profile() {
        let (_temporary, paths) = temporary_paths();
        write_profile(&paths, "agentrouter", "VENDOR_API_KEY");
        write_profile(&paths, "qiandao", "OTHER_API_KEY");
        let configuration = fs::read(paths.codex_home.join("agentrouter.config.toml")).unwrap();
        fs::write(&paths.auth_file, b"fixture stays unchanged").unwrap();
        let environment = Arc::new(FakeEnvironment::default());
        environment
            .users
            .lock()
            .unwrap()
            .insert("OTHER_API_KEY".into(), text("test-key-other-not-real"));
        let service = service_at(paths.clone(), environment.clone());

        let edit = service.prepare_key_edit("agentrouter").unwrap();
        assert!(!edit.replaces_value);
        assert!(!paths.app_data_dir.exists());
        assert!(save(&service, edit).await.unwrap().windows_notified);
        assert!(
            environment.user_value("VENDOR_API_KEY").unwrap()
                == text("test-key-replacement-not-real")
        );
        assert!(
            environment.user_value("OTHER_API_KEY").unwrap() == text("test-key-other-not-real")
        );
        assert!(
            service
                .prepare_key_edit("agentrouter")
                .unwrap()
                .replaces_value
        );
        assert!(!service.list().unwrap().pending_write);
        assert_eq!(
            fs::read(paths.codex_home.join("agentrouter.config.toml")).unwrap(),
            configuration
        );
        assert_eq!(
            fs::read(&paths.auth_file).unwrap(),
            b"fixture stays unchanged"
        );
        assert!(!paths.config_file.exists());
        assert!(!paths.providers_file.exists());
        assert!(!paths.backups_dir.exists());
    }

    #[tokio::test]
    async fn refuses_changed_config_changed_value_and_reused_edit_tokens() {
        let (_temporary, paths) = temporary_paths();
        write_profile(&paths, "agentrouter", "VENDOR_API_KEY");
        let environment = Arc::new(FakeEnvironment::default());
        let service = service_at(paths.clone(), environment.clone());
        let edit = service.prepare_key_edit("agentrouter").unwrap();
        environment
            .users
            .lock()
            .unwrap()
            .insert("VENDOR_API_KEY".into(), text("test-key-external-not-real"));
        let replay = ProfileKeyEdit {
            token: edit.token.clone(),
            profile_name: edit.profile_name.clone(),
            provider_id: edit.provider_id.clone(),
            host: edit.host.clone(),
            variable: edit.variable.clone(),
            replaces_value: edit.replaces_value,
            relay_process_has_value: edit.relay_process_has_value,
            provider_keys: edit.provider_keys.clone(),
        };
        assert_eq!(
            save(&service, edit).await.unwrap_err().code(),
            "PROFILE_KEY_CONFLICT"
        );
        assert_eq!(
            save(&service, replay).await.unwrap_err().code(),
            "PROFILE_KEY_EDIT_EXPIRED"
        );
        let edit = service.prepare_key_edit("agentrouter").unwrap();
        write_profile(&paths, "agentrouter", "CHANGED_API_KEY");
        assert_eq!(
            save(&service, edit).await.unwrap_err().code(),
            "PROFILE_KEY_CONFLICT"
        );
        assert!(
            environment.user_value("VENDOR_API_KEY").unwrap() == text("test-key-external-not-real")
        );
        assert!(environment.user_value("CHANGED_API_KEY").unwrap() == UserValue::Missing);
        assert!(!paths.app_data_dir.exists());
    }

    #[tokio::test]
    async fn verifies_failed_saves_and_keeps_the_recovery_marker_when_outcome_is_unknown() {
        for fault in [
            SaveFault::Stage,
            SaveFault::Commit,
            SaveFault::ReadBack,
            SaveFault::CompetingWriter,
            SaveFault::Abort,
        ] {
            let (_temporary, paths) = temporary_paths();
            write_profile(&paths, "agentrouter", "VENDOR_API_KEY");
            let environment = Arc::new(FakeEnvironment::default());
            let old = if fault == SaveFault::Stage || fault == SaveFault::ReadBack {
                UserValue::Missing
            } else {
                text("test-key-original-not-real")
            };
            environment
                .users
                .lock()
                .unwrap()
                .insert("VENDOR_API_KEY".into(), old.clone());
            let service = service_at(paths.clone(), environment.clone());
            let edit = service.prepare_key_edit("agentrouter").unwrap();
            *environment.failure.lock().unwrap() = Some(fault);
            let error = save(&service, edit).await.unwrap_err();
            let unknown = matches!(fault, SaveFault::CompetingWriter | SaveFault::Abort);
            assert_eq!(
                error.code(),
                if unknown {
                    "ROLLBACK_INCOMPLETE"
                } else {
                    "PROFILE_KEY_WRITE_FAILED"
                }
            );
            assert_eq!(service.list().unwrap().pending_write, unknown);
            let actual = environment.user_value("VENDOR_API_KEY").unwrap();
            assert!(
                actual
                    == if fault == SaveFault::CompetingWriter {
                        text("test-key-external-not-real")
                    } else {
                        old
                    }
            );
            assert!(!*environment.notified.lock().unwrap());
            if unknown {
                let markers = fs::read_dir(paths.app_data_dir.join("profile-env-pending")).unwrap();
                for marker in markers {
                    let metadata = fs::read_to_string(marker.unwrap().path()).unwrap();
                    assert!(!metadata.contains("test-key-"));
                }
            }
        }
    }
}
