# Provider 可用性与 Codex 兼容性测试规范

## Scenario：用户显式测试 Provider

### 1. 范围/触发条件

- 触发条件：用户在 Provider 详情点击“测试 API 可用性”或在确认高级对话框后点击“运行 Codex 兼容性测试”。
- 启动、自检、Provider 列表刷新、文件监控和普通配置写入不得隐式触发模型请求或 Codex 回合。
- 两类测试共用单活动注册表，但结果、超时和错误分类彼此独立；结果只存在本次前端会话。

### 2. 签名

```rust
pub async fn test_provider_api(
    state: State<'_, AppState>,
    provider_id: String,
    request_id: String,
    use_proxy: bool,
) -> Result<CommandResult<ProviderAvailabilityResult>, ()>;

pub async fn test_provider_codex_compatibility(
    state: State<'_, AppState>,
    provider_id: String,
    request_id: String,
    use_proxy: bool,
) -> Result<CommandResult<ProviderAvailabilityResult>, ()>;

pub fn cancel_provider_test(
    state: State<'_, AppState>,
    request_id: String,
) -> CommandResult<bool>;
```

服务边界为 `ProviderAvailabilityService::test_api(provider_id, request_id, use_proxy)` 和
`ProviderAvailabilityService::test_codex(provider_id, request_id, use_proxy)`。command 只负责 UUID/参数解析、
一次服务调用和 `CommandResult<T>` 映射；密钥不出现在 command 参数或返回值。

### 3. 请求/响应/环境契约

- `ProviderAvailabilityResult` 使用 camelCase，字段仅包括 `providerId`、`kind`、`status`、`code`、
  `message`、`model`、`durationMs`、`testedAt`、可选 `httpStatus` 和 `codexVersion`。
- API 请求固定发送一次 `/responses`：当前 Provider、当前偏好模型、Bearer 密钥、无 `tools`、
  `stream=false`、最多 16 个输出 token；30 秒超时、256 KiB 响应上限、不跟随重定向、不重试。
- 前端 IPC 使用 camelCase `useProxy: boolean`，Rust command 接收 `use_proxy: bool`。`false` 表示
  默认直连：不读取网络代理设置，API 客户端与 Codex gateway 都保持 `.no_proxy()`；`true` 表示请求
  使用已保存的 Relay 代理。
- 目标解析和测试读取密钥、Provider 偏好与网络代理时使用只读边界。`use_proxy=true` 只能在
  `network_proxy.enabled=true` 且地址非空时取得代理；缺失、禁用或空地址返回稳定错误，绝不静默
  回退直连。缺少 `providers.json` 或 `settings.json` 不得创建、备份或改写应用数据。
- API 探针与 Codex 兼容性 gateway 必须接收同一个已解析代理值；不得继承环境代理，也不得把代理
  地址记录到公开结果、日志或通知。
- Codex 请求先探测精确允许版本 `0.144.4`，再创建位于系统临时目录内的唯一 `CODEX_HOME`、
  `CODEX_SQLITE_HOME`、工作目录和纯文本 catalog。子进程只继承显式环境白名单和一次性假密钥；
  Rust 监控 gateway 才向目标 Provider 注入真实密钥。
- Codex 预检和真实运行都要求工具集合精确为 `update_plan`、`view_image`；任何工具/Hook/MCP/
  plugin/web/权限事件或未知 JSONL 均不得放行。
- preflight 回环服务器必须区分传输失败与完整请求验证失败：读取阶段的 `Io` 直接关闭该连接并
  继续等待，不得伪装成 HTTP 400；只有已完整解析但方法、端点、认证、模型、形状或工具集合
  不匹配时才返回 400 并 fail closed。
- preflight 通过需要两份独立证据同时成立：服务器得到的请求验证报告，以及 Codex 子进程的
  成功 JSONL 输出。响应写入/半关闭的本机收尾错误不得覆盖已经得到的验证报告；若 Codex 未
  收到完整 SSE，其进程输出验证仍会失败。
- 状态固定为 `passed`、`failed`、`unsupported`、`cancelled`。结果不得包含原始请求/响应、命令行、
  环境变量、临时路径、堆栈或密钥。

### 4. 验证与错误矩阵

| 条件 | 状态 | 稳定 code |
|---|---|---|
| Provider、模型或密钥缺失/配置无效 | `failed` | `PROVIDER_TEST_*` |
| `use_proxy=true` 且网络代理未启用或地址为空 | command failure | `PROVIDER_TEST_PROXY_DISABLED` |
| API 401/403/404/429/5xx、DNS/连接/TLS/超时 | `failed` | `API_HTTP_*` / `API_NETWORK_*` / `API_TIMEOUT` |
| 非 JSON、非 Responses、正文超过 256 KiB | `failed` | `API_RESPONSE_INVALID` / `API_RESPONSE_TOO_LARGE` |
| 找不到 Codex CLI | `unsupported` | `CODEX_CLI_MISSING` |
| CLI 版本不在精确允许列表 | `unsupported` | `CODEX_VERSION_UNSUPPORTED` |
| managed requirements、临时路径、工具预检或严格配置门禁失败 | `unsupported` | `CODEX_*_UNSUPPORTED` / `CODEX_PREFLIGHT_FAILED` |
| preflight 连接读取发生瞬时 `Io` | 不返回 400；继续等待下一连接，最终仍受原超时约束 |
| preflight 完整请求不符合安全契约 | 返回 400，并映射 `CODEX_PREFLIGHT_FAILED` |
| Codex 工具调用、未知 JSONL、远端协议/退出异常 | `failed` | `CODEX_TOOL_CALL_BLOCKED` / `CODEX_JSONL_INVALID` / `CODEX_PROCESS_FAILED` |
| 超时或进程树无法终止 | `failed` | `CODEX_TIMEOUT` / `CODEX_PROCESS_TREE_FAILED` |
| 用户取消 | `cancelled` | `PROVIDER_TEST_CANCELLED` |
| 临时目录清理未验证成功 | `failed` | `CODEX_CLEANUP_FAILED` |

