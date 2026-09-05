use codex_relay_core::infrastructure::path_service::AppPaths;
use codex_relay_core::models::provider::{
    CreateProviderInput, UpdateProviderInput, UpdateProviderPreferenceInput,
};
use codex_relay_core::services::provider_service::ProviderService;
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
const INITIAL_PREFERENCES: &str = r#"{
  "version": 1,
  "providers": {
    "provider-a": {
      "models": [
        "gpt-5.6-sol"
      ],
      "selectedModel": "gpt-5.6-sol",
      "reasoningEfforts": {
        "gpt-5.6-sol": "medium"
      }
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
    fs::write(&paths.provider_preferences_file, INITIAL_PREFERENCES).unwrap();
    let service = ProviderService::new(paths.clone(), "0.1.0");
    (directory, paths, service)
}

#[tokio::test]
async fn editing_provider_to_gpt_6_astra_saves_and_applies_model_preferences() {
    let (_directory, paths, service) = setup();
    let state = service.list_providers().unwrap();
    let astra = state
        .model_catalog
        .iter()
        .find(|model| model.id == "gpt-6-astra")
        .expect("the editor catalog should offer gpt-6-astra");
    assert_eq!(astra.default_reasoning_effort, "low");
    assert_eq!(
        astra.reasoning_efforts,
        ["low", "medium", "high", "xhigh", "max", "ultra"]
    );
    assert!(astra.supports_fast);

    service
        .update_provider(UpdateProviderInput {
            id: "provider-a".into(),
            name: "Provider A".into(),
            wire_api: "responses".into(),
            models: vec!["gpt-6-astra".into()],
            fast_enabled: true,
            sync_if_active: true,
            expected_files: state.fingerprints,
        })
        .await
        .unwrap();

    let state = service.list_providers().unwrap();
    let provider = &state.providers[0];
    assert_eq!(provider.models, ["gpt-6-astra"]);
    assert_eq!(provider.selected_model.as_deref(), Some("gpt-6-astra"));
    assert_eq!(provider.reasoning_efforts["gpt-6-astra"], "low");
    assert!(provider.fast_enabled);
    let config = fs::read_to_string(&paths.config_file).unwrap();
    let document = config.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(document["model"].as_str(), Some("gpt-6-astra"));
    assert_eq!(document["model_reasoning_effort"].as_str(), Some("low"));
    assert_eq!(document["service_tier"].as_str(), Some("fast"));

    service
        .update_provider_preference(UpdateProviderPreferenceInput {
            provider_id: "provider-a".into(),
            model: "gpt-6-astra".into(),
            reasoning_effort: "ultra".into(),
            expected_files: state.fingerprints,
        })
        .await
        .unwrap();

    let state = service.list_providers().unwrap();
    assert_eq!(state.providers[0].reasoning_efforts["gpt-6-astra"], "ultra");
    let config = fs::read_to_string(&paths.config_file).unwrap();
    let document = config.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(document["model_reasoning_effort"].as_str(), Some("ultra"));
    assert_eq!(document["model_provider"].as_str(), Some("provider-a"));
    assert_eq!(document["features"]["web_search"].as_bool(), Some(true));
    assert_eq!(
        document["model_providers"]["provider-a"]["custom_option"].as_str(),
        Some("keep-me")
    );
    assert!(config.contains("# preserve this user comment"));
    assert_eq!(fs::read_to_string(&paths.auth_file).unwrap(), INITIAL_AUTH);
}

#[tokio::test]
async fn provider_workflow_preserves_unknown_config_and_restores_original_bytes() {
    let (_directory, paths, service) = setup();
    let original_config = fs::read(&paths.config_file).unwrap();
    let original_auth = fs::read(&paths.auth_file).unwrap();
    let original_providers = fs::read(&paths.providers_file).unwrap();
    let original_preferences = fs::read(&paths.provider_preferences_file).unwrap();

    let state = service.list_providers().unwrap();
    service
        .create_provider(CreateProviderInput {
            id: "provider-b".into(),
            name: "Provider B".into(),
            base_url_name: "主用地址".into(),
            base_url: "https://provider-b.example.test/v1".into(),
            wire_api: "responses".into(),
            models: vec!["gpt-5.6-sol".into(), "gpt-5.4-mini".into()],
            fast_enabled: false,
            api_key_name: "主用密钥".into(),
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
            wire_api: "responses".into(),
            models: vec!["gpt-5.4-mini".into()],
            fast_enabled: false,
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
    assert_eq!(auth["OPENAI_API_KEY"], "test-key-b-not-real");

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
        .backups
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
    assert_eq!(
        fs::read(&paths.provider_preferences_file).unwrap(),
        original_preferences
    );
}
