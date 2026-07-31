# v0.4.0 发布设计

## 发布边界

本任务沿用现有 GitHub Actions 发布链路，只同步版本、发布结构测试和最终说明，不改变
应用架构、updater 信任根、NSIS 模板、安装/卸载行为或历史清理算法。权威应用版本来自
`package.json`，package lock、根与 core Cargo manifest、Cargo lock 和结构测试跟随同步；
`tauri-action` 依据该版本创建 `v0.4.0` Tag、Draft Release 和 updater 资产。

## 文件与职责

- `src/release-config.test.ts`：先固定 `0.4.0`、`v0.3.0 → v0.4.0` 和本次最终说明的
  结构契约；同时固定 Vitest worker 上限，形成可观察的红色测试。
- `vite.config.ts`：把 Vitest worker 上限固定为 4，避免 jsdom 与同步 PowerShell Sandbox
  子进程在 8 核 Windows 主机上争用并触发测试预算/RPC 超时；不修改单测试和生产超时。
- `package.json`、`package-lock.json`：npm 权威版本及锁文件根包版本。
- `src-tauri/Cargo.toml`、`src-tauri/crates/codex-relay-core/Cargo.toml`、
  `src-tauri/Cargo.lock`：Rust 根包、core 包及锁文件版本。
- `.github/workflows/release.yml`：公开 Release 与 `latest.json.notes` 共用的最终简体中文
  发布说明，仍保持手动触发和 `releaseDraft: true`。
- 当前 Trellis 任务材料：保存非秘密规划、验证、候选、Draft、公开、清理和未执行场景
  证据。

## 状态流

1. 本地红色门禁：更新结构测试并确认它只因旧版本和旧说明失败。
2. 本地候选：同步 `0.4.0` 与最终说明，运行专项测试、完整检查和无 updater 私钥的
   普通构建。
3. 远端候选：精确提交并推送到 `main`，固定候选 SHA；同时把 `v0.3.0` 安装器保存到
   安全临时目录。
4. Draft：手动触发唯一发布 Run，等待它在同一候选 SHA 上完成完整检查、构建和 updater
   签名资产生成。
5. 审计：读取 Draft 元数据，把三个资产下载到系统临时目录，核对大小、SHA-256、
   `latest.json` 版本/说明/平台 URL、独立 `.sig` 和清单内联签名。
6. 公开：仅在审计全部通过后公开；再次读取 Latest 和公开资产，确认没有漂移。
7. 清理：等待独立清理工作流，确认只保留 `v0.4.0` Release/Tag。
8. 升级：使用公开前保存的 `v0.3.0` 安装器准备安全 Sandbox；环境允许时执行真实应用内
   升级。构建、托管、签名关联、安装和升级证据始终分别记录。

## 发布说明契约

Release 正文与 `latest.json.notes` 使用同一份最终文案，至少包括：

- 可保持顶层 `model_provider` 身份，把另一 Provider 已选中的 Base URL 与 API Key
  应用为当前连接，以便沿原 Provider 身份继续使用既有会话。
- 卡片提供“仅应用连接 / 已应用 / 更新连接 / 恢复自身连接”状态和结构化确认；普通
  前端状态不显示 API Key 值。
- 首次覆盖会固定恢复条目；普通切换 Provider 时先恢复旧目标，再应用新 Provider。
- 已安装 `v0.3.0` 的用户可通过应用内更新或 GitHub Releases 升级到 `v0.4.0`。
- 活动连接覆盖使用 v4 私有偏好；降级到不理解 v4 的旧版前应先恢复自身连接并保留备份。
- Windows Authenticode 未启用，可能显示“未知发布者”；升级不会主动删除 Codex 配置、
  Relay 应用数据、日志或备份。

## 安全与数据边界

- GitHub 认证只由 `gh` 和 Actions 自身凭据处理；任务、命令输出和日志不得包含 Token、
  私钥、密码、完整认证文件或用户数据。
- 本地检查使用系统临时目录中的成对 `CODEX_RELAY_CODEX_HOME` 与
  `CODEX_RELAY_APP_DATA_DIR` 覆盖；禁止访问真实 `.codex` 与 Relay 应用数据。
- 下载与 Sandbox staging 必须位于系统临时目录真子路径，不得经过 reparse point；fixture
  只使用 `test-key-*-not-real`。
- 历史 Release/tag 清理只属于公开资产保留策略，不触及本机配置、日志、备份或密钥。

## 兼容性与回滚

- `v0.4.0` 严格高于 `v0.3.0`，现有 updater 可通过固定 Latest 清单发现它。
- 本次产品数据格式 v4 由待发布代码提供 v1/v2/v3 只读兼容；若已经创建活动连接覆盖，
  降级到旧版前应先在支持 v4 的版本中恢复自身连接并保留备份。
- 本地检查失败时不提交；发布工作流失败时记录失败步骤并用修复后的新提交重试。
- Draft 审计失败时不公开，删除错误 Draft/资产后修正源码并重新生成。
- 正式公开后不替换同版本资产；发现缺陷时发布更高 SemVer 修复版本。
- 清理工作流部分失败时保留真实列表，通过其手动入口安全重试，不使用通配符删除。
