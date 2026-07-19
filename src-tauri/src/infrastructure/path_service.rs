use crate::error::AppError;
use std::env;
use std::path::{Path, PathBuf};

const RELAY_CODEX_HOME: &str = "CODEX_RELAY_CODEX_HOME";
const CODEX_HOME: &str = "CODEX_HOME";
const RELAY_APP_DATA: &str = "CODEX_RELAY_APP_DATA_DIR";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathMode {
    Production,
    Test,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppPaths {
    pub codex_home: PathBuf,
    pub config_file: PathBuf,
    pub auth_file: PathBuf,
    pub app_data_dir: PathBuf,
    pub providers_file: PathBuf,
    pub settings_file: PathBuf,
    pub backups_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl AppPaths {
    fn from_roots(codex_home: PathBuf, app_data_dir: PathBuf) -> Self {
        Self {
            config_file: codex_home.join("config.toml"),
            auth_file: codex_home.join("auth.json"),
            providers_file: app_data_dir.join("providers.json"),
            settings_file: app_data_dir.join("settings.json"),
            backups_dir: app_data_dir.join("backups"),
            logs_dir: app_data_dir.join("logs"),
            codex_home,
            app_data_dir,
        }
    }

    pub fn for_test(codex_home: PathBuf, app_data_dir: PathBuf) -> Result<Self, AppError> {
        ensure_test_paths_are_safe(&codex_home, &app_data_dir)?;
        Ok(Self::from_roots(codex_home, app_data_dir))
    }
}

pub fn resolve_paths(mode: PathMode) -> Result<AppPaths, AppError> {
    let codex_home = resolve_codex_home(mode)?;
    let app_data_dir = resolve_app_data_dir(mode)?;

    match mode {
        PathMode::Production => Ok(AppPaths::from_roots(codex_home, app_data_dir)),
        PathMode::Test => AppPaths::for_test(codex_home, app_data_dir),
    }
}

fn resolve_codex_home(mode: PathMode) -> Result<PathBuf, AppError> {
    if let Some(path) = non_empty_env_path(RELAY_CODEX_HOME) {
        return Ok(path);
    }

    if mode == PathMode::Test {
        return Err(AppError::new(
            "TEST_PATH_OVERRIDE_REQUIRED",
            "测试必须设置 CODEX_RELAY_CODEX_HOME。",
            "test path resolution attempted without CODEX_RELAY_CODEX_HOME",
        ));
    }

    if let Some(path) = non_empty_env_path(CODEX_HOME) {
        return Ok(path);
    }

    let profile = required_env_path("USERPROFILE", "无法确定 Windows 用户目录。")?;
    Ok(profile.join(".codex"))
}

fn resolve_app_data_dir(mode: PathMode) -> Result<PathBuf, AppError> {
    if let Some(path) = non_empty_env_path(RELAY_APP_DATA) {
        return Ok(path);
    }

    if mode == PathMode::Test {
        return Err(AppError::new(
            "TEST_PATH_OVERRIDE_REQUIRED",
            "测试必须设置 CODEX_RELAY_APP_DATA_DIR。",
            "test path resolution attempted without CODEX_RELAY_APP_DATA_DIR",
        ));
    }

    let local_app_data = required_env_path("LOCALAPPDATA", "无法确定 Windows 应用数据目录。")?;
    Ok(local_app_data.join("CodexRelay"))
}

fn non_empty_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn required_env_path(name: &str, public_message: &str) -> Result<PathBuf, AppError> {
    non_empty_env_path(name).ok_or_else(|| {
        AppError::new(
            "REQUIRED_ENVIRONMENT_MISSING",
            public_message,
            format!("required environment variable {name} is missing or empty"),
        )
    })
}

fn ensure_test_paths_are_safe(codex_home: &Path, app_data_dir: &Path) -> Result<(), AppError> {
    let real_codex = non_empty_env_path("USERPROFILE").map(|path| path.join(".codex"));
    let real_app_data = non_empty_env_path("LOCALAPPDATA").map(|path| path.join("CodexRelay"));

    if real_codex
        .as_deref()
        .is_some_and(|real| is_same_or_descendant(codex_home, real))
        || real_app_data
            .as_deref()
            .is_some_and(|real| is_same_or_descendant(app_data_dir, real))
    {
        return Err(AppError::new(
            "UNSAFE_TEST_PATH",
            "测试路径不能指向真实 Codex 或 Codex Relay 数据目录。",
            "test path resolved to a protected real user data directory",
        ));
    }

    Ok(())
}

fn is_same_or_descendant(candidate: &Path, protected: &Path) -> bool {
    let candidate = normalized_path(candidate);
    let protected = normalized_path(protected);
    let protected_prefix = format!("{protected}\\");
    candidate == protected || candidate.starts_with(&protected_prefix)
}

fn normalized_path(path: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };

    absolute
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

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

    #[test]
    #[serial]
    fn relay_override_wins_over_codex_home() {
        let relay = tempfile::tempdir().unwrap();
        let codex = tempfile::tempdir().unwrap();
        let app_data = tempfile::tempdir().unwrap();
        let _guard = EnvGuard::set(&[
            ("CODEX_RELAY_CODEX_HOME", Some(relay.path())),
            ("CODEX_HOME", Some(codex.path())),
            ("CODEX_RELAY_APP_DATA_DIR", Some(app_data.path())),
        ]);

        let paths = resolve_paths(PathMode::Production).unwrap();

        assert_eq!(paths.codex_home, relay.path());
    }

    #[test]
    #[serial]
    fn codex_home_wins_over_user_profile_default() {
        let codex = tempfile::tempdir().unwrap();
        let profile = tempfile::tempdir().unwrap();
        let app_data = tempfile::tempdir().unwrap();
        let _guard = EnvGuard::set(&[
            ("CODEX_RELAY_CODEX_HOME", None),
            ("CODEX_HOME", Some(codex.path())),
            ("USERPROFILE", Some(profile.path())),
            ("CODEX_RELAY_APP_DATA_DIR", Some(app_data.path())),
        ]);

        let paths = resolve_paths(PathMode::Production).unwrap();

        assert_eq!(paths.codex_home, codex.path());
    }

    #[test]
    #[serial]
    fn default_codex_home_uses_user_profile_dot_codex() {
        let profile = tempfile::tempdir().unwrap();
        let app_data = tempfile::tempdir().unwrap();
        let _guard = EnvGuard::set(&[
            ("CODEX_RELAY_CODEX_HOME", None),
            ("CODEX_HOME", None),
            ("USERPROFILE", Some(profile.path())),
            ("CODEX_RELAY_APP_DATA_DIR", Some(app_data.path())),
        ]);

        let paths = resolve_paths(PathMode::Production).unwrap();

        assert_eq!(paths.codex_home, profile.path().join(".codex"));
    }

    #[test]
    #[serial]
    fn application_data_override_wins() {
        let codex = tempfile::tempdir().unwrap();
        let app_data = tempfile::tempdir().unwrap();
        let local = tempfile::tempdir().unwrap();
        let _guard = EnvGuard::set(&[
            ("CODEX_RELAY_CODEX_HOME", Some(codex.path())),
            ("CODEX_RELAY_APP_DATA_DIR", Some(app_data.path())),
            ("LOCALAPPDATA", Some(local.path())),
        ]);

        let paths = resolve_paths(PathMode::Production).unwrap();

        assert_eq!(paths.app_data_dir, app_data.path());
    }

    #[test]
    #[serial]
    fn default_application_data_uses_local_app_data() {
        let codex = tempfile::tempdir().unwrap();
        let local = tempfile::tempdir().unwrap();
        let _guard = EnvGuard::set(&[
            ("CODEX_RELAY_CODEX_HOME", Some(codex.path())),
            ("CODEX_RELAY_APP_DATA_DIR", None),
            ("LOCALAPPDATA", Some(local.path())),
        ]);

        let paths = resolve_paths(PathMode::Production).unwrap();

        assert_eq!(paths.app_data_dir, local.path().join("CodexRelay"));
    }

    #[test]
    #[serial]
    fn test_constructor_rejects_real_user_directories() {
        let profile = PathBuf::from(std::env::var_os("USERPROFILE").unwrap());
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());

        let result = AppPaths::for_test(profile.join(".codex"), local.join("CodexRelay"));

        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_mode_requires_both_relay_overrides() {
        let profile = tempfile::tempdir().unwrap();
        let local = tempfile::tempdir().unwrap();
        let _guard = EnvGuard::set(&[
            ("CODEX_RELAY_CODEX_HOME", None),
            ("CODEX_RELAY_APP_DATA_DIR", None),
            ("CODEX_HOME", None),
            ("USERPROFILE", Some(profile.path())),
            ("LOCALAPPDATA", Some(local.path())),
        ]);

        let error = resolve_paths(PathMode::Test).unwrap_err();

        assert_eq!(error.code(), "TEST_PATH_OVERRIDE_REQUIRED");
    }

    #[test]
    fn derived_file_paths_use_resolved_roots() {
        let codex = tempfile::tempdir().unwrap();
        let app_data = tempfile::tempdir().unwrap();

        let paths = AppPaths::for_test(codex.path().into(), app_data.path().into()).unwrap();

        assert_eq!(paths.config_file, codex.path().join("config.toml"));
        assert_eq!(paths.auth_file, codex.path().join("auth.json"));
        assert_eq!(paths.providers_file, app_data.path().join("providers.json"));
        assert_eq!(paths.settings_file, app_data.path().join("settings.json"));
        assert_eq!(paths.backups_dir, app_data.path().join("backups"));
        assert_eq!(paths.logs_dir, app_data.path().join("logs"));
    }
}
