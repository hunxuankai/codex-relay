use crate::infrastructure::safe_log::format_error_for_log;
use crate::models::provider::ProviderProfile;
use crate::models::settings::{Settings, WindowBounds};
use crate::services::file_watch_service::{ConfigFilesChanged, FileWatchEventSink};
use crate::{app_state::AppState, error::AppError};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, Window, WindowEvent};
use tauri_plugin_notification::NotificationExt;

pub const MENU_OPEN_ID: &str = "open";
pub const MENU_CURRENT_ID: &str = "current";
pub const MENU_SELF_CHECK_ID: &str = "self-check";
pub const MENU_OPEN_DIRECTORY_ID: &str = "open-config-directory";
pub const MENU_AUTOSTART_ID: &str = "autostart";
pub const MENU_EXIT_ID: &str = "exit";
pub const PROVIDER_MENU_PREFIX: &str = "provider:";
pub const TRAY_ID: &str = "codex-relay-tray";
pub const PROVIDERS_CHANGED_EVENT: &str = "providers-changed";
pub const CONFIG_FILES_CHANGED_EVENT: &str = "config-files-changed";
pub const SELF_CHECK_COMPLETED_EVENT: &str = "self-check-completed";
pub const SETTINGS_CHANGED_EVENT: &str = "settings-changed";
pub const APP_NOTIFICATION_EVENT: &str = "app-notification";
const WINDOW_BOUNDS_DEBOUNCE: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub struct TrayRuntime {
    switching: Arc<AtomicBool>,
    exit_requested: AtomicBool,
    bounds_revision: AtomicU64,
}

impl TrayRuntime {
    pub fn try_begin_switch(&self) -> Option<SwitchGuard> {
        self.switching
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| SwitchGuard {
                switching: self.switching.clone(),
            })
    }

    pub fn is_switching(&self) -> bool {
        self.switching.load(Ordering::Acquire)
    }

    pub fn request_exit(&self) {
        self.exit_requested.store(true, Ordering::Release);
    }

    pub fn exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::Acquire)
    }

    fn next_bounds_revision(&self) -> u64 {
        self.bounds_revision.fetch_add(1, Ordering::AcqRel) + 1
    }

    fn bounds_revision_is_current(&self, revision: u64) -> bool {
        self.bounds_revision.load(Ordering::Acquire) == revision
    }
}

pub struct SwitchGuard {
    switching: Arc<AtomicBool>,
}

