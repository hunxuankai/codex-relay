use crate::infrastructure::file_fingerprint::FileSetFingerprint;
use crate::infrastructure::path_service::AppPaths;
use crate::models::health::{HealthCheck, HealthLevel, HealthReport};
use crate::services::auth_service::AuthService;
use crate::services::autostart_service::AutostartService;
use crate::services::backup_service::BackupService;
use crate::services::config_service;
use crate::services::provider_secret_service::ProviderSecretService;
use crate::services::provider_service::ProviderService;
use crate::services::settings_service::SettingsService;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::future::Future;
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

const CODEX_PROBE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodexProbeResult {
    Detected(String),
    Missing,
    TimedOut,
    Failed(String),
}

pub trait CodexCommandProbe: Send + Sync {
    fn probe(
        &self,
        timeout_duration: Duration,
    ) -> Pin<Box<dyn Future<Output = CodexProbeResult> + Send + '_>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemCodexCommandProbe;

impl CodexCommandProbe for SystemCodexCommandProbe {
    fn probe(
        &self,
        timeout_duration: Duration,
    ) -> Pin<Box<dyn Future<Output = CodexProbeResult> + Send + '_>> {
        Box::pin(async move {
            let mut command = Command::new("codex");
            command.arg("--version").kill_on_drop(true);
            match timeout(timeout_duration, command.output()).await {
                Err(_) => CodexProbeResult::TimedOut,
                Ok(Err(error)) if error.kind() == ErrorKind::NotFound => CodexProbeResult::Missing,
                Ok(Err(error)) => CodexProbeResult::Failed(error.kind().to_string()),
                Ok(Ok(output)) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .chars()
                        .take(200)
                        .collect::<String>();
                    CodexProbeResult::Detected(if version.is_empty() {
                        "已检测".into()
                    } else {
                        version
                    })
                }
                Ok(Ok(output)) => CodexProbeResult::Failed(format!(
                    "exit-code-{}",
                    output.status.code().unwrap_or(-1)
                )),
            }
        })
    }
}

#[derive(Clone)]
pub struct SelfCheckService {
    paths: AppPaths,
    settings_service: SettingsService,
    autostart_service: AutostartService,
    codex_probe: Arc<dyn CodexCommandProbe>,
    backup_service: BackupService,
    app_version: String,
    baseline: Arc<Mutex<Option<FileSetFingerprint>>>,
}

impl SelfCheckService {
    pub fn new(
        paths: AppPaths,
        settings_service: SettingsService,
        autostart_service: AutostartService,
        codex_probe: Arc<dyn CodexCommandProbe>,
        app_version: impl Into<String>,
    ) -> Self {
        let app_version = app_version.into();
        Self {
            backup_service: BackupService::new(paths.backups_dir.clone(), app_version.clone()),
            paths,
            settings_service,
            autostart_service,
            codex_probe,
            app_version,
            baseline: Arc::new(Mutex::new(None)),
        }
    }

    pub fn run_critical_checks(&self) -> HealthReport {
        self.collect_critical_checks(true)
    }

    pub async fn run_extended_checks(&self) -> HealthReport {
        let mut report = self.collect_critical_checks(false);
        self.append_configuration_checks(&mut report.checks);
        self.append_autostart_check(&mut report.checks);
        self.append_transaction_and_backup_checks(&mut report.checks);
        self.append_external_modification_check(&mut report.checks);
        report.checks.push(self.codex_cli_check().await);
        report.level = aggregate_level(&report.checks);
        report.generated_at = Utc::now().to_rfc3339();
        report
    }

