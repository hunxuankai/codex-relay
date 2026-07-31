# 发布当前更新

## 目标

把当前本地 `master` 中已经完成并提交的“保持 Provider 身份同步连接”能力发布到
GitHub 默认分支 `main`，生成、审计并公开可供现有安装用户升级的 Windows
`v0.4.0` Release。

## 已确认事实

- 2026-07-31 查询到当前公开 Latest 是 `v0.3.0`，目标提交为
  `ce76f43e25fd3ca2fa9b3a2dac4a8930d2d975a1`，且不是 Draft 或 Prerelease。
- 任务创建前本地工作树干净；本地 `master` 相对 `origin/main` 领先 3 个提交，其中
  产品提交 `6d16c58` 新增保持顶层 `model_provider` 身份、仅同步其他 Provider 已选
  Base URL/API Key、显式恢复以及事务化切换复原能力，另外两个提交是该开发任务的
  归档和会话日志。
- 该改动新增用户能力并把 `provider-preferences.json` 的当前格式提升到 v4；按仓库
  SemVer 约定采用下一个次版本 `0.4.0`，不得复用已公开的 `0.3.0`。
- 当前 npm、Cargo、锁文件和发布结构测试仍指向 `0.3.0`，发布工作流说明仍描述
  `v0.2.1 → v0.3.0`，必须在本次候选中同步。
- GitHub Actions 中存在 `TAURI_SIGNING_PRIVATE_KEY` 和
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 两个 Secret 名称；本任务不读取或记录其值。
- 当前没有活动中的发布 Run；Windows Authenticode 尚未启用，安装器仍可能显示
  “未知发布者”。
- 本轮首次完整检查暴露出测试基础设施争用：默认 8 个 Vitest worker 并行运行 jsdom 与
  多个同步 PowerShell Sandbox 子进程时，一个安全负向用例从单独运行的 2.98 秒放大到
  20.313 秒并越过 20 秒测试预算，同时触发 60 秒 `onTaskUpdate` RPC 超时；限制为 4 个
  worker 后全套 230 项通过，该用例为 4.818 秒。

## 需求

### R1：准备 v0.4.0 发布候选

- 将 `package.json`、`package-lock.json`、根与 core Cargo manifest、
  `src-tauri/Cargo.lock` 中的应用包版本同步为 `0.4.0`。
- 先把 `src/release-config.test.ts` 改为本次版本与说明契约并观察预期失败，再修改版本
  和 `.github/workflows/release.yml` 使专项测试恢复通过。
- 最终简体中文发布说明必须准确描述：保持 `model_provider` 身份应用其他 Provider
  连接、结构化确认与状态提示、首次覆盖恢复点与普通切换复原、从 `v0.3.0` 升级、
  旧版降级注意事项、未知发布者提示和数据保留边界。
- 不修改 updater endpoint、公钥、Secrets 名称、NSIS 安装范围、安装目录、更新密钥、
  历史 Release 清理算法或卸载数据保留策略。

### R2：本地验证、提交与推送

- 使用安全临时目录和成对 Relay 路径覆盖运行本轮新鲜的专项发布测试及
  `npm run check`；不得读取或写入真实 `%USERPROFILE%\.codex` 与
  `%LOCALAPPDATA%\CodexRelay`。
- 在 `vite.config.ts` 固定 `maxWorkers: 4`，并以结构测试锁定该测试基础设施契约；只降低
  测试并发争用，不放宽 Sandbox 用例的 20 秒预算、生产超时或安全断言。
- 从当前 PowerShell 进程移除两个 updater 签名环境变量后运行普通
  `npm run build`，枚举 Release 主程序和 NSIS 安装器的实际路径、大小、时间与
  SHA-256；普通构建不得描述为 updater 签名、安装或升级成功。
- 运行版本搜索、`git diff --check`、状态/忽略文件/跟踪文件和秘密扫描，确认没有真实
  API Key、Authorization Header、认证文件、用户数据或构建产物进入 Git。
- 精确暂存本任务相关文件，提交并推送 `HEAD:main`，确认远端 `main` 与候选 SHA
  一致；不得手工创建 Release Tag。

### R3：生成、审计并公开 Release

- 公开前把当前 `v0.3.0` NSIS 安装器保存到系统临时目录真子路径并核对公开大小与
  SHA-256，避免历史 Release 清理后丢失升级基线。
- 触发且只保留一个“发布 Windows 更新”候选 Run；工作流必须基于候选提交成功生成
  `v0.4.0` Draft。
- 公开前核对 Draft 的 Tag、标题、目标提交、最终说明、NSIS、`.sig` 和
  `latest.json`；记录三个资产的实际大小与 SHA-256，并确认清单版本、说明、平台 URL
  和内联签名与独立 `.sig` 一致。
- 只有全部 Draft 门禁通过后才公开 Release；公开后确认 `releases/latest` 和固定
  `latest.json` 均返回 `v0.4.0`，且公开资产没有相对 Draft 漂移。
- 等待并核对“清理历史 GitHub Releases”工作流成功，只保留当前 Latest Release 与
  Tag；远端历史资产清理不得描述成本机用户数据清理。

### R4：保留真实升级证据边界

- 优先使用仓库 Sandbox 准备器，以保存的 `v0.3.0` 安装器和 `v0.4.0` 目标版本生成
  安全 staging；两项 Relay 覆盖必须成对设置，fixture 只能使用
  `test-key-*-not-real`。
- 若当前环境允许，再在 Windows Sandbox 或隔离 VM 中执行 `v0.3.0 → v0.4.0` 应用内
  升级并读取 `before.json` / `after.json`；不得访问真实 Codex/Relay 用户目录。
- 安装、UAC、重启、Tauri 验签或升级场景若受环境限制未执行或失败，必须如实记录，
  不得用本地构建、CI 成功或公开托管状态替代成功证据。

## 验收标准

- [x] AC1：所有权威版本来源、最终发布说明和结构测试一致指向 `0.4.0`，且严格高于
      当前公开 `v0.3.0`。
- [x] AC2：专项测试先因本次预期原因失败，完成版本与说明修改后通过；本轮完整检查、
      普通构建、差异与安全审计通过，实际普通构建产物已记录 SHA-256。
- [x] AC3：候选提交已精确推送到 GitHub `main`，远端提交与发布工作流输入一致。
- [x] AC4：`v0.3.0` 基线安装器已安全暂存并核对；发布工作流成功生成经逐项审计的
      `v0.4.0` Draft，资产与清单满足 updater 契约且不含秘密或用户数据。
- [x] AC5：`v0.4.0` 已公开为 Latest，公开端点与资产复核一致，历史 Release/tag 清理
      Run 成功且只保留当前 Latest。
- [x] AC6：实际执行的 Sandbox/VM 准备与升级结果、真实失败和所有未执行项均以本轮
      证据记录，没有越权声明安装、签名或升级成功。

## 范围外

- 不新增或修改产品功能行为；本任务只准备并执行现有改动的发布。
- 不更换 updater 公钥或私钥，不查看、复制或输出 GitHub Actions Secret 值。
- 不启用或宣称 Windows Authenticode 签名。
- 不原地替换任何已公开版本的二进制、`.sig` 或 `latest.json`。
