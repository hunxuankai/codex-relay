# Windows 更新发布操作指南

本文面向 Codex Relay 维护者，说明如何把已经完成并验证的改动发布为 Windows NSIS 安装包和 Tauri 应用内更新。发布工作流只允许手动触发，并且必须先生成 Draft Release，核对完成后再人工公开；公开后由独立清理工作流只保留当前 Latest。

## 0. 推荐入口：Codex Relay 发布控制台

默认使用仓库内独立的便携 EXE 完成候选准备、检查、精确提交推送、GitHub Run、Draft 审计、
人工公开和在线复核。控制台不进入正式 Codex Relay 安装包，不读取 updater 私钥，也不会把
GitHub Actions 替换成本地发布后端。

发布电脑需已有 Git、Node/npm、Rust/Cargo、GitHub CLI，并完成 `gh auth login`。构建入口：

```powershell
npm run build:release-console
```

构建后运行 `dist/release-console/CodexRelayReleaseConsole.exe`。日常操作顺序：

1. 选择仓库，确认目标远端为 `hunxuankai/codex-relay`、默认分支为 `main`，并完成只读预检；
   预检通过后控制台会记住规范化本地路径，并显示线上 Latest tag。
2. 输入严格更高的 SemVer，检查基于 Git 提交与固定模板生成的简体中文说明和固定六文件计划；
   该生成过程不调用 Codex 或其他外部 AI。
3. 点击“开始发布”，等待本地门禁、精确提交推送和唯一 GitHub Run 完成。
4. 核对 Draft Release ID、tag、候选 SHA、说明、NSIS、`.sig`、`latest.json`、大小、SHA-256 与签名关联。
5. 在控制台的独立确认对话框中公开同一 Release ID，再等待 Latest、tag、manifest、公开资产和清理 Run 复核。
6. 导出不含秘密的发布摘要；失败、未执行项和 cleanup warning 必须保留。

控制台通过 workflow 的必填 `expected_version` 与 `expected_sha` 输入绑定候选版本和提交，最终说明
来自 `.github/release-notes.md`。push 失败但候选提交已创建时，会话保持 `Committed` 检查点，恢复时
只重试 push。push 后不执行伪回滚，也不自动删除错误 Draft。

首版明确不执行 Windows Sandbox、真实安装、UAC、应用内升级、重启、卸载或数据保留验证；这些
行为未执行时必须在结果中保持“未验证”。下文的命令行与 GitHub 页面步骤是控制台不可用时的人工
恢复入口，也是核对控制台行为的权威契约。

### 0.1 发布控制台运行时安全与恢复契约

#### 1. 范围/触发条件

修改发布控制台的会话状态、Git 提交推送、子进程取消、GitHub Run 发现/轮询或恢复逻辑时，必须遵循本节。
这些边界决定一次点击是否可能重复发布、在取消后继续 push，或因真实冷构建超过 30 分钟而误报失败。

#### 2. 签名

- `ReleaseStateStore::initialize(session)`：只能在持有 `RepositorySessionLock` 时初始化新会话。
- `GitBackend::new_cancellable(executable, environment, cancel)`：本地提交与 push 使用会话取消信号。
- `ReleasePushBackend::rollback_uncommitted(repository, plan)`：提交检查点创建前失败时精确清理计划文件暂存项。
- Run 发现预算至少 2 分钟；发布 Run 与 cleanup Run 监控预算至少 4 小时，当前轮询间隔为 5 秒。

#### 3. 契约

- 同一 Git dir 只允许一个非终态 session；同一 session 只允许一个已注册后台管线。
- `idle`、`inspected`、`planned` 等已落盘但尚未写源码的会话必须可安全取消，防止启动瞬间崩溃后永久卡住。
- commit 创建前失败或取消：先用不可取消的短 Git 操作清理计划文件暂存项，再恢复六文件原字节并验证；任一步失败都保留真实失败与恢复标记。
- commit 已创建、push 失败：持久化 `Committed` 与候选 SHA，恢复只重试 push，不重复 commit，也不自动重写本地历史。
- local check、普通构建、Git commit/push 共享 Windows Job Object 取消边界；取消信号不得阻断索引和文件回滚本身。
- Run 暂时不可见时在发现预算内继续查询；状态为 queued/in_progress 时在 4 小时预算内持续监控，不把本次已观测的 1 小时以上冷构建误判为超时。

#### 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| 同仓库已有非终态 session | `RELEASE_SESSION_ALREADY_ACTIVE`，不覆盖 `session.json` |
| 同 session 已有后台管线 | `RELEASE_SESSION_ALREADY_RUNNING`，不启动第二个任务 |
| commit 前失败且索引/六文件恢复完成 | session=`failed`，暂存区为空，六文件等于原字节 |
| 索引清理或六文件恢复任一失败 | `RELEASE_ROLLBACK_INCOMPLETE`，不得声称已恢复 |
| commit 成功、push 失败 | session=`committed`，保留候选 SHA 和事务标记供 push 重试 |
| 本地 Git 运行中取消 | 终止整个进程树，再执行不可取消的清理与回滚 |
| Run 在 30 分钟后仍运行、但未超过 4 小时 | 继续监控，不返回 `GITHUB_RUN_TIMEOUT` |

