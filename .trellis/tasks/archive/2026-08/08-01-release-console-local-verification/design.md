# 发布控制台本地门禁失败诊断设计

## 1. 设计目标

用两个最小、相互独立的契约修复当前失败：机器判断只依赖 ASCII 稳定码；本地门禁错误在 service → orchestrator → application event 的既有路径中保留命令身份和可选退出码。候选事务、门禁顺序和敏感输出边界保持不变。

## 2. 编码无关的验证脚本契约

`scripts/validate-release-request.ps1` 在疑似秘密分支抛出：

```text
RELEASE_NOTES_SECRET_DETECTED: 发布说明包含疑似秘密，已停止发布。
```

ASCII 前缀是自动化契约，后续中文是人工可读说明。`src/release-request.test.ts` 只断言前缀、非零退出状态和 workflow output 不存在。这样即使 Windows 子进程把中文按其他代码页输出，ASCII 部分仍可稳定解码。

该修复只覆盖当前被测试消费的秘密检测错误，不把所有 PowerShell 文案一次性改造成错误码体系。

## 3. 本地门禁错误数据流

### 3.1 Service 层

`LocalVerificationError::CommandFailed` 扩展为：

```rust
CommandFailed {
    command_id: String,
    exit_code: Option<i32>,
}
```

- backend 成功返回 `LocalCommandEvidence` 但 `exit_code != 0`：保存 `Some(exit_code)`。
- backend 返回通用启动/运行失败：保存 `None`。
- backend 返回取消：继续映射为 `Cancelled`。

不修改 `LocalCommandEvidence`，也不把 `ProcessOutput.stdout/stderr` 提升到 service 层。

### 3.2 Orchestrator 层

`ReleaseOrchestratorError::LocalVerificationFailed` 保存相同字段。`run_to_pushed` 仍先调用 `ReleaseCandidateTransaction::rollback_active`：

1. 回滚失败时优先返回 `RollbackFailed`；
2. 取消时保持 `Cancelled`；
3. 普通命令失败在会话推进到 `Failed` 后返回携带命令证据的 `LocalVerificationFailed`。

错误类型提供安全事件投影：默认阶段仍是 `releasePipeline` 和现有通用消息；只有本地门禁失败使用动态命令 ID，并根据 `exit_code` 生成“退出码 N”或“命令未能启动”的文案。

### 3.3 Application event 层

`SystemReleaseApplication` 增加接收完整 `ReleaseOrchestratorError` 的内部收尾入口，再复用现有会话终态处理和取消注册逻辑。原先只持有字符串错误码的调用仍走通用 `finish_with_error`，避免扩大其他错误类型的改动。

本地门禁事件示例：

```text
[release-structure-tests] RELEASE_LOCAL_VERIFICATION_FAILED：本地发布门禁退出码 1；候选文件已回滚，尚未提交或推送。
```

进程后端失败且没有可用退出码的示例：

```text
[release-structure-tests] RELEASE_LOCAL_VERIFICATION_FAILED：本地发布门禁命令未能完成，且没有可用退出码；候选文件已回滚，尚未提交或推送。
```

## 4. 测试设计

### 行为切片 A：稳定错误码

- 先把现有 Vitest 断言改为 `RELEASE_NOTES_SECRET_DETECTED` 并补充 workflow output 不存在断言，确认脚本尚未输出该码时测试失败。
- 修改脚本后重跑目标测试。
- 通过 `ProcessLocalVerificationBackend` 的真实过滤环境运行固定发布结构测试，覆盖用户实际失败路径；只断言退出码，不采集或打印原始输出。

### 行为切片 B：失败证据传播

- Rust service 测试先要求非零退出保留 `Some(1)`，backend 失败保留 `None`。
- orchestrator 测试先要求候选回滚后错误仍带命令 ID/退出码。
- application 测试先要求 `StepFailed` 使用具体步骤和安全消息；没有退出码时不得虚构启动或超时类别。
- 实施最小字段传播与事件投影后重跑专项测试。

所有文件系统测试使用 `tempfile` 或系统临时目录，不读取、写入或删除真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`。

## 5. 兼容性与安全

- `ReleaseEvent` 的 JSON/TypeScript 结构不变，只改变本地门禁失败事件的字段值，无 schema 迁移。
- `RELEASE_LOCAL_VERIFICATION_FAILED` 稳定码不变，现有消费者继续兼容。
- 不存储 stdout/stderr；测试失败也不得打印可能含秘密的原始输出。
- PowerShell 中文文案保留，人工直接运行脚本仍可读。

## 6. 回滚

代码回滚只需还原脚本错误前缀、错误字段和事件投影。候选事务格式、发布会话 schema、Git 状态和远端状态均未迁移，无额外数据回滚步骤。
