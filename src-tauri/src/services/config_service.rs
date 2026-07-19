use crate::error::AppError;
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, Table, TableLike, value};
use url::Url;

const MAX_PROVIDER_ID_LEN: usize = 64;
const MAX_PROVIDER_NAME_LEN: usize = 100;
const MAX_BASE_URL_LEN: usize = 2048;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub id: String,
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub wire_api: Option<String>,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub wire_api: String,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedProviderInput {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub wire_api: String,
    pub model: Option<String>,
}

pub fn parse_document(source: &str) -> Result<DocumentMut, AppError> {
    source.parse::<DocumentMut>().map_err(|error| {
        AppError::new(
            "INVALID_CONFIG_TOML",
            "无法解析 config.toml。软件没有修改该文件，请查看自检详情或从备份恢复。",
            error.to_string(),
        )
    })
}

pub fn current_provider_id(document: &DocumentMut) -> Option<String> {
    document
        .get("model_provider")
        .and_then(Item::as_str)
        .map(str::to_owned)
}

pub fn list_provider_configs(document: &DocumentMut) -> Result<Vec<ProviderConfig>, AppError> {
    let Some(item) = document.get("model_providers") else {
        return Ok(Vec::new());
    };
    let table = item.as_table_like().ok_or_else(|| {
        AppError::new(
            "INVALID_MODEL_PROVIDERS",
            "config.toml 中的 model_providers 格式无效。",
            "model_providers is not a TOML table",
        )
    })?;

    Ok(table
        .iter()
        .filter_map(|(id, item)| {
            let provider = item.as_table_like()?;
            Some(ProviderConfig {
                id: id.to_string(),
                name: string_field(provider, "name"),
                base_url: string_field(provider, "base_url"),
                wire_api: string_field(provider, "wire_api"),
                model: string_field(provider, "model"),
            })
        })
        .collect())
}

pub fn validate_provider_id(id: &str) -> Result<String, AppError> {
    let normalized = id.trim().to_ascii_lowercase();
    let valid = !normalized.is_empty()
        && normalized.len() <= MAX_PROVIDER_ID_LEN
        && normalized
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');

    if !valid {
        return Err(AppError::new(
            "INVALID_PROVIDER_ID",
            "Provider ID 只能包含英文字母、数字、下划线和连字符。",
            format!("invalid provider id: {normalized:?}"),
        ));
    }

    Ok(normalized)
}

pub fn validate_provider_input(input: &ProviderInput) -> Result<ValidatedProviderInput, AppError> {
    let id = validate_provider_id(&input.id)?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > MAX_PROVIDER_NAME_LEN {
        return Err(AppError::new(
            "INVALID_PROVIDER_NAME",
            "Provider 名称不能为空且长度不能超过 100 个字符。",
            "provider name is empty or too long",
        ));
    }

    let raw_base_url = input.base_url.trim();
    if raw_base_url.is_empty() || raw_base_url.len() > MAX_BASE_URL_LEN {
        return Err(invalid_base_url("base URL is empty or too long"));
    }
    let parsed_url = Url::parse(raw_base_url)
        .map_err(|error| invalid_base_url(&format!("URL parse failed: {error}")))?;
    if !matches!(parsed_url.scheme(), "http" | "https") || parsed_url.host_str().is_none() {
        return Err(invalid_base_url("base URL is not HTTP(S) or has no host"));
    }

    let wire_api = input.wire_api.trim();
    if wire_api != "responses" {
        return Err(AppError::new(
            "INVALID_WIRE_API",
            "Wire API 当前只支持 responses。",
            format!("unsupported wire_api: {wire_api:?}"),
        ));
    }

    let model = input
        .model
        .as_deref()
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_owned);

    Ok(ValidatedProviderInput {
        id,
        name: name.to_owned(),
        base_url: parsed_url.to_string(),
        wire_api: wire_api.to_owned(),
        model,
    })
}

pub fn validate_provider_config(
    provider: &ProviderConfig,
) -> Result<ValidatedProviderInput, AppError> {
    validate_provider_input(&ProviderInput {
        id: provider.id.clone(),
        name: provider.name.clone().unwrap_or_default(),
        base_url: provider.base_url.clone().unwrap_or_default(),
        wire_api: provider.wire_api.clone().unwrap_or_default(),
        model: provider.model.clone(),
    })
}

pub fn create_provider(source: &str, input: &ValidatedProviderInput) -> Result<String, AppError> {
    let mut document = parse_document(source)?;
    let providers = model_providers_table_mut(&mut document, true)?;
    if providers.contains_key(&input.id) {
        return Err(AppError::new(
            "PROVIDER_ALREADY_EXISTS",
            "该 Provider ID 已存在。",
            format!("duplicate provider id: {}", input.id),
        ));
    }

    let mut provider = Table::new();
    set_provider_fields(&mut provider, input);
    providers.insert(&input.id, Item::Table(provider));
    Ok(document.to_string())
}

