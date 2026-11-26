use crate::error::AppError;
use crate::infrastructure::path_service::AppPaths;
use crate::infrastructure::safe_log::redact;
use crate::infrastructure::user_environment::{
    UserEnvironment, UserValue, desktop_environment, open_environment_settings,
};
use crate::models::cli_profile::{
    CliProfile, CliProfileList, KeyRestriction, ProfileKeyEdit, ProfileKeySaved, ProfileKeyStatus,
    SaveProfileKeyInput, VariablePresence,
};
use crate::services::cli_profile_catalog::{CliProfileCatalog, ProfileConfig};
use crate::services::transaction_service::{TransactionService, profile_key_conflict};
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
    variable: String,
    opened: Instant,
    catalog_fingerprint: [u8; 32],
    variable_fingerprint: [u8; 32],
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
        };
        *self.pending_edit.lock().map_err(|_| expired_edit())? = Some(EditBaseline {
            token: edit.token.clone(),
            profile_name: edit.profile_name.clone(),
            variable: edit.variable.clone(),
            opened: Instant::now(),
            catalog_fingerprint: catalog.fingerprint,
            variable_fingerprint: current.fingerprint(),
        });
        Ok(edit)
    }

    pub async fn save_key(&self, input: SaveProfileKeyInput) -> Result<ProfileKeySaved, AppError> {
        let key = Zeroizing::new(input.new_key);
        let baseline = {
            let mut pending = self.pending_edit.lock().map_err(|_| expired_edit())?;
            if !pending.as_ref().is_some_and(|edit| {
                edit.token == input.token
                    && edit.profile_name == input.profile_name
                    && edit.opened.elapsed() < EDIT_TTL
            }) {
                return Err(expired_edit());
            }
            pending.take().ok_or_else(expired_edit)?
        };
        if key.is_empty()
            || key.len() > MAX_KEY_LENGTH
            || key.trim() != key.as_str()
            || key.chars().any(char::is_control)
        {
            return Err(AppError::new(
                "INVALID_PROFILE_KEY",
                "Key 不能为空、包含控制字符或首尾空白，且不能超过 4096 字节。",
                "profile key input rejected",
            ));
        }
        let replacement = UserValue::text(&key);
        drop(key);
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
                    Ok(())
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
                key_status: self.key_status(&catalog, profile),
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
    ) -> Option<ProfileKeyStatus> {
        if profile.problem.is_some() {
            return None;
        }
        let variable = profile.variable.as_deref()?;
        let user_value = self.environment.user_value(variable);
        let shared_with = shared_references(catalog, profile, variable);
        let restriction = key_restriction(&*self.environment, variable, &shared_with, &user_value);
        Some(ProfileKeyStatus {
            user: match user_value {
                Ok(UserValue::Text(bytes)) if bytes.len() > 2 => VariablePresence::Set,
                Ok(UserValue::Text(_) | UserValue::Missing) => VariablePresence::NotDetected,
                Ok(UserValue::Unsupported) | Err(_) => VariablePresence::Unavailable,
            },
            relay_process: match self.environment.process_has_value(variable) {
                Ok(true) => VariablePresence::Set,
                Ok(false) => VariablePresence::NotDetected,
                Err(_) => VariablePresence::Unavailable,
            },
            restriction,
            shared_with,
        })
    }
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
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;

    #[derive(Default)]
    struct FakeEnvironment {
        users: Mutex<BTreeMap<String, UserValue>>,
        machine: BTreeSet<String>,
        process: BTreeSet<String>,
        failure: Mutex<Option<SaveFault>>,
        fail_next_read: Mutex<bool>,
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