impl Drop for SwitchGuard {
    fn drop(&mut self) {
        self.switching.store(false, Ordering::Release);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrayMenuItemKind {
    Action,
    Provider,
    Label,
    Separator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrayMenuItemModel {
    pub id: Option<String>,
    pub label: String,
    pub kind: TrayMenuItemKind,
    pub enabled: bool,
    pub checked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrayMenuModel {
    pub items: Vec<TrayMenuItemModel>,
}

impl TrayMenuModel {
    pub fn provider_items(&self) -> impl Iterator<Item = &TrayMenuItemModel> {
        self.items
            .iter()
            .filter(|item| item.kind == TrayMenuItemKind::Provider)
    }
}

pub fn build_tray_menu_model(
    providers: &[ProviderProfile],
    switching: bool,
    autostart_enabled: bool,
) -> TrayMenuModel {
    let current_name = providers
        .iter()
        .find(|provider| provider.is_active)
        .map(|provider| provider.name.as_str())
        .unwrap_or("未选择");
    let mut items = vec![
        action(MENU_OPEN_ID, "打开 Codex Relay", true, false),
        label(MENU_CURRENT_ID, format!("当前：{current_name}")),
        separator(),
    ];

    items.extend(providers.iter().map(|provider| TrayMenuItemModel {
        id: Some(format!("{PROVIDER_MENU_PREFIX}{}", provider.id)),
        label: provider.name.clone(),
        kind: TrayMenuItemKind::Provider,
        enabled: !switching && provider.is_valid && provider.api_key_configured,
        checked: provider.is_active,
    }));
    items.extend([
        separator(),
        action(MENU_SELF_CHECK_ID, "运行自检", true, false),
        action(MENU_OPEN_DIRECTORY_ID, "打开 Codex 配置目录", true, false),
        action(MENU_AUTOSTART_ID, "开机自动启动", true, autostart_enabled),
        separator(),
        action(MENU_EXIT_ID, "退出", true, false),
    ]);

    TrayMenuModel { items }
}

pub fn is_autostart_launch(arguments: &[String]) -> bool {
    arguments.iter().any(|argument| argument == "--autostart")
}

pub fn should_show_window(autostart_launch: bool, settings: &Settings) -> bool {
    if autostart_launch {
        !settings.tray_only_on_autostart
    } else {
        settings.show_window_on_manual_start
    }
}

pub fn window_bounds_intersect_monitors(bounds: &WindowBounds, monitors: &[MonitorRect]) -> bool {
    let (Some(window_x), Some(window_y)) = (bounds.x, bounds.y) else {
        return false;
    };
    let window_right = i64::from(window_x) + i64::from(bounds.width);
    let window_bottom = i64::from(window_y) + i64::from(bounds.height);

    monitors.iter().any(|monitor| {
        let monitor_right = i64::from(monitor.x) + i64::from(monitor.width);
        let monitor_bottom = i64::from(monitor.y) + i64::from(monitor.height);
        i64::from(window_x) < monitor_right
            && window_right > i64::from(monitor.x)
            && i64::from(window_y) < monitor_bottom
            && window_bottom > i64::from(monitor.y)
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppNotification {
    pub level: &'static str,
    pub message: String,
}

pub struct TauriFileWatchSink {
    app: tauri::AppHandle,
}

impl TauriFileWatchSink {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

impl FileWatchEventSink for TauriFileWatchSink {
    fn emit(&self, event: ConfigFilesChanged) -> Result<(), AppError> {
        self.app
            .emit(CONFIG_FILES_CHANGED_EVENT, event)
            .map_err(|error| runtime_error("EVENT_EMIT_FAILED", "无法刷新主界面。", error))?;
        refresh_tray_from_disk(&self.app)
    }
}

pub fn create_initial_tray(app: &tauri::AppHandle) -> Result<(), AppError> {
    let model = build_tray_menu_model(&[], false, false);
    let menu = build_native_menu(app, &model)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Codex Relay")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                let _ = show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder
        .build(app)
        .map(|_| ())
        .map_err(|error| runtime_error("TRAY_CREATE_FAILED", "无法创建系统托盘。", error))
}

pub fn refresh_tray_from_disk(app: &tauri::AppHandle) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let providers = state.provider_service.list_providers()?;
    let autostart_enabled = state
        .settings_state()
        .map(|settings| settings.autostart.actual_enabled)
        .unwrap_or_else(|_| {
            state
                .settings_service
                .load_or_create()
                .map(|settings| settings.autostart_enabled)
                .unwrap_or(false)
        });
    let model = build_tray_menu_model(
        &providers.providers,
        state.tray_runtime.is_switching(),
        autostart_enabled,
    );
    let menu = build_native_menu(app, &model)?;
    let tray = app.tray_by_id(TRAY_ID).ok_or_else(|| {
        AppError::new(
            "TRAY_NOT_FOUND",
            "系统托盘尚未就绪。",
            "managed tray icon is missing",
        )
    })?;
    tray.set_menu(Some(menu))
        .map_err(|error| runtime_error("TRAY_UPDATE_FAILED", "无法更新系统托盘。", error))?;
    app.emit(PROVIDERS_CHANGED_EVENT, providers)
        .map_err(|error| runtime_error("EVENT_EMIT_FAILED", "无法刷新主界面。", error))?;
    Ok(())
}

pub fn after_provider_mutation(
    app: &tauri::AppHandle,
    message: impl Into<String>,
    system_notification: bool,
) {
    let message = message.into();
    tracing::info!("provider mutation completed: {}", message);
    if let Err(error) = refresh_tray_from_disk(app) {
        tracing::warn!("{}", format_error_for_log(&error));
    }
    emit_app_notification(app, "success", message.clone());
    if system_notification
        && let Err(error) = app
            .notification()
            .builder()
            .title("Codex Relay")
            .body(message)
            .show()
    {
        tracing::warn!("notification failed: {}", error);
    }
}

pub fn after_settings_change(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(settings) = state.settings_state() {
        let _ = app.emit(SETTINGS_CHANGED_EVENT, settings);
    }
    if let Err(error) = refresh_tray_from_disk(app) {
        tracing::warn!("{}", format_error_for_log(&error));
    }
}

pub fn show_main_window(app: &tauri::AppHandle) -> Result<(), AppError> {
    let window = if let Some(window) = app.get_webview_window("main") {
        window
    } else {
        let config = app
            .config()
            .app
            .windows
            .iter()
            .find(|config| config.label == "main")
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "WINDOW_CONFIG_MISSING",
                    "无法打开主窗口。",
                    "main window configuration is missing",
                )
            })?;
        tauri::WebviewWindowBuilder::from_config(app, &config)
            .and_then(|builder| builder.build())
            .map_err(|error| runtime_error("WINDOW_CREATE_FAILED", "无法打开主窗口。", error))?
    };
    let _ = window.unminimize();
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|error| runtime_error("WINDOW_SHOW_FAILED", "无法显示主窗口。", error))
}

pub fn restore_window_bounds(app: &tauri::AppHandle, bounds: &WindowBounds) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let monitors = window
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|monitor| MonitorRect {
            x: monitor.position().x,
            y: monitor.position().y,
            width: monitor.size().width,
            height: monitor.size().height,
        })
        .collect::<Vec<_>>();
    if !window_bounds_intersect_monitors(bounds, &monitors) {
        return;
    }
    let _ = window.set_size(PhysicalSize::new(bounds.width, bounds.height));
    if let (Some(x), Some(y)) = (bounds.x, bounds.y) {
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}

pub fn handle_window_event(window: &Window, event: &WindowEvent) {
    if window.label() != "main" {
        return;
    }
    let app = window.app_handle();
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    match event {
        WindowEvent::CloseRequested { api, .. } if !state.tray_runtime.exit_requested() => {
            if state
                .settings_service
                .load_or_create()
                .map(|settings| settings.close_to_tray)
                .unwrap_or(true)
            {
                api.prevent_close();
                let _ = window.hide();
            }
        }
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => schedule_window_bounds_save(window),
        _ => {}
    }
}

pub fn run_startup_checks(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let critical = state.self_check_service.run_critical_checks();
    let _ = app.emit(SELF_CHECK_COMPLETED_EVENT, critical);
    let service = state.self_check_service.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let report = service.run_extended_checks().await;
        let _ = app.emit(SELF_CHECK_COMPLETED_EVENT, report);
    });
}

