# 发布控制台网络与仓库同步设计

## 1. 设计目标

在不把发布控制台扩展成通用 Git 客户端、不修改用户全局 Git/Windows 配置、也不保存任何认证秘密的前提下，增加三类能力：

1. 显式、跨启动持久化的 HTTP/SOCKS5 代理设置与双通道连接测试；
2. 对本地严格领先 `origin/main` 的提交执行可审计、无 force 的安全同步；
3. 启动时本地检测并按阶段恢复持久化发布会话。

这三类能力共享同一个网络设置快照、稳定错误体系和仓库事实模型，因此在一个任务内实施；每个行为切片仍可独立测试。

## 2. 总体架构

### 2.1 单一网络设置来源

前端新增版本化代理偏好 composable，作为界面内唯一设置来源：

```typescript
type ReleaseProxyType = 'http' | 'socks5'

interface ReleaseProxySettings {
  enabled: boolean
  proxyType: ReleaseProxyType
  host: string
  port: number | null
}
```

偏好键固定为：

```text
codex-relay-release-console.proxy-preference.v1
```

localStorage 只保存以上四个非认证字段。损坏 JSON、未知版本、无效类型和存储异常回退默认值：`enabled=false`、`proxyType=http`、空地址、空端口。关闭开关仍保留地址与端口。

每个可能访问 GitHub 的 typed Tauri 调用都接收当前 `ReleaseProxySettings` 快照。Rust 不维护可漂移的全局代理状态；统一由 `release_network` 模块校验 DTO、生成安全代理 URL，并构造 Git/`gh` 共用的过滤环境。后台管线在动作开始时捕获一次快照；恢复、继续 Push 和公开操作重新使用用户当前设置。

### 2.2 Rust 网络配置边界

Rust DTO 使用 camelCase 序列化：

```rust
pub enum ReleaseProxyType { Http, Socks5 }

pub struct ReleaseProxySettings {
    pub enabled: bool,
    pub proxy_type: ReleaseProxyType,
    pub host: String,
    pub port: Option<u16>,
}
```

新增 `services/release_network.rs`，职责仅包括：

- 用 `url::Host` 校验域名、IPv4、IPv6；拒绝协议、路径、user info、查询和片段；
- `enabled=true` 时要求端口存在且为 `1–65535`，渲染 `http://host:port` 或 `socks5://host:port`；
- 先调用现有环境白名单，再移除大小写不敏感的 `HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY`、`NO_PROXY`；
- 启用时加入统一的 `HTTP_PROXY` 与 `HTTPS_PROXY`；关闭时保持这些变量缺失；
- 所有 Git 网络动作加入 `GIT_TERMINAL_PROMPT=0` 与 `GCM_INTERACTIVE=Never`，避免隐藏窗口等待不可见认证提示；
- 为 GitBackend 生成 `Direct` 或 `Custom(url)` 模式。

不修改 Windows 环境变量或 Git 配置文件。GitBackend 对每条命令前置临时配置：

```text
git -c http.proxy=<值或空> -c https.proxy=<值或空> <原参数>
```

因此开关关闭能覆盖 Git 全局代理，打开也能覆盖继承环境与全局配置。`gh` 只消费过滤后的显式环境。

## 3. 前端组件与状态边界

### 3.1 组件图

- `App.vue`：只组合偏好、资源 composable 与各发布面板；负责启动时触发一次会话检测。
- `components/release/ProxySettingsPanel.vue`：纯表单和连接测试结果展示；typed props/emits，不调用 Tauri、不访问 storage。
- `components/release/RepositorySetupPanel.vue`：保留仓库/版本输入与同步状态摘要，不拥有 Push 对话框状态。
- `components/release/RepositorySyncConfirmDialog.vue`：展示远端、SHA、提交清单与安全说明；取消/确认事件上抛，默认聚焦取消按钮，禁止遮罩关闭。
- `components/release/ReleaseRecoveryPanel.vue`：根据持久化阶段展示“取消并验证回滚”“继续 Push”“继续监控”“查看并确认公开”或“查看上次结果”。
- `composables/useReleaseProxyPreference.ts`：版本化持久化与显式 `update`；返回只读设置。
- `composables/useReleaseNetwork.ts`：管理连接测试的 busy、两项结果和稳定错误；不复制代理设置。
- `composables/useReleaseSession.ts`：继续拥有检查、计划、Push、发布和恢复状态；从只读代理设置 Ref 获取每次动作快照。

