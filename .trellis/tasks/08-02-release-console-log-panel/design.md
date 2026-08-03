# 发布控制台固定日志区域设计

## 1. 设计目标

在不改变发布成功、失败、取消、回滚和远端审批语义的前提下，建立一条从本地进程与发布编排到
持久化 JSONL、Tauri 实时事件和 Vue 固定日志区的统一诊断链路。该链路优先保留足够的错误上下文，
同时在 Rust 权威边界阻止秘密、认证内容、代理地址和不必要的本机路径进入公开日志。

本功能是一个跨层行为，Rust 记录、IPC、前端状态和界面必须共同使用同一 DTO 与 sequence 事实源，
不拆成可独立发布的子任务。

## 2. 当前缺口

```text
SafeProcessRunner
  -> 已能在进程完成前回调 stdout/stderr 字节
  -> ProcessLocalVerificationBackend 传入 None
  -> LocalCommandEvidence 只保留 exit_code/duration

ReleaseEvent::StepLog
  -> Rust/TypeScript 已有类型
  -> 生产 application/orchestrator 从不发送

ReleaseStepDetails
  -> 在右侧滚动工作区内过滤内存 events
  -> 开始新会话或加载会话时 events 被清空
  -> session.json 只保留失败步骤，不保留诊断日志
```

因此当前失败只能恢复 `phase/stepId/code`，无法恢复编译器、测试或远端状态上下文。

## 3. 目标架构

```text
本地 npm/Cargo 输出 -> ReleaseProcessLogSink -> ReleaseLogSanitizer --+
发布编排阶段 -------> ReleaseProgressSink ---------------------------+--> ReleaseLogRecorder
Git/GitHub 状态 -----> 结构化状态变化/心跳 ---------------------------+       |
                                                                            +-> ReleaseLogStore
                                                                            |   session.log.jsonl
                                                                            +-> ReleaseEventSink
                                                                                stepLog/started/completed/failed

get_release_session/get_release_logs -> ReleaseLogPage -> typed Tauri client
实时 ReleaseEvent ---------------------> useReleaseSession sequence reducer
                                                    |
                                                    +-> ReleaseLogPanel
```

`ReleaseLogRecorder` 是 sequence、时间戳、持久化与实时事件的唯一所有者。持久化不依赖 WebView channel
仍在线；Tauri 发送失败只失去当前订阅，不能停止记录。日志 I/O 失败也不改变发布结果，但 recorder 会
向仍在线的 channel 最多发送一次易懂警告。

## 4. Rust 数据契约

### 4.1 公开 DTO

在 `tools/release-console/src-tauri/src/models.rs` 增加以下 camelCase DTO，并在
`tools/release-console/src/types/release.ts` 保持同构：

```rust
enum ReleaseLogSource {
    Lifecycle,
    Stdout,
    Stderr,
}

enum ReleaseLogLevel {
    Info,
    Warning,
    Error,
}

struct ReleaseLogEntry {
    session_id: String,
    sequence: u64,
    timestamp: String,
    step_id: String,
    source: ReleaseLogSource,
    level: ReleaseLogLevel,
    message: String,
}

struct ReleaseLogPage {
    entries: Vec<ReleaseLogEntry>,
    next_before_sequence: Option<u64>,
    has_earlier: bool,
    total_entries: u64,
    total_bytes: u64,
    truncated: bool,
    warning: Option<String>,
}

struct ReleaseSessionSnapshot {
    session: ReleaseSession,
    logs: ReleaseLogPage,
}
```

`ReleaseEvent::StepLog { entry: ReleaseLogEntry }` 改为携带一个完整 entry。`StepStarted`、`StepCompleted`、
`StepFailed` 继续提供时间线需要的结构化行为；生产流程统一经 recorder 发送，避免日志面板与时间线
各自生成一套事件顺序。

### 4.2 command 契约

```text
get_release_session(repositoryPath)
  -> CommandResult<ReleaseSessionSnapshot | null>

get_release_logs(sessionId, beforeSequence)
  -> CommandResult<ReleaseLogPage>
```

