use codex_relay_release_console_lib::services::release_notes::{
    CommitSummary, ReleaseNotesError, ReleaseNotesService,
};

#[test]
fn deterministic_chinese_notes_include_only_user_visible_commits() {
    let commits = vec![
        CommitSummary {
            sha: "1111111".into(),
            subject: "feat(console): 增加仓库预检".into(),
        },
        CommitSummary {
            sha: "2222222".into(),
            subject: "chore(task): 记录过程".into(),
        },
        CommitSummary {
            sha: "3333333".into(),
            subject: "fix(release): 阻止错误 Draft 公开".into(),
        },
        CommitSummary {
            sha: "4444444".into(),
            subject: "perf(build): 缩短重复检查时间".into(),
        },
        CommitSummary {
            sha: "5555555".into(),
            subject: "docs: 更新内部说明".into(),
        },
    ];

    let first = ReleaseNotesService::generate("0.4.0", "0.5.0", &commits).unwrap();
    let second = ReleaseNotesService::generate("0.4.0", "0.5.0", &commits).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.visible_commit_count, 3);
    assert!(!first.requires_manual_content);
    assert_eq!(
        first.body,
        "## 更新内容\n\n- 新增：增加仓库预检\n- 修复：阻止错误 Draft 公开\n- 性能：缩短重复检查时间\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可在设置页点击“检查更新”，再选择“下载并安装”；也可从 GitHub Releases 手动下载安装器，从 `v0.4.0` 更新到 `v0.5.0`。下载会经过 Tauri updater 签名校验，安装阶段应用会退出，并可能请求 Windows 管理员权限。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n"
    );
    assert!(!first.body.contains("记录过程"));
    assert!(!first.body.contains("更新内部说明"));
    assert!(!first.body.contains("1111111"));
}

#[test]
fn edited_notes_require_manual_content_and_reject_credential_shaped_text() {
    let draft = ReleaseNotesService::generate(
        "0.4.0",
        "0.5.0",
        &[CommitSummary {
            sha: "1111111".into(),
            subject: "chore(task): 仅记录任务".into(),
        }],
    )
    .unwrap();

    assert!(draft.requires_manual_content);
    assert_eq!(
        ReleaseNotesService::validate("0.4.0", "0.5.0", &draft.body),
        Err(ReleaseNotesError::ManualContentRequired)
    );

    let edited = draft.body.replace(
        "请填写本版本的用户可见变化。",
        "修复发布流程中的错误状态判断。",
    );
    ReleaseNotesService::validate("0.4.0", "0.5.0", &edited).unwrap();

    let unsafe_notes = edited.replace(
        "## 注意事项",
        "Authorization: Bearer test-key-release-not-real\n\n## 注意事项",
    );
    assert_eq!(
        ReleaseNotesService::validate("0.4.0", "0.5.0", &unsafe_notes),
        Err(ReleaseNotesError::SecretDetected)
    );
    assert_eq!(
        ReleaseNotesService::generate("0.4.0", "0.4.0", &[]),
        Err(ReleaseNotesError::TargetVersionNotHigher)
    );
}

#[test]
fn edited_notes_must_keep_required_release_contract() {
    let draft = ReleaseNotesService::generate(
        "0.4.0",
        "0.5.0",
        &[CommitSummary {
            sha: "1111111".into(),
            subject: "fix(release): 修复发布状态判断".into(),
        }],
    )
    .unwrap();

    let invalid_bodies = [
        draft.body.replace("## 更新内容", "## 其他内容"),
        draft.body.replace("## 更新方式", "## 其他方式"),
        draft.body.replace("## 注意事项", "## 其他事项"),
        draft.body.replace("`v0.4.0`", "`v0.3.0`"),
        draft.body.replace("`v0.5.0`", "`v0.6.0`"),
        draft.body.replace("Windows 可能显示“未知发布者”", ""),
        draft.body.replace(
            "安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。",
            "",
        ),
    ];

    for body in invalid_bodies {
        assert_eq!(
            ReleaseNotesService::validate("0.4.0", "0.5.0", &body),
            Err(ReleaseNotesError::MissingRequiredContent)
        );
    }
}

#[test]
fn release_note_errors_expose_stable_codes() {
    assert_eq!(
        ReleaseNotesError::InvalidVersion.code(),
        "RELEASE_VERSION_INVALID"
    );
    assert_eq!(
        ReleaseNotesError::TargetVersionNotHigher.code(),
        "RELEASE_VERSION_NOT_HIGHER"
    );
    assert_eq!(
        ReleaseNotesError::ManualContentRequired.code(),
        "RELEASE_NOTES_MANUAL_CONTENT_REQUIRED"
    );
    assert_eq!(
        ReleaseNotesError::MissingRequiredContent.code(),
        "RELEASE_NOTES_REQUIRED_CONTENT_MISSING"
    );
    assert_eq!(
        ReleaseNotesError::SecretDetected.code(),
        "RELEASE_NOTES_SECRET_DETECTED"
    );
}
