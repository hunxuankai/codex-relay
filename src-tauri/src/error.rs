use serde::Serialize;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult<T>
where
    T: Serialize,
{
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CommandError>,
}

impl<T> CommandResult<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn failure(error: &AppError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_command_error()),
        }
    }
}

pub struct AppError {
    code: String,
    public_message: String,
    internal_detail: String,
}

impl AppError {
    pub fn new(
        code: impl Into<String>,
        public_message: impl Into<String>,
        internal_detail: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            public_message: public_message.into(),
            internal_detail: internal_detail.into(),
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn public_message(&self) -> &str {
        &self.public_message
    }

    pub fn to_command_error(&self) -> CommandError {
        CommandError {
            code: self.code.clone(),
            message: self.public_message.clone(),
        }
    }
}

impl fmt::Debug for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppError")
            .field("code", &self.code)
            .field("public_message", &self.public_message)
            .field(
                "internal_detail",
                &(!self.internal_detail.is_empty()).then_some("[REDACTED]"),
            )
            .finish()
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.public_message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new("IO_ERROR", "文件操作失败。", error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::new("JSON_ERROR", "JSON 文件格式无效。", error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_error_never_exposes_internal_secret() {
        let error = AppError::new(
            "FILE_WRITE_FAILED",
            "配置文件写入失败。",
            "could not write test-key-a-not-real",
        );

        let public = error.to_command_error();
        let json = serde_json::to_string(&public).unwrap();
        assert_eq!(public.code, "FILE_WRITE_FAILED");
        assert_eq!(public.message, "配置文件写入失败。");
        assert!(!json.contains("test-key-a-not-real"));
        assert!(!error.to_string().contains("test-key-a-not-real"));
    }
}