分页大小由后端固定为 2,000，不接收任意 limit。`beforeSequence=null` 表示最新页；更早页只返回
`sequence < beforeSequence` 的最后 2,000 条。application 先从已加载的 `SessionContext` 验证 session ID，
再读取该仓库实际 Git dir，前端不能提交任意日志路径。

`start_release`、`resume_release` 和 `publish_release` 仍返回 `ReleaseSession`，并通过原有 Channel 推送
实时日志。只有本地恢复 command 返回 snapshot，避免把 50 MiB 文件塞入每个操作响应。

## 5. 日志存储

### 5.1 文件与 schema

Rust 职责按现有分层拆开：`services/release_log.rs` 只负责 policy、JSONL store、recorder 和
`ReleaseProgressSink`；`infrastructure/release_log.rs` 负责原始进程输出专用的 sanitizer、增量 decoder
和 `ReleaseProcessLogSink`。文件固定为：

```text
<git-dir>/codex-relay-release-console/session.log.jsonl
```

每一行是独立 envelope，包含 `schemaVersion=1` 和完整 `ReleaseLogEntry`。session ID 每行重复保存，
使轮换失败、复制残留或其他会话数据无法静默混入。`session.json` schema v1 保持不变，不把大日志数组
嵌入会话快照。

### 5.2 Store 职责

`ReleaseLogStore` 公开以下 crate 内接口：

```rust
initialize(session_id)
open(session_id)
append(entry)
load_page(session_id, before_sequence)
```

- `initialize` 使用现有 `atomic_write` 原子替换旧文件，成功后 sequence 从 1 开始。
- `open` 验证指定 session 的现有有效前缀，恢复最后 sequence、当前字节数、记录数和截断状态，供
  `resume_release` / `publish_release` 继续递增；缺少文件时以该 session 的空日志状态打开。遇到不完整
  尾行或中间损坏时先原子重写为有效前缀并返回 recovery warning，重写失败则切换为易失模式，绝不在
  不可信后缀后继续追加。
- `append` 在互斥区内验证 session、sequence、单条序列化大小和当前计数，再以单个 JSONL 行追加并 flush。
- `load_page` 在 blocking 文件边界解析并分页，不在 Tauri async executor 上同步扫描 50 MiB。
- 缺少日志文件返回空页；不完整末行被忽略并产生 warning。
- 非末行 JSON、schema、session ID 或 sequence 无效时保留此前有效前缀、停止信任后续记录，并返回 warning。
- recorder 是所有生产写入的唯一入口，并在构造公开 entry 前执行基础 `safe_log::redact`；store 在序列化
  前再次执行同一基础脱敏作为纵深防御。进程输出的路径、ANSI、代理和增量 UTF-8 处理仍由
  infrastructure sanitizer 负责，store 不复制完整 sanitizer。
- `ReleaseLogEntry` 不派生会打印 `message` 的默认 `Debug`；自定义 `Debug` 只输出元数据和消息长度，
  防止断言、错误链或 tracing 意外泄露诊断正文。

### 5.3 容量与压缩

生产 `ReleaseLogPolicy` 固定：

```text
maxBytes       = 50 MiB
maxEntries     = 100,000
maxEntryBytes  = 1 MiB
streamChunk    = 64 KiB
pageSize       = 2,000
```

测试通过注入同一生产类型的小 policy 触发边界，不添加 test-only 方法，也不在测试中反复写 50 MiB。

当下一条记录将越过总量或条数上限时，store 读取当前有效记录并原子压缩到约 80% 水位：

1. 保留 lifecycle、warning、error 和最近普通输出；
2. 从最旧的 info stdout/stderr 开始淘汰；
3. 插入一条持久化 warning，记录最早保留 sequence 和“早期普通输出已截断”；
4. 如果高优先级记录自身超过上限，仍以最新错误和最新阶段为优先，不能突破硬上限。

store 不分配新的实时 sequence；压缩标记复用被淘汰记录中的最大 sequence 并按 sequence 重排，recorder
继续作为后续 sequence 的唯一分配者。单条序列化结果超过 1 MiB 时，recorder 在 UTF-8 边界截断消息并
写入明确 warning，store 仍把 1 MiB 作为不可突破的最终防线。

压缩失败时不覆盖原文件，recorder 切换为当前窗口易失日志并发送一次持久化警告。日志不是受管配置，
不经过 `TransactionService`；轮换和压缩仍使用原子替换，避免损坏现有有效日志。

