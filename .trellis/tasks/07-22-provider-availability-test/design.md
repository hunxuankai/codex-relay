# 技术设计

## 架构与边界

新增独立的 Provider 可用性域，不把测试状态塞入长期 Provider 真相，也不让前端接触密钥。

```text
ProviderAvailabilityPanel
  → useProviderAvailability 显式动作与会话内结果
  → src/services/tauri.ts typed commands
  → provider_availability_commands
  → ProviderAvailabilityService
     ├→ ProviderService::resolve_availability_target（只读配置/偏好/密钥）
     ├→ ProviderApiProbe（reqwest，无工具最小 Responses）
     ├→ CodexCompatibilityRunner（临时状态、回环预检、受监控 codex exec）
     ├→ CodexCompatibilityGateway（真实 Provider 转发、SSE 工具调用阻断）
     └→ ActiveTestRegistry（单任务、取消）
        ↓
   bounded HTTP / bounded JSONL / Windows Job Object / verified cleanup
```

`ProviderAvailabilityService` 是公开业务入口。Tauri command 只校验 UUID 请求 ID、委托一次服务
调用并映射 `CommandResult<T>`；它不读取 Provider 文件、不构造 HTTP、不拼接 Codex 参数。

本功能只读受管文件，不经过 `TransactionService`，也不创建备份。任何产品配置写入仍沿用原有
事务链；测试临时文件位于系统临时目录，不属于受管 Codex/Relay 文件。

## DTO 与公开契约

Rust 在 `models/provider_availability.rs` 定义并以 camelCase 序列化；TypeScript 在独立
`types/providerAvailability.ts` 镜像：

```ts
type ProviderTestKind = 'api' | 'codex'
type ProviderTestStatus = 'passed' | 'failed' | 'unsupported' | 'cancelled'

interface ProviderAvailabilityResult {
  providerId: string
  kind: ProviderTestKind
  status: ProviderTestStatus
  code: string
  message: string
  model: string
  durationMs: number
  testedAt: string
  httpStatus?: number | null
  codexVersion?: string | null
}
```

稳定 code 由后端统一拥有。UI 只按 `status` 选择 Tag 类型并显示 `message`，不解析远端错误正文、
JSONL 字段或退出码。结果 DTO 不含 Base URL、请求正文、响应文本、临时路径、argv 或环境变量，
避免把外部不可信内容变成 UI/日志契约。

三个 command：

- `test_provider_api(providerId, requestId)`
- `test_provider_codex_compatibility(providerId, requestId)`
- `cancel_provider_test(requestId)`

请求 ID 由前端 `crypto.randomUUID()` 生成，后端重新解析为 UUID。`ActiveTestRegistry` 只允许一个
活动测试，注册取消信号后立即释放锁；完成、失败和 panic-safe 清理路径都移除注册项。

## 测试目标与密钥生命周期

`ProviderService::resolve_availability_target` 复用现有一致读取与 Provider 校验，返回仅 Rust 内部
可见的 `AvailabilityTarget`：Provider ID、规范化 Base URL、当前偏好模型和 API Key。该结构
自定义 `Debug`，只输出 `api_key_configured=true/false`。

- API 路径将密钥直接放入一次性 `Authorization: Bearer` Header；请求结束即释放。
- Codex 子进程始终只收到每次运行生成的回环假密钥；真实 API Key 仅由 Rust 转发层在上游
  `Authorization` Header 中短暂使用，不进入子进程环境、argv 或临时文件。
- 不把目标或密钥保存在 `AppState`、活动测试注册表、普通前端状态或错误对象中。
- 内部日志只记录测试 kind、Provider ID、稳定 code 和耗时；不记录 URL、Header、正文、argv、
  stderr 或 JSONL。

## API 测试

### 请求

直接依赖当前锁文件中的 `reqwest 0.13`，使用 Rustls/platform verifier 和 JSON 支持。每次请求
按当前设置构造客户端：

- `redirect::Policy::none()`；
- `no_proxy()` 后，仅在 Relay 设置显式启用时添加该无认证 HTTP(S) proxy；
- connect timeout 5 秒、总 timeout 30 秒；
- 无自动重试；
- 固定 User-Agent，只包含 Relay 版本；
- 最多读取 256 KiB 响应体。

Base URL 作为 API 根处理：保留已有路径，确保末尾只有一个 `/` 后追加 `responses`。若 Base URL
本身已以 `/responses` 结束则直接使用，避免双重追加。URL query/fragment 不进入日志；fragment
在校验阶段拒绝。