    fn collect_critical_checks(&self, record_baseline: bool) -> HealthReport {
        let mut checks = Vec::new();
        match self.settings_service.bootstrap() {
            Ok(_) => checks.push(normal_check(
                "bootstrap",
                "应用目录",
                "应用数据目录和默认文件已就绪。",
            )),
            Err(error) => checks.push(error_check("bootstrap", "应用目录", error.public_message())),
        }
        checks.push(directory_check(
            "codex-home",
            "Codex 配置目录",
            &self.paths.codex_home,
        ));
        checks.push(directory_check(
            "app-data",
            "Codex Relay 数据目录",
            &self.paths.app_data_dir,
        ));
        checks.push(directory_check(
            "backup-directory",
            "备份目录",
            &self.paths.backups_dir,
        ));
        checks.push(directory_check(
            "log-directory",
            "日志目录",
            &self.paths.logs_dir,
        ));
        checks.push(file_presence_check(
            "config-file",
            "config.toml",
            &self.paths.config_file,
            "config.toml 尚不存在，可通过首次引导新增 Provider。",
        ));
        checks.push(file_presence_check(
            "auth-file",
            "auth.json",
            &self.paths.auth_file,
            "auth.json 尚不存在，启用 Provider 时会创建。",
        ));

        let provider_state =
            ProviderService::new(self.paths.clone(), self.app_version.clone()).list_providers();
        let current_provider = match provider_state {
            Ok(state) => {
                checks.push(normal_check(
                    "provider-list",
                    "Provider 列表",
                    &format!("已加载 {} 个 Provider。", state.providers.len()),
                ));
                state.active_provider_id
            }
            Err(error) => {
                checks.push(error_check(
                    "provider-list",
                    "Provider 列表",
                    error.public_message(),
                ));
                None
            }
        };

        if record_baseline
            && let Ok(fingerprint) = FileSetFingerprint::from_paths(
                &self.paths.config_file,
                &self.paths.auth_file,
                &self.paths.providers_file,
            )
        {
            *self
                .baseline
                .lock()
                .expect("self-check baseline lock poisoned") = Some(fingerprint);
        }

        HealthReport {
            level: aggregate_level(&checks),
            checks,
            config_directory: self.paths.codex_home.to_string_lossy().into_owned(),
            current_provider,
            generated_at: Utc::now().to_rfc3339(),
        }
    }

    fn append_configuration_checks(&self, checks: &mut Vec<HealthCheck>) {
        let source = match fs::read_to_string(&self.paths.config_file) {
            Ok(source) => source,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                checks.push(warning_check(
                    "config-parse",
                    "Codex 配置",
                    "config.toml 尚不存在。",
                ));
                return;
            }
            Err(_) => {
                checks.push(error_check(
                    "config-parse",
                    "Codex 配置",
                    "无法读取 config.toml。",
                ));
                return;
            }
        };
        let document = match config_service::parse_document(&source) {
            Ok(document) => {
                checks.push(normal_check(
                    "config-parse",
                    "Codex 配置",
                    "config.toml 格式有效。",
                ));
                document
            }
            Err(error) => {
                checks.push(error_check(
                    "config-parse",
                    "Codex 配置",
                    error.public_message(),
                ));
                return;
            }
        };

        let providers = match config_service::list_provider_configs(&document) {
            Ok(providers) => providers,
            Err(error) => {
                checks.push(error_check(
                    "provider-configs",
                    "Provider 配置",
                    error.public_message(),
                ));
                return;
            }
        };
        let invalid_count = providers
            .iter()
            .filter(|provider| config_service::validate_provider_config(provider).is_err())
            .count();
        if invalid_count == 0 {
            checks.push(normal_check(
                "provider-configs",
                "Provider 配置",
                "所有 Provider 的 Name、Base URL 和 Wire API 均有效。",
            ));
        } else {
            checks.push(error_check(
                "provider-configs",
                "Provider 配置",
                &format!("发现 {invalid_count} 个无效 Provider 配置。"),
            ));
        }

        let active = config_service::current_provider_id(&document);
        match active.as_deref() {
            Some(active) if providers.iter().any(|provider| provider.id == active) => checks.push(
                normal_check("active-provider", "当前 Provider", "当前 Provider 存在。"),
            ),
            Some(_) => checks.push(error_check(
                "active-provider",
                "当前 Provider",
                "顶层 model_provider 不存在于 Provider 列表。",
            )),
            None if providers.is_empty() => checks.push(warning_check(
                "active-provider",
                "当前 Provider",
                "尚未配置 Provider。",
            )),
            None => checks.push(error_check(
                "active-provider",
                "当前 Provider",
                "config.toml 缺少顶层 model_provider。",
            )),
        }

        let store =
            match ProviderSecretService::new(self.paths.providers_file.clone()).load_or_create() {
                Ok(store) => store,
                Err(error) => {
                    checks.push(error_check(
                        "provider-secrets",
                        "Provider API Key",
                        error.public_message(),
                    ));
                    return;
                }
            };
        let auth_key = match AuthService::new(self.paths.auth_file.clone()).read_api_key() {
            Ok(key) => key,
            Err(error) => {
                checks.push(error_check(
                    "auth-json",
                    "Codex 认证",
                    error.public_message(),
                ));
                return;
            }
        };