CLI 缺失和版本漂移必须保持不同 code；不能用“版本不支持”掩盖可执行文件解析失败。

### 5. 良好/基线/错误用例

- 良好：回环 Provider 返回完成的 Responses JSON；API 结果为 `passed`，请求只出现一次，Debug/日志/DTO 不含密钥。
- 良好：默认 `use_proxy=false` 在没有 `settings.json` 时仍直连成功且文件保持不存在；启用代理后 API
  与 Codex gateway 都使用同一已保存代理。
- 基线：Provider 没有密钥、没有模型偏好或用户取消；不建立外部请求，返回稳定安全结果并释放活动注册项。
- 基线：测试目录中缺少 `providers.json`/`settings.json` 时，测试结束后两文件仍不存在。
- 错误：Codex 预检工具集合漂移、真实 SSE 含 function/custom tool call、stdout/stderr 超限、派生进程未退出或清理失败；
  必须在相应边界 fail closed，不得把部分成功显示为通过。请求已启用但不存在的代理后悄然改走直连同样禁止。

### 6. 必需测试及断言点

- Rust 单元：DTO 序列化、目标解析、API 请求构造/响应上限/错误分类、单活动取消、版本与 CLI 缺失分类。
- Rust 服务/command：`use_proxy=false` 不创建设置且 API/Codex 都不使用代理；`use_proxy=true` 的禁用或空
  代理返回 `PROVIDER_TEST_PROXY_DISABLED`，camelCase `useProxy` 正确映射到 command 参数。
- Rust 回环集成：预检核对 Provider/模型/Bearer 假密钥/工具集合；真实 gateway 只注入目标测试密钥，工具 SSE 到达 Codex 前被阻断。
- Windows 进程专项：Job Object 加入、父子树终止、1 MiB stdout/stderr 上限、超时/取消、清理前路径校验和清理失败。
- preflight 协议单元测试使用内存 `Read + Write`，断言传输 `Io` 不产生 400、完整非法请求仍产生
  400；服务集成测试保留真实 TCP，并分别覆盖正常 gateway 与工具调用阻断。
- `path_safety`：默认 `.codex` 与 `CodexRelay` 哨兵递归快照前后一致；测试只使用 `AppPaths::for_test` 和回环边界。
- Vitest：API/Codex 结果独立、确认门禁、取消、指纹失效、禁用原因、aria-label 和窄窗口布局。

### 7. 错误与正确做法

#### 错误

```rust
// 把找不到可执行文件和版本漂移合并，调用方无法给出准确行动建议。
CodexRunnerError::UnsupportedVersion | CodexRunnerError::ExecutableUnavailable
```

#### 正确

```rust
match error {
    CodexRunnerError::ExecutableUnavailable => "CODEX_CLI_MISSING",
    CodexRunnerError::UnsupportedVersion => "CODEX_VERSION_UNSUPPORTED",
    _ => /* 其他稳定安全分类 */,
}
```

传输层 `Io` 不能改写成协议拒绝：

```rust
match parsed_request {
    Err(CodexPreflightFailure::Io) => Err(CodexPreflightFailure::Io),
    Err(validation_failure) => {
        let _ = write_bad_request(stream);
        Err(validation_failure)
    }
    Ok(report) => {
        let _ = write_preflight_sse(stream);
        Ok(report)
    }
}
```

代理选择必须在 service 统一完成，不能只由 UI 或某一个测试路径决定：

```rust
// 错误：API 与 Codex 对代理模式采用不同默认值，或禁用代理后静默直连。
let api_proxy = settings.network_proxy.enabled.then_some(settings.network_proxy.url);
let gateway = CodexCompatibilityGateway::start(target, None).await?;

// 正确：两个路径使用相同的只读解析结果。
let proxy = self.resolve_test_proxy(use_proxy)?;
provider_http::probe_api(&target, proxy.as_deref(), &self.app_version, &mut active.cancel).await;
let gateway = CodexCompatibilityGateway::start(target, proxy.as_deref()).await?;
```

两类测试都必须先解析目标 Provider 的密钥；但 API 密钥只在后端 HTTP 客户端短暂使用，Codex 密钥只在受监控 gateway 的上游
Header 中使用，任何一层都不得把它放进 argv、临时文件、JSONL、日志或前端普通状态。