#### 5. 良好/基线/错误用例

- 良好：1 小时 09 分的 Windows 冷构建持续显示 `workflowRunning`，完成后进入 Draft 审计。
- 良好：`git commit` 失败后，计划文件取消暂存、六文件逐字节恢复，session 进入 `failed`。
- 基线：push 失败保留 `Committed`，点击继续只执行 push 与后续远端阶段。
- 错误：固定轮询 900×2 秒后把仍正常运行的 Action 报为超时。
- 错误：用户取消时只终止 npm，却让 Git push 继续；或让已取消信号同时阻止回滚 Git 命令。
- 错误：第二次点击“开始/继续”覆盖活动 `session.json` 或启动并发远端管线。

#### 6. 必需测试

- `remote_monitor_budgets_cover_slow_github_actions_runs`：断言发现预算 ≥2 分钟、监控预算 ≥4 小时。
- `cancellable_backend_terminates_the_active_process_tree`：断言 Git 子进程收到取消并在测试预算内退出。
- `commit_failure_unstages_and_rolls_back_candidate_before_marking_failed`：断言索引清理、原字节恢复、marker 删除和 `failed` 状态。
- `unstage_candidate_clears_the_planned_index_without_reverting_candidate_bytes`：断言只清暂存，不提前修改候选工作树。
- `active_session_blocks_reinitialization_until_it_reaches_a_terminal_phase` 与重复管线注册测试：断言会话和后台任务互斥。

#### 7. 错误与正确做法

错误：

```rust
for _ in 0..900 {
    sleep(Duration::from_secs(2)).await;
}
// 真实构建超过 30 分钟即误报失败
```

正确：

```rust
// 发现至少 2 分钟；远端运行最多监控 4 小时、每 5 秒刷新。
// commit 前失败：不可取消地清索引 -> 六文件回滚 -> 验证 -> failed。
// commit 后 push 失败：持久化 Committed，只重试 push。
```

### 0.2 仓库偏好与 Latest 预检契约

#### 1. 范围/触发条件

修改发布控制台的仓库输入恢复、`ReleasePreflightResult`、GitHub Release 列表解析或仓库摘要展示时，
必须遵循本节。仓库偏好只解决“如何找到仓库”，不能替代 worktree Git 元数据中的发布会话事实。

#### 2. 签名

前后端预检 DTO 使用 camelCase：

```typescript
interface ReleasePreflightResult {
  repositoryPath: string
  repository: RepositoryInspection
  external: {
    tools: ToolchainInspection
    activeReleaseRuns: number
    conflictingDrafts: number
    latestReleaseTag: string | null
  }
}
```

WebView 偏好键和值固定为：

```text
codex-relay-release-console.repository-preference.v1
{"version":1,"repositoryPath":"D:\\safe-temp\\repository"}
```

#### 3. 契约

- `repositoryPath` 必须由 Rust 在 Git 预检前 `canonicalize`，成功响应返回规范化绝对路径。
- 只有预检成功后，Vue 才能用响应中的 `repositoryPath` 更新输入并保存偏好；未校验的击键不得落盘。
- 偏好读取、JSON 解析、schema 版本或写入失败时回退为空值或当前会话内存值，不得阻断手动输入。
- 偏好只保存仓库路径，不保存 GitHub Token、Authorization Header、API Key、发布说明或 session 证据。
- GitHub Release 列表在 Rust 边界只解析一次：Draft 数量用于冲突门禁；首个
  `draft=false && prerelease=false` 且 tag 非空的对象投影为 `latestReleaseTag`。
- 没有正式 Release 时 `latestReleaseTag=null`，UI 显示“尚无正式版本”；GitHub 命令失败或 JSON
  无效继续返回 `GITHUB_PREFLIGHT_FAILED` / `GITHUB_RESPONSE_INVALID`，不得显示缓存或猜测版本。
- 发布说明仍由 `ReleaseNotesService` 根据 Git 提交和固定模板确定性生成，不调用 Codex；提交主题
  保留原语言，维护者可编辑正文，发布前继续执行必需段落、版本和秘密扫描校验。

#### 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| 首次启动或偏好不存在 | 仓库输入为空，可手动填写 |
| v1 偏好有效 | 启动时恢复仓库路径，但仍需重新执行只读预检 |
| 偏好损坏、版本未知或 storage 抛错 | 安全回退，不显示发布失败 |
| 预检成功且输入为非规范路径 | 输入和偏好改为后端返回的规范化路径 |
| 预检失败 | 保留当前输入，不覆盖上次成功偏好 |
| Release 列表含 Draft/prerelease/正式版本 | 跳过前两者，显示首个正式 tag |
| Release 列表无正式版本 | 显示“尚无正式版本” |
| GitHub 查询或 JSON 解析失败 | 整体预检失败，显示稳定错误，不伪造 Latest |

