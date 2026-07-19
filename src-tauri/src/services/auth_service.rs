use crate::error::AppError;
use crate::infrastructure::atomic_file::atomic_write;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AuthService {
    path: PathBuf,
}

impl AuthService {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read_api_key(&self) -> Result<Option<String>, AppError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(AppError::from(error)),
        };
        let document = parse_auth_json(&bytes)?;
        Ok(document.openai_api_key.filter(|key| !key.is_empty()))
    }

    pub fn write_api_key(&self, api_key: &str) -> Result<(), AppError> {
        let bytes = render_auth_json(api_key)?;
        ensure_parent_exists(&self.path)?;
        atomic_write(&self.path, &bytes, |candidate| {
            parse_auth_json(candidate).and_then(|document| {
                document
                    .openai_api_key
                    .filter(|key| !key.is_empty())
                    .map(|_| ())
                    .ok_or_else(|| {
                        AppError::new(
                            "AUTH_KEY_MISSING",
                            "auth.json 中缺少 OPENAI_API_KEY。",
                            "generated auth JSON has no non-empty OPENAI_API_KEY",
                        )
                    })
            })
        })
    }

    pub fn matches_api_key(&self, api_key: &str) -> Result<bool, AppError> {
        Ok(self.read_api_key()?.as_deref() == Some(api_key))
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct AuthDocument {
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
}

pub fn render_auth_json(api_key: &str) -> Result<Vec<u8>, AppError> {
    let api_key = api_key.trim_matches(|character| matches!(character, '\r' | '\n'));
    if api_key.is_empty() {
        return Err(AppError::new(
            "EMPTY_API_KEY",
            "API Key 不能为空。",
            "attempted to render auth.json with an empty API key",
        ));
    }
    let document = AuthDocument {
        openai_api_key: Some(api_key.to_owned()),
    };
    let mut json = serde_json::to_string_pretty(&document).map_err(AppError::from)?;
    json.push('\n');
    Ok(json.into_bytes())
}

fn parse_auth_json(bytes: &[u8]) -> Result<AuthDocument, AppError> {
    serde_json::from_slice(bytes).map_err(|error| {
        AppError::new(
            "INVALID_AUTH_JSON",
            "无法解析 auth.json。软件没有修改该文件，请查看自检详情或从备份恢复。",
            error.to_string(),
        )
    })
}

fn ensure_parent_exists(path: &Path) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "INVALID_FILE_PATH",
            "Codex 认证文件路径无效。",
            format!("auth path has no parent: {}", path.display()),
        )
    })?;
    fs::create_dir_all(parent).map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn renders_exact_two_space_json_with_terminal_newline() {
        let rendered = render_auth_json("test-key-a-not-real").unwrap();

        assert_eq!(
            String::from_utf8(rendered).unwrap(),
            "{\n  \"OPENAI_API_KEY\": \"test-key-a-not-real\"\n}\n"
        );
    }

    #[test]
    fn writes_reads_and_compares_current_key() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("auth.json");
        let service = AuthService::new(path.clone());

        service.write_api_key("test-key-a-not-real").unwrap();

        assert_eq!(
            service.read_api_key().unwrap().as_deref(),
            Some("test-key-a-not-real")
        );
        assert!(service.matches_api_key("test-key-a-not-real").unwrap());
        assert!(!service.matches_api_key("test-key-b-not-real").unwrap());
        assert!(fs::read(&path).unwrap().ends_with(b"\n"));
    }

    #[test]
    fn missing_auth_file_has_no_key() {
        let directory = tempfile::tempdir().unwrap();
        let service = AuthService::new(directory.path().join("auth.json"));

        assert_eq!(service.read_api_key().unwrap(), None);
    }

    #[test]
    fn invalid_auth_json_returns_safe_error_and_is_not_changed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("auth.json");
        let invalid = include_str!("../../../fixtures/auth-invalid.json");
        fs::write(&path, invalid).unwrap();
        let service = AuthService::new(path.clone());

        let error = service.read_api_key().unwrap_err();

        assert_eq!(error.code(), "INVALID_AUTH_JSON");
        assert_eq!(fs::read_to_string(path).unwrap(), invalid);
        assert!(!error.to_string().contains("test-key-a-not-real"));
        assert!(!format!("{error:?}").contains("test-key-a-not-real"));
    }
}