        if let Some(active) = active {
            let provider_key = store
                .providers
                .get(&active)
                .map(|secret| secret.api_key.as_str())
                .filter(|key| !key.is_empty());
            match provider_key {
                Some(_) => checks.push(normal_check(
                    "current-provider-key",
                    "当前 Provider API Key",
                    "已配置。",
                )),
                None => checks.push(error_check(
                    "current-provider-key",
                    "当前 Provider API Key",
                    "当前 Provider 尚未设置 API Key。",
                )),
            }
            match auth_key.as_deref() {
                Some(_) => checks.push(normal_check(
                    "auth-json",
                    "Codex 认证",
                    "auth.json 包含 OPENAI_API_KEY。",
                )),
                None => checks.push(error_check(
                    "auth-json",
                    "Codex 认证",
                    "auth.json 缺少 OPENAI_API_KEY。",
                )),
            }
            if provider_key.is_some() && auth_key.as_deref() == provider_key {
                checks.push(normal_check(
                    "auth-key-match",
                    "API Key 一致性",
                    "当前认证密钥与 Provider 密钥一致。",
                ));
            } else {
                checks.push(error_check(
                    "auth-key-match",
                    "API Key 一致性",
                    "当前认证密钥与 Provider 密钥不一致。",
                ));
            }
        }
    }

    fn append_autostart_check(&self, checks: &mut Vec<HealthCheck>) {
        let settings = match self.settings_service.load_or_create() {
            Ok(settings) => settings,
            Err(error) => {
                checks.push(error_check("autostart", "开机启动", error.public_message()));
                return;
            }
        };
        match self.autostart_service.inspect(settings.autostart_enabled) {
            Ok(state) if state.is_consistent => checks.push(normal_check(
                "autostart",
                "开机启动",
                if state.actual_enabled {
                    "已启用，设置与 Windows 实际状态一致。"
                } else {
                    "未启用，设置与 Windows 实际状态一致。"
                },
            )),
            Ok(_) => checks.push(warning_check(
                "autostart",
                "开机启动",
                "软件设置与 Windows 实际开机启动状态不一致。",
            )),
            Err(error) => checks.push(warning_check(
                "autostart",
                "开机启动",
                error.public_message(),
            )),
        }
    }

    fn append_transaction_and_backup_checks(&self, checks: &mut Vec<HealthCheck>) {
        if self.paths.app_data_dir.join("transaction.json").exists() {
            checks.push(error_check(
                "transaction-marker",
                "配置事务",
                "检测到失败或未完成的配置写入事务。",
            ));
        } else {
            checks.push(normal_check(
                "transaction-marker",
                "配置事务",
                "没有未完成的配置事务。",
            ));
        }

        match self.backup_service.list_backups() {
            Ok(backups) if backups.len() <= 20 => checks.push(normal_check(
                "backup-count",
                "配置备份",
                &format!("当前有 {} 份事务备份。", backups.len()),
            )),
            Ok(backups) => checks.push(warning_check(
                "backup-count",
                "配置备份",
                &format!("当前有 {} 份备份，超过建议上限 20。", backups.len()),
            )),
            Err(error) => checks.push(error_check(
                "backup-count",
                "配置备份",
                error.public_message(),
            )),
        }
    }

    fn append_external_modification_check(&self, checks: &mut Vec<HealthCheck>) {
        let baseline = self
            .baseline
            .lock()
            .expect("self-check baseline lock poisoned")
            .clone();
        let Some(baseline) = baseline else {
            return;
        };
        match FileSetFingerprint::from_paths(
            &self.paths.config_file,
            &self.paths.auth_file,
            &self.paths.providers_file,
        ) {
            Ok(current) if current == baseline => checks.push(normal_check(
                "external-modification",
                "外部修改",
                "关键自检后配置文件未发生外部变化。",
            )),
            Ok(_) => checks.push(warning_check(
                "external-modification",
                "外部修改",
                "关键自检后配置文件发生了变化。",
            )),
            Err(error) => checks.push(warning_check(
                "external-modification",
                "外部修改",
                error.public_message(),
            )),
        }
    }

    async fn codex_cli_check(&self) -> HealthCheck {
        match self.codex_probe.probe(CODEX_PROBE_TIMEOUT).await {
            CodexProbeResult::Detected(version) => {
                normal_check("codex-cli", "Codex CLI", &format!("已检测：{version}"))
            }
            CodexProbeResult::Missing => warning_check(
                "codex-cli",
                "Codex CLI",
                "未检测到 Codex CLI，但 Provider 配置管理功能仍可使用。",
            ),
            CodexProbeResult::TimedOut => warning_check(
                "codex-cli",
                "Codex CLI",
                "执行 codex --version 超时，但 Provider 配置管理功能仍可使用。",
            ),
            CodexProbeResult::Failed(_) => warning_check(
                "codex-cli",
                "Codex CLI",
                "Codex CLI 检测失败，但 Provider 配置管理功能仍可使用。",
            ),
        }
    }
}

