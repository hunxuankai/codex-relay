use crate::error::AppError;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

impl Default for WindowBounds {
    fn default() -> Self {
        Self {
            width: 900,
            height: 620,
            x: None,
            y: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NetworkProxySettings {
    pub enabled: bool,
    pub url: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub autostart_enabled: bool,
    pub tray_only_on_autostart: bool,
    pub close_to_tray: bool,
    pub show_window_on_manual_start: bool,
    pub window: WindowBounds,
    pub first_run_completed: bool,
    pub network_proxy: NetworkProxySettings,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutostartState {
    pub configured_enabled: bool,
    pub actual_enabled: bool,
    pub is_consistent: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsState {
    pub settings: Settings,
    pub autostart: AutostartState,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            autostart_enabled: false,
            tray_only_on_autostart: true,
            close_to_tray: true,
            show_window_on_manual_start: true,
            window: WindowBounds::default(),
            first_run_completed: false,
            network_proxy: NetworkProxySettings::default(),
        }
    }
}

pub fn normalize_network_proxy_url(input: &str) -> Result<String, AppError> {
    let parsed = Url::parse(input.trim()).map_err(|_| invalid_proxy_url())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || !matches!(parsed.path(), "" | "/")
    {
        return Err(invalid_proxy_url());
    }
    Ok(parsed.origin().ascii_serialization())
}

fn invalid_proxy_url() -> AppError {
    AppError::new(
        "INVALID_PROXY_URL",
        "代理地址必须是无认证的 HTTP 或 HTTPS 地址，例如 http://127.0.0.1:7890。",
        "invalid network proxy URL",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_defaults_keep_application_in_tray() {
        let settings = Settings::default();

        assert!(settings.close_to_tray);
        assert!(settings.show_window_on_manual_start);
        assert!(settings.tray_only_on_autostart);
        assert!(!settings.first_run_completed);
    }

    #[test]
    fn legacy_settings_default_to_disabled_network_proxy() {
        let settings: Settings = serde_json::from_str(
            r#"{"autostartEnabled":false,"trayOnlyOnAutostart":true,"closeToTray":true,"showWindowOnManualStart":true,"window":{"width":900,"height":620,"x":null,"y":null},"firstRunCompleted":true}"#,
        )
        .unwrap();

        assert_eq!(settings.network_proxy, NetworkProxySettings::default());
    }

    #[test]
    fn validates_and_normalizes_uncredentialed_http_proxy_urls() {
        assert_eq!(
            normalize_network_proxy_url("  http://127.0.0.1:7897/  ").unwrap(),
            "http://127.0.0.1:7897"
        );
        assert_eq!(
            normalize_network_proxy_url("https://proxy.example.test:8443").unwrap(),
            "https://proxy.example.test:8443"
        );

        for invalid in [
            "127.0.0.1:7890",
            "socks5://127.0.0.1:1080",
            "http://user@127.0.0.1:7890",
            "http://user:password@127.0.0.1:7890",
            "http://127.0.0.1:7890/path",
            "http://127.0.0.1:7890?token=value",
            "http://127.0.0.1:7890#fragment",
        ] {
            assert!(normalize_network_proxy_url(invalid).is_err(), "{invalid}");
        }
    }
}
