use codex_relay_lib::infrastructure::path_service::AppPaths;
use codex_relay_lib::models::provider::{ApiKeyChange, CreateProviderInput, UpdateProviderInput};
use codex_relay_lib::services::provider_service::ProviderService;
use serde_json::Value;
use std::fs;

const INITIAL_CONFIG: &str = r#"# preserve this user comment
model = "original-model"
model_provider = "provider-a"

[features]
web_search = true

[model_providers.provider-a]
name = "Provider A"
base_url = "https://provider-a.example.test/v1"
wire_api = "responses"
custom_option = "keep-me"
"#;

const INITIAL_AUTH: &str = "{\n  \"OPENAI_API_KEY\": \"test-key-a-not-real\"\n}\n";
const INITIAL_PROVIDERS: &str = r#"{
  "version": 1,
  "providers": {
    "provider-a": {
      "apiKey": "test-key-a-not-real"
    }
  }
}
"#;

fn setup() -> (tempfile::TempDir, AppPaths, ProviderService) {
    let directory = tempfile::tempdir().unwrap();
    let paths = AppPaths::for_test(
        directory.path().join("codex"),
        directory.path().join("app-data"),
    )
    .unwrap();
    fs::create_dir_all(&paths.codex_home).unwrap();
    fs::create_dir_all(&paths.app_data_dir).unwrap();
    fs::write(&paths.config_file, INITIAL_CONFIG).unwrap();
    fs::write(&paths.auth_file, INITIAL_AUTH).unwrap();
    fs::write(&paths.providers_file, INITIAL_PROVIDERS).unwrap();
    let service = ProviderService::new(paths.clone(), "0.1.0");
    (directory, paths, service)
}

#[tokio::test]
async fn provider_workflow_preserves_unknown_config_and_restores_original_bytes() {
    let (_directory, paths, service) = setup();
    let original_config = fs::read(&paths.config_file).unwrap();
    let original_auth = fs::read(&paths.auth_file).unwrap();
    let original_providers = fs::read(&paths.providers_file).unwrap();

    let state = service.list_providers().unwrap();
    service
        .create_provider(CreateProviderInput {
            id: "provider-b".into(),
            name: "Provider B".into(),
            base_url: "https://provider-b.example.test/v1".into(),
            wire_api: "responses".into(),
            model: Some("model-b".into()),
            api_key: "test-key-b-not-real".into(),
            activate_after_save: false,
            expected_files: state.fingerprints,
        })
        .await
        .unwrap();

    let state = service.list_providers().unwrap();
    service
        .update_provider(UpdateProviderInput {
            id: "provider-b".into(),
            name: "Provider B Updated".into(),
            base_url: "https://provider-b.example.test/responses".into(),
            wire_api: "responses".into(),
            model: Some("model-b-updated".into()),
            api_key_change: ApiKeyChange::Set("test-key-b-updated-not-real".into()),
            sync_if_active: false,
            expected_files: state.fingerprints,
        })
        .await
        .unwrap();

    let config_after_update = fs::read_to_string(&paths.config_file).unwrap();
    assert!(config_after_update.contains("# preserve this user comment"));
    assert!(config_after_update.contains("[features]"));
    assert!(config_after_update.contains("web_search = true"));
    assert!(config_after_update.contains("custom_option = \"keep-me\""));

    service.switch_provider("provider-b").await.unwrap();
    let auth: Value = serde_json::from_slice(&fs::read(&paths.auth_file).unwrap()).unwrap();
    assert_eq!(auth["OPENAI_API_KEY"], "test-key-b-updated-not-real");

    let active_delete = service
        .delete_provider("provider-b", service.list_providers().unwrap().fingerprints)
        .await
        .unwrap_err();
    assert_eq!(active_delete.code(), "ACTIVE_PROVIDER_DELETE_FORBIDDEN");

    service.switch_provider("provider-a").await.unwrap();
    service
        .delete_provider("provider-b", service.list_providers().unwrap().fingerprints)
        .await
        .unwrap();

    let original_snapshot = service
        .list_backups()
        .unwrap()
        .into_iter()
        .find(|backup| backup.metadata.operation == "create_provider")
        .expect("create backup should capture the original state");
    service
        .restore_backup(&original_snapshot.directory_name)
        .await
        .unwrap();

    assert_eq!(fs::read(&paths.config_file).unwrap(), original_config);
    assert_eq!(fs::read(&paths.auth_file).unwrap(), original_auth);
    assert_eq!(fs::read(&paths.providers_file).unwrap(), original_providers);
}
