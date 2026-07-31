pub mod backup_commands;
pub mod provider_availability_commands;
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
    use crate::models::backup::BackupFileName;
    use crate::models::health::HealthLevel;
    use crate::models::provider::{
        ApplyProviderConnectionInput, CreateProviderInput, ReorderProvidersInput,
        RestoreProviderConnectionInput, UpdateProviderFastInput,
    };
    use crate::models::settings::Settings;
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
        fs::write(
            &paths.provider_preferences_file,
            include_str!("../../../fixtures/provider-preferences-multiple.json"),
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
    async fn reorder_provider_command_returns_safe_success_and_order() {
        let (_directory, state) = create_state();
        let current = state.provider_service.list_providers().unwrap();
        let result = provider_commands::reorder_providers_inner(
            &state,
            ReorderProvidersInput {
                provider_ids: vec!["provider-b".into(), "provider-a".into()],
                expected_files: current.fingerprints,
            },
        )
        .await;

        assert!(result.success);
        let ids = result
            .data
            .unwrap()
            .providers
            .into_iter()
            .map(|provider| provider.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, ["provider-b", "provider-a"]);
    }

    #[tokio::test]
    async fn update_provider_fast_command_returns_typed_success() {
        let (_directory, state) = create_state();
        let current = state.provider_service.list_providers().unwrap();

        let result = provider_commands::update_provider_fast_inner(
            &state,
            UpdateProviderFastInput {
                provider_id: "provider-a".into(),
                enabled: true,
                expected_files: current.fingerprints,
            },
        )
        .await;

        assert!(result.success);
        assert!(result.error.is_none());
        assert!(result.data.unwrap().providers[0].fast_enabled);
    }

    #[tokio::test]
    async fn apply_provider_connection_command_returns_safe_typed_success() {
        let (_directory, state) = create_state();
        let current = state.provider_service.list_providers().unwrap();

        let result = provider_commands::apply_provider_connection_inner(
            &state,
            ApplyProviderConnectionInput {
                source_provider_id: "provider-b".into(),
                expected_files: current.fingerprints,
            },
        )
        .await;
        let json = serde_json::to_string(&result).unwrap();

        assert!(result.success);
        assert!(result.error.is_none());
        assert!(!json.contains("test-key-a-not-real"));
        assert!(!json.contains("test-key-b-not-real"));
        assert!(!json.contains("\"apiKey\":"));
        assert!(!json.contains("CodexRelay"));
    }

    #[tokio::test]
    async fn restore_provider_connection_command_returns_safe_typed_success() {
        let (_directory, state) = create_state();
        let current = state.provider_service.list_providers().unwrap();
        state
            .provider_service
            .apply_provider_connection(ApplyProviderConnectionInput {
                source_provider_id: "provider-b".into(),
                expected_files: current.fingerprints,
            })
            .await
            .unwrap();
        let routed = state.provider_service.list_providers().unwrap();

        let result = provider_commands::restore_provider_connection_inner(
            &state,
            RestoreProviderConnectionInput {
                expected_files: routed.fingerprints,
            },
        )
        .await;
        let json = serde_json::to_string(&result).unwrap();

        assert!(result.success);
        assert!(result.error.is_none());
        assert!(
            result
                .data
                .as_ref()
                .unwrap()
                .providers
                .iter()
                .all(|provider| provider.connection.status
                    == crate::models::provider::ProviderConnectionStatus::None)
        );
        assert!(!json.contains("test-key-a-not-real"));
        assert!(!json.contains("test-key-b-not-real"));
        assert!(!json.contains("\"apiKey\":"));
        assert!(!json.contains("CodexRelay"));
    }

    #[tokio::test]
    async fn invalid_create_command_returns_safe_code_without_stack() {
        let (_directory, state) = create_state();
        let current = state.provider_service.list_providers().unwrap();
        let input = CreateProviderInput {
            id: "invalid.id".into(),
            name: "Provider".into(),
            base_url_name: "主用地址".into(),
            base_url: "https://example.com/v1".into(),
            wire_api: "responses".into(),
            models: vec!["gpt-5.6-sol".into()],
            fast_enabled: false,
            api_key_name: "主用密钥".into(),
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
    fn saving_preferences_preserves_latest_window_bounds() {
        let (_directory, state) = create_state();
        let latest_window = crate::models::settings::WindowBounds {
            width: 1100,
            height: 760,
            x: Some(120),
            y: Some(80),
        };
        state
            .settings_service
            .update(|settings| settings.window = latest_window.clone())
            .unwrap();
        let stale_form = Settings {
            close_to_tray: false,
            ..Settings::default()
        };

        state.save_settings(stale_form).unwrap();
        let saved = state.settings_service.load_or_create().unwrap();

        assert!(!saved.close_to_tray);
        assert_eq!(saved.window, latest_window);
    }

    #[test]
    fn critical_self_check_command_does_not_require_async_runtime() {
        let (_directory, state) = create_state();

        let result = self_check_commands::run_critical_self_check_inner(&state);

        assert!(result.success);
        assert_eq!(result.data.unwrap().level, HealthLevel::Normal);
    }

    #[test]
    fn exit_command_marks_the_explicit_exit_guard() {
        let (_directory, state) = create_state();

        settings_commands::request_exit_inner(&state);

        assert!(state.tray_runtime.exit_requested());
    }

    #[tokio::test]
    async fn backup_commands_list_transaction_backups_after_switch() {
        let (_directory, state) = create_state();
        provider_commands::switch_provider_inner(&state, "provider-b".into()).await;

        let backups = backup_commands::list_backups_inner(&state);

        assert!(backups.success);
        assert_eq!(backups.data.unwrap().backups.len(), 1);
    }

    #[test]
    fn backup_open_command_rejects_traversal_without_exposing_a_path() {
        let (_directory, state) = create_state();

        let result = backup_commands::open_backup_file_inner(
            &state,
            "..\\outside".into(),
            BackupFileName::Metadata,
        );
        let json = serde_json::to_string(&result).unwrap();

        assert!(!result.success);
        assert_eq!(result.error.unwrap().code, "INVALID_BACKUP_NAME");
        assert!(!json.contains("outside"));
        assert!(!json.contains("CodexRelay"));
    }

    #[tokio::test]
    async fn provider_availability_commands_validate_request_ids_without_writing_files() {
        let (_directory, state) = create_state();
        let before_config = fs::read(&state.paths.config_file).unwrap();
        let before_auth = fs::read(&state.paths.auth_file).unwrap();

        let api = provider_availability_commands::test_provider_api_inner(
            &state,
            "provider-a".into(),
            "not-a-uuid".into(),
            false,
        )
        .await;
        let codex = provider_availability_commands::test_provider_codex_compatibility_inner(
            &state,
            "provider-a".into(),
            "not-a-uuid".into(),
            true,
        )
        .await;
        let cancel =
            provider_availability_commands::cancel_provider_test_inner(&state, "not-a-uuid".into());

        for result in [
            serde_json::to_string(&api).unwrap(),
            serde_json::to_string(&codex).unwrap(),
            serde_json::to_string(&cancel).unwrap(),
        ] {
            assert!(result.contains("INVALID_PROVIDER_TEST_REQUEST_ID"));
            assert!(!result.contains("test-key-a-not-real"));
            assert!(!result.contains("CodexRelay"));
        }
        assert_eq!(fs::read(&state.paths.config_file).unwrap(), before_config);
        assert_eq!(fs::read(&state.paths.auth_file).unwrap(), before_auth);
    }
}