pub fn update_provider(
    source: &str,
    id: &str,
    input: &ValidatedProviderInput,
) -> Result<String, AppError> {
    let id = validate_provider_id(id)?;
    if id != input.id {
        return Err(AppError::new(
            "PROVIDER_ID_IMMUTABLE",
            "Provider ID 创建后不可修改。",
            format!("attempted to change provider id from {id} to {}", input.id),
        ));
    }

    let mut document = parse_document(source)?;
    let provider = provider_table_mut(&mut document, &id)?;
    set_provider_fields(provider, input);
    Ok(document.to_string())
}

pub fn delete_provider(source: &str, id: &str) -> Result<String, AppError> {
    let id = validate_provider_id(id)?;
    let mut document = parse_document(source)?;
    if current_provider_id(&document).as_deref() == Some(id.as_str()) {
        return Err(AppError::new(
            "ACTIVE_PROVIDER_DELETE_FORBIDDEN",
            "当前 Provider 不能删除，请先切换到其他 Provider。",
            format!("attempted to delete active provider {id}"),
        ));
    }

    let providers = model_providers_table_mut(&mut document, false)?;
    if providers.remove(&id).is_none() {
        return Err(provider_not_found(&id));
    }
    Ok(document.to_string())
}

pub fn select_provider(source: &str, provider: &ProviderConfig) -> Result<String, AppError> {
    let validated = validate_provider_config(provider)?;
    let mut document = parse_document(source)?;
    {
        let target = provider_table_mut(&mut document, &validated.id)?;
        set_provider_fields(target, &validated);
    }
    document.insert("model_provider", value(&validated.id));
    document.insert("cli_auth_credentials_store", value("file"));
    if let Some(model) = validated.model.as_deref() {
        document.insert("model", value(model));
    }
    Ok(document.to_string())
}

fn model_providers_table_mut(
    document: &mut DocumentMut,
    create_if_missing: bool,
) -> Result<&mut Table, AppError> {
    if document.get("model_providers").is_none() && create_if_missing {
        document.insert("model_providers", Item::Table(Table::new()));
    }

    document
        .get_mut("model_providers")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| {
            AppError::new(
                "INVALID_MODEL_PROVIDERS",
                "config.toml 中的 model_providers 格式无效。",
                "model_providers is missing or is not a standard TOML table",
            )
        })
}

fn provider_table_mut<'a>(
    document: &'a mut DocumentMut,
    id: &str,
) -> Result<&'a mut dyn TableLike, AppError> {
    let providers = model_providers_table_mut(document, false)?;
    providers
        .get_mut(id)
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| provider_not_found(id))
}

fn set_provider_fields(provider: &mut dyn TableLike, input: &ValidatedProviderInput) {
    provider.insert("name", value(&input.name));
    provider.insert("base_url", value(&input.base_url));
    provider.insert("wire_api", value(&input.wire_api));
    match input.model.as_deref() {
        Some(model) => {
            provider.insert("model", value(model));
        }
        None => {
            provider.remove("model");
        }
    }
}

fn string_field(table: &dyn TableLike, name: &str) -> Option<String> {
    table.get(name).and_then(Item::as_str).map(str::to_owned)
}

fn invalid_base_url(internal_detail: &str) -> AppError {
    AppError::new(
        "INVALID_BASE_URL",
        "Base URL 必须是有效的 HTTP 或 HTTPS 地址。",
        internal_detail,
    )
}

