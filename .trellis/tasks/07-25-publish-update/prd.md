# 发布 Codex Relay v0.2.0

## 目标

把 `v0.1.2` 之后已经完成的产品改动发布为新的 Windows NSIS 安装包和 Tauri 应用内更新，使现有用户能够从公开 Release 获取本轮新增能力与修复。

## 已确认事实

- 当前公开正式版本为 `v0.1.2`，发布时间为 2026-07-21，且 `draft=false`、`prerelease=false`。
- 当前源码版本为 `0.1.2`；`package.json`、`package-lock.json`、`src-tauri/Cargo.toml` 和 `src-tauri/Cargo.lock` 一致。
- 本地 `master` 与最新 `origin/main` 无反向差异，并领先 66 个提交；工作区在创建本任务前干净。
- 发布工作流是 `.github/workflows/release.yml` 的手动 `workflow_dispatch`，先创建 Draft Release。
- GitHub Actions 已配置 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 两个 Secret 名称；本任务不读取或记录其值。
- 本轮包含多项向后兼容的新功能与修复，按 SemVer 采用次版本 `0.2.0`。
- 用户已明确要求“发布更新”，构成实施与公开发布授权；仍须在发布前满足仓库规定的 Draft 审计门禁。

## 需求

### R1 版本与说明一致

- 将 npm、Cargo 和锁文件中的应用版本统一提升为 `0.2.0`。
- 更新发布结构测试中的版本契约。
- 将工作流 `releaseBody` 改为可直接公开的简体中文最终说明，准确概括 `v0.1.2 → v0.2.0` 的用户可见变化。
- 发布说明必须保留未启用 Windows Authenticode、可能显示“未知发布者”，以及升级不主动删除 Codex 配置、Codex Relay 应用数据、日志或备份的提示。

### R2 发布候选质量门禁

- 发布候选必须通过本轮完整 `npm run check`。
- 显式移除当前进程中的 updater 私钥环境变量后，普通 `npm run build` 必须成功，且只把实际生成的 Release 主程序与 NSIS 安装器作为构建证据。
- 必须记录实际产物路径、大小、最后写入时间与 SHA-256；普通构建不能被描述为 updater 签名、安装或升级成功。
- 提交前必须通过差异检查、任务校验和秘密/真实用户路径审计。

### R3 Draft Release

- 精确提交并推送本次发布候选到远端 `main`，不得混入未审查的无关改动。
- 手动触发“发布 Windows 更新”工作流，并确认其检出候选提交、完整检查和 Draft 构建均成功。
- Draft 必须为 `v0.2.0`、`draft=true`、`prerelease=false`，目标提交与候选一致。
- Draft 资产必须包含预期 NSIS、对应 `.sig` 和 `latest.json`；版本、说明、平台 URL、内联签名和独立签名必须一致。
- Draft 审计只记录非秘密元数据、大小与 SHA-256，不输出 Token、私钥、密码、认证 Header 或用户数据。

### R4 公开发布与复核

- Draft 全部核对通过后公开 Release；发现任何版本、说明、签名或资产问题时停止公开并重新生成更高质量的 Draft。
- 公开后确认 `releases/latest`、目标 Tag 和 `latest.json` 均指向 `v0.2.0`，且公开资产与 Draft 审计结果没有漂移。
- 不得原地替换已经公开的同版本资产；公开后发现缺陷时只能发布更高 SemVer 修复。

### R5 隔离升级证据

- 在 Windows Sandbox 或隔离 VM 中，以公开 `v0.1.2` 为基线验证应用内升级到 `v0.2.0`。
- 所有测试数据必须位于安全临时 staging，并成对覆盖 `CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR`；不得使用真实 `%USERPROFILE%\.codex`、`%LOCALAPPDATA%\CodexRelay` 或真实密钥。
- 核对实际版本、安装目录、应用重启、受保护 fixture 的长度与 SHA-256；UAC、取消、断网、错误签名和下载失败等未执行场景必须明确标为未验证。

## 验收标准

- [ ] npm、主 Tauri crate、`codex-relay-core` crate、锁文件与发布结构测试一致为 `0.2.0`。
- [ ] 最终中文发布说明准确覆盖本轮主要新增能力、升级方式、未知发布者与数据保留边界。
- [ ] 本轮 `npm run check` 退出码为 0，测试数量和真实失败均已记录。
- [ ] 无 updater 私钥的普通 `npm run build` 退出码为 0，并记录实际 EXE/NSIS 产物元数据与 SHA-256。
- [ ] 候选提交已推送到远端 `main`，GitHub Actions 发布工作流成功生成经核对的 Draft。
- [ ] Draft 的版本、目标提交、说明、NSIS、`.sig`、`latest.json`、平台 URL 与签名关系全部一致。
- [ ] `v0.2.0` 已公开，公开端点与资产复核通过且未发生漂移。
- [ ] 已在隔离环境执行 `v0.1.2 → v0.2.0` 应用内升级；若环境或人工交互阻止完成，报告中明确保留未验证项，不虚报成功。
- [ ] Git、日志、任务材料、测试输出与发布说明不含真实密钥、完整认证文件、Authorization Header 或真实用户数据。

## 范围外

- 不启用或配置 Windows Authenticode。
- 不更换 Tauri updater 公钥、私钥或固定更新 endpoint。
- 不修改安装范围、默认目录、卸载数据保留策略或产品功能逻辑；发布过程中发现产品缺陷时停止发布并另行修复。
- 不执行真实用户配置或应用数据的迁移、清理或删除。
