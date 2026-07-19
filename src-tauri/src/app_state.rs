use crate::error::AppError;
use crate::infrastructure::path_service::AppPaths;
use crate::infrastructure::safe_log::LogGuard;
use crate::models::settings::{Settings, SettingsState};
use crate::services::autostart_service::AutostartService;
use crate::services::file_watch_service::{ApplicationWriteGuard, FileWatchService};
use crate::services::provider_service::ProviderService;
use crate::services::self_check_service::{CodexCommandProbe, SelfCheckService};
use crate::services::settings_service::SettingsService;
use crate::tray::TrayRuntime;
use std::process::Command;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub paths: AppPaths,
    pub provider_service: ProviderService,
    pub settings_service: SettingsService,
    pub autostart_service: AutostartService,
    pub self_check_service: SelfCheckService,
    pub tray_runtime: TrayRuntime,
    file_watch_service: Mutex<Option<FileWatchService>>,
    log_guard: Mutex<Option<LogGuard>>,
}

impl AppState {
    pub fn new(
        paths: AppPaths,
        app_version: impl Into<String>,
        autostart_service: AutostartService,
        codex_probe: Arc<dyn CodexCommandProbe>,
    ) -> Result<Self, AppError> {
        let app_version = app_version.into();
        let settings_service = SettingsService::new(paths.clone());
        settings_service.bootstrap()?;
        let provider_service = ProviderService::new(paths.clone(), app_version.clone());
        let self_check_service = SelfCheckService::new(
            paths.clone(),
            settings_service.clone(),
            autostart_service.clone(),
            codex_probe,
            app_version,
        );
        Ok(Self {
            paths,
            provider_service,
            settings_service,
            autostart_service,
            self_check_service,
            tray_runtime: TrayRuntime::default(),
            file_watch_service: Mutex::new(None),
            log_guard: Mutex::new(None),
        })
    }

    pub fn install_file_watch_service(&self, service: FileWatchService) -> Result<(), AppError> {
        *self.file_watch_service.lock().map_err(|_| {
            AppError::new(
                "FILE_WATCH_STATE_FAILED",
                "无法保存配置文件监控状态。",
                "file-watch service lock poisoned",
            )
        })? = Some(service);
        Ok(())
    }

    pub fn hold_log_guard(&self, guard: LogGuard) -> Result<(), AppError> {
        *self.log_guard.lock().map_err(|_| {
            AppError::new(
                "LOG_STATE_FAILED",
                "无法保存软件日志状态。",
                "log guard lock poisoned",
            )
        })? = Some(guard);
        Ok(())
    }

    pub fn begin_application_write(&self) -> Result<Option<ApplicationWriteGuard>, AppError> {
        let service = self.file_watch_service.lock().map_err(|_| {
            AppError::new(
                "FILE_WATCH_STATE_FAILED",
                "无法更新配置文件监控状态。",
                "file-watch service lock poisoned",
            )
        })?;
        service
            .as_ref()
            .map(FileWatchService::begin_application_write)
            .transpose()
    }

    pub fn shutdown_runtime_services(&self) {
        let file_watch = self
            .file_watch_service
            .lock()
            .ok()
            .and_then(|mut service| service.take());
        drop(file_watch);
        let log_guard = self
            .log_guard
            .lock()
            .ok()
            .and_then(|mut guard| guard.take());
        drop(log_guard);
    }

    pub fn settings_state(&self) -> Result<SettingsState, AppError> {
        let settings = self.settings_service.load_or_create()?;
        let autostart = self.autostart_service.inspect(settings.autostart_enabled)?;
        Ok(SettingsState {
            settings,
            autostart,
        })
    }

    pub fn save_settings(&self, mut settings: Settings) -> Result<SettingsState, AppError> {
        let actual = self
            .autostart_service
            .inspect(settings.autostart_enabled)?
            .actual_enabled;
        settings.autostart_enabled = actual;
        self.settings_service.update(|current| {
            current.autostart_enabled = settings.autostart_enabled;
            current.tray_only_on_autostart = settings.tray_only_on_autostart;
            current.close_to_tray = settings.close_to_tray;
            current.show_window_on_manual_start = settings.show_window_on_manual_start;
            current.first_run_completed = settings.first_run_completed;
        })?;
        self.settings_state()
    }

    pub fn set_autostart(&self, enabled: bool) -> Result<SettingsState, AppError> {
        let actual = self.autostart_service.set_enabled(enabled)?;
        self.settings_service
            .update(|settings| settings.autostart_enabled = actual)?;
        self.settings_state()
    }

    pub fn open_codex_directory(&self) -> Result<(), AppError> {
        Command::new("explorer.exe")
            .arg(&self.paths.codex_home)
            .spawn()
            .map(|_| ())
            .map_err(|error| {
                AppError::new(
                    "OPEN_DIRECTORY_FAILED",
                    "无法打开 Codex 配置目录。",
                    error.to_string(),
                )
            })
    }
}