#### 5. 良好/基线/错误用例

- 良好：维护者首次填写仓库并预检成功；下次打开路径已恢复，点击检查后显示 `v0.4.0`。
- 基线：WebView 数据被清除后重新手动选择仓库，不影响 worktree 内已有 session 的真实性。
- 良好：Release 列表前两项分别为 Draft 和 prerelease，第三项为正式 Release；摘要显示第三项 tag。
- 错误：输入框每次变化都写 localStorage，把拼写中间态或未验证目录保存为下次默认值。
- 错误：Vue 自行解析 GitHub JSON，或在查询失败时沿用旧 tag 并把预检标为通过。

#### 6. 必需测试

- `useRepositoryPreference.test.ts`：断言 v1 恢复、损坏/未知版本/空路径回退、显式保存和 storage 异常容错。
- `App.test.ts`：断言启动恢复、成功预检保存规范化路径、失败预检不覆盖旧偏好，以及范围横幅不再显示。
- `release_application.rs` 单元测试：断言 Latest 投影跳过 Draft/prerelease，无正式版本返回 `None`。
- command/Git preflight 测试：断言 `repositoryPath` 规范化、`latestReleaseTag` 透传且 camelCase DTO 类型同步。
- `RepositorySetupPanel.test.ts`：断言真实 tag 与“尚无正式版本”空态。
- `ReleasePlanPanel.test.ts`：断言用户可见文案明确“不调用 Codex”。

#### 7. 错误与正确做法

错误：保存未验证输入，并在组件内猜测 Latest。

```typescript
watch(repositoryPath, value => localStorage.setItem('repository', value))
const latest = releases[0]?.tag_name
```

正确：只在 typed 预检成功后保存后端事实，Vue 只展示 DTO。

```typescript
const inspection = await release.inspect(repositoryPath.value)
if (inspection) repositoryPreference.remember(inspection.repositoryPath)

const latest = inspection.external.latestReleaseTag ?? '尚无正式版本'
```

### 0.3 发布控制台代理、仓库同步与自动恢复契约

#### 1. 范围/触发条件

修改发布控制台的代理偏好、Git/`gh` 子进程环境、连接测试、仓库同步状态、安全 Push、启动会话检测、
恢复入口或相关错误映射时，必须遵循本节。该边界允许维护者在 GitHub 网络不稳定环境中显式使用
HTTP/SOCKS5 代理，但不得把控制台扩展成通用 Git 客户端，也不得保存代理认证或 GitHub 凭据。

#### 2. 签名

所有 DTO 使用 camelCase。网络 command 固定为：

```typescript
type ReleaseProxyType = 'http' | 'socks5'

interface ReleaseProxySettings {
  enabled: boolean
  proxyType: ReleaseProxyType
  host: string
  port: number | null
}

test_release_connection(proxy): CommandResult<{
  git: ConnectionProbeResult
  github: ConnectionProbeResult
}>

inspect_release_repository(repositoryPath, proxy): CommandResult<ReleasePreflightResult>
prepare_release_plan(repositoryPath, targetVersion, notes, proxy): CommandResult<ReleasePlanSummary>
start_release(planId, proxy, onEvent): CommandResult<ReleaseSession>
resume_release(sessionId, proxy, onEvent): CommandResult<ReleaseSession>
publish_release(sessionId, expectedDraftIdentity, proxy, onEvent): CommandResult<ReleaseSession>

interface SafeRepositoryPushRequest {
  repositoryPath: string
  expectedHeadSha: string
  expectedRemoteMainSha: string
  proxy: ReleaseProxySettings
}

push_release_repository(request): CommandResult<ReleasePreflightResult>
get_release_session(repositoryPath): CommandResult<ReleaseSession | null>
```

仓库同步事实固定为 `synced | ahead | behind | diverged`，并携带 `aheadCount`、`behindCount` 与
`aheadCommits[{sha, subject}]`。只有后端可生成 `safePush` 预览；前端不得自行重建 Push 资格或提交集合。

#### 3. 契约

- 代理偏好键固定为 `codex-relay-release-console.proxy-preference.v1`，只保存版本号及
  `enabled/proxyType/host/port`；损坏 JSON、未知版本、非法类型或存储异常安全回退。关闭开关仍保留
  地址和端口。
- 地址只接受主机名、IPv4 或 IPv6；拒绝协议、路径、查询、片段、URL userinfo、账号和密码。端口仅为
  `1–65535`。Vue 做即时校验，Rust 使用 `url::Host`/IPv6 解析再次防御校验。
