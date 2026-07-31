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
    let github_redacted = github_token_regex().replace_all(&assigned_redacted, REDACTED);
    let bearer_redacted =
        bearer_regex().replace_all(&github_redacted, format!("Bearer {REDACTED}"));

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
            r#"(?i)("(?:OPENAI_API_KEY|apiKey|Authorization|GH_TOKEN|GITHUB_TOKEN|TAURI_SIGNING_PRIVATE_KEY|TAURI_SIGNING_PRIVATE_KEY_PASSWORD|token|api[_-]?key|key)"\s*:\s*")([^"]*)(")"#,
        )
        .expect("valid JSON secret regex")
    })
}

fn assignment_secret_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r#"(?i)\b(OPENAI_API_KEY|apiKey|Authorization|GH_TOKEN|GITHUB_TOKEN|TAURI_SIGNING_PRIVATE_KEY|TAURI_SIGNING_PRIVATE_KEY_PASSWORD)\b\s*[:=]\s*(?:Bearer\s+)?(?:"[^"]*"|[^\s,;&]+)"#,
        )
        .expect("valid assignment secret regex")
    })
}

fn github_token_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(?:gh[pousr]_[A-Za-z0-9_-]{12,}|github_pat_[A-Za-z0-9_-]{12,})\b")
            .expect("valid GitHub token regex")
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

    #[test]
    fn release_credentials_are_redacted_by_assignment_name_and_token_shape() {
        let input = concat!(
            "GH_TOKEN=github_pat_test-release-token-not-real\n",
            "GITHUB_TOKEN: ghp_test-release-token-not-real\n",
            "TAURI_SIGNING_PRIVATE_KEY=untrusted-comment-test-not-real\n",
            "TAURI_SIGNING_PRIVATE_KEY_PASSWORD=\"test-password-not-real\"\n",
            "Authorization: Bearer test-key-release-not-real\n",
        );

        let redacted = redact(input);

        for secret in [
            "github_pat_test-release-token-not-real",
            "ghp_test-release-token-not-real",
            "untrusted-comment-test-not-real",
            "test-password-not-real",
            "test-key-release-not-real",
        ] {
            assert!(!redacted.contains(secret));
        }
        assert!(redacted.contains("GH_TOKEN=[REDACTED]"));
        assert!(redacted.contains("GITHUB_TOKEN=[REDACTED]"));
        assert!(redacted.contains("TAURI_SIGNING_PRIVATE_KEY=[REDACTED]"));
        assert!(redacted.contains("TAURI_SIGNING_PRIVATE_KEY_PASSWORD=[REDACTED]"));
        assert!(redacted.contains("Authorization=[REDACTED]"));
    }
}