fn directory_check(id: &str, label: &str, path: &Path) -> HealthCheck {
    if !path.is_dir() || fs::read_dir(path).is_err() {
        return error_check(id, label, "目录不存在或不可读。");
    }
    let probe = path.join(format!(".codex-relay-write-probe-{}", Uuid::new_v4()));
    let writable = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
        .and_then(|mut file| file.write_all(b"probe"))
        .is_ok();
    let _ = fs::remove_file(&probe);
    if writable {
        normal_check(id, label, "目录可读写。")
    } else {
        error_check(id, label, "目录不可写。")
    }
}

fn file_presence_check(id: &str, label: &str, path: &Path, missing_message: &str) -> HealthCheck {
    if path.is_file() {
        normal_check(id, label, "文件存在。")
    } else {
        warning_check(id, label, missing_message)
    }
}

fn normal_check(id: &str, label: &str, message: &str) -> HealthCheck {
    check(id, label, HealthLevel::Normal, message)
}

fn warning_check(id: &str, label: &str, message: &str) -> HealthCheck {
    check(id, label, HealthLevel::Warning, message)
}

fn error_check(id: &str, label: &str, message: &str) -> HealthCheck {
    check(id, label, HealthLevel::Error, message)
}

fn check(id: &str, label: &str, level: HealthLevel, message: &str) -> HealthCheck {
    HealthCheck {
        id: id.into(),
        label: label.into(),
        level,
        message: message.into(),
    }
}