## 6. 诊断采集与安全处理

### 6.1 本地进程输出

`infrastructure/release_log.rs` 中的 `ReleaseProcessLogSink` 实现 core 的 `ProcessEventSink`，为
stdout/stderr 分别维护：

- 不完整 UTF-8 尾部；
- 不完整文本行；
- 当前命令 ID 和 repository root；
- 到 recorder 的弱耦合引用。

每次收到字节后增量解码并按最多 64 KiB 输出；命令成功、失败、超时或取消退出前显式 `finish()`，
把两个流的剩余片段刷新。超长无换行内容拆成带连续 sequence 的多条记录，不等待 1 MiB 硬上限。

`ProcessLocalVerificationBackend` 为每个固定命令创建对应 sink 并传给 `SafeProcessRunner`。runner 继续
保留现有 1 MiB 原始输出上限、Job Object、超时和取消语义；日志 sink 不改变退出码或进程树判断。

### 6.2 安全处理

`infrastructure/release_log.rs` 中的 `ReleaseLogSanitizer` 位于发布控制台原始输出边界并复用 core
`safe_log::redact`：

1. 增量 UTF-8 解码，无效字节用明确替换符表示；
2. 移除 ANSI/终端控制序列；
3. 精确替换当前代理 URL、已知敏感环境值和高置信度 token；
4. 将 repository root 转成 `<repo>` 或相对路径，保留文件名、行列和错误上下文；
5. 执行现有 JSON、赋值、GitHub token、Bearer 和 query secret 脱敏；
6. 只把安全字符串交给 recorder；recorder 对结构化和进程消息统一再执行基础脱敏，原始 bytes 不进入
   事件、文件或 Debug。

本地发布命令的环境白名单继续不包含 API Key 和签名私钥。Git 与 gh 不连接原始 process sink，避免
公开代理参数、认证错误、命令行或机器 JSON；它们只由编排层生成结构化安全日志。

### 6.3 结构化进度

`services/release_log.rs` 中新增 `ReleaseProgressSink`，提供 started/log/completed/failed 四类公开行为。
`ReleaseOrchestrator::new()` 继续使用 no-op sink 兼容纯业务测试；生产通过 `with_progress` 注入 recorder。
主要 step ID 使用现有时间线和失败契约，不发明第二套阶段名称：

```text
candidate
release-structure-tests
release-console-rust-tests
full-project-check
ordinary-build
sourceAudit
commitPush
remoteRun
draftAudit
publishApproval
onlineVerification
cleanup
```

本地固定命令记录实际开始、完成、耗时和退出码。Git 提交/Push 记录操作边界和 SHA 短摘要，不记录
argv/stderr。发布 Run 的轮询位于 `release_orchestrator.rs` 的生产 remote backend，cleanup Run 的发现与
轮询位于 `github_release.rs::monitor_cleanup`；两处都在 status、conclusion、Job 或 Step 安全投影变化时
记录，并在无变化时每 5 分钟记录一次心跳，不能按 5 秒轮询间隔重复刷日志。Draft、公开、在线复核
与 cleanup 记录被验证的 ID、tag、状态和稳定失败分类。

两处轮询复用 `services/release_log.rs` 中的 `ReleaseRunProgressTracker`。tracker 接受安全状态投影与
单调时间值，纯粹决定“变化、心跳或静默”，不拥有轮询、睡眠或发布结果；生产传入 `Instant::now()`，
tracker 模块测试直接推进输入时间，绝不真实等待 5 分钟。两处真实轮询各有首个/最终投影的 wiring
测试，多次未变化与心跳分支由同一个 tracker 测试覆盖，避免为了测试等待真实 5 秒轮询。

## 7. Application 与错误顺序

`SystemReleaseApplication` 在新会话初始化 `session.json` 后调用 `ReleaseLogStore::initialize`；恢复、继续
发布或只加载现有会话时调用 `ReleaseLogStore::open`，从有效前缀恢复最后 sequence 后再构造 recorder，
并传给本地、Git 和 GitHub 管线。日志初始化/打开失败不阻止真实发布动作，但 recorder 必须发送易失
warning，并且旧文件因 session ID 不同不会被加载为新会话日志。

