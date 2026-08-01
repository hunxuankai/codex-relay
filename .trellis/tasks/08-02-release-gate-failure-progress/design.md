# 发布门禁进程收尾与失败进度持久化设计

## 1. 设计目标

在不放宽发布安全门禁的前提下修复两个相关缺陷：成功的本地完整检查不应因可安全清理的残留后代
被误报为失败；真正失败时，具体安全原因和失败步骤必须跨层保留并持久化，使发布进度在控制台重启后
仍停留在失败现场。

## 2. 方案比较与选择

### 方案 A：只在前端回放实时事件

改动最小，但事件未持久化，控制台重启后无法恢复失败位置，也不能解决进程错误被折叠的问题。

### 方案 B：持久化完整事件日志

可以重建所有时间线细节，但会引入新的日志 schema、保留策略、校验和迁移责任；当前需求只需要终态
失败检查点，属于过度设计。

### 方案 C：修复安全收尾、保留类型化错误、在现有会话持久化紧凑失败检查点

复用既有 Job Object、`ReleaseStateStore` 和 `ReleaseSession`，只增加必要字段与转换。该方案同时满足
真实完成语义、诊断能力和跨重启展示，采用此方案。

## 3. 跨层数据流

```text
SafeProcessRunner
  → LocalVerificationBackendError
  → LocalVerificationError
  → ReleaseOrchestratorError
  → ReleaseStateStore::fail
  → ReleaseSession.failure
  → ReleaseTimeline
```

每层只保存相邻层需要的最小安全证据；原始输出和进程明细停留在进程边界，不进入上层。

## 4. SafeProcessRunner 收尾语义

主进程通过 `try_wait` 返回退出状态后，继续等待现有 `PROCESS_TERMINATION_GRACE`。若宽限结束时 Job
仍有活动进程：

1. 调用现有 `terminate_job_and_wait`，记录并终止 Job 中的剩余进程；
2. 验证 Job 活动进程为零，且已跟踪 PID 均已退出；
3. 验证成功后继续收集有界 stdout/stderr，并返回主进程的真实退出码；
4. 验证失败才返回 `ProcessError::ProcessTreeTermination`。

该调整不改变超时、取消和输出过大路径。超时/取消仍要求终止整棵进程树后返回相应错误；无法终止时
仍升级为 `ProcessTreeTermination`。

## 5. 本地门禁错误模型

service 层新增与基础设施解耦的安全分类：

```rust
enum LocalVerificationProcessError {
    JobUnavailable,
    JobAssignment,
    ProcessStart,
    ProcessResume,
    OutputTooLarge,
    Timeout,
    ProcessTreeTermination,
    OutputRead,
    InputTooLarge,
    InputWrite,
}

enum LocalVerificationFailure {
    ExitCode(i32),
    Process(LocalVerificationProcessError),
}
```

`LocalVerificationBackendError` 使用 `Cancelled` 与 `Process(LocalVerificationProcessError)`；
`LocalVerificationError::CommandFailed` 和 `ReleaseOrchestratorError::LocalVerificationFailed`
携带同一个 `LocalVerificationFailure`。这样不存在“两个 Option 必须恰好一个有值”的隐式不变量。

application 继续发送稳定 code `RELEASE_LOCAL_VERIFICATION_FAILED`，消息按分类生成：

| 失败证据 | 用户消息核心内容 |
|---|---|
| `ExitCode(N)` | 本地发布门禁退出码 N |
| `ProcessStart` / `ProcessResume` / Job 错误 | 本地发布门禁进程无法安全启动 |
| `Timeout` | 本地发布门禁超过允许时间 |
| `OutputTooLarge` | 本地发布门禁输出超过安全上限 |
| `ProcessTreeTermination` | 本地发布门禁进程树未能安全结束 |
| `OutputRead` | 无法完整读取本地发布门禁结果 |
| `InputTooLarge` / `InputWrite` | 本地发布门禁输入边界失败 |

所有消息继续附带候选已回滚且尚未提交/推送的真实边界；回滚不完整时不使用这些消息。

## 6. 持久化失败检查点

`ReleaseSession` 增加可选字段：

```rust
struct ReleaseFailureEvidence {
    phase: ReleasePhase,
    step_id: String,
    code: String,
}

failure: Option<ReleaseFailureEvidence>
```

- `phase` 是进入 `Failed` 前的最后阶段；`step_id` 优先使用具体本地命令或公开时间线步骤；`code`
  是稳定错误码。
- `ReleaseStateStore::fail(session, step_id, code)` 在一次原子保存中完成阶段转换和失败证据写入。
- 当前直接 `advance(..., Failed)` 的失败路径改用 `fail`；application 的通用收尾入口也使用 `fail`
  作为兜底。
