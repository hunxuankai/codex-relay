pub mod backup_commands;
pub mod provider_commands;
pub mod self_check_commands;
pub mod settings_commands;

use crate::error::{AppError, CommandResult};
use serde::Serialize;

pub(crate) fn command_result<T>(result: Result<T, AppError>) -> CommandResult<T>
where
    T: Serialize,
{
    match result {
        Ok(data) => CommandResult::success(data),
        Err(error) => CommandResult::failure(&error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_state::AppState;
    use crate::error::AppError;
    use crate::infrastructure::path_service::AppPaths;
    use crate::models::health::HealthLevel;
    use crate::models::provider::CreateProviderInput;
    use crate::services::autostart_service::{AutostartBackend, AutostartService};
    use crate::services::self_check_service::{CodexCommandProbe, CodexProbeResult};
    use std::fs;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeAutostartBackend {
        enabled: Mutex<bool>,
    }

    impl AutostartBackend for FakeAutostartBackend {
        fn is_enabled(&self) -> Result<bool, AppError> {
            Ok(*self.enabled.lock().unwrap())
        }

        fn enable(&self) -> Result<(), AppError> {
            *self.enabled.lock().unwrap() = true;
            Ok(())
        }

        fn disable(&self) -> Result<(), AppError> {
            *self.enabled.lock().unwrap() = false;
            Ok(())
        }
    }

    struct FakeCodexProbe;

    impl CodexCommandProbe for FakeCodexProbe {
        fn probe(
            &self,
            _timeout: std::time::Duration,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = CodexProbeResult> + Send + '_>>
        {
            Box::pin(async { CodexProbeResult::Detected("codex-cli 1.0.0".into()) })
        }
    }

    fn create_state() -> (tempfile::TempDir, AppState) {
        let directory = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap();
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        fs::write(
            &paths.config_file,
            include_str!("../../../fixtures/config-multiple-providers.toml"),
        )
        .unwrap();
        fs::write(
            &paths.auth_file,
            include_str!("../../../fixtures/auth-api-key.json"),
        )
        .unwrap();
        fs::write(
            &paths.providers_file,
            include_str!("../../../fixtures/providers-multiple.json"),
        )
        .unwrap();
        let autostart = AutostartService::new(Arc::new(FakeAutostartBackend::default()));
        let state = AppState::new(paths, "0.1.0", autostart, Arc::new(FakeCodexProbe)).unwrap();
        (directory, state)
    }

    #[test]
    fn provider_list_command_uses_uniform_result_without_secrets() {
        let (_directory, state) = create_state();

        let result = provider_commands::list_providers_inner(&state);
        let json = serde_json::to_string(&result).unwrap();

        assert!(result.success);
        assert!(result.error.is_none());
        assert!(!json.contains("test-key-a-not-real"));
        assert!(!json.contains("test-key-b-not-real"));
        assert!(!json.contains("\"apiKey\":"));
    }

    #[tokio::test]
    async fn invalid_create_command_returns_safe_code_without_stack() {
        let (_directory, state) = create_state();
        let current = state.provider_service.list_providers().unwrap();
        let input = CreateProviderInput {
            id: "invalid.id".into(),
            name: "Provider".into(),
            base_url: "https://example.com/v1".into(),
            wire_api: "responses".into(),
            model: None,
            api_key: "test-key-command-not-real".into(),
            activate_after_save: false,
            expected_files: current.fingerprints,
        };

        let result = provider_commands::create_provider_inner(&state, input).await;
        let json = serde_json::to_string(&result).unwrap();

        assert!(!result.success);
        assert_eq!(result.error.unwrap().code, "INVALID_PROVIDER_ID");
        assert!(!json.contains("test-key-command-not-real"));
        assert!(!json.to_lowercase().contains("backtrace"));
    }

    #[test]
    fn settings_command_reports_actual_autostart_state() {
        let (_directory, state) = create_state();

        let initial = settings_commands::get_settings_inner(&state);
        assert!(!initial.data.unwrap().autostart.actual_enabled);

        let enabled = settings_commands::set_autostart_inner(&state, true);
        let data = enabled.data.unwrap();
        assert!(data.settings.autostart_enabled);
        assert!(data.autostart.actual_enabled);
        assert!(data.autostart.is_consistent);
    }

    #[test]
    fn critical_self_check_command_does_not_require_async_runtime() {
        let (_directory, state) = create_state();

        let result = self_check_commands::run_critical_self_check_inner(&state);

        assert!(result.success);
        assert_eq!(result.data.unwrap().level, HealthLevel::Normal);
    }

    #[tokio::test]
    async fn backup_commands_list_transaction_backups_after_switch() {
        let (_directory, state) = create_state();
        provider_commands::switch_provider_inner(&state, "provider-b".into()).await;

        let backups = backup_commands::list_backups_inner(&state);

        assert!(backups.success);
        assert_eq!(backups.data.unwrap().len(), 1);
    }
}
