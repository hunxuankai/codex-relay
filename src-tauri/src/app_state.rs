use crate::error::AppError;
use crate::infrastructure::path_service::AppPaths;
use crate::models::settings::{Settings, SettingsState};
use crate::services::autostart_service::AutostartService;
use crate::services::provider_service::ProviderService;
use crate::services::self_check_service::{CodexCommandProbe, SelfCheckService};
use crate::services::settings_service::SettingsService;
use std::process::Command;
use std::sync::Arc;

pub struct AppState {
    pub paths: AppPaths,
    pub provider_service: ProviderService,
    pub settings_service: SettingsService,
    pub autostart_service: AutostartService,
    pub self_check_service: SelfCheckService,
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
        })
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
        self.settings_service.save(&settings)?;
        self.settings_state()
    }

    pub fn set_autostart(&self, enabled: bool) -> Result<SettingsState, AppError> {
        let actual = self.autostart_service.set_enabled(enabled)?;
        let mut settings = self.settings_service.load_or_create()?;
        settings.autostart_enabled = actual;
        self.settings_service.save(&settings)?;
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