- `ReleaseSession::new` 固定 `failure=None`，因此新发布自然清空旧现场。
- 字段使用 `#[serde(default)]`，schema version 保持 1。旧文件缺少字段时读取为 `None`；不迁移、不猜测。
- TypeScript DTO 使用必需的 `failure: ReleaseFailureEvidence | null`，与后端序列化结果精确对应；
  不用可选属性掩盖测试 fixture 漂移。
- 校验规则：非 `Failed` 会话不得携带 failure；failure 的 `phase` 必须为非终态，`step_id/code`
  不得为空。为兼容旧文件，`Failed + None` 仍允许读取。

不持久化 message，避免本地化文案成为状态契约；界面从稳定 code/分类生成消息，时间线只消费
`phase/stepId`。

## 7. 时间线失败状态投影

`ReleaseTimeline` 的失败位置来源优先级：

1. `session.failure`，用于跨重启权威恢复；
2. 当前事件流中最后一个 `stepFailed`，兼容尚未刷新到 session 的短暂窗口；
3. 当前事件流中最后一个非终态 `sessionUpdated.phase`，兼容旧后端；
4. 旧失败会话完全没有证据时保持保守“未开始”，不虚构位置。

固定命令映射：

| 后端 stepId | 可见步骤 |
|---|---|
| `release-structure-tests` | 发布专项 |
| `release-console-rust-tests` | 发布专项 |
| `full-project-check` | 完整检查 |
| `ordinary-build` | 普通构建 |

已是时间线 step ID 的值直接使用；`releasePipeline` 或未知值按 failure.phase 映射到现有阶段步骤。
确定失败索引后：之前为 `completed`，当前为 `failed`，之后为 `waiting`。失败标签使用 Element Plus
`danger` 类型和可见文本“失败”，不依赖颜色。取消状态保持现状。

## 8. 兼容性与安全

- Rust/TypeScript DTO 只新增可选失败证据，不改变 command 或事件 schema。
- session schema version 不变，已有终态文件继续可读；新字段由后端权威生成，前端不自行写入。
- 不改变候选事务、Git 索引清理、commit、push、取消或远端发布流程。
- 不记录 stdout、stderr、环境值、代理、Token、PID 或进程命令行。
- 所有进程与状态测试使用系统临时目录；需要环境覆盖时使用安全临时 USERPROFILE/APPDATA/
  LOCALAPPDATA/NPM cache，不访问真实 Codex/Relay 数据目录。

## 9. 测试设计

### 行为切片 A：成功主进程的安全后代清理

- 核心 runner 测试构造“父进程退出 0、后代继续运行”的真实 Windows 进程树。
- 预期 runner 安全终止后代并返回 `Some(0)`；另用 fake Job 保留无法终止的失败断言。
- `ProcessLocalVerificationBackend` 集成测试通过相同边界验证不会折叠为 backend failure。

### 行为切片 B：类型化错误传播

- service fake backend 分别返回退出码和进程分类；断言后续命令停止且证据不丢失。
- orchestrator 断言回滚后仍保留同一分类且不 Push。
- application 断言稳定 code、具体 stepId、分类消息与回滚文案，不含原始输出。

### 行为切片 C：失败检查点原子持久化

- state 测试先要求 `fail` 同时保存原阶段、stepId、code 和 `phase=failed`。
- 旧 schema version 1 JSON 缺少 failure 时继续读取；非 failed 携带 failure 被拒绝且原文件不重写。
- 新 session 初始化覆盖终态失败会话且 `failure=None`。

### 行为切片 D：跨重启时间线

- Vue 组件使用失败 session DTO 验证此前完成、当前失败、后续未开始和“失败”文本。
- 相同 DTO 在没有 events 时仍正确，模拟控制台重启加载。
- 新 session `failure=null` 时恢复正常阶段投影；旧事件回退不覆盖新 session。

### 行为切片 E：生产门禁回归

- 保留快速、确定性的进程树集成测试作为默认套件回归。
- 另提供显式慢速探针，通过生产 `ProcessLocalVerificationBackend` 执行真实
  `full-project-check`；探针默认忽略，验证时在外层 Cargo 已退出的独立测试进程中运行，避免嵌套
  Cargo target 锁与自递归。

## 10. 回滚

- runner 收尾语义、错误分类、session failure 字段和时间线投影可以按层回滚。
- 新字段为可选且 schema version 未变化，代码回滚后 serde 会忽略未知字段；没有数据迁移或文件清理。
- 回滚不得删除已有发布 session、候选备份或用户数据。
