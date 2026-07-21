# 记录发布更新操作步骤

## 目标

为 Codex Relay 维护者提供一份可直接执行的 Windows 更新发布指南，覆盖候选准备、版本同步、本地验证、推送、GitHub Actions Draft、发布前核对、公开发布、发布后验证和失败处置，避免依赖临时对话或上一版本任务记录。

## 已确认事实

- 应用版本来自 `package.json`，Cargo 包版本与锁文件需要保持一致。
- `.github/workflows/release.yml` 只通过 `workflow_dispatch` 手动触发，运行完整检查后生成 Draft Release。
- 发布资产包括 Windows x64 NSIS 安装器、对应 `.sig` 和 `latest.json`。
- `releaseBody` 同时进入 Release 页面和 `latest.json.notes`；Draft 生成后只修改页面说明不会重写清单。
- 更新私钥只允许存在于 GitHub Actions Secrets 和开发者控制的离线备份中。
- 当前未使用 Windows Authenticode，安装器可能显示“未知发布者”；Tauri updater 签名仍为强制信任边界。

## 需求

- 在 `.trellis/spec/release/` 新增独立的发布操作指南，并由该领域 `index.md` 收录。
- README 的“手动检查更新与发布”章节增加指南入口，但不复制完整内部操作清单。
- 指南使用 `<新版本>`、`<上一版本>` 等占位符，不把 `v0.1.2` 的一次性恢复验收文案误写为未来固定流程。
- 指南明确列出 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、发布说明和发布结构测试的同步检查。
- 指南提供实际 PowerShell 命令，覆盖完整检查、无私钥普通构建、差异检查、提交推送、产物枚举和 SHA-256。
- 指南说明 GitHub Actions 的触发路径、所需 Secret 名称、Draft 核对项目和人工发布门禁。
- 指南说明发布后公开端点、应用内升级、安装目录、重启和数据保留验证，以及未执行场景不得声称成功。
- 指南明确 Draft 可删除重建；已公开版本不得原地替换资产，必须发布更高 SemVer 修复。
- 文档不得包含任何真实密钥、Token、Authorization Header 或认证文件内容。

## 验收标准

- [x] 发布规范目录存在一份可独立阅读、按阶段执行的简体中文发布指南。
- [x] 发布规范索引和 README 都能导航到该指南。
- [x] 指南与当前 `release.yml`、updater 配置、版本来源、Secret 名称和 Draft 行为一致。
- [x] 指南区分本地普通构建、GitHub 签名发布构建、Draft 发布和真实系统升级证据。
- [x] 指南包含失败与回滚边界，且没有把未使用的 Authenticode 描述为已签名。
- [x] 相对链接、Markdown 结构、`git diff --check` 和项目完整检查通过；真实失败或未执行项如实记录。

## 范围外事项

- 不修改 GitHub Actions、Tauri updater、NSIS 或产品逻辑。
- 不创建、轮换或读取 GitHub Actions Secrets。
- 不触发工作流，不创建 Draft Release，不公开 Release，不执行真实安装或应用内升级。