请求体固定且不含 tools：

```json
{
  "model": "<selected model>",
  "input": "Reply with exactly OK.",
  "max_output_tokens": 16,
  "stream": false
}
```

### 响应与分类

状态码先分类，正文只在大小上限内解析，随后立即丢弃：

| 条件 | 结果 code |
|---|---|
| 2xx + 合法完成 Responses JSON | `API_TEST_PASSED` |
| DNS/连接/TLS | `API_NETWORK_FAILED` / `API_TLS_FAILED` |
| 总超时 | `API_TIMEOUT` |
| 401/403 | `API_AUTH_FAILED` |
| 404 | `API_ENDPOINT_OR_MODEL_NOT_FOUND` |
| 429 | `API_RATE_LIMITED` |
| 5xx | `API_PROVIDER_ERROR` |
| 其他非 2xx | `API_HTTP_FAILED` |
| 超过 256 KiB | `API_RESPONSE_TOO_LARGE` |
| 非 JSON/非 Responses 完成结构 | `API_RESPONSE_INVALID` |

不反射 Provider 的 `error.message`。取消通过 `tokio::select!` 中止请求 future，返回
`PROVIDER_TEST_CANCELLED`。

## Codex 兼容性测试

### 启动前门禁

在产生真实 Provider 流量前依次完成：

1. 解析 `codex --version`，3 秒超时，只接受精确允许列表中的 `0.144.4`。
2. 检查 Windows `%ProgramData%\OpenAI\Codex\requirements.toml`；初版只要存在即返回
   `CODEX_MANAGED_CONFIG_UNSUPPORTED`，不尝试解释或绕过管理员策略。
3. 创建并规范化系统临时根，确认它是系统 temp 的后代且不是真实 Codex/Relay 目录。
4. 写入不含密钥的最小纯文本 model catalog；再次解析确认 schema 是本版本内置模板。
5. 用假密钥和回环 Provider 运行同一套 CLI 配置，捕获首个请求并核对工具集合、目标 Provider、
   模型、Authorization 形态、请求大小和一次正常结束。
6. 真实阶段仍让 Codex 连接本机回环转发层；转发层复核相同请求契约后才向目标 Provider 发起
   一次上游流式请求，并在任何 function/custom tool call SSE 到达 Codex 前阻断。

实施中核对 `codex-cli 0.144.4` 官方源码发现，`codex exec --json` 不会序列化全部内部工具类型
（例如 `ImageView`/部分 dynamic tool call）。因此 JSONL 只作为第二道判定门禁，不能单独证明
“无工具调用”；受监控转发层是完整工具调用可见性的必要安全边界。

`codex doctor` 不参与门禁：它会主动执行网络可达性检查并检查安装来源，不满足“无额外网络、
最小状态访问”的测试契约。

### CLI 配置

真实与回环运行共用唯一参数构造器，核心参数包括：

- `exec --json --strict-config --ignore-user-config --ignore-rules --ephemeral`
- `--skip-git-repo-check -C <empty-work-dir> --sandbox read-only --model <model>`
- `approval_policy="never"`、`sandbox_mode="read-only"`
- 临时 `model_provider` 与 `model_providers.<id>.{name,base_url,wire_api,env_key}`
- `model_catalog_json=<temp catalog>`、`project_root_markers=[]`
- `tools.experimental_request_user_input.enabled=false`
- `features.shell_tool/unified_exec/apps/browser_use/browser_use_external/
  browser_use_full_cdp_access/computer_use/image_generation/in_app_browser/plugins/
  remote_plugin/plugin_sharing/tool_suggest/code_mode_host/auth_elicitation/
  tool_call_mcp_elicitation/skill_mcp_dependency_install/hooks=false`
- `web_search="disabled"`、`mcp_servers={}`

纯文本 catalog 把目标模型限制为 `input_modalities=["text"]`、`shell_type="disabled"`、
`apply_patch_tool_type=null`。回环预检要求首个请求工具名集合精确为 `update_plan`、`view_image`；
任何新增、缺失或重复都视为契约漂移。

环境先 `env_clear()`，再加入 Windows 运行必需变量、`TEMP/TMP`、允许的 HTTP(S)/NO_PROXY/CA
变量、临时 `CODEX_HOME`、临时 `CODEX_SQLITE_HOME` 和唯一回环假密钥变量。先从 PATH 解析 Codex
绝对可执行路径，子进程不继承其他任意环境变量。

### 进程、输出与成功判定

