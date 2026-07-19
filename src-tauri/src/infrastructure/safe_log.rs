use crate::error::AppError;
use regex::{Captures, Regex};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

const REDACTED: &str = "[REDACTED]";
const DEFAULT_LOG_RETENTION: usize = 7;

pub struct LogGuard {
    _worker_guard: WorkerGuard,
}

pub fn init_logging(log_directory: &Path) -> Result<LogGuard, AppError> {
    fs::create_dir_all(log_directory).map_err(AppError::from)?;
    cleanup_old_logs(log_directory, DEFAULT_LOG_RETENTION)?;

    let file_appender = tracing_appender::rolling::daily(log_directory, "codex-relay.log");
    let (writer, worker_guard) = tracing_appender::non_blocking(file_appender);
    let default_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(filter)
        .with_writer(writer)
        .try_init()
        .map_err(|error| {
            AppError::new("LOG_INIT_FAILED", "无法初始化软件日志。", error.to_string())
        })?;

    Ok(LogGuard {
        _worker_guard: worker_guard,
    })
}

pub fn redact(input: &str) -> String {
    let json_redacted = json_secret_regex().replace_all(input, |captures: &Captures<'_>| {
        format!("{}{}{}", &captures[1], REDACTED, &captures[3])
    });
    let assigned_redacted = assignment_secret_regex()
        .replace_all(&json_redacted, |captures: &Captures<'_>| {
            format!("{}={REDACTED}", &captures[1])
        });
    let bearer_redacted =
        bearer_regex().replace_all(&assigned_redacted, format!("Bearer {REDACTED}"));

    query_secret_regex()
        .replace_all(&bearer_redacted, |captures: &Captures<'_>| {
            format!("{}{REDACTED}", &captures[1])
        })
        .into_owned()
}

pub fn format_error_for_log(error: &AppError) -> String {
    let internal_detail = error.internal_detail();
    let safe_detail = if internal_detail.contains('{')
        && ["OPENAI_API_KEY", "apiKey", "Authorization"]
            .iter()
            .any(|marker| internal_detail.contains(marker))
    {
        "[REDACTED DOCUMENT]".to_string()
    } else {
        redact(internal_detail)
    };

    format!(
        "{}: {} | detail={}",
        error.code(),
        error.public_message(),
        safe_detail
    )
}

pub fn cleanup_old_logs(log_directory: &Path, max_files: usize) -> Result<(), AppError> {
    if !log_directory.exists() {
        return Ok(());
    }

    let mut logs = fs::read_dir(log_directory)
        .map_err(AppError::from)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            (file_type.is_file() && name.starts_with("codex-relay") && name.contains(".log"))
                .then(|| entry.path())
        })
        .collect::<Vec<PathBuf>>();

    logs.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    let remove_count = logs.len().saturating_sub(max_files);
    for path in logs.into_iter().take(remove_count) {
        fs::remove_file(path).map_err(AppError::from)?;
    }

    Ok(())
}

fn json_secret_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?i)("(?:OPENAI_API_KEY|apiKey|Authorization|token|api[_-]?key|key)"\s*:\s*")([^"]*)(")"#,
        )
        .expect("valid JSON secret regex")
    })
}

fn assignment_secret_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?i)\b(OPENAI_API_KEY|apiKey|Authorization)\b\s*[:=]\s*(?:Bearer\s+)?(?:"[^"]*"|[^\s,;&]+)"#,
        )
        .expect("valid assignment secret regex")
    })
}

fn bearer_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]+").expect("valid bearer token regex")
    })
}

fn query_secret_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)([?&](?:token|api[_-]?key|key)=)[^&#\s]+")
            .expect("valid query secret regex")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use std::fs;

    #[test]
    fn redacts_structured_headers_bearer_and_query_secrets() {
        let input = concat!(
            "OPENAI_API_KEY=test-key-a-not-real ",
            "apiKey: test-key-b-not-real ",
            "Authorization: Bearer secret-token ",
            "Bearer standalone-token ",
            "https://example.test/v1?token=query-secret&api_key=other-secret"
        );

        let output = redact(input);

        for secret in [
            "test-key-a-not-real",
            "test-key-b-not-real",
            "secret-token",
            "standalone-token",
            "query-secret",
            "other-secret",
        ] {
            assert!(!output.contains(secret), "leaked {secret}: {output}");
        }
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_secret_values_inside_json() {
        let input = r#"{"OPENAI_API_KEY":"test-key-a-not-real","apiKey":"test-key-b-not-real","Authorization":"Bearer json-token"}"#;

        let output = redact(input);

        assert!(!output.contains("test-key-a-not-real"));
        assert!(!output.contains("test-key-b-not-real"));
        assert!(!output.contains("json-token"));
        assert_eq!(output.matches("[REDACTED]").count(), 3);
    }

    #[test]
    fn formats_internal_errors_without_exposing_secrets() {
        let error = AppError::new(
            "REQUEST_FAILED",
            "请求处理失败。",
            "Authorization: Bearer test-key-a-not-real",
        );

        let output = format_error_for_log(&error);

        assert!(output.contains("REQUEST_FAILED"));
        assert!(output.contains("请求处理失败。"));
        assert!(!output.contains("test-key-a-not-real"));
    }

    #[test]
    fn error_log_does_not_repeat_complete_secret_json_documents() {
        let error = AppError::new(
            "AUTH_PARSE_FAILED",
            "无法解析 auth.json。",
            r#"{"OPENAI_API_KEY":"test-key-a-not-real","other":"must-not-be-logged"}"#,
        );

        let output = format_error_for_log(&error);

        assert!(!output.contains("test-key-a-not-real"));
        assert!(!output.contains("must-not-be-logged"));
        assert!(output.contains("[REDACTED DOCUMENT]"));
    }

    #[test]
    fn cleanup_keeps_only_newest_log_files() {
        let directory = tempfile::tempdir().unwrap();
        for name in [
            "codex-relay.2026-07-18.log",
            "codex-relay.2026-07-19.log",
            "codex-relay.2026-07-20.log",
        ] {
            fs::write(directory.path().join(name), "safe log line\n").unwrap();
        }
        fs::write(directory.path().join("keep.txt"), "not a log\n").unwrap();

        cleanup_old_logs(directory.path(), 2).unwrap();

        assert!(!directory.path().join("codex-relay.2026-07-18.log").exists());
        assert!(directory.path().join("codex-relay.2026-07-19.log").exists());
        assert!(directory.path().join("codex-relay.2026-07-20.log").exists());
        assert!(directory.path().join("keep.txt").exists());
    }
}