fn build_native_menu(
    app: &tauri::AppHandle,
    model: &TrayMenuModel,
) -> Result<Menu<tauri::Wry>, AppError> {
    let menu = Menu::new(app)
        .map_err(|error| runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error))?;
    for item in &model.items {
        match item.kind {
            TrayMenuItemKind::Separator => {
                let separator = PredefinedMenuItem::separator(app).map_err(|error| {
                    runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error)
                })?;
                menu.append(&separator).map_err(|error| {
                    runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error)
                })?;
            }
            TrayMenuItemKind::Provider => {
                let native = CheckMenuItem::with_id(
                    app,
                    item.id.as_deref().unwrap_or_default(),
                    &item.label,
                    item.enabled,
                    item.checked,
                    None::<&str>,
                )
                .map_err(|error| runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error))?;
                menu.append(&native).map_err(|error| {
                    runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error)
                })?;
            }
            TrayMenuItemKind::Action if item.id.as_deref() == Some(MENU_AUTOSTART_ID) => {
                let native = CheckMenuItem::with_id(
                    app,
                    MENU_AUTOSTART_ID,
                    &item.label,
                    item.enabled,
                    item.checked,
                    None::<&str>,
                )
                .map_err(|error| runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error))?;
                menu.append(&native).map_err(|error| {
                    runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error)
                })?;
            }
            TrayMenuItemKind::Action | TrayMenuItemKind::Label => {
                let native = MenuItem::with_id(
                    app,
                    item.id.as_deref().unwrap_or_default(),
                    &item.label,
                    item.enabled,
                    None::<&str>,
                )
                .map_err(|error| runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error))?;
                menu.append(&native).map_err(|error| {
                    runtime_error("TRAY_MENU_FAILED", "无法创建托盘菜单。", error)
                })?;
            }
        }
    }
    Ok(menu)
}

