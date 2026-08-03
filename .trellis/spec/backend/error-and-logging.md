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
| 当前偏好模型不支持 Fast | `MODEL_FAST_UNSUPPORTED`，事务前失败且不修改四个受管文件 |
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

## 发布诊断日志的安全上下文与流式切分契约

### 1. 范围/触发条件

修改 core `safe_log`、发布控制台 stdout/stderr sanitizer、流式分块、已知敏感值替换、公开日志
`Debug` 或远端结构化进度时，必须遵循本节。目标是只替换敏感值并保留测试名、相对文件位置、行列、
错误正文、退出码和相邻上下文，而不是把所有诊断退化成通用摘要。

### 2. 签名

```rust
safe_log::redact(input: &str) -> String
safe_log::redaction_safe_split_index(input: &str, preferred: usize) -> usize

ReleaseLogSanitizer::new(repository_root, sensitive_values)
ReleaseLogSanitizer::sanitize(input: &str) -> String
ReleaseProcessLogSink::on_output(stream, bytes)
ReleaseProcessLogSink::finish()
```

`redaction_safe_split_index` 返回不穿过 core 权威敏感匹配的 UTF-8 前置切点；调用方仍负责仓库路径、
ANSI、代理 URL、已知环境值和增量 UTF-8 的发布专用处理。

### 3. 契约

- 数据最小化优先：Git/`gh` 不接入原始 process sink，只记录结构化安全投影；任何路径都不得记录
  argv、完整 environment、认证文件、Authorization Header、代理 URL 或机器 JSON。
- 本地门禁分别增量解码 stdout/stderr，移除 ANSI 与控制符，把仓库根和本地 executable 绝对路径
  规范化为安全占位或相对位置，再精确替换代理 URL、已知敏感环境值和高置信度 token。
- core `redact` 是 JSON 密钥字段、赋值、GitHub token、Bearer 和 query secret 的唯一权威正则集合；
  发布层调用它，不复制第二套等价模式。recorder 构造 entry 前和 store 序列化前再次调用它作为纵深
  防御。
- 流式输出先形成完整逻辑行并整体 sanitize，再按不超过 `64 KiB` 的公开 entry 拆分。超长无换行流
  只可在保留 256 字节安全 lookahead 后尝试切分；切点穿过敏感匹配或 ANSI 序列时必须把该段留到
  后续字节，而不是按固定大小泄漏 continuation。
- 已知敏感值可能超过 256 字节。切点探测使用其 UTF-8 安全前缀定位起点，但按完整值长度判断是否
  穿过切点；不得只搜索末尾 lookahead 内的完整值。
- `finish()` 在成功、非零、取消、超时和 process error 所有出口刷新两个流的剩余片段，且重复调用
  不得重复记录。
- `ReleaseLogEntry` 的 `Debug` 只输出 session、sequence、时间、step/source/level 和消息长度；公开
  event、错误链或测试失败不得通过派生 Debug 打印 message 正文。

### 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| UTF-8 字符跨输入 chunk | 延迟不完整尾字节；完成后只产生原字符，不重复 replacement character |
| Bearer/不完整 JSON 密钥跨 64 KiB 切点 | 切点退到匹配起点，等待完整值后输出 `[REDACTED]` |
| 已知敏感值长度超过 256 字节 | 前缀探针阻止 continuation 泄漏，最终只显示占位符 |
| ANSI 序列跨安全尾边界 | 不拆开控制序列；最终公开文本不含控制符 |
| 仓库或工具绝对路径出现 | 只保留 `<repo>`、相对文件、行列或安全工具名，不泄漏本机目录 |
| 输出含普通错误上下文 | 测试名、错误正文、退出码和相邻非敏感文本继续可见 |
| 任一 process 出口仍有尾部 | `finish()` 先刷新尾部，再记录稳定失败；第二次 `finish()` 无重复 |

### 5. 良好/基线/错误用例

- 良好：编译器输出保留 `src/release.rs:17:9` 与 assertion 正文，同时把仓库根、Bearer 和代理值替换
  为明确占位符。
- 良好：长 Bearer 从一个 64 KiB entry 开始并延续到下一输入 chunk，公开页中没有任一 token 片段。
- 基线：无敏感匹配的超长普通诊断按 64 KiB 连续 entry 输出，sequence 严格递增。
- 错误：先固定切成 64 KiB，再分别对每块运行正则；后一块不含 `Bearer` 前缀时会泄漏 continuation。
- 错误：因担心秘密而只保存“命令失败”，丢掉测试名、相对位置、错误正文和退出码。

### 6. 必需测试

- core `safe_log`：完整与不完整 JSON、赋值、GitHub token、Bearer、query secret，以及跨切点返回索引。
- 发布 sanitizer：增量 UTF-8、CRLF、ANSI、仓库/工具路径、代理、已知环境值和高置信度 token。
- 流式边界：长 Bearer、长已知敏感值、敏感匹配和 ANSI 恰好跨 256 字节安全尾；断言公开 entry、
  JSONL 和 Debug 都不含测试 secret 的任一 continuation。
- 真实临时 PowerShell：首段在进程完成前可见；非零退出时尾部早于稳定错误日志。
- fixture 只使用 `test-key-*-not-real`，并在成对安全 Relay 覆盖下运行最终秘密/路径扫描。

### 7. 错误与正确做法

错误：固定切块后再脱敏，敏感值跨块时第二段失去上下文。

```rust
for chunk in input.as_bytes().chunks(64 * 1024) {
    recorder.record(redact(String::from_utf8_lossy(chunk)));
}
```

正确：由权威正则决定安全切点，完整逻辑段先 sanitize，再形成公开 entry。

```rust
let split = redaction_safe_split_index(&pending, preferred);
let safe = sanitizer.sanitize(&pending[..split]);
recorder.record(safe);
```