- 每个网络动作在开始时捕获一次当前代理快照。Rust 先清除大小写不敏感的
  `HTTP_PROXY/HTTPS_PROXY/ALL_PROXY/NO_PROXY`：开启时只注入统一的 `HTTP_PROXY` 与
  `HTTPS_PROXY`；关闭时保持缺失。Git 每次额外使用临时
  `-c http.proxy=<URL或空> -c https.proxy=<URL或空>`，不得修改全局 Git 配置、Windows 系统代理或
  用户环境变量。
- Git 网络动作设置 `GIT_TERMINAL_PROMPT=0` 与 `GCM_INTERACTIVE=Never`。公开 DTO、错误、日志、通知、
  localStorage 和测试输出不得包含代理 URL、Authorization、Bearer、Token 或子进程原始 stderr。
- 连接测试只执行固定公开只读目标：
  `git ls-remote --exit-code https://github.com/hunxuankai/codex-relay.git refs/heads/main` 与
  `gh api repos/hunxuankai/codex-relay --silent`。两项独立返回成功、稳定 code、安全消息和耗时；一项失败
  不覆盖另一项，不写仓库、不触发 workflow，也不是发布强制门禁。
- 仓库预检 Fetch 后由 Rust 计算同步关系；脏工作区、领先、落后和分叉是可展示事实，不得全部折叠成
  command failure。工具链摘要必须按 DTO 真实显示缺失项，不能硬编码“已就绪”。
- `safePush` 仅在固定远端、工作区干净、本地严格领先且不落后、工具链就绪、无活动发布 Run、无冲突
  Draft、无本地非终态发布 session 时存在。仓库输入变化后必须立即清除旧 inspection、plan 和 Push 预览。
- 用户确认 Push 后，后端重新执行完整预检与 Fetch，复核工作区、两端 SHA 和领先关系，再只执行：

  ```text
  git push origin <expectedHeadSha>:refs/heads/main
  ```

  禁止 force、Tag、其他分支和任意 RefSpec。Push 后远端 `main` 精确等于 `expectedHeadSha` 才算 Push
  成功；随后刷新预检。刷新时新出现的 Run/Draft 可以使 `releaseReady=false`，但不能把已经验证成功的
  Push 伪报为 `GIT_REMOTE_VERIFICATION_FAILED`。
- 启动时只对已记住路径调用 `get_release_session`；该命令只读取本地 Git 元数据、工作区事实、session 与
  发布说明，不 Fetch、不调用 GitHub。无 session 不显示恢复入口，也不保留“加载活动会话”按钮。
- 本地中断阶段只能“取消并验证回滚”；`committed` 继续 Push；远端阶段继续监控；等待公开进入确认；终态
  只查看结果。代理无效只阻止继续 Push/监控/公开等网络动作，不得阻止取消回滚或查看本地结果。
- 终态结果必须分别判断 Release 是否已公开、在线复核是否完成、cleanup 是否完成。仅有
  `published != null` 不能推导在线复核或历史清理成功。

#### 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| 代理开启但地址/端口无效 | `RELEASE_PROXY_HOST_INVALID` / `RELEASE_PROXY_PORT_INVALID`，不启动子进程 |
| Git 启动/超时/取消/进程树/普通失败 | 分别保留 `GIT_PROCESS_START_FAILED`、`GIT_PROCESS_TIMEOUT`、`GIT_PROCESS_CANCELLED`、`GIT_PROCESS_TREE_TERMINATION_FAILED`、`GIT_COMMAND_FAILED` |
| Fetch 超时或非零退出 | `GIT_FETCH_TIMEOUT` / `GIT_FETCH_FAILED`，不得只返回泛化 `GIT_COMMAND_FAILED` |
| GitHub CLI 启动/超时/取消/进程树/普通失败 | 对应 `GITHUB_PROCESS_*` 或 `GITHUB_COMMAND_FAILED` |
| 工作区脏、HEAD 变化、远端变化 | `GIT_WORKTREE_DIRTY` / `GIT_HEAD_MOVED` / `GIT_REMOTE_MOVED`，不 Push |
| 本地落后或分叉 | `GIT_REPOSITORY_BEHIND` / `GIT_REPOSITORY_DIVERGED`，不提供或执行 Push |
| Push 非零且远端未移动 | `GIT_PUSH_FAILED` |
| Push 后远端不等于确认 SHA | `GIT_REMOTE_VERIFICATION_FAILED`，不得报告成功 |
| Push 已验证，但刷新后出现活动 Run/Draft | 返回刷新后的 `releaseReady=false` 事实，Push 本身仍成功 |
| 启动检测无 session | 返回 `null`，不执行 Fetch/GitHub，不显示恢复面板 |

#### 5. 良好/基线/错误用例