fn aggregate_level(checks: &[HealthCheck]) -> HealthLevel {
    if checks.iter().any(|check| check.level == HealthLevel::Error) {
        HealthLevel::Error
    } else if checks
        .iter()
        .any(|check| check.level == HealthLevel::Warning)
    {
        HealthLevel::Warning
    } else {
        HealthLevel::Normal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::infrastructure::path_service::AppPaths;
    use crate::services::autostart_service::{AutostartBackend, AutostartService};
    use crate::services::settings_service::SettingsService;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeAutostartBackend {
        enabled: Mutex<bool>,
    }

    impl AutostartBackend for FakeAutostartBackend {
        fn is_enabled(&self) -> Result<bool, AppError> {
            Ok(*self.enabled.lock().unwrap())
        }

        fn enable(&self) -> Result<(), AppError> {
            *self.enabled.lock().unwrap() = true;
            Ok(())
        }

        fn disable(&self) -> Result<(), AppError> {
            *self.enabled.lock().unwrap() = false;
            Ok(())
        }
    }

    struct FakeCodexProbe {
        result: CodexProbeResult,
        calls: AtomicUsize,
    }

    impl FakeCodexProbe {
        fn new(result: CodexProbeResult) -> Self {
            Self {
                result,
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl CodexCommandProbe for FakeCodexProbe {
        fn probe(
            &self,
            _timeout: std::time::Duration,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = CodexProbeResult> + Send + '_>>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let result = self.result.clone();
            Box::pin(async move { result })
        }
    }

    fn create_paths(directory: &tempfile::TempDir) -> AppPaths {
        AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap()
    }

    fn valid_service(
        probe: Arc<FakeCodexProbe>,
        autostart_enabled: bool,
    ) -> (tempfile::TempDir, AppPaths, SelfCheckService) {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let settings_service = SettingsService::new(paths.clone());
        let mut settings = settings_service.bootstrap().unwrap();
        settings.autostart_enabled = autostart_enabled;
        settings_service.save(&settings).unwrap();
        fs::write(
            &paths.config_file,
            include_str!("../../../fixtures/config-multiple-providers.toml"),
        )
        .unwrap();
        fs::write(
            &paths.auth_file,
            include_str!("../../../fixtures/auth-api-key.json"),
        )
        .unwrap();
        fs::write(
            &paths.providers_file,
            include_str!("../../../fixtures/providers-multiple.json"),
        )
        .unwrap();
        let backend = Arc::new(FakeAutostartBackend::default());
        *backend.enabled.lock().unwrap() = autostart_enabled;
        let service = SelfCheckService::new(
            paths.clone(),
            settings_service,
            AutostartService::new(backend),
            probe,
            "0.1.0",
        );
        (directory, paths, service)
    }

    #[test]
    fn critical_checks_are_local_and_never_invoke_codex_command() {
        let probe = Arc::new(FakeCodexProbe::new(CodexProbeResult::Detected(
            "codex-cli 1.0.0".into(),
        )));
        let (_directory, _paths, service) = valid_service(probe.clone(), false);

        let report = service.run_critical_checks();

        assert_eq!(report.level, HealthLevel::Normal);
        assert_eq!(probe.calls.load(Ordering::SeqCst), 0);
        assert!(report.checks.iter().any(|check| check.id == "codex-home"));
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.id == "provider-list")
        );
    }

    #[tokio::test]
    async fn valid_extended_check_reports_cli_and_consistent_autostart_as_normal() {
        let probe = Arc::new(FakeCodexProbe::new(CodexProbeResult::Detected(
            "codex-cli 1.0.0".into(),
        )));
        let (_directory, _paths, service) = valid_service(probe, false);
        service.run_critical_checks();

        let report = service.run_extended_checks().await;

        assert_eq!(report.level, HealthLevel::Normal);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.id == "codex-cli" && check.level == HealthLevel::Normal)
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.id == "autostart" && check.level == HealthLevel::Normal)
        );
    }

    #[tokio::test]
    async fn missing_or_timed_out_cli_is_only_a_warning() {
        for probe_result in [CodexProbeResult::Missing, CodexProbeResult::TimedOut] {
            let probe = Arc::new(FakeCodexProbe::new(probe_result));
            let (_directory, _paths, service) = valid_service(probe, false);

            let report = service.run_extended_checks().await;

            assert_eq!(report.level, HealthLevel::Warning);
            assert!(
                report
                    .checks
                    .iter()
                    .any(|check| check.id == "codex-cli" && check.level == HealthLevel::Warning)
            );
        }
    }

    #[tokio::test]
    async fn invalid_toml_and_key_mismatch_are_errors_without_secret_leaks() {
        let probe = Arc::new(FakeCodexProbe::new(CodexProbeResult::Detected(
            "codex-cli 1.0.0".into(),
        )));
        let (_directory, paths, service) = valid_service(probe, false);
        fs::write(
            &paths.auth_file,
            "{\n  \"OPENAI_API_KEY\": \"test-key-b-not-real\"\n}\n",
        )
        .unwrap();

        let mismatch = service.run_extended_checks().await;
        let mismatch_json = serde_json::to_string(&mismatch).unwrap();
        assert_eq!(mismatch.level, HealthLevel::Error);
        assert!(!mismatch_json.contains("test-key-a-not-real"));
        assert!(!mismatch_json.contains("test-key-b-not-real"));

        fs::write(&paths.config_file, "model_provider = \"unterminated\n").unwrap();
        let invalid = service.run_extended_checks().await;
        assert_eq!(invalid.level, HealthLevel::Error);
        assert!(
            invalid
                .checks
                .iter()
                .any(|check| check.id == "config-parse" && check.level == HealthLevel::Error)
        );
    }

    #[tokio::test]
    async fn autostart_mismatch_and_transaction_marker_are_reported() {
        let probe = Arc::new(FakeCodexProbe::new(CodexProbeResult::Detected(
            "codex-cli 1.0.0".into(),
        )));
        let (_directory, paths, _service) = valid_service(probe, true);
        fs::write(
            paths.app_data_dir.join("transaction.json"),
            "{\"id\":\"unfinished\"}\n",
        )
        .unwrap();
        // Rebuild service with actual autostart disabled while settings request enabled.
        let settings_service = SettingsService::new(paths.clone());
        let mismatched = SelfCheckService::new(
            paths,
            settings_service,
            AutostartService::new(Arc::new(FakeAutostartBackend::default())),
            Arc::new(FakeCodexProbe::new(CodexProbeResult::Detected(
                "codex-cli 1.0.0".into(),
            ))),
            "0.1.0",
        );

        let report = mismatched.run_extended_checks().await;

        assert_eq!(report.level, HealthLevel::Error);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.id == "autostart" && check.level == HealthLevel::Warning)
        );
        assert!(report.checks.iter().any(|check| {
            check.id == "transaction-marker" && check.level == HealthLevel::Error
        }));
    }
}