fn provider_not_found(id: &str) -> AppError {
    AppError::new(
        "PROVIDER_NOT_FOUND",
        "指定的 Provider 不存在。",
        format!("provider not found: {id}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTIPLE: &str = include_str!("../../../fixtures/config-multiple-providers.toml");
    const WITH_COMMENTS: &str = include_str!("../../../fixtures/config-with-comments.toml");
    const WITH_UNKNOWN: &str = include_str!("../../../fixtures/config-with-unknown-fields.toml");

    fn valid_input(id: &str) -> ProviderInput {
        ProviderInput {
            id: id.into(),
            name: "  Provider C  ".into(),
            base_url: " https://provider-c.example.com/v1 ".into(),
            wire_api: "responses".into(),
            model: Some("  test-model-c  ".into()),
        }
    }

    #[test]
    fn reads_multiple_providers_and_active_provider() {
        let document = parse_document(MULTIPLE).unwrap();
        let providers = list_provider_configs(&document).unwrap();

        assert_eq!(current_provider_id(&document), Some("provider-a".into()));
        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].id, "provider-a");
        assert_eq!(providers[1].id, "provider-b");
        assert_eq!(providers[1].model.as_deref(), Some("test-model-b"));
    }

    #[test]
    fn provider_input_is_trimmed_normalized_and_validated() {
        let validated = validate_provider_input(&valid_input(" Provider-C ")).unwrap();

        assert_eq!(validated.id, "provider-c");
        assert_eq!(validated.name, "Provider C");
        assert_eq!(validated.base_url, "https://provider-c.example.com/v1");
        assert_eq!(validated.wire_api, "responses");
        assert_eq!(validated.model.as_deref(), Some("test-model-c"));
    }

    #[test]
    fn rejects_toml_path_injection_id() {
        for id in [
            "provider.a",
            "provider[a]",
            "provider a",
            "provider/a",
            "提供商",
        ] {
            let error = validate_provider_id(id).unwrap_err();
            assert_eq!(error.code(), "INVALID_PROVIDER_ID", "accepted {id}");
        }
    }

    #[test]
    fn rejects_blank_name_invalid_url_and_wire_api() {
        let mut input = valid_input("provider-c");
        input.name = "   ".into();
        assert_eq!(
            validate_provider_input(&input).unwrap_err().code(),
            "INVALID_PROVIDER_NAME"
        );

        input.name = "Provider C".into();
        input.base_url = "ftp://provider-c.example.com".into();
        assert_eq!(
            validate_provider_input(&input).unwrap_err().code(),
            "INVALID_BASE_URL"
        );

        input.base_url = "https://provider-c.example.com/v1".into();
        input.wire_api = "chat_completions".into();
        assert_eq!(
            validate_provider_input(&input).unwrap_err().code(),
            "INVALID_WIRE_API"
        );
    }

    #[test]
    fn create_rejects_duplicate_and_preserves_unrelated_toml() {
        let duplicate = validate_provider_input(&valid_input("provider-a")).unwrap();
        assert_eq!(
            create_provider(WITH_COMMENTS, &duplicate)
                .unwrap_err()
                .code(),
            "PROVIDER_ALREADY_EXISTS"
        );

        let created = validate_provider_input(&valid_input("provider-c")).unwrap();
        let output = create_provider(WITH_COMMENTS, &created).unwrap();

        assert!(output.contains("# This leading comment must survive every edit."));
        assert!(output.contains("custom_header = \"preserve-me\""));
        assert!(output.contains("[features]"));
        assert!(output.contains("[mcp_servers.sample]"));
        assert!(output.contains("[model_providers.provider-c]"));
        assert!(output.contains("name = \"Provider C\""));
    }

    #[test]
    fn update_preserves_unknown_fields_and_other_providers() {
        let mut input = valid_input("provider-a");
        input.name = "Updated Provider A".into();
        input.base_url = "https://updated.example.com/v1".into();
        input.model = None;
        let validated = validate_provider_input(&input).unwrap();

        let output = update_provider(WITH_UNKNOWN, "provider-a", &validated).unwrap();

        assert!(output.contains("unknown_number = 42"));
        assert!(output.contains("unknown_array = [\"a\", \"b\"]"));
        assert!(output.contains("[sandbox_workspace_write]"));
        assert!(output.contains("[profiles.personal]"));
        assert!(output.contains("name = \"Updated Provider A\""));
    }

    #[test]
    fn delete_rejects_current_and_removes_only_target_provider() {
        assert_eq!(
            delete_provider(MULTIPLE, "provider-a").unwrap_err().code(),
            "ACTIVE_PROVIDER_DELETE_FORBIDDEN"
        );

        let output = delete_provider(MULTIPLE, "provider-b").unwrap();
        assert!(output.contains("[model_providers.provider-a]"));
        assert!(!output.contains("[model_providers.provider-b]"));
        assert!(output.contains("model_provider = \"provider-a\""));
    }

    #[test]
    fn select_provider_updates_required_top_level_fields_and_model() {
        let document = parse_document(MULTIPLE).unwrap();
        let provider = list_provider_configs(&document)
            .unwrap()
            .into_iter()
            .find(|provider| provider.id == "provider-b")
            .unwrap();

        let output = select_provider(MULTIPLE, &provider).unwrap();

        assert!(output.contains("model_provider = \"provider-b\""));
        assert!(output.contains("model = \"test-model-b\""));
        assert!(output.contains("cli_auth_credentials_store = \"file\""));
        assert!(output.contains("[model_providers.provider-a]"));
    }

    #[test]
    fn select_provider_without_model_keeps_existing_top_level_model() {
        let document = parse_document(MULTIPLE).unwrap();
        let provider = list_provider_configs(&document)
            .unwrap()
            .into_iter()
            .find(|provider| provider.id == "provider-a")
            .unwrap();

        let output = select_provider(MULTIPLE, &provider).unwrap();

        assert!(output.contains("model = \"test-model\""));
        assert!(output.contains("model_provider = \"provider-a\""));
    }
}