失败顺序固定为：

```text
flush process stdout/stderr tails
  -> recorder 持久化 error 日志
  -> ReleaseStateStore::fail 原子保存权威 failure
  -> 发送带 failure 的 SessionUpdated
  -> 发送 StepFailed
```

回滚不完整仍优先使用 `RELEASE_ROLLBACK_INCOMPLETE`，不得因已有日志就声称候选已恢复。event channel
发送失败不回滚日志文件，也不取消后台发布。

## 8. Vue 状态与组件边界

### 8.1 组件图

| 单元 | 单一职责 | 契约 |
|---|---|---|
| `App.vue` | 组合标题、上方发布区域和底部日志区 | 向下传 readonly 状态，转发分页动作 |
| `useReleaseSession.ts` | 会话、实时 sequence reducer、当前日志页和游标 | 暴露 readonly `logPage`、`logViewMode`、`unreadLogCount`、`logRequestPending`、`logError` 和三个显式动作 |
| `ReleaseLogPanel.vue` | 展示一页日志、跟随、分页、复制和键盘交互 | 接收上述 readonly 状态与 `failure`；发出 `load-earlier`、`refresh-log-page`、`return-to-latest` |
| `ReleaseStepDetails.vue` | 只展示会话 ID、版本、候选和 Run 摘要 | 不再拥有日志过滤或滚动状态 |
| `services/tauri.ts` | 唯一 Tauri command/Channel 边界 | 解包 snapshot/page 并保持 camelCase 类型 |

`useReleaseSession` 继续使用 channel generation 丢弃旧仓库/旧订阅事件。事件 reducer 遇到实时
`stepLog` 时只把 `event.entry` 路由到日志页状态并立即返回，绝不再追加到通用 `events` 数组；低频
started/completed/failed/session 事件才进入现有时间线状态。entry 以 `sessionId + sequence` 去重并只
追加到最新页；
最新页超过 2,000 条时移除最旧显示项，完整记录仍在 Rust 文件中。查看历史页时实时事件只更新最新页
元数据和未读提示，不覆盖当前阅读页。

### 8.2 布局

桌面 `app-shell` 使用三行：

```text
auto                          header
minmax(0, 1fr)               timeline + workspace
clamp(180px, 30vh, 280px)    ReleaseLogPanel
```

日志区是全宽工具带，不放在 `ElCard` 内，也不悬浮覆盖上方内容。桌面时间线和 workspace 继续分别滚动；
窄于 820px 时，上方布局变成单列并由外层统一滚动，日志行仍保留在 100dvh 网格底部。

### 8.3 日志交互

- 最新页默认跟随底部；用户向上滚动或进入历史页后停止跟随。
- “返回最新”调用 `returnToLatestLogs` 请求最新页并恢复跟随；失败事件不强制打断历史阅读，只显示
  明确失败提示和未读状态。
- “更早”调用 `loadEarlierLogs` 并使用 `nextBeforeSequence`；“更新”调用 `refreshLogPage` 重新读取
  当前页；“复制当前页”只复制当前安全页。
- 日志文本使用 escaped interpolation 和 `white-space: pre-wrap`，不用 `v-html`。
- 容器 `tabindex=0`、`aria-label=\"发布诊断日志\"`，按钮使用可见命令文本；状态不只靠颜色表达。
- 添加页面范围、总条数、总字节数、截断和持久化 warning，动态内容不能改变固定日志区高度。

## 9. 兼容性

- `session.json` schema version 继续为 1；`ReleaseSession` 不新增大日志字段。
- 旧仓库没有 `session.log.jsonl` 时返回空页，不显示虚构日志。
- 新日志 JSONL 自有 schema version 1；未来不兼容版本只影响日志读取，不阻止会话恢复。
- `get_release_session` 的前后端 DTO 在同一便携控制台内同步升级，不构成跨版本网络 API。
- 现有时间线 failure 优先级、旧事件 fallback 和旧失败会话保守显示继续保留。
- 发布摘要不嵌入完整日志；本任务只提供当前页复制。

## 10. TDD 与 mock 边界

### 10.1 被测公开接口