- 良好：SOCKS5 `127.0.0.1:1080` 跨启动恢复；Git 与 `gh` 使用同一快照，连接测试分别显示耗时。
- 良好：本地领先两个已审核提交，确认框显示远端、两端 SHA 和主题；后端重新 Fetch 后只推送确认 SHA。
- 基线：关闭代理后仍保留填写值，但 Git 临时代理配置为空、`gh` 环境没有继承代理，实际直连。
- 基线：`committed` 会话重启后先修改代理，再继续 Push；启动检测本身仍保持本地只读。
- 错误：把代理 URL 或认证字段写入 localStorage、普通 DTO、stderr 错误或任务材料。
- 错误：仓库输入已换成另一个 clone，界面仍保留旧 `safePush` 并允许确认。
- 错误：代理无效时把“取消并验证回滚”显示成永久 loading，或仅因新出现 Draft 就把已成功 Push 报成失败。
- 错误：`published` 存在就显示“在线复核和历史清理已完成”，忽略失败阶段和 cleanup 证据。

#### 6. 必需测试

- `useReleaseProxyPreference.test.ts`：v1 恢复、损坏/未知版本/存储异常回退、关闭保留字段、无认证字段。
- `ProxySettingsPanel.test.ts` 与 `useReleaseNetwork.test.ts`：HTTP/SOCKS5、无效 IPv4/IPv6、设置变化失效、
  晚响应丢弃、Git/GitHub 两项独立结果。
- `release_network.rs` / `tests/release_network.rs`：Direct/Custom 环境覆盖、Git 临时 `-c`、固定只读探针、
  工具缺失和稳定错误分类。
- `tests/git_release.rs`：同步/领先/落后/分叉/脏状态、精确 SHA Push、Tag/分支隔离、确认后远端移动、
  Push 后远端漂移和非 fast-forward 竞态。
- `commands.rs` / `services/tauri.test.ts`：camelCase 请求、只包含固定 Push 字段、单次 application 调用。
- `App.test.ts` / `ReleaseRecoveryPanel.test.ts`：启动只调用本地 session command、无会话隐藏、阶段动作、
  当前代理透传、仓库路径变化清除旧 Push 预览、无效代理不阻止取消/查看。
- `RepositorySetupPanel.test.ts` / `ReleaseResultPanel.test.ts`：权威同步/工具链事实、Latest、公开后失败与 cleanup
  未确认时不产生成功声明。

#### 7. 错误与正确做法

错误：继承未知代理、由前端猜 Push 条件，并把分支名直接传给 Git。

```typescript
if (ahead) await invoke('push', { repositoryPath, branch: 'main' })
```

正确：后端提供不可扩展的预览；确认后重新验证固定事实并推送精确 SHA。

```typescript
await releaseConsoleTauri.pushRepository({
  repositoryPath: inspection.repositoryPath,
  expectedHeadSha: inspection.safePush.expectedHeadSha,
  expectedRemoteMainSha: inspection.safePush.expectedRemoteMainSha,
  proxy: currentProxy,
})
```

```rust
// 每次命令临时覆盖代理；不修改全局配置。
git -c http.proxy=<url-or-empty> -c https.proxy=<url-or-empty> \
    push origin <expected_sha>:refs/heads/main
```

## 1. 发布边界

- 默认发布源是 GitHub 默认分支 `main`，工作流为 `.github/workflows/release.yml`。
- 工作流触发键固定为 `workflow_dispatch:`，并通过 `releaseDraft: true` 保证只先生成 Draft；普通提交不得自动公开 Release。
- `.github/workflows/cleanup-old-releases.yml` 只响应正式 Release 的 `published` 事件（或手动重试），以 `releases/latest` 为唯一保留对象，删除其他 Release、资产和对应 Git tag；Draft 阶段不执行清理。
- 应用版本使用 SemVer，发布版本必须严格高于当前公开版本。
- 应用版本来源是 `package.json`；package lock、Cargo 元数据和发布说明必须同步。
- 普通本地构建不需要更新私钥；带 `.sig` 和 `latest.json` 的 updater 资产只由 GitHub Actions 发布构建生成。
- 发布工作流使用 GitHub 自动提供的 `GITHUB_TOKEN`，并从 Actions Secrets 读取 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。不得读取、复制或输出 Secret 值。
- Tauri updater 签名用于验证更新资产；Windows Authenticode 用于验证 Windows 发布者身份，两者相互独立。当前项目未启用 Authenticode，因此安装器可能显示“未知发布者”。
- 发布、安装、升级、重启和数据保留是不同证据。未执行的步骤必须明确记录为未验证。

开始前同时阅读 [Tauri 应用内更新](updater.md)、[Tauri 与 NSIS](tauri-nsis.md)、[代码签名](signing.md)、[验证与完成证据](../testing/verification.md) 和 [数据保留与清理](../security/data-retention.md)。

## 2. 发布前确认

1. 确认本次产品改动、测试和文档已经完成，不把无关改动混入发布提交。
2. 确认当前公开版本和目标版本。以下命令只读取公开 Release，不需要 Token：

   ```powershell
   $headers = @{ "User-Agent" = "Codex-Relay-Release-Check" }
   $latest = Invoke-RestMethod `
     -Headers $headers `
     -Uri "https://api.github.com/repos/hunxuankai/codex-relay/releases/latest"
   $latest | Select-Object tag_name, name, published_at, draft, prerelease
   ```

