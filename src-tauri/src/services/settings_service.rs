use crate::error::AppError;
use crate::infrastructure::atomic_file::atomic_write;
use crate::infrastructure::path_service::AppPaths;
use crate::models::settings::Settings;
use crate::services::provider_secret_service::ProviderSecretService;
use chrono::Utc;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const MIN_WINDOW_WIDTH: u32 = 720;
const MIN_WINDOW_HEIGHT: u32 = 520;
const MAX_WINDOW_WIDTH: u32 = 7680;
const MAX_WINDOW_HEIGHT: u32 = 4320;

#[derive(Clone, Debug)]
pub struct SettingsService {
    paths: AppPaths,
    update_lock: Arc<Mutex<()>>,
}

impl SettingsService {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            update_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn bootstrap(&self) -> Result<Settings, AppError> {
        fs::create_dir_all(&self.paths.codex_home).map_err(AppError::from)?;
        fs::create_dir_all(&self.paths.app_data_dir).map_err(AppError::from)?;
        fs::create_dir_all(&self.paths.backups_dir).map_err(AppError::from)?;
        fs::create_dir_all(&self.paths.logs_dir).map_err(AppError::from)?;
        ProviderSecretService::new(self.paths.providers_file.clone()).load_or_create()?;
        self.load_or_create()
    }

    pub fn load_or_create(&self) -> Result<Settings, AppError> {
        let _guard = self.lock_updates()?;
        self.load_or_create_unlocked()
    }

    pub fn save(&self, settings: &Settings) -> Result<(), AppError> {
        let _guard = self.lock_updates()?;
        self.save_unlocked(settings)
    }

    pub fn update(&self, update: impl FnOnce(&mut Settings)) -> Result<Settings, AppError> {
        let _guard = self.lock_updates()?;
        let mut settings = self.load_or_create_unlocked()?;
        update(&mut settings);
        self.save_unlocked(&settings)?;
        self.load_or_create_unlocked()
    }

    fn load_or_create_unlocked(&self) -> Result<Settings, AppError> {
        let bytes = match fs::read(&self.paths.settings_file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                let settings = Settings::default();
                self.save_unlocked(&settings)?;
                return Ok(settings);
            }
            Err(error) => return Err(AppError::from(error)),
        };

        serde_json::from_slice::<Settings>(&bytes).map_err(|error| {
            let backup_result = self.back_up_corrupt_settings();
            match backup_result {
                Ok(_) => AppError::new(
                    "INVALID_SETTINGS_JSON",
                    "无法解析 settings.json。损坏文件已备份。",
                    error.to_string(),
                ),
                Err(backup_error) => backup_error,
            }
        })
    }

    fn save_unlocked(&self, settings: &Settings) -> Result<(), AppError> {
        fs::create_dir_all(&self.paths.app_data_dir).map_err(AppError::from)?;
        let mut normalized = settings.clone();
        normalized.window.width = normalized
            .window
            .width
            .clamp(MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH);
        normalized.window.height = normalized
            .window
            .height
            .clamp(MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT);
        let mut json = serde_json::to_string_pretty(&normalized).map_err(AppError::from)?;
        json.push('\n');
        atomic_write(&self.paths.settings_file, json.as_bytes(), |candidate| {
            serde_json::from_slice::<Settings>(candidate)
                .map(|_| ())
                .map_err(|error| {
                    AppError::new(
                        "INVALID_TEMP_SETTINGS",
                        "临时 settings.json 验证失败。",
                        error.to_string(),
                    )
                })
        })
    }

    fn lock_updates(&self) -> Result<std::sync::MutexGuard<'_, ()>, AppError> {
        self.update_lock.lock().map_err(|_| {
            AppError::new(
                "SETTINGS_LOCK_FAILED",
                "无法更新软件设置。",
                "settings update lock poisoned",
            )
        })
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    fn back_up_corrupt_settings(&self) -> Result<PathBuf, AppError> {
        fs::create_dir_all(&self.paths.app_data_dir).map_err(AppError::from)?;
        let backup = self.paths.app_data_dir.join(format!(
            "settings.json.corrupt-{}-{}",
            Utc::now().format("%Y%m%d-%H%M%S"),
            Uuid::new_v4()
        ));
        fs::copy(&self.paths.settings_file, &backup).map_err(|error| {
            AppError::new(
                "CORRUPT_SETTINGS_BACKUP_FAILED",
                "settings.json 已损坏，并且无法创建损坏文件备份。",
                error.to_string(),
            )
        })?;
        Ok(backup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::path_service::AppPaths;
    use crate::models::settings::{Settings, WindowBounds};
    use std::fs;

    fn create_paths(directory: &tempfile::TempDir) -> AppPaths {
        AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap()
    }

    #[test]
    fn bootstrap_creates_safe_directories_and_default_data_without_fake_config() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let service = SettingsService::new(paths.clone());

        let settings = service.bootstrap().unwrap();

        assert!(paths.codex_home.is_dir());
        assert!(paths.app_data_dir.is_dir());
        assert!(paths.backups_dir.is_dir());
        assert!(paths.logs_dir.is_dir());
        assert!(paths.providers_file.is_file());
        assert!(paths.settings_file.is_file());
        assert!(!paths.config_file.exists());
        assert!(!paths.auth_file.exists());
        assert!(settings.close_to_tray);
        assert!(settings.tray_only_on_autostart);
        assert_eq!(
            fs::read_to_string(&paths.providers_file).unwrap(),
            "{\n  \"version\": 1,\n  \"providers\": {}\n}\n"
        );
    }

    #[test]
    fn settings_round_trip_uses_pretty_json_and_terminal_newline() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let service = SettingsService::new(paths.clone());
        service.bootstrap().unwrap();
        let settings = Settings {
            first_run_completed: true,
            autostart_enabled: true,
            ..Settings::default()
        };

        service.save(&settings).unwrap();
        let loaded = service.load_or_create().unwrap();
        let text = fs::read_to_string(&paths.settings_file).unwrap();

        assert_eq!(loaded, settings);
        assert!(text.contains("  \"firstRunCompleted\": true"));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn invalid_window_bounds_are_normalized_before_save() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        let service = SettingsService::new(paths);
        let settings = Settings {
            window: WindowBounds {
                width: 100,
                height: 80,
                ..WindowBounds::default()
            },
            ..Settings::default()
        };

        service.save(&settings).unwrap();
        let loaded = service.load_or_create().unwrap();

        assert_eq!(loaded.window.width, 720);
        assert_eq!(loaded.window.height, 520);
    }

    #[test]
    fn malformed_settings_are_backed_up_without_overwriting_original() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        let invalid = "{ invalid settings";
        fs::write(&paths.settings_file, invalid).unwrap();
        let service = SettingsService::new(paths.clone());

        let error = service.load_or_create().unwrap_err();

        assert_eq!(error.code(), "INVALID_SETTINGS_JSON");
        assert_eq!(fs::read_to_string(&paths.settings_file).unwrap(), invalid);
        let backups = fs::read_dir(&paths.app_data_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("settings.json.corrupt-")
            })
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read_to_string(backups[0].path()).unwrap(), invalid);
    }
}
