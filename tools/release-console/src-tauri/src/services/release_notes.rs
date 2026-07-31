use codex_relay_core::infrastructure::safe_log::redact;
use semver::Version;
use serde::{Deserialize, Serialize};

const MANUAL_CONTENT_PLACEHOLDER: &str = "请填写本版本的用户可见变化。";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSummary {
    pub sha: String,
    pub subject: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseNotesDraft {
    pub body: String,
    pub visible_commit_count: usize,
    pub requires_manual_content: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReleaseNotesError {
    #[error("版本号不是有效的 SemVer")]
    InvalidVersion,
    #[error("目标版本必须严格高于当前公开版本")]
    TargetVersionNotHigher,
    #[error("发布说明仍包含需要人工填写的更新内容")]
    ManualContentRequired,
    #[error("发布说明缺少必需的版本、段落或安全提示")]
    MissingRequiredContent,
    #[error("发布说明包含疑似密钥或认证信息")]
    SecretDetected,
}

impl ReleaseNotesError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidVersion => "RELEASE_VERSION_INVALID",
            Self::TargetVersionNotHigher => "RELEASE_VERSION_NOT_HIGHER",
            Self::ManualContentRequired => "RELEASE_NOTES_MANUAL_CONTENT_REQUIRED",
            Self::MissingRequiredContent => "RELEASE_NOTES_REQUIRED_CONTENT_MISSING",
            Self::SecretDetected => "RELEASE_NOTES_SECRET_DETECTED",
        }
    }
}

pub struct ReleaseNotesService;

impl ReleaseNotesService {
    pub fn generate(
        previous_version: &str,
        target_version: &str,
        commits: &[CommitSummary],
    ) -> Result<ReleaseNotesDraft, ReleaseNotesError> {
        let previous =
            Version::parse(previous_version).map_err(|_| ReleaseNotesError::InvalidVersion)?;
        let target =
            Version::parse(target_version).map_err(|_| ReleaseNotesError::InvalidVersion)?;
        if target <= previous {
            return Err(ReleaseNotesError::TargetVersionNotHigher);
        }

        let visible_changes = commits
            .iter()
            .filter_map(|commit| format_visible_change(&commit.subject))
            .collect::<Vec<_>>();
        let requires_manual_content = visible_changes.is_empty();
        let changes = if requires_manual_content {
            format!("- {MANUAL_CONTENT_PLACEHOLDER}")
        } else {
            visible_changes
                .iter()
                .map(|change| format!("- {change}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let body = format!(
            "## 更新内容\n\n{changes}\n\n## 更新方式\n\n已安装 `v{previous}` 的用户可在设置页点击“检查更新”，再选择“下载并安装”；也可从 GitHub Releases 手动下载安装器，从 `v{previous}` 更新到 `v{target}`。下载会经过 Tauri updater 签名校验，安装阶段应用会退出，并可能请求 Windows 管理员权限。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n"
        );

        Ok(ReleaseNotesDraft {
            body,
            visible_commit_count: visible_changes.len(),
            requires_manual_content,
        })
    }

    pub fn validate(
        previous_version: &str,
        target_version: &str,
        body: &str,
    ) -> Result<(), ReleaseNotesError> {
        let previous =
            Version::parse(previous_version).map_err(|_| ReleaseNotesError::InvalidVersion)?;
        let target =
            Version::parse(target_version).map_err(|_| ReleaseNotesError::InvalidVersion)?;
        if target <= previous {
            return Err(ReleaseNotesError::TargetVersionNotHigher);
        }
        if body.contains(MANUAL_CONTENT_PLACEHOLDER) {
            return Err(ReleaseNotesError::ManualContentRequired);
        }
        let previous_marker = format!("`v{previous_version}`");
        let target_marker = format!("`v{target_version}`");
        if [
            "## 更新内容",
            "## 更新方式",
            "## 注意事项",
            previous_marker.as_str(),
            target_marker.as_str(),
            "Windows 可能显示“未知发布者”",
            "安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。",
        ]
        .iter()
        .any(|required| !body.contains(required))
        {
            return Err(ReleaseNotesError::MissingRequiredContent);
        }
        if redact(body) != body {
            return Err(ReleaseNotesError::SecretDetected);
        }

        Ok(())
    }
}

fn format_visible_change(subject: &str) -> Option<String> {
    let trimmed = subject.trim();
    if trimmed.is_empty() {
        return None;
    }
    let Some((prefix, description)) = trimmed.split_once(':') else {
        return Some(format!("改进：{trimmed}"));
    };
    let change_type = prefix
        .trim()
        .split(['(', '!'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let description = description.trim();
    if description.is_empty() {
        return None;
    }
    let label = match change_type.as_str() {
        "feat" => "新增",
        "fix" => "修复",
        "perf" => "性能",
        "refactor" => "改进",
        "revert" => "回退",
        "chore" | "test" | "ci" | "docs" | "build" | "style" => return None,
        _ => "改进",
    };
    Some(format!("{label}：{description}"))
}