3. 选择严格更高的 SemVer，例如从 `1.2.3` 提升到 `1.2.4`。不要复用已存在的 Tag 或 Release 版本。
4. 确认 Git 工作区状态和远端分支：

   ```powershell
   git status --short --branch
   git remote -v
   git branch -vv
   ```

5. 在 GitHub 仓库的 `Settings → Secrets and variables → Actions` 中确认以下 Secret 名称存在：

   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

   只确认名称和更新时间，不查看或输出值。私钥丢失、公钥需要更换或 Secret 状态不确定时停止发布，先处理信任根恢复方案。

## 3. 同步版本与发布说明

### 3.1 更新 npm 版本

在 PowerShell 中输入不带 `v` 的新版本号：

```powershell
$newVersion = Read-Host "输入新版本号，例如 0.1.3"
npm version $newVersion --no-git-tag-version
```

该命令应更新 `package.json`、`package-lock.json` 和 lock 根包版本，但不会创建 Git Tag。

### 3.2 更新 Cargo 版本

把 `src-tauri/Cargo.toml` 中 `[package]` 的 `version` 改为同一个版本。随后运行 Cargo 或项目检查，让 `src-tauri/Cargo.lock` 中 `codex-relay` 根包版本同步。

核对四个版本来源：

```powershell
Select-String -LiteralPath package.json,package-lock.json -Pattern '"version"'
Select-String -LiteralPath src-tauri/Cargo.toml -Pattern '^version\s*='
rg -n -A 2 '^name = "codex-relay"$' src-tauri/Cargo.lock
```

### 3.3 更新发布结构测试

检查 `src/release-config.test.ts`。如果其中仍有上一版本专用的测试名称、固定版本断言或“上一版本 → 新版本”说明断言，必须同步到本次候选；更通用的长期契约可以保留为版本一致性断言。

### 3.4 写入最终发布说明

在 `.github/release-notes.md` 中写入可以公开的最终简体中文说明，至少包含：

- 本版本的用户可见变化；
- 从哪个已公开版本更新；
- 安装或升级注意事项；
- 当前没有 Windows Authenticode、可能显示“未知发布者”；
- 升级不会主动删除 Codex 配置、Codex Relay 应用数据、日志或备份。

workflow 的“验证发布请求”步骤会校验该文件并把正文作为 `tauri-action.releaseBody`，因此同一内容会同时进入 Release 页面和 `latest.json.notes`。Draft 生成后只编辑 GitHub 页面说明不会重写已经上传的 `latest.json`；发现说明错误时必须修正该文件、创建新候选提交并重新生成 Draft。

最后搜索待发布版本和上一版本，人工确认没有遗漏需要同步的位置：

```powershell
rg -n -S $newVersion package.json package-lock.json src-tauri .github README.md .trellis/spec/release
```

历史任务和历史示例可以保留旧版本；当前配置、测试和发布说明不得漂移。

## 4. 本地验证候选

### 4.1 运行完整检查

```powershell
npm run check
```

任何测试、格式、Clippy 或编译失败都必须先修复。成功重试不能抹掉首次失败和根因记录。

### 4.2 验证普通构建不依赖更新私钥

只从当前 PowerShell 进程移除签名环境变量，然后运行普通构建：

```powershell
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
npm run build
```

不要把私钥值放进命令行、脚本、日志、Trellis 任务或普通环境文件。`npm run build` 成功只证明普通 Release 主程序和 NSIS 安装器已生成，不证明 updater 签名、安装或升级成功。

### 4.3 枚举实际产物

```powershell
$artifacts = @(
  Get-Item -LiteralPath "src-tauri/target/release/CodexRelay.exe"
  Get-ChildItem -LiteralPath "src-tauri/target/release/bundle/nsis" -Filter "*.exe"
)

$artifacts | Select-Object FullName, Length, LastWriteTime
$artifacts | Get-FileHash -Algorithm SHA256
```

必须记录实际路径、大小、时间和 SHA-256，不能根据约定猜测文件名。普通构建不应生成本次发布的 `.sig` 或 `latest.json`。

### 4.4 提交前检查

```powershell
git diff --check
git status --short --ignored
git diff -- package.json package-lock.json src-tauri/Cargo.toml `
  src-tauri/Cargo.lock src/release-config.test.ts .github/workflows/release.yml
