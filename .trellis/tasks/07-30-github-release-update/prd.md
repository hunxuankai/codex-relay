# 发布 GitHub 更新

## 目标

把当前本地 `master` 中已完成的 Provider 交互、私有排序和 Fast 配置能力发布到
GitHub 默认分支 `main`，并生成、核对和公开可供现有安装用户升级的 Windows
`v0.3.0` Release。

## 已确认事实

- 2026-07-30 查询到当前公开 Latest 是 `v0.2.1`，目标提交为
  `7b3d5f2c793b21b8a5aea01f3fc482bbd9de3ffe`，且不是 Draft 或 Prerelease。
- 本地工作树在任务创建前干净，本地 `master` 相对 `origin/main` 领先 21 个提交；
  远端默认分支是 `main`。
- 待发布用户变化包括主界面版本标题、Provider 拖动排序及本地私有持久化、API
  测试详情交互，以及按 Provider 保存并安全投影的 Fast 配置。
- 当前应用版本仍为已公开的 `0.2.1`，不能复用；这批改动包含新增能力，因此目标
  版本采用下一个次版本 `0.3.0`。
- GitHub Actions 中存在 `TAURI_SIGNING_PRIVATE_KEY` 和
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 两个 Secret 名称；不得读取或记录其值。
- Windows Authenticode 尚未启用，安装器仍可能显示“未知发布者”。

## 需求

### R1：准备发布候选

- 将 npm、Cargo、锁文件和发布结构测试中的应用版本同步为 `0.3.0`。
- `.github/workflows/release.yml` 必须包含最终简体中文发布说明，准确描述从
  `v0.2.1` 更新到 `v0.3.0` 的用户可见变化、更新方式、未知发布者提示和数据保留
  边界。
- 不修改 updater endpoint、公钥、Secrets 名称、NSIS 安装范围或卸载数据保留策略。

### R2：验证并推送

- 提交前运行本轮新鲜的 `npm run check`、无签名环境变量的普通 `npm run build`、
  `git diff --check` 和秘密/路径审计。
- 枚举普通 Release 主程序和 NSIS 安装器的实际路径、大小、时间与 SHA-256；普通
  构建不得被描述为 updater 签名、安装或升级成功。
- 精确提交本任务改动，将候选提交推送到 `origin/main`，并确认远端 `main` 指向同一
  提交。不得手工创建 Release Tag。

### R3：生成、审计并公开 Release

- 触发且只保留一个“发布 Windows 更新”候选 Run；工作流必须基于候选提交成功
  生成 `v0.3.0` Draft。
- 公开前核对 Draft 的目标提交、最终说明、NSIS、`.sig` 和 `latest.json`；记录三个
  资产的实际大小与 SHA-256，并确认清单版本、说明、平台 URL 和签名关联一致。
- 公开 Release 后确认 `releases/latest` 和公开 `latest.json` 都返回 `v0.3.0`，且
  公开资产没有相对 Draft 漂移。
- 核对“清理历史 GitHub Releases”工作流成功，只保留当前 Latest Release 与 Tag；
  历史公开资产清理不得描述成本机用户数据清理。

### R4：保留真实升级证据边界

- 在公开前把 `v0.2.1` NSIS 安装器下载到系统临时目录并核对公开大小与 SHA-256，
  避免历史 Release 清理后失去升级基线。
- Release 公开后，优先在 Windows Sandbox 或隔离 VM 中从 `v0.2.1` 执行到
  `v0.3.0` 的应用内升级，使用成对 Relay 覆盖和明确的 `test-key-*-not-real`
  fixture；不得访问真实 `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay`。
- 安装、UAC、重启或升级场景若受环境限制未执行或失败，必须如实记录，不得用构建
  或托管状态替代成功证据。

## 验收标准

- [x] AC1：所有权威版本来源、发布说明和结构测试一致指向 `0.3.0`，且版本严格高于
      当前公开 `v0.2.1`。
- [x] AC2：本轮完整检查、普通构建、差异与安全审计通过，实际普通构建产物已枚举并
      记录 SHA-256。
- [x] AC3：候选提交已推送到 GitHub `main`，远端提交与工作流输入一致。
- [ ] AC4：发布工作流成功生成经逐项审计的 `v0.3.0` Draft，资产与清单满足 updater
      契约且不含秘密或用户数据。
- [ ] AC5：`v0.3.0` 已公开为 Latest，公开端点与资产复核一致，历史 Release/tag 清理
      Run 成功且只保留当前 Latest。
- [ ] AC6：旧版安装器已安全暂存；实际执行的 Sandbox/VM 升级结果和所有未执行项均
      以真实证据记录。

## 范围外

- 不新增产品能力，不修改本批已完成特性的行为。
- 不更换 updater 密钥，不查看或复制 GitHub Actions Secret 值。
- 不启用或宣称 Windows Authenticode 签名。
- 不原地替换任何已公开版本的资产。