所有 Vue 文件继续使用 Composition API、`<script setup lang="ts">`、props down/events up。Element Plus 使用 `ElSwitch`、`ElSelect`、`ElInput`、`ElInputNumber`、`ElAlert`、`ElTag` 与 `ElDialog`；控件保持项目 36px/32px 点击目标和窄窗口单列布局。

### 3.2 代理交互

- 开关关闭时，类型/地址/端口仍可编辑并持久化；“测试连接”测试当前实际模式，即直连。
- 开关打开且字段无效时，检查、计划、Push、继续/公开和连接测试都在前端即时禁用并显示原因；Rust 同时执行防御校验。
- 连接测试不是发布门禁；上一次测试结果在设置改变后立即失效，避免展示过期成功。

## 4. 连接测试

新增 Tauri command `test_release_connection`，输入只有代理设置，输出：

```typescript
interface ConnectionProbeResult {
  success: boolean
  code: string | null
  message: string
  durationMillis: number
}

interface ReleaseConnectionTestResult {
  git: ConnectionProbeResult
  github: ConnectionProbeResult
}
```

后端分别执行：

```text
git ls-remote --exit-code https://github.com/hunxuankai/codex-relay.git refs/heads/main
gh api repos/hunxuankai/codex-relay --silent
```

两项均为公开只读访问，不需要仓库目录，不触发 workflow。使用安全临时工作目录并可并行运行；结果独立返回，一项失败不抹掉另一项证据。

## 5. 仓库同步事实与安全 Push

### 5.1 结构化同步状态

`RepositoryInspection` 增加：

```rust
pub enum RepositorySyncStatus { Synced, Ahead, Behind, Diverged }

pub struct RepositoryCommitSummary {
    pub sha: String,
    pub subject: String,
}

pub struct RepositorySyncInspection {
    pub status: RepositorySyncStatus,
    pub ahead_count: u32,
    pub behind_count: u32,
    pub ahead_commits: Vec<RepositoryCommitSummary>,
}
```

仓库检查 Fetch 后使用：

```text
git rev-list --left-right --count refs/remotes/origin/main...HEAD
git log --format=%H%x00%s refs/remotes/origin/main..HEAD
```

生成权威状态。远端错误、仓库无效和远端身份不符仍是 command error；工作区脏、领先、落后、分叉属于可展示的仓库事实。`ReleasePreflightResult` 增加 `releaseReady`、安全阻止原因与可选 `safePush` 预览。只有全部门禁满足时才返回预览；前端不自行重建提交集合或猜测 Push 资格。

### 5.2 安全 Push command

新增 `push_release_repository`：

```typescript
interface SafeRepositoryPushRequest {
  repositoryPath: string
  expectedHeadSha: string
  expectedRemoteMainSha: string
  proxy: ReleaseProxySettings
}
```

后端流程：

1. 重新执行包含 Fetch 的完整只读预检，验证固定远端、工具链、活动 Run 和 Draft 冲突；
2. 重新验证工作区干净、HEAD 与预期一致、远端 SHA 与预期一致；
3. 重新计算状态，必须仍为只领先且提交集合非空；
4. 设置非交互式认证边界，执行：

   ```text
   git push origin <expectedHeadSha>:refs/heads/main
   ```

5. 读取远端 `main`，必须精确等于 `expectedHeadSha`；
6. 重新执行完整仓库检查并返回新的 `ReleasePreflightResult`。

不接受分支名、任意远端、force、Tag 或自定义 RefSpec 输入。远端在确认后移动返回 `GIT_REMOTE_MOVED`，Push 非零返回 `GIT_PUSH_FAILED`，远端验证不一致返回 `GIT_REMOTE_VERIFICATION_FAILED`。

