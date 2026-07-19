use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub autostart_enabled: bool,
    pub tray_only_on_autostart: bool,
    pub close_to_tray: bool,
    pub show_window_on_manual_start: bool,
    pub window: WindowBounds,
    pub first_run_completed: bool,
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
        }
    }
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
}
