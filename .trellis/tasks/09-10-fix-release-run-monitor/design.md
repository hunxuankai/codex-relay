# 远端 Run 监控故障修复设计

## 根因与边界

`GithubReleaseService::get_release_run` 把 `GhBackend` 的所有错误归为 `BackendFailed`；`GithubRemoteBackend::wait_for_run` 在第一次查询错误时通过 `?` 结束轮询；编排层再次用 `Err(_)` 丢弃原因。application 随后持久化 `failed`，恢复路由只返回快照。这条路径可以把仍在运行、最终成功的 Run 变成无法继续的本地失败会话。

历史日志和 session 没有原始类别，因此本次 08:30:25 的具体触发原因不可还原。不会把猜测的网络原因写为事实。远端已成功并保留 Draft，只进行只读核验。

## 行为切片与契约

### 1. 有界监控与安全错误

- `ReleaseRemoteBackend::wait_for_run` 改为返回类型化 `GithubReleaseError`，不让任意字符串进入日志、事件或 session。
- Run 查询保留现有安全进程码，映射为明确的 `GITHUB_COMMAND_FAILED`、`GITHUB_PROCESS_TIMEOUT`、取消、启动、进程树终止和输出上限类别；未知错误只保留固定通用码。
- 仅对只读查询的命令失败与进程超时自动重试。最多连续失败 4 次，成功查询清零计数；默认仍每 5 秒轮询。重试 warning 记录 Run ID、固定错误码与次数，不输出 stderr、JSON、环境或 URL 参数。
- 保留现有至少 4 小时监控预算与次数边界，并以单调时钟检查实际经过时间。到期后不再开始新查询；已启动的安全进程自行完成或在既有 60 秒进程预算内安全终止，禁止外层粗暴丢弃进程 future。
- JSON、Run ID、固定仓库 URL、SHA 或状态不可信时立即停止；取消、安全进程错误与真实非成功结论不能被重试掩盖。每次观察核对身份，不等到最终成功才检查。
- 编排错误携带类型化来源，以 `remoteRun` 记录、持久化和发送具体 code/message。真实远端失败与查询中断分别陈述。

### 2. 持久化恢复

- 新监控失败仍使用现有 `failed + failure`，保留精确失败证据与 Run。session schema version 继续为 1。
- 只允许 `failure.phase=workflowRunning`、来源为可重试查询/监控预算错误的会话恢复。兼容历史 `stepId=releasePipeline && code=RELEASE_REMOTE_FAILED`，但必须有完整一致的候选/已推送 SHA 和固定 Run 身份，且没有 Draft/公开/清理的更晚证据。
- 后端模型提供唯一权威恢复条件；专用状态方法在仓库锁内重新核对持久化 session 与调用方一致，再通过现有 atomic_write 原子恢复 `workflowRunning` 并清除当前 failure。原失败仍保留于日志。通用 phase transition 不放开 `failed`。
- 失败收尾按 session ID 限定所有权；锁失败或恢复快照已失效只报告当前请求错误，不修改较新/未拥有的会话。恢复日志重新记录上次稳定失败码，避免旧日志缺失时只留下无区分度的恢复消息。
- application 把这些已验证会话路由到现有远端编排。恢复后重查同一 Run，复核 SHA、成功结论和 Draft；不执行本地工作、push、dispatch 或自动公开。

### 3. UI 与组件边界

- `ReleaseRecoveryPanel` 继续只接收 typed `session/busy/proxyInvalid`，根据保守的失败阶段/代码与已记录 Run 显示“继续监控”，只 emit `resume`。
- UI 条件只决定按钮是否可用；Rust 恢复门禁重新校验所有证据，不信任 UI。保持现有 IPC/session DTO，无新增派生持久化字段。
- nullable 的更晚阶段证据严格按 `null` 判定，不能把空字符串当作不存在。
- `App` 与 `useReleaseSession` 继续承担既有调用与事件状态管理，不引入新页面、网络调用、全局 store 或大规模组件重构。

## 测试与权衡

- 用序列化 mock GhBackend 验证真实监控入口：运行中 → 查询失败 → 成功；失败计数重置；连续故障有界退出；安全/身份错误不重试。
- 监控策略只作为内部测试注入点缩短测试等待，不提供用户可调的安全边界。
- 用真实 tempfile 状态文件验证旧 session 恢复、错误恢复拒绝、不放开通用终态转换、陈旧状态不覆盖；用 mock remote 验证 dispatch=0、停在公开审批。
- application 测试验证错误日志 → sessionUpdated → stepFailed 顺序与 code/step，Vue 测试验证按钮、busy/代理门禁及原有失败结果入口。
- 不扩展到 cleanup 或其他写操作的自动重试；这些行为与本次已确认 Run 监控故障独立。

## 回滚与交付

代码改动可由 Git 回退；不直接编辑真实发布 session 或远端 Draft。恢复后的 session 仍由既有 schema 读取。构建独立发布控制台并记录实际 EXE 路径、大小、时间和 SHA-256；进程占用时保留明确限制，不强制终止用户应用。测试与构建不读取真实 Codex/Relay 用户数据。
