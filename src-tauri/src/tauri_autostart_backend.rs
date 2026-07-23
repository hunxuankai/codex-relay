use crate::error::AppError;
use crate::services::autostart_service::AutostartBackend;
use tauri_plugin_autostart::ManagerExt;

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