fn handle_menu_event(app: &tauri::AppHandle, menu_id: &str) {
    if let Some(provider_id) = menu_id.strip_prefix(PROVIDER_MENU_PREFIX) {
        start_provider_switch(app, provider_id.to_string());
        return;
    }
    match menu_id {
        MENU_OPEN_ID => {
            let _ = show_main_window(app);
        }
        MENU_SELF_CHECK_ID => run_manual_self_check(app),
        MENU_OPEN_DIRECTORY_ID => {
            let state = app.state::<AppState>();
            if let Err(error) = state.open_codex_directory() {
                emit_app_notification(app, "error", error.public_message().to_string());
            }
        }
        MENU_AUTOSTART_ID => toggle_autostart(app),
        MENU_EXIT_ID => {
            let state = app.state::<AppState>();
            state.tray_runtime.request_exit();
            state.shutdown_runtime_services();
            app.exit(0);
        }
        _ => {}
    }
}

fn start_provider_switch(app: &tauri::AppHandle, provider_id: String) {
    let state = app.state::<AppState>();
    let Some(guard) = state.tray_runtime.try_begin_switch() else {
        return;
    };
    let service = state.provider_service.clone();
    let application_write = match state.begin_application_write() {
        Ok(application_write) => application_write,
        Err(error) => {
            drop(guard);
            tracing::warn!("{}", format_error_for_log(&error));
            emit_app_notification(app, "error", error.public_message().to_string());
            return;
        }
    };
    let app = app.clone();
    let _ = refresh_tray_from_disk(&app);
    tauri::async_runtime::spawn(async move {
        let result = service.switch_provider(&provider_id).await;
        drop(application_write);
        drop(guard);
        match result {
            Ok(outcome) => after_provider_mutation(&app, outcome.message, true),
            Err(error) => {
                tracing::warn!("{}", format_error_for_log(&error));
                let _ = refresh_tray_from_disk(&app);
                emit_app_notification(&app, "error", error.public_message().to_string());
            }
        }
    });
}

fn run_manual_self_check(app: &tauri::AppHandle) {
    let service = app.state::<AppState>().self_check_service.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let report = service.run_extended_checks().await;
        let _ = app.emit(SELF_CHECK_COMPLETED_EVENT, report);
        emit_app_notification(&app, "success", "自检已完成。".into());
    });
}

fn toggle_autostart(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let enabled = state
        .settings_state()
        .map(|settings| settings.autostart.actual_enabled)
        .unwrap_or(false);
    match state.set_autostart(!enabled) {
        Ok(_) => {
            after_settings_change(app);
            emit_app_notification(
                app,
                "success",
                if enabled {
                    "已关闭开机自动启动。"
                } else {
                    "已启用开机自动启动。"
                }
                .into(),
            );
        }
        Err(error) => {
            tracing::warn!("{}", format_error_for_log(&error));
            emit_app_notification(app, "error", error.public_message().to_string());
            let _ = refresh_tray_from_disk(app);
        }
    }
}

