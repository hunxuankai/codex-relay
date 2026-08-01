# 发布控制台网络与仓库同步实施计划

> **For agentic workers:** REQUIRED SUB-SKILL：使用 `trellis-before-dev` 与 `superpowers:test-driven-development`，按 Codex inline 执行；不得派发写入或检查子 Agent。

**Goal:** 交付跨启动 HTTP/SOCKS5 代理、双通道连接测试、结构化仓库同步、安全 Push 和自动会话恢复。

**Architecture:** 前端版本化偏好是单一设置来源；Rust `release_network` 统一验证并生成 Git/`gh` 运行配置；所有网络动作捕获 typed 设置快照，安全 Push 在后端重新验证全部事实。

**Tech Stack:** Vue 3 `<script setup lang="ts">`、Element Plus、Vitest/Vue Test Utils、Tauri 2、Rust 2024、Serde、Tokio、Git/GitHub CLI 子进程。

---

## 目标与技术栈

- 执行方式：Codex inline。每个行为切片严格执行 RED（目标测试因行为缺失而失败）→ GREEN（最小实现）→ REFACTOR。

## 文件职责映射

### 新增

- `tools/release-console/src/types/network.ts`：前端代理与连接测试 DTO。
- `tools/release-console/src/composables/useReleaseProxyPreference.ts`：代理设置持久化与只读状态。
- `tools/release-console/src/composables/useReleaseProxyPreference.test.ts`：存储恢复、损坏回退、更新持久化。
- `tools/release-console/src/composables/useReleaseNetwork.ts`：连接测试资源状态和请求序列。
- `tools/release-console/src/composables/useReleaseNetwork.test.ts`：双项结果、稳定错误、晚响应丢弃。
- `tools/release-console/src/components/release/ProxySettingsPanel.vue` 与 `.test.ts`：代理表单、校验文案、测试结果。
- `tools/release-console/src/components/release/RepositorySyncConfirmDialog.vue` 与 `.test.ts`：安全 Push 确认。
- `tools/release-console/src/components/release/ReleaseRecoveryPanel.vue` 与 `.test.ts`：会话阶段动作投影。
- `tools/release-console/src-tauri/src/services/release_network.rs`：Rust 代理校验、URL 与环境/Git模式构造。
- `tools/release-console/src-tauri/tests/release_network.rs`：跨边界代理与连接探针测试。

### 修改

- `tools/release-console/src/types/release.ts`：同步状态、提交摘要、Push 预览与 connection DTO 引用。
- `tools/release-console/src/services/tauri.ts` 与 `.test.ts`：新增连接测试、安全 Push及所有网络动作的 proxy 参数。
- `tools/release-console/src/composables/useReleaseSession.ts` 与 `.test.ts`：代理快照、safePush、启动检测与恢复动作。
- `tools/release-console/src/components/release/RepositorySetupPanel.vue` 与 `.test.ts`：同步事实和条件 Push 入口。
- `tools/release-console/src/App.vue` 与 `.test.ts`：组件编排、启动自动检测、确认对话框与恢复入口。
- `tools/release-console/src/style.css`：仅在需要时增加共享响应式变量；不覆盖 Element Plus 私有状态颜色。
- `tools/release-console/src-tauri/src/models.rs`：代理、连接结果、同步状态、提交摘要和 Push 请求/预览 DTO。
- `tools/release-console/src-tauri/src/infrastructure/process.rs` 与测试：显式代理覆盖环境。
- `tools/release-console/src-tauri/src/infrastructure/git.rs`：Git Direct/Custom 临时配置和错误分类。
- `tools/release-console/src-tauri/src/infrastructure/gh.rs`：连接测试 operation 与稳定内部失败标记。
- `tools/release-console/src-tauri/src/services/git_release.rs` 与测试：结构化关系、Fetch 分类、安全 Push。
- `tools/release-console/src-tauri/src/services/release_application.rs`：网络快照贯穿检查/计划/开始/恢复/公开，连接测试与 Push command。
- `tools/release-console/src-tauri/src/app_state.rs`、`commands.rs`、`lib.rs` 与 command 测试：新增 typed request/response/commands。
- `.trellis/spec/release/publishing.md`：完成后记录代理、安全同步与自动恢复契约。

## 行为切片与公开接口

### 切片 1：代理偏好持久化

公开接口：`useReleaseProxyPreference({ storage? })`。

- [x] 在 `useReleaseProxyPreference.test.ts` 写 RED：v1 设置恢复；损坏/未知版本/非法字段回退；`update` 跨启动保存；关闭后保留字段；序列化结果没有 username/password/token 字段。
- [x] 运行：

  ```powershell
  npm run test --workspace @codex-relay/release-console -- --run src/composables/useReleaseProxyPreference.test.ts
  ```

  预期：因模块不存在而失败。