```

人工复核：

- 没有真实 API Key、Authorization Header、Token、认证文件或密钥内容；
- 没有真实 `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay` 数据；
- 没有把构建目录、临时下载或 Sandbox staging 纳入 Git；
- 发布说明与实际改动一致。

## 5. 提交并推送候选

按本次实际改动精确暂存，不使用会把无关文件一并加入的宽泛命令。提交信息默认使用简体中文，例如：

```powershell
git add <本次发布文件>
git commit -m "chore(release): 准备 v<新版本> 发布"
git push origin HEAD:main
```

推送后确认远端 `main` 精确包含候选提交。不要手动创建 Tag；`tauri-action` 会依据应用版本和 `tagName: v__VERSION__` 创建 Release Tag。

## 6. 触发发布工作流

控制台会使用 `gh workflow run release.yml --ref main --json`，通过 stdin 传入
`expected_version` 和 `expected_sha`，然后按 Run URL 或 workflow/main/SHA/触发时间唯一定位 Run。

人工恢复入口：

1. 打开 GitHub 仓库的 `Actions` 页面。
2. 选择“发布 Windows 更新”。
3. 点击 `Run workflow`，选择 `main`。
4. 填写与远端 `main` 完全一致的 `expected_version` 和 `expected_sha`，再次确认后启动。
5. 等待所有步骤完成：检出源码、验证发布请求、配置 Node.js、配置 Rust、`npm ci`、`npm run check` 和“构建 Draft Release”。

工作流失败时不得创建或描述为成功发布。先记录失败步骤、日志边界和对应提交，修复后使用新提交重新运行。

### 6.1 运行时长与外部状态门禁

发布工作流同时包含外部排队、Windows 冷环境准备、完整检查和 Draft 构建；这些阶段的证据必须分开
记录，不能把排队时间误写成构建耗时，也不能把测试失败归因于 GitHub 服务故障。

触发前和每次重试前：

- 只保留一个活动候选 Run；先检查 GitHub Actions 状态页和仓库 Run 队列。若状态页显示
  `major_outage` 或 Run 长时间 `queued`，记录为外部阻塞，不重复触发相同提交。
- 记录 `createdAt`、`startedAt`、`completedAt`、候选提交和失败步骤；分别计算排队时长与 Job 执行时长。
- 将失败归类为：外部服务/排队、工作流或工具诊断、CI 环境时序、产品/测试契约。只有修复并完成本地
  针对性回归和完整检查后，才用新候选提交重试；不得用相同提交盲目重跑来掩盖失败。
- 迭代期间先运行专项测试和本地安全检查，最后才运行完整 `npm run check`、普通构建和唯一的发布 Run。
  完整检查、Draft 构建、Draft 审计、公开复核和 Sandbox 升级是不同时间段，发布记录应分别列出。

历史 Windows 冷 runner 的完整检查和 Draft 构建耗时只能作为排程参考，不能改写成产品性能承诺；测试预算
必须留在测试层，不能为了缩短发布时间放宽生产超时或错误断言。

## 7. 核对 Draft Release

Action 成功后，Release 必须仍为 Draft。发布前逐项核对：

- Tag 和标题是目标版本，例如 `v0.1.3` 和 `Codex Relay v0.1.3`；
- 目标提交与触发工作流的候选提交一致；
- `draft=true`、`prerelease=false`；
- Release 页面说明是最终文案；
- 资产包含且只包含预期的 NSIS 安装器、对应 `.sig` 和 `latest.json`；
- `latest.json.version` 等于目标版本；
- `latest.json.notes` 与 Release 说明一致；
- 平台包含 `windows-x86_64` 和 `windows-x86_64-nsis`，并指向本次 NSIS 资产；
- 清单内联签名与独立 `.sig` 内容一致；
- 安装器、`.sig` 和 `latest.json` 的实际大小与 SHA-256 已记录；
- 没有私钥、密码、Token、认证 Header 或用户数据进入说明和资产。

发布控制台会自动完成上述身份、资产、manifest、size、GitHub digest、SHA-256 和签名关联审计。
如果平台 URL 是 GitHub REST asset API，按 updater 的下载语义核对实际字节，而不是使用普通 GET：

```powershell
curl.exe -L -H "Accept: application/octet-stream" `
  "https://api.github.com/repos/hunxuankai/codex-relay/releases/assets/<asset-id>" `
  -o "$env:TEMP\codex-relay-updater.exe"
Get-FileHash -Algorithm SHA256 "$env:TEMP\codex-relay-updater.exe"
Remove-Item -LiteralPath "$env:TEMP\codex-relay-updater.exe"
```

Draft 说明、版本、签名或资产有任何问题时不要公开。控制台首版不会自动删除错误 Draft；维护者需在
GitHub 明确处理后，修正源码或 workflow、创建新候选提交并重新运行。

## 8. 人工公开 Release

只有 Draft 核对全部通过后，才在控制台确认对话框中核对版本、候选 SHA 和 Release ID 并公开；
控制台会在 PATCH 前完整重做同一 Draft 审计，并只按 Release ID 更新 `draft=false`。控制台不可用时，
才在 GitHub Release 页面点击 `Publish release`。公开后立即确认：

```powershell
$headers = @{ "User-Agent" = "Codex-Relay-Release-Check" }
$release = Invoke-RestMethod `
  -Headers $headers `
  -Uri "https://api.github.com/repos/hunxuankai/codex-relay/releases/latest"