fn schedule_window_bounds_save(window: &Window) {
    let app = window.app_handle().clone();
    let state = app.state::<AppState>();
    let revision = state.tray_runtime.next_bounds_revision();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(WINDOW_BOUNDS_DEBOUNCE).await;
        let state = app.state::<AppState>();
        if !state.tray_runtime.bounds_revision_is_current(revision) {
            return;
        }
        let Some(window) = app.get_webview_window("main") else {
            return;
        };
        let (Ok(size), Ok(position)) = (window.inner_size(), window.outer_position()) else {
            return;
        };
        let bounds = WindowBounds {
            width: size.width,
            height: size.height,
            x: Some(position.x),
            y: Some(position.y),
        };
        if let Err(error) = state.settings_service.update(|settings| {
            settings.window = bounds;
        }) {
            tracing::warn!("{}", format_error_for_log(&error));
        }
    });
}

fn emit_app_notification(app: &tauri::AppHandle, level: &'static str, message: String) {
    let _ = app.emit(APP_NOTIFICATION_EVENT, AppNotification { level, message });
}

fn runtime_error(
    code: &'static str,
    public_message: &'static str,
    error: impl ToString,
) -> AppError {
    AppError::new(code, public_message, error.to_string())
}

fn action(id: &str, label: &str, enabled: bool, checked: bool) -> TrayMenuItemModel {
    TrayMenuItemModel {
        id: Some(id.into()),
        label: label.into(),
        kind: TrayMenuItemKind::Action,
        enabled,
        checked,
    }
}

fn label(id: &str, label: String) -> TrayMenuItemModel {
    TrayMenuItemModel {
        id: Some(id.into()),
        label,
        kind: TrayMenuItemKind::Label,
        enabled: false,
        checked: false,
    }
}