- [x] 最小实现 `types/network.ts` 与 composable，使用 `shallowRef`、`readonly`、显式动作和 storage 异常容错。
- [x] 重跑专项测试，预期全部通过；再运行 `npm run typecheck:release-console`。

### 切片 2：Rust 代理校验与运行环境

公开接口：`ReleaseProxySettings::profile()` / `ReleaseNetworkProfile::environment()` / `GitProxyMode`。

- [x] 在 `release_network.rs` 单元测试和 `tests/release_network.rs` 写 RED：HTTP/SOCKS5、域名/IPv4/IPv6、端口、协议/userinfo/路径拒绝；开启覆盖继承代理；关闭清空代理；Git Direct/Custom 参数精确。
- [x] 运行：

  ```powershell
  cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml release_network -- --nocapture
  ```

  预期：因类型/模块不存在而失败。
- [x] 在 `models.rs` 新增 camelCase DTO；新增 `services/release_network.rs`；给 release-console crate 增加已锁定的 `url` 依赖；扩展 `GitBackend` 为显式 `GitProxyMode`。
- [x] 重跑专项 Rust 测试；现有 Git/环境测试同步改为显式 Direct/Custom fixture 后保持绿色。

### 切片 3：双通道连接测试

公开接口：Tauri `test_release_connection(proxy)`；前端 `releaseConsoleTauri.testConnection` 与 `useReleaseNetwork.test`。

- [x] 先在 Rust command/application 测试写 RED：一次请求返回 Git/gh 两项独立结果、耗时、稳定 code；一项失败不覆盖另一项；无仓库写入。
- [x] 先在 `services/tauri.test.ts` 和 `useReleaseNetwork.test.ts` 写 RED：camelCase 参数、结果透传、设置变化后旧结果失效、晚响应不覆盖。
- [x] 运行 Rust/前端专项命令，确认分别因 command/composable 缺失失败。
- [x] 新增 `GhOperation::ConnectionTest` 与固定公开 API；在 `SystemReleaseApplication` 使用相同 `ReleaseNetworkProfile` 并行执行两个 probe。
- [x] 实现 typed Tauri 方法与 `useReleaseNetwork`，重跑所有专项测试至绿色。

### 切片 4：代理设置组件

公开接口：`ProxySettingsPanel` props `settings/result/busy/error`；emits `update:settings` 与 `test`。

- [x] 在组件测试写 RED：开关、HTTP/SOCKS5、地址、端口、直连/代理说明、无效原因、测试按钮、Git/gh 分项结果、窄布局类和键盘标签。
- [x] 运行该组件测试，确认因组件不存在失败。
- [x] 用 Element Plus typed 控件实现纯组件；App 通过 `useReleaseProxyPreference` 和 `useReleaseNetwork` 编排。
- [x] 重跑组件、App 专项测试和 typecheck。

### 切片 5：结构化仓库同步状态

公开接口：`RepositoryInspection.sync`、`ReleasePreflightResult.releaseReady` 与可选 `safePush`。

- [x] 扩展 `tests/git_release.rs` 写 RED：同步、领先、落后、分叉、领先提交顺序与主题、脏工作区事实；不再把所有 HEAD 不一致折叠为 `GIT_HEAD_REMOTE_MISMATCH`。
- [x] 运行目标测试，确认 DTO/行为缺失导致失败。
- [x] 使用 `rev-list --left-right --count` 与固定格式 `git log` 实现最小关系投影；application 结合远端运行/Draft 冲突生成唯一 `safePush` 预览和阻止原因。
- [x] 更新 Rust/TS fixtures 与 RepositorySetupPanel 测试，重跑 Rust/前端专项套件。

### 切片 6：安全 Push 后端

公开接口：Tauri `push_release_repository(request)`，返回刷新后的 `ReleasePreflightResult`。

- [x] 在 Git 集成测试写 RED：只领先成功；脏、落后、分叉拒绝；预期 HEAD/remote 不符拒绝；确认后远端移动拒绝；精确 SHA 推送成功；无 force/Tag/其他分支。
- [x] 在 command 测试写 RED：typed request 只包含仓库、两个预期 SHA 和代理设置，并调用 application 一次。
- [x] 运行专项测试确认失败原因正确。
- [x] 在 `GitReleaseService` 增加独立于发布候选的 `push_existing_commits`；application 在 Push 前重新执行包含 Fetch、工具链与 GitHub 冲突检查的完整预检，再使用精确 RefSpec，成功后再次完整检查。
- [x] 重跑 Git、command、application 相关测试至绿色。

### 切片 7：安全 Push 前端

公开接口：`RepositorySetupPanel` emit `requestPush`；`RepositorySyncConfirmDialog` emit `confirm/cancel`；`useReleaseSession.safePush`。