$manifest = Invoke-RestMethod `
  -Headers $headers `
  -Uri "https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json"

$release | Select-Object tag_name, target_commitish, draft, prerelease, published_at
$manifest | Select-Object version, pub_date
```

预期结果：

- `releases/latest` 返回新 Tag；
- `draft=false`、`prerelease=false`；
- `latest.json` 返回新版本；
- tag 下载和 `releases/latest/download/latest.json` 的清单内容一致；
- 公开资产的大小、SHA-256、URL 和签名与 Draft 核对结果没有漂移。

### 8.1 核对历史版本清理

Release 公开后，检查名为“清理历史 GitHub Releases”的 Actions Run。该 Run 必须：

- 以 `releases/latest` 返回的新 Tag/Release 为唯一保留对象；
- 删除其余 Release、三项打包资产和对应 Git tag；
- 清理失败时保持失败状态，不把部分完成写成成功。

清理前后分别保存只读列表（只记录 Tag、Release ID、状态和资产名称，不记录 Token）：

```powershell
gh release list --repo hunxuankai/codex-relay --limit 100
gh api --paginate repos/hunxuankai/codex-relay/tags --jq '.[].name'
```

旧 Release、安装器、`latest.json`、Tag 下载链接和源码快照在清理后不再保证可用；已安装旧版本仍通过固定 `releases/latest` 清单更新。清理动作不涉及 Codex 配置、应用数据、日志或备份。

## 9. 验证应用内升级

公开 Release 只能证明托管状态，不能替代真实升级验证。使用 Windows Sandbox 或隔离 VM，从已知良好的上一公开版本执行：

1. 使用安全临时 staging 和成对的 `CODEX_RELAY_CODEX_HOME`、`CODEX_RELAY_APP_DATA_DIR` 覆盖准备测试数据。
2. 核对基线安装器版本、大小和 SHA-256 后安装上一版本。
3. 确认上一版本能进入设置页。
4. 点击“检查更新”，确认发现目标版本。
5. 点击“下载并安装”，观察下载、Tauri 签名接受、NSIS 启动、可能出现的 UAC 和应用退出。
6. 升级后运行 `guest-verify.ps1 -ExpectedVersion <新版本>` 或对应桌面核验入口。
7. 读取 `after.json`，确认：
   - 实际版本等于目标版本；
   - `CodexRelay.exe` 存在；
   - 升级沿用登记安装目录；
   - 白名单 fixture 的长度和 SHA-256 前后相同；
   - 报告不含 fixture 内容、API Key 或认证 Header。
8. 单独记录应用是否自动重启、是否出现 UAC，以及取消、断网、错误签名和下载失败路径是否执行。

禁止使用真实 `.codex`、真实 `%LOCALAPPDATA%\CodexRelay`、仓库目录或经过 junction/symlink 的可写路径作为 Sandbox 数据目录。人工覆盖安装只能记录为“恢复已知良好基线”，不能算作 updater 成功。

## 10. 失败、修复与回滚

- 本地检查失败：停止发布，修复并重新运行完整检查。
- GitHub Actions 失败：不得发布；记录失败步骤，使用新提交重试。
- 历史 Release/tag 清理失败：不得把清理报告为成功；保留已删除与未删除对象的真实列表，修复权限或网络后通过清理工作流的 `workflow_dispatch` 安全重试，不使用通配符删除。
- Draft 错误：删除 Draft 和错误资产，修正后重新生成。
- UAC 取消、安装器失败或升级后无法启动：不得报告升级成功；允许用户重新打开旧版本或人工安装已知良好版本。
- 已公开版本有缺陷：不得原地替换安装器、`.sig` 或 `latest.json`，发布严格更高的 SemVer 修复。
- 更新私钥丢失或公钥必须更换：现有客户端不能信任新密钥，必须设计手动安装更高版本的恢复路径。
- 没有 Authenticode 证据：继续按“未知发布者”报告，不能把 Tauri updater 签名描述成 Windows 发布者签名。

## 11. 发布记录最低内容

每次发布至少保留以下非秘密证据：

- 版本、候选提交和工作流 Run URL；
- 本地 `npm run check` 与普通 `npm run build` 的退出状态；
- Release/NSIS 实际路径、大小、时间和 SHA-256；
- Draft Release ID、Tag、目标提交和三个资产的大小、SHA-256；
- `latest.json` 的版本、说明、平台 URL 和签名关联核对；
- 公开时间与公开端点复核结果；
- 历史 Release/tag 清理 Run 的 URL、保留 Tag、删除前后列表、失败步骤和未完成项；
- Sandbox/VM 中实际执行和未执行的安装、升级、UAC、重启及数据保留场景；
- 所有真实失败、限制和未完成项。

这些证据可以记录在对应 Trellis 发布任务中，但不得包含 Secret 值、完整认证文件、API Key、Authorization Header 或用户数据内容。