Codex 子进程创建后立即加入设置了 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 的 Windows Job Object；
无法建立或加入 Job 时立即终止并在联系真实 Provider 前失败。取消、超时、输出超限和解析失败
关闭 Job 并等待整棵进程树退出，`taskkill /T /F` 只作为 Job 终止失败后的保底诊断路径，不能把
保底成功当作 Job 门禁通过。

stdout/stderr 并发读取，分别限制 1 MiB。stdout 逐行从 `unknown` 解码为中央 JSONL 事件枚举，
stderr 只扫描已知安全标记；两者均不落盘。未知事件、非 JSON 行、截断或远端正文回显都返回
安全错误，不把原文带到 UI/日志。

兼容性通过必须同时满足：

- 回环预检通过；
- 真实转发层只看到一个契约匹配的 Codex 请求，且上游 SSE 中没有任何工具调用事件；
- 真实运行退出码 0；
- JSONL 中存在正常完成事件；
- 没有 function/tool call、Hook、MCP、web search、plugin 或权限请求事件；
- stderr 没有配置回退、安全警告或密钥迹象；
- Job 中进程全部退出；
- 临时根递归清理并验证不存在。

任一工具调用返回 `CODEX_TOOL_CALL_BLOCKED`。CLI 缺失、版本不支持或安全门禁失败使用
`unsupported`；远端失败、协议异常和清理问题使用 `failed`；用户取消使用 `cancelled`。

## 前端组件与状态

遵循 Vue 3 Composition API、`<script setup lang="ts">` 和 props-down/events-up：

- `ProviderAvailabilityPanel.vue`：只负责两类测试说明、按钮、状态 Tag、耗时和安全消息；接收
  Provider、两类 result、running kind、disabled，发出 `test-api`、`request-codex-test`、
  `cancel`。不调用 Tauri。
- `useProviderAvailability.ts`：拥有只读结果 Map、当前 request ID/kind/provider、busy 和显式
  `testApi`、`testCodex`、`cancel`、`invalidateAll`。只通过 typed service 访问后端。
- `ProvidersView.vue`：组合 `useProviders` 与新 composable，派生统一交互禁用状态，并用现有
  `ConfirmDialog` 承担高级测试确认；不解析结果 code。
- `src/services/tauri.ts`：唯一 command 字符串、参数 camelCase 和 `CommandResult` 解包位置。

Provider 文件指纹变化、Provider mutation 成功或 `providers-changed` 事件到达时调用
`invalidateAll`。测试晚返回时用 request ID/序列号丢弃已取消或已失效结果，避免旧配置结果覆盖
新状态。结果只驻留内存，不写 localStorage。

布局在详情字段与 CRUD actions 之间插入面板；默认 API 按钮使用 primary plain，高级入口使用
普通 plain 并带“高级”文字。状态不只靠颜色，按钮均有明确可见文本和 aria-label。约 760px
窄窗口下测试行改为单列，不产生页面横向滚动。

## 兼容性、产品契约与回滚

- 更新产品契约：仍禁止启动/定时自动访问模型网络，但允许用户显式执行 Provider API/Codex
  测试；Relay 代理只用于 API 测试，不改变普通 Codex CLI 请求。
- 更新 AboutView/README，说明两类测试语义、API 最小请求与 Codex 正常回合的 token 差异、
  结果不持久化和不修改配置。
- 不迁移任何磁盘数据、不新增受管文件。回滚产品代码即可完全移除功能；用户配置、密钥和备份
  无需恢复。
- `reqwest`、`tempfile` 和 Windows Job Object 依赖是主要构建风险。若 Job Object、输出上限或
  路径验证无法可靠实现，保留 API 测试并不交付 Codex 高级入口，不能降级安全条件。
- 自动化测试不运行用户已安装 Codex。真实端到端人工验证只能使用位于受保护目录之外的安全
  CLI 发行版和假 Provider；否则如实记录“未执行”。

## 官方与实验依据

- 官方 Codex 环境变量与 `CODEX_HOME`：<https://learn.chatgpt.com/docs/config-file/environment-variables>
- 官方单次 `-c` 覆盖、Provider `env_key` 与 hooks：
  <https://learn.chatgpt.com/docs/config-file/config-advanced>
- 官方 Hook 与 managed hooks 限制：<https://learn.chatgpt.com/docs/hooks>
- `--ignore-user-config`、`--ignore-rules`、`--ephemeral`、`--strict-config`、JSONL 工具面和
  Windows 清理结论来自任务 `research/findings.md` 中针对 `codex-cli 0.144.4` 的本地 Mock
  实验，不外推到其他版本。