## 6. 发布会话检测与恢复

启动时仅对已记住且非空的仓库路径调用本地 `get_release_session`。Recovery 检查继续跳过 Fetch、GitHub 状态、工作区干净和 HEAD 同步门禁；检测失败只显示安全提示，不清空仓库偏好，也不启动远端动作。

前端按阶段投影动作：

- `applyingCandidate` / `localChecks` / `localBuild` / `sourceAudit`：取消并验证回滚；
- `committed`：继续 Push；
- `pushed` / `workflowQueued` / `workflowRunning` / `auditingDraft`：继续监控；
- `awaitingPublishApproval`：查看并确认公开；
- `publishing` / `verifyingPublishedRelease` / `monitoringCleanup`：继续远端收尾；
- `completed` / `completedWithWarnings` / `failed` / `cancelled`：查看上次结果；
- 无会话：不渲染恢复入口。

`resume_release` 与 `publish_release` 接收当前代理设置，因此维护者可以先修改代理，再恢复失败的远端阶段。

## 7. 错误与日志

GitBackend 保留 `ProcessError` 分类，公开映射至少包括：

- `GIT_PROCESS_START_FAILED`
- `GIT_PROCESS_TIMEOUT`
- `GIT_PROCESS_CANCELLED`
- `GIT_PROCESS_TREE_TERMINATION_FAILED`
- `GIT_COMMAND_FAILED`
- `GIT_FETCH_TIMEOUT`
- `GIT_FETCH_FAILED`
- `GIT_PUSH_FAILED`
- `GIT_REMOTE_MOVED`
- `GIT_REMOTE_VERIFICATION_FAILED`

GitHub CLI 的系统 backend 使用稳定内部标记区分启动、超时和命令失败；application/service 映射为 `GITHUB_PROCESS_START_FAILED`、`GITHUB_PROCESS_TIMEOUT` 或现有阶段错误。任何公开错误都不包含 stderr、代理 URL、环境变量值或认证内容。

## 8. 测试边界与 TDD 切片

### 前端公开行为

- `useReleaseProxyPreference`：真实内存 Storage；不 mock 私有函数。
- `ProxySettingsPanel`：通过可见标签、输入和 emitted DTO 验证；Tauri 在 service/composable 边界 mock。
- `useReleaseNetwork`：mock typed client，验证晚响应丢弃和设置变化失效。
- `RepositorySetupPanel` / `RepositorySyncConfirmDialog`：验证状态文案、提交列表、按钮门禁与确认事件。
- `App`：mock typed Tauri service，验证启动自动检测、无会话隐藏、终态弱入口和当前代理透传。

### Rust 公开行为

- 网络设置纯单元测试：HTTP/SOCKS5、域名/IPv4/IPv6、端口、user info/路径拒绝、直接/代理环境覆盖。
- Process/Git 基础设施测试：断言 Git 临时 `-c` 参数和稳定错误分类，不依赖真实网络。
- 连接测试：通过可替换 probe backend 或安全假进程确定性触发成功、超时和命令失败；不访问真实用户目录。
- Git 集成测试：全部使用 `tempfile` bare remote，构造同步、领先、落后、分叉、脏工作区、远端竞态和精确 Push。
- Command 测试：断言 camelCase 请求字段、单次 application 调用和错误原码。

自动测试不得读取或写入真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`。真实 GitHub 连接只作为最终人工烟雾检查，不能替代自动化测试。

## 9. 兼容性、回滚与交付

- 仓库偏好键保持 v1 不变；新增代理偏好使用独立 v1 key。
- 发布 session schema 暂不加入代理字段，避免迁移；恢复动作显式接收当前代理设置。
- 结构化仓库状态需要同步修改 Rust/TypeScript DTO 和现有测试 fixture。
- 回滚本功能只需还原发布控制台源码；不会留下 Git 全局配置、系统代理或用户环境变量修改。
- 完成后运行专项前端/Rust测试、`npm run check`、`npm run build:release-console`，并枚举交付 EXE 大小、时间和 SHA-256。
