# 发布 Codex Relay v0.2.1

## 目标

把设置页更新信息展示修复提交并发布为 Windows NSIS 安装包与 Tauri 应用内更新，使 `v0.2.0` 用户获得友好的发布日期和安全的 Markdown 更新说明。

## 已确认事实

- 当前公开正式版本为 `v0.2.0`，`draft=false`、`prerelease=false`，目标提交为 `d745f285e4d0d12301a31a5f318b30952b3fba33`。
- 本次用户可见变化仅是设置页更新发布日期本地化，以及 Release Markdown 说明的安全渲染，按 SemVer 使用补丁版本 `0.2.1`。
- 本地 `master` 与 `origin/main` 同步，待提交改动仅包含本次更新面板修复、测试、Markdown/HTML 清理依赖与本任务材料。
- 发布工作流为 `.github/workflows/release.yml` 的手动 `workflow_dispatch`，只先生成 Draft Release。
- GitHub Actions 的 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` Secret 名称存在；任务不得读取或记录其值。
- GitHub Actions 状态当前正常，远端不存在 `v0.2.1` Release 或 Tag，也没有活动发布 Run。
- 用户明确要求“提交并发布更新”，构成提交、推送、触发工作流、审计 Draft 和公开 Release 的授权。

## 需求

### R1 提交产品修复

- 提交设置页友好日期显示、安全 Markdown 渲染、响应式样式、依赖与回归测试。
- Markdown 输出必须在进入 `v-html` 前转义原始 HTML并使用 DOMPurify 清理；危险元素、图片和 `javascript:` URL 不得执行或加载。

### R2 准备 v0.2.1 候选

- 将 npm、主 Tauri crate、`codex-relay-core` crate 与 Cargo lock 版本统一为 `0.2.1`。
- 更新发布结构测试，要求 `v0.2.0 → v0.2.1` 和本轮最终说明。
- 工作流 `releaseBody` 必须使用可直接公开的简体中文最终说明，包含本轮变化、升级方式、未知发布者和数据保留边界。

### R3 本地发布门禁

- 发布候选必须通过完整 `npm run check`。
- 显式移除当前进程的 updater 签名环境变量后，普通 `npm run build` 必须成功。
- 记录实际 Release 主程序与 NSIS 安装器路径、大小、时间和 SHA-256；普通构建不得描述为 updater 签名、安装或升级成功。
- 通过任务校验、差异检查、依赖审计、秘密扫描和真实用户路径审计。

### R4 候选提交、Draft 与公开发布

- 精确提交并推送候选到远端 `main`，确认远端 SHA 一致，不手动创建 Tag。
- 触发唯一的“发布 Windows 更新”工作流并等待完成；失败时保留证据、停止公开并修复后用新提交重试。
- Draft 必须为 `v0.2.1`、目标提交与 Run 一致、恰含 NSIS、`.sig`、`latest.json` 三项资产；说明、版本、平台 URL、内联签名与独立签名必须一致。
- Draft 审计全部通过后公开并标记 Latest；公开后复核 `releases/latest`、Tag、`latest.json` 和资产摘要无漂移。

### R5 隔离升级证据

- 在 Windows Sandbox 或隔离 VM 中从公开 `v0.2.0` 验证应用内升级到 `v0.2.1`。
- 只使用系统临时 staging、成对 Relay 路径覆盖和明确假密钥，不触碰真实 `.codex` 或 Relay 应用数据。
- 记录版本、安装目录、可执行文件、数据保留、UAC 与自动重启观察；取消、断网、错误签名和下载失败等未执行场景明确标为未验证。

## 验收标准

- [x] 产品修复与回归测试已提交，Markdown 安全边界保持。
- [x] npm、两个 Cargo package、锁文件、发布结构测试和最终说明一致为 `0.2.1`。
- [x] 完整 `npm run check` 退出 0；无签名普通 `npm run build` 退出 0并记录实际产物证据。
- [x] 候选提交已推送，远端 `main` SHA 与发布 Run head SHA 一致。
- [x] GitHub Actions 成功生成经完整审计的 `v0.2.1` Draft。
- [x] `v0.2.1` 已公开并成为 Latest，公开端点及三个资产与 Draft 无漂移。
- [x] 已完成或如实记录 `v0.2.0 → v0.2.1` 隔离应用内升级验证及未验证场景。
- [x] Git、日志、任务材料、测试输出和发布说明不含真实密钥、认证文件、Authorization Header 或真实用户数据。

## 范围外

- 不启用 Windows Authenticode，不更换 Tauri updater 公钥、私钥或 endpoint。
- 不修改安装范围、安装目录、卸载或数据保留契约。
- 不原地替换已公开同版本资产；公开后发现缺陷只能发布更高 SemVer。