fn separator() -> TrayMenuItemModel {
    TrayMenuItemModel {
        id: None,
        label: String::new(),
        kind: TrayMenuItemKind::Separator,
        enabled: false,
        checked: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::provider::{ProviderProfile, WireApi};
    use crate::models::settings::{Settings, WindowBounds};

    fn provider(
        id: &str,
        name: &str,
        active: bool,
        valid: bool,
        api_key_configured: bool,
    ) -> ProviderProfile {
        ProviderProfile {
            id: id.into(),
            name: name.into(),
            base_url: format!("https://{id}.example.test/v1"),
            wire_api: WireApi::Responses,
            model: None,
            api_key_configured,
            is_active: active,
            is_valid: valid,
            validation_message: None,
        }
    }

    fn action<'a>(model: &'a TrayMenuModel, id: &str) -> &'a TrayMenuItemModel {
        model
            .items
            .iter()
            .find(|item| item.id.as_deref() == Some(id))
            .expect("tray action should exist")
    }

    fn provider_item<'a>(model: &'a TrayMenuModel, id: &str) -> &'a TrayMenuItemModel {
        action(model, &format!("{PROVIDER_MENU_PREFIX}{id}"))
    }

    #[test]
    fn menu_contains_all_required_commands_and_current_label() {
        let model = build_tray_menu_model(
            &[provider("provider-a", "Provider A", true, true, true)],
            false,
            true,
        );

        assert_eq!(action(&model, MENU_OPEN_ID).label, "打开 Codex Relay");
        assert_eq!(action(&model, MENU_CURRENT_ID).label, "当前：Provider A");
        assert_eq!(action(&model, MENU_SELF_CHECK_ID).label, "运行自检");
        assert_eq!(
            action(&model, MENU_OPEN_DIRECTORY_ID).label,
            "打开 Codex 配置目录"
        );
        assert_eq!(action(&model, MENU_AUTOSTART_ID).label, "开机自动启动");
        assert!(action(&model, MENU_AUTOSTART_ID).checked);
        assert_eq!(action(&model, MENU_EXIT_ID).label, "退出");
    }

    #[test]
    fn active_provider_is_checked() {
        let model = build_tray_menu_model(
            &[
                provider("provider-a", "Provider A", true, true, true),
                provider("provider-b", "Provider B", false, true, true),
            ],
            false,
            false,
        );

        assert!(provider_item(&model, "provider-a").checked);
        assert!(!provider_item(&model, "provider-b").checked);
    }

    #[test]
    fn invalid_or_keyless_providers_are_disabled() {
        let model = build_tray_menu_model(
            &[
                provider("invalid", "Invalid", false, false, true),
                provider("keyless", "Keyless", false, true, false),
                provider("ready", "Ready", false, true, true),
            ],
            false,
            false,
        );

        assert!(!provider_item(&model, "invalid").enabled);
        assert!(!provider_item(&model, "keyless").enabled);
        assert!(provider_item(&model, "ready").enabled);
    }

    #[test]
    fn switching_disables_every_provider_item() {
        let model = build_tray_menu_model(
            &[
                provider("provider-a", "Provider A", true, true, true),
                provider("provider-b", "Provider B", false, true, true),
            ],
            true,
            false,
        );

        assert!(model.provider_items().all(|item| !item.enabled));
    }

    #[test]
    fn rebuilding_after_provider_changes_replaces_items_and_active_label() {
        let initial = build_tray_menu_model(
            &[provider("provider-a", "Provider A", true, true, true)],
            false,
            false,
        );
        let rebuilt = build_tray_menu_model(
            &[provider("provider-b", "Provider B", true, true, true)],
            false,
            false,
        );

        assert!(provider_item(&initial, "provider-a").checked);
        assert_eq!(action(&rebuilt, MENU_CURRENT_ID).label, "当前：Provider B");
        assert!(provider_item(&rebuilt, "provider-b").checked);
        assert!(
            rebuilt
                .items
                .iter()
                .all(|item| item.id.as_deref() != Some("provider:provider-a"))
        );
    }

    #[test]
    fn autostart_flag_is_detected_as_an_exact_argument() {
        assert!(is_autostart_launch(&[
            "CodexRelay.exe".into(),
            "--autostart".into(),
        ]));
        assert!(!is_autostart_launch(&[
            "CodexRelay.exe".into(),
            "--autostart=false".into(),
        ]));
    }

    #[test]
    fn startup_visibility_respects_launch_mode_and_settings() {
        let mut settings = Settings::default();
        assert!(should_show_window(false, &settings));
        assert!(!should_show_window(true, &settings));

        settings.show_window_on_manual_start = false;
        settings.tray_only_on_autostart = false;
        assert!(!should_show_window(false, &settings));
        assert!(should_show_window(true, &settings));
    }

    #[test]
    fn saved_window_bounds_restore_only_when_visible_on_a_monitor() {
        let monitor = MonitorRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let partly_visible = WindowBounds {
            width: 900,
            height: 620,
            x: Some(1800),
            y: Some(900),
        };
        let offscreen = WindowBounds {
            x: Some(2500),
            y: Some(1200),
            ..partly_visible.clone()
        };
        let unpositioned = WindowBounds {
            x: None,
            y: None,
            ..partly_visible.clone()
        };

        assert!(window_bounds_intersect_monitors(
            &partly_visible,
            &[monitor]
        ));
        assert!(!window_bounds_intersect_monitors(&offscreen, &[monitor]));
        assert!(!window_bounds_intersect_monitors(&unpositioned, &[monitor]));
    }

    #[test]
    fn switch_guard_rejects_duplicates_and_releases_busy_state_on_drop() {
        let runtime = TrayRuntime::default();
        let guard = runtime.try_begin_switch().expect("first switch starts");

        assert!(runtime.is_switching());
        assert!(runtime.try_begin_switch().is_none());

        drop(guard);
        assert!(!runtime.is_switching());
        assert!(runtime.try_begin_switch().is_some());
    }

    #[test]
    fn exit_is_requested_only_after_explicit_tray_action() {
        let runtime = TrayRuntime::default();

        assert!(!runtime.exit_requested());
        runtime.request_exit();
        assert!(runtime.exit_requested());
    }
}
