use crate::error::AppError;
use regex::{Captures, Regex};
use std::sync::OnceLock;

const REDACTED: &str = "[REDACTED]";

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
