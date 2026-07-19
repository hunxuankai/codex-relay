use crate::error::AppError;
use crate::models::settings::AutostartState;
use std::sync::Arc;
use tauri_plugin_autostart::ManagerExt;

pub trait AutostartBackend: Send + Sync {
    fn is_enabled(&self) -> Result<bool, AppError>;
    fn enable(&self) -> Result<(), AppError>;
    fn disable(&self) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct TauriAutostartBackend {
    app: tauri::AppHandle,
}

impl TauriAutostartBackend {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl AutostartBackend for TauriAutostartBackend {
    fn is_enabled(&self) -> Result<bool, AppError> {
        self.app.autolaunch().is_enabled().map_err(|error| {
            AppError::new(
                "AUTOSTART_BACKEND_QUERY_FAILED",
                "无法读取 Windows 开机启动状态。",
                error.to_string(),
            )
        })
    }

    fn enable(&self) -> Result<(), AppError> {
        self.app.autolaunch().enable().map_err(|error| {
            AppError::new(
                "AUTOSTART_BACKEND_ENABLE_FAILED",
                "启用开机启动失败。",
                error.to_string(),
            )
        })
    }

    fn disable(&self) -> Result<(), AppError> {
        self.app.autolaunch().disable().map_err(|error| {
            AppError::new(
                "AUTOSTART_BACKEND_DISABLE_FAILED",
                "关闭开机启动失败。",
                error.to_string(),
            )
        })
    }
}

#[derive(Clone)]
pub struct AutostartService {
    backend: Arc<dyn AutostartBackend>,
}

impl AutostartService {
    pub fn new(backend: Arc<dyn AutostartBackend>) -> Self {
        Self { backend }
    }

    pub fn inspect(&self, configured_enabled: bool) -> Result<AutostartState, AppError> {
        let actual_enabled = self.backend.is_enabled().map_err(|error| {
            AppError::new(
                "AUTOSTART_QUERY_FAILED",
                "无法读取 Windows 开机启动状态。",
                format!("autostart backend query failed with code {}", error.code()),
            )
        })?;
        Ok(AutostartState {
            configured_enabled,
            actual_enabled,
            is_consistent: configured_enabled == actual_enabled,
        })
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<bool, AppError> {
        let current = self.backend.is_enabled().map_err(|error| {
            AppError::new(
                "AUTOSTART_QUERY_FAILED",
                "无法读取 Windows 开机启动状态。",
                format!("autostart backend query failed with code {}", error.code()),
            )
        })?;
        if current != enabled {
            let change = if enabled {
                self.backend.enable()
            } else {
                self.backend.disable()
            };
            change.map_err(|error| {
                AppError::new(
                    if enabled {
                        "AUTOSTART_ENABLE_FAILED"
                    } else {
                        "AUTOSTART_DISABLE_FAILED"
                    },
                    if enabled {
                        "启用开机启动失败。"
                    } else {
                        "关闭开机启动失败。"
                    },
                    format!("autostart backend change failed with code {}", error.code()),
                )
            })?;
        }

        let actual = self.backend.is_enabled().map_err(|error| {
            AppError::new(
                "AUTOSTART_QUERY_FAILED",
                "无法确认 Windows 开机启动状态。",
                format!("autostart verification failed with code {}", error.code()),
            )
        })?;
        if actual != enabled {
            return Err(AppError::new(
                "AUTOSTART_STATE_MISMATCH",
                "Windows 开机启动状态与设置不一致。",
                "autostart backend did not reach requested state",
            ));
        }
        Ok(actual)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeBackend {
        enabled: Mutex<bool>,
        ignore_changes: bool,
        fail_query: bool,
    }

    impl AutostartBackend for FakeBackend {
        fn is_enabled(&self) -> Result<bool, AppError> {
            if self.fail_query {
                Err(AppError::new(
                    "FAKE_QUERY_FAILED",
                    "无法读取开机启动状态。",
                    "injected query failure",
                ))
            } else {
                Ok(*self.enabled.lock().unwrap())
            }
        }

        fn enable(&self) -> Result<(), AppError> {
            if !self.ignore_changes {
                *self.enabled.lock().unwrap() = true;
            }
            Ok(())
        }

        fn disable(&self) -> Result<(), AppError> {
            if !self.ignore_changes {
                *self.enabled.lock().unwrap() = false;
            }
            Ok(())
        }
    }

    #[test]
    fn enable_and_disable_are_verified_against_actual_backend_state() {
        let backend = Arc::new(FakeBackend::default());
        let service = AutostartService::new(backend);

        assert!(service.set_enabled(true).unwrap());
        assert!(service.inspect(true).unwrap().is_consistent);
        assert!(!service.set_enabled(false).unwrap());
        assert!(service.inspect(false).unwrap().is_consistent);
    }

    #[test]
    fn configured_and_actual_state_mismatch_is_reported() {
        let backend = Arc::new(FakeBackend::default());
        *backend.enabled.lock().unwrap() = true;
        let service = AutostartService::new(backend);

        let state = service.inspect(false).unwrap();

        assert!(state.actual_enabled);
        assert!(!state.configured_enabled);
        assert!(!state.is_consistent);
    }

    #[test]
    fn ignored_enable_request_returns_clear_mismatch_error() {
        let backend = Arc::new(FakeBackend {
            ignore_changes: true,
            ..FakeBackend::default()
        });
        let service = AutostartService::new(backend);

        let error = service.set_enabled(true).unwrap_err();

        assert_eq!(error.code(), "AUTOSTART_STATE_MISMATCH");
    }

    #[test]
    fn backend_query_failure_is_mapped_to_safe_error() {
        let backend = Arc::new(FakeBackend {
            fail_query: true,
            ..FakeBackend::default()
        });
        let service = AutostartService::new(backend);

        let error = service.inspect(false).unwrap_err();

        assert_eq!(error.code(), "AUTOSTART_QUERY_FAILED");
        assert!(!format!("{error:?}").contains("injected query failure"));
    }
}