- [x] 写 RED：仅 safePush 预览存在时显示按钮；确认对话框展示远端/SHA/提交主题/禁止范围；取消无 IPC；确认传递预期 SHA和当前代理；成功刷新 inspection；失败保留原预览与稳定错误。
- [x] 运行组件/composable/App 专项测试确认失败。
- [x] 实现组件、composable 与 App 编排；对话框关闭遮罩、默认安全焦点、Escape/焦点恢复遵循现有 PublishConfirmDialog 模式。
- [x] 重跑专项测试与 typecheck。

### 切片 8：自动会话检测与阶段恢复 UI

公开接口：App 启动行为；`ReleaseRecoveryPanel` props/session 与 typed emits。

- [x] 写 RED：已记住路径启动后只调用本地 `get_release_session`；无会话隐藏；本地中断/committed/远端阶段/等待公开/终态分别显示正确动作；检测不触发 inspect/fetch；恢复/公开传当前代理。
- [x] 运行 App、panel、composable 测试确认失败。
- [x] 实现一次性启动检测和纯展示组件；移除常驻“加载活动会话”，保留现有恢复状态机。
- [x] 重跑专项测试与 typecheck。

### 切片 9：精确错误分类

公开接口：`GitBackendError::code`、Fetch/connection/public application errors。

- [x] 写 RED：ProcessStart、Timeout、Cancelled、ProcessTreeTermination、CommandFailed 映射不同稳定码；Fetch/Push/GitHub API 保留阶段；公开消息不含测试代理 URL、Bearer、Authorization 或 stderr fixture。
- [x] 运行 Rust 专项测试确认旧泛化映射导致失败。
- [x] 最小重构 GitBackend/SystemGhBackend 与 application 映射；不记录原始 stderr。
- [x] 重跑 Rust 专项与所有 release-console Rust tests。

### 切片 10：规范、全量验证与打包

- [x] 更新 `.trellis/spec/release/publishing.md` 的代理、安全同步、连接测试和自动恢复契约及必需测试。
- [x] 运行前端专项总检查：

  ```powershell
  npm run typecheck:release-console
  npm run test:release-console
  ```

- [x] 运行 Rust 格式、Clippy 与 release-console 测试：

  ```powershell
  cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
  cargo clippy --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --all-targets --all-features -- -D warnings
  cargo test --manifest-path tools/release-console/src-tauri/Cargo.toml
  ```

- [x] 运行项目完整门禁：

  ```powershell
  npm run check
  ```

- [x] 重新打包：

  ```powershell
  npm run build:release-console
  ```

- [x] 枚举 `dist/release-console/CodexRelayReleaseConsole.exe` 的完整路径、大小、最后写入时间和 SHA-256；确认源/交付 EXE 哈希一致。
- [ ] 运行 `git diff --check`、`git status --short --ignored`、相关差异与秘密/真实路径审计；精确暂存本任务文件并提交简体中文 Conventional Commit。

### 本轮验证证据

- `npm run typecheck:release-console`：退出 0。
- `npm run test:release-console`：16 个测试文件、61 项测试通过。
- `cargo fmt --all --check --manifest-path src-tauri/Cargo.toml`：退出 0。
- `cargo clippy --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --all-targets --all-features -- -D warnings`：退出 0。
- `npm run check`：退出 0；Trellis 8 项、前端 59 个文件/309 项、发布控制台 16 个文件/61 项及整个 Rust workspace 通过。
- `npm run build:release-console`：退出 0。
- 源与交付 EXE：`12,658,688` 字节，最后写入时间 `2026-08-01T22:04:44.7396218+08:00`，SHA-256 均为
  `66C85FC2454C2B87CA63719431D1C012994FDE59CB741B08F40C7E5932CE2658`。
- `git diff --check`：退出 0；高置信度秘密前缀、真实用户目录和真实工作区路径扫描无命中。

## Mock 边界

- 前端组件只 mock typed Tauri client 或 composable，不 mock Vue 私有响应式实现和 Element Plus 私有 DOM。
- Git 关系与 Push 使用真实临时 bare repository，不 mock Git 命令顺序；远端竞态通过第二个临时 clone 制造。
- Process 超时/启动错误使用安全假 executable 或现有 JobFactory 测试点；不访问真实网络。
- GitHub 远端业务继续使用 `GhBackend` fixture；连接 probe 的进程分类在基础设施边界确定性测试。
- 自动会话检测使用临时 Git dir 与真实 `ReleaseStateStore`；前端 App 测试只 mock typed command 返回。

## 回滚点

- 代理偏好使用独立 localStorage key，可单独移除而不影响仓库偏好。
- 不修改 Git/Windows 全局配置，失败或回滚不需要恢复外部状态。
- 安全 Push 前所有检查均只读；一旦 Push 成功不执行伪回滚，按远端真实状态报告。
- 发布 session schema 不迁移；若恢复 UI 回滚，现有 `session.json` 仍可由旧逻辑读取。
