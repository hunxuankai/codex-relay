use crate::error::AppError;
use crate::infrastructure::path_service::AppPaths;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use zeroize::Zeroizing;

#[cfg(windows)]
mod windows;

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum UserValue {
    Missing,
    Text(Zeroizing<Vec<u8>>),
    Unsupported,
}

impl UserValue {
    pub fn text(value: &str) -> Self {
        let mut bytes = Zeroizing::new(Vec::with_capacity((value.len() + 1) * 2));
        bytes.extend(value.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
        Self::Text(bytes)
    }

    pub fn fingerprint(&self) -> [u8; 32] {
        let mut fingerprint = Sha256::new();
        match self {
            Self::Missing => fingerprint.update([0]),
            Self::Text(bytes) => {
                fingerprint.update([1]);
                fingerprint.update(bytes.as_slice());
            }
            Self::Unsupported => fingerprint.update([2]),
        }
        fingerprint.finalize().into()
    }
}

pub(crate) trait UserEnvironment: Send + Sync {
    fn user_value(&self, name: &str) -> Result<UserValue, AppError>;
    fn system_has_value(&self, name: &str) -> Result<bool, AppError>;
    fn process_has_value(&self, name: &str) -> Result<bool, AppError>;
    fn begin(&self) -> Result<Box<dyn UserEnvironmentTransaction + '_>, AppError>;
    fn notify_changed(&self) -> bool;
}

pub(crate) trait UserEnvironmentTransaction {
    fn read(&self, name: &str) -> Result<UserValue, AppError>;
    fn stage(&mut self, name: &str, value: &UserValue) -> Result<(), AppError>;
    fn commit(self: Box<Self>) -> Result<(), AppError>;
    fn roll_back(self: Box<Self>) -> Result<(), AppError>;
}

pub(crate) fn desktop_environment(paths: &AppPaths) -> Arc<dyn UserEnvironment> {
    #[cfg(windows)]
    if paths.permits_native_user_environment() {
        return Arc::new(windows::WindowsUserEnvironment);
    }
    let _ = paths;
    Arc::new(UnavailableEnvironment)
}

pub(crate) fn open_environment_settings() -> Result<(), AppError> {
    #[cfg(windows)]
    if windows::open_settings() {
        return Ok(());
    }
    Err(AppError::new(
        "PROFILE_ENVIRONMENT_SETTINGS_FAILED",
        "无法打开环境变量设置。可在 Windows 开始菜单搜索「编辑帐户的环境变量」。",
        "Windows environment settings could not be opened",
    ))
}

struct UnavailableEnvironment;

impl UserEnvironment for UnavailableEnvironment {
    fn user_value(&self, _name: &str) -> Result<UserValue, AppError> {
        Err(unavailable())
    }

    fn system_has_value(&self, _name: &str) -> Result<bool, AppError> {
        Err(unavailable())
    }

    fn process_has_value(&self, _name: &str) -> Result<bool, AppError> {
        Err(unavailable())
    }

    fn begin(&self) -> Result<Box<dyn UserEnvironmentTransaction + '_>, AppError> {
        Err(unavailable())
    }

    fn notify_changed(&self) -> bool {
        false
    }
}

pub(crate) fn unavailable() -> AppError {
    AppError::new(
        "PROFILE_ENVIRONMENT_UNAVAILABLE",
        "无法访问 Windows 用户环境变量。",
        "user environment is unavailable for the current application mode",
    )
}
