# 错误、日志与事件

## 公开错误

公开错误必须包含稳定 `code` 和可理解的简体中文 `message`。现有关键错误类别包括验证失败、配置损坏、缺少密钥、Provider 不存在、当前 Provider 禁止删除、外部修改冲突、事务失败、回滚不完整、自启失败和系统集成错误。

底层错误、Rust backtrace、完整路径上下文和秘密只可在脱敏后进入内部日志；前端、托盘和 Windows 通知不得收到它们。

## 真实性规则

- 只有所有触及文件恢复并逐字节/存在状态验证成功，消息才可说“原配置已恢复”。
- 回滚不完整必须返回 `ROLLBACK_INCOMPLETE`，保留事务标记并引导备份恢复。
- 恢复文件成功但后续 Provider/自检刷新失败时，分别报告“恢复完成”和“状态刷新未完全成功”。
- Codex CLI 缺失或超时是 warning，不阻止 Provider 管理。

## 脱敏范围

必须覆盖 `OPENAI_API_KEY`、`apiKey`、Authorization、Bearer、JSON 密钥字段以及 URL 查询中的 token/key。新增日志时优先不传入秘密，不能依赖正则脱敏替代数据最小化。

## 错误矩阵

| 条件 | 公开行为 |
|---|---|
| `config.toml` 无法解析 | 返回安全错误，不修改文件 |
| `providers.json` 无法解析 | 保存损坏副本，返回错误，不覆盖原件 |
| 编辑指纹过期 | 返回外部修改冲突，不创建旧内容写入 |
| 目标 Provider 无密钥 | 返回缺少密钥，不修改配置 |
| 自动回滚未完全验证 | 报告回滚不完整，保留事务标记 |

## 事件与通知

事件只传 DTO、状态、指纹或安全消息。禁止传 `auth.json`、`providers.json` 全文、Authorization Header 或 API Key。测试快照和 Debug 输出适用同一规则。

## Provider 测试错误与日志契约

### 范围与触发

仅适用于用户显式触发的 API 可用性测试和 Codex 兼容性测试；启动、自检和文件监控不得产生
Provider 测试错误或网络日志。

### 公开签名与契约

后端返回 `ProviderAvailabilityResult` 的稳定 `status/code/message`，command 失败只表示无法
建立安全测试上下文。用户显式 API 测试结果可携带同次请求生成的有界 trace；除此之外，公开结果
不得带 SSE、JSONL、argv、环境变量、临时路径或堆栈。trace 不含 Header、API Key 或代理地址，
Codex 结果、日志、通知、事件与 Debug 不得包含 trace 正文。

### 验证与错误矩阵

- `CODEX_CLI_MISSING` 只表示 PATH 中没有可解析的 Codex 可执行文件。
- `CODEX_VERSION_UNSUPPORTED` 只表示 CLI 已找到但版本不在实验允许列表。
- `CODEX_TOOL_CALL_BLOCKED`、`CODEX_JSONL_INVALID`、`CODEX_PROCESS_FAILED` 和
  `CODEX_CLEANUP_FAILED` 必须分别保留安全边界、协议、进程和清理语义。
- 取消返回 `cancelled`；不能把用户取消改写成普通超时或泛化失败。

### 良好/基线/错误用例

- 良好：日志只记录测试类型、Provider ID、稳定 code、耗时和 HTTP 状态/版本。
- 良好：`ProviderAvailabilityTarget` 与 trace 的 Debug 只记录“已配置”、method、状态和长度等元数据，
  不记录 Base URL、请求/响应正文或密钥。
- 基线：远端 401/429/5xx 只记录分类，不记录响应正文。
- 错误：将 `Debug` 的 `ProviderAvailabilityTarget`、Authorization 或 child stderr 原样写入日志。

### 必需测试

单元测试断言每个错误映射的 code/message；公开 API trace 序列化需包含约定的请求/响应正文，
同时断言不含 `test-key`、Bearer 值、URL userinfo、查询 token、Header 或临时路径。Debug、日志捕获、通知、
事件和 Codex DTO 仍必须断言不含正文。CLI 缺失回归必须与版本漂移回归分别断言。

### 错误与正确做法

#### 错误

```rust
tracing::warn!(error = ?error, "Codex test failed");
```

#### 正确

```rust
tracing::warn!(code = "CODEX_CLI_MISSING", "Codex 兼容性测试不可用");
```

底层错误只可在脱敏、最小化且不含路径/秘密的内部诊断中使用；面向用户的消息必须来自稳定分类表。