- `ReleaseLogStore::{initialize, open, append, load_page}`。
- `ReleaseProgressSink` 与 `ReleaseProcessLogSink` 的公开行为。
- `ReleaseOrchestrator` 的既有 run/push/publish 方法在注入 progress sink 后的可观察日志。
- `ReleaseApplicationBackend::execute` 对 snapshot/page 请求和实时 event 的结果。
- typed `ReleaseConsoleClient`、`useReleaseSession` readonly 日志状态和分页动作。
- `ReleaseLogPanel` props/emits、可见日志、滚动与控制行为。

### 10.2 mock 边界

- Store 测试使用真实 `tempfile`、JSONL 追加和原子替换，不 mock 文件系统。
- 输出流测试优先使用真实 `SafeProcessRunner` 临时 PowerShell；纯分块边界可直接测试模块内部 decoder，
  不添加 test-only 生产 API。
- Orchestrator 集成测试只替换既有 LocalVerification/Push/Remote trait，并真实运行 recorder/store；
  `release_orchestrator.rs` 模块测试使用 fake `GhBackend` 覆盖生产 `GithubRemoteBackend` 的 tracker wiring。
- cleanup 测试使用 fake `GhBackend` 驱动真实 `GithubReleaseService::monitor_cleanup` 的 tracker wiring；
  共享 tracker 测试直接传入可控时间，不真实等待。
- Application/command 测试使用完整 typed request/response 和内存 event sink，不断言 mock UI 元素。
- Vue 测试 mock 完整 `ReleaseConsoleClient`，挂载真实 composable/component，不 mock `ReleaseLogPanel`。
- 所有路径位于系统临时目录；需要 Relay 覆盖时必须成对设置且先验证不指向真实目录。

### 10.3 行为切片顺序

1. Store schema、轮换、分页、损坏与小 policy 上限。
2. 本地 stdout/stderr 在完成前可见，失败前 flush，安全处理保留上下文。
3. Orchestrator 全阶段结构化日志、状态变化去重与心跳。
4. Application snapshot/page、channel 断开仍持久化、旧会话隔离。
5. Vue 实时 reducer、分页、旧 channel 失效和一页渲染上限。
6. 固定日志区、交互、可访问性、桌面和窄窗口布局。

每个切片必须先运行红测，确认因目标行为缺失而失败，再写最小实现并重跑绿色；不得一次写完所有生产
代码后补测试。

## 11. 规范与验证

- 更新 `.trellis/spec/release/publishing.md` 的本地门禁契约：禁止原始输出直通，同时允许本设计定义的
  Rust 安全诊断日志、独立 JSONL、分页与容量边界。
- 更新 `.trellis/spec/backend/error-and-logging.md`：记录发布日志只替换敏感值、保留诊断上下文的边界。
- 专项 Rust、Vitest、Vue typecheck 后运行成对安全临时 Relay 覆盖下的 `npm run check`。
- 运行 `npm run build:release-console`，枚举实际 EXE 路径、大小、时间与 SHA-256。
- 启动发布控制台前端，在 900x620 和 600x760 视口截图，检查日志区位置、文本适配、页面横向溢出和
  空态；动态日志、分页和失败行为由组件/集成测试提供证据。
- 不执行签名、安装、升级、卸载、真实发布或远端清理，不把构建证据扩张为这些行为已验证。

## 12. 权衡与回滚

- 选择混合日志而非所有命令原始输出，保留本地编译/测试细节，同时避免 Git/gh 认证与 JSON 噪声。
- 选择独立 JSONL 而非嵌入 `session.json`，避免高频日志重写会话状态和 50 MiB IPC。
- 选择后端分页而非前端加载 10 万条，保证 WebView DOM 和单次响应有界。
- 选择日志 I/O 非致命，防止诊断系统改变发布事实；明确 warning 保留可观察限制。

若实施中必须回滚，应同时移除新 log store/DTO/command、process/progress sink、Vue panel/layout 和两份
规范更新，恢复 `ReleaseStepDetails` 现有日志空态。回滚不得删除 `session.json`、候选事务、用户配置或
远端发布对象；遗留的 `session.log.jsonl` 是 Git 元数据中的非秘密诊断文件，旧控制台会忽略它。
