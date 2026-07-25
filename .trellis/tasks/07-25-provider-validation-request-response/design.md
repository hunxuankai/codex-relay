# 技术设计：展示 Provider API 验证请求与响应

## 设计摘要

在现有 API 可用性测试结果上增加一个只在当前会话存在的 `trace`。trace 由 Rust 的 API 网络边界在真实请求构造、发送和响应读取过程中生成，随同一次 `ProviderAvailabilityResult` 返回；Vue 只消费该 DTO，不重新推导请求。用户点击结果卡片上的入口后，在独立 `ElDialog` 中查看请求和响应。

本任务只覆盖 `stream=false` 的 API 探针。现有 Codex 兼容性测试的 SSE gateway、工具门禁和进程环境保持不变。

## 边界与职责

| 层 | 所有者 | 责任 |
|---|---|---|
| HTTP 基础设施 | `provider_http.rs` | 生成真实请求 trace；捕获响应状态和有界正文；分类网络/HTTP/协议错误；清除意外回显的真实 API Key。 |
| 可用性服务 | `provider_availability_service.rs` | 将成功或失败的探针结果与 trace、状态、稳定错误码、模型和耗时合并；Codex 结果不生成 API trace。 |
| Rust DTO | `models/provider_availability.rs` | 定义可序列化的嵌套 trace；自定义 Debug 只输出元数据，不输出请求/响应正文。 |
| typed IPC | `src/services/tauri.ts` | 复用现有测试 command；不新增详情查询 command。 |
| composable | `useProviderAvailability.ts` | 继续缓存结果、取消测试和按指纹失效；不建立第二套 trace store。 |
| API 面板 | `ProviderAvailabilityPanel.vue` | 在 API 结果旁显示入口，管理弹窗开关和 Provider 变化时的关闭。 |
| 详情组件 | `ProviderAvailabilityTraceDialog.vue` | 只读展示请求/响应文本、HTTP 状态和耗时，处理关闭与可访问性。 |

## 跨层数据契约

公开 TypeScript DTO 使用 camelCase，语义等价于：

```text
ProviderAvailabilityTrace {
  request: {
    method: string,
    url: string,
    body: string
  },
  response: {
    status: number,
    body: string,
    bodyTruncated: boolean
  } | null
}

ProviderAvailabilityResult.trace: ProviderAvailabilityTrace | null
```

- `trace === null` 表示请求在构造前失败，或没有可公开的请求信息。
- `response === null` 表示请求已构造/发送但没有收到 HTTP 响应。
- `body` 是 UTF-8 文本视图；非 UTF-8 字节使用安全的 lossy 转换，并保持失败状态。
- `bodyTruncated=true` 只表示展示正文被有界截断，不表示请求成功。
- 现有 `httpStatus` 字段继续兼容；HTTP 错误的实际状态同时写入 trace，必要时补齐摘要字段的状态映射。

## Rust trace 生成流程

1. `responses_endpoint` 成功后构造固定 payload（当前模型、固定输入、`max_output_tokens=16`、`stream=false`），序列化为请求 trace body。
2. 使用同一 endpoint 发起实际 `POST`；不记录任何 Header。代理选择仍由上层解析，trace 不包含代理地址。
3. 收到响应后先记录状态，再使用现有 256 KiB 协议上限读取正文；HTTP 非成功也读取有界正文，以便展示错误响应。
4. 将正文转换为文本并标记截断；Responses JSON 校验仍按原规则执行，校验失败仍返回原稳定错误码。
5. 在 trace 进入 DTO 前替换当前 Provider 的真实 API Key；不改变其他非敏感正文。
6. 任何错误路径都携带当前已形成的 trace：请求构造失败前为 `None`，网络/超时/取消可能只有 request，收到 HTTP 后至少有 response status。

为避免 trace 通过 Rust Debug、日志或错误上下文泄漏正文，trace 实现自定义 `Debug`，只显示 method、URL、状态和字节数，不显示 body。序列化给前端仍包含约定的正文。

内部基础设施建议使用显式失败包装：

```text
ApiProbeReport  { http_status, trace }
ApiProbeFailure { error: ApiProbeError, trace: Option<trace> }
```

这样保留现有 `ApiProbeError` 分类和测试，同时允许 service 在失败结果中附加 trace。

## Vue 交互设计

- `ProviderAvailabilityPanel` 仅在 `apiResult.trace` 非空时渲染 `查看请求与响应` 按钮。
- 新组件 `ProviderAvailabilityTraceDialog` 接收 `open`、Provider 名称、trace 和测试耗时，向上发出 `close`。
- `ElDialog` 使用项目既有 Element Plus 主题、响应式宽度和销毁策略；内容区使用 `pre`/滚动容器，文本通过插值渲染，不使用 `v-html`。
- 弹窗打开/关闭不触发服务调用；Escape、关闭按钮和模型值变化都走同一个 close 事件。
- 面板监听 Provider ID/结果变化，切换 Provider、指纹失效或结果清除时关闭弹窗；trace 随结果一起被 composable 清除。
- Codex 结果维持现有摘要，不显示 API trace 入口。

## 错误与安全处理

- Base URL、Provider、模型或密钥在请求构造前失败：保留现有失败摘要，不提供 trace 入口。
- HTTP 4xx/5xx：trace 保留实际状态和有界正文；错误码/中文消息保持稳定。
- 非 JSON、非 Responses、响应过大：trace 尽量展示已读取正文/截断标记，状态仍为 `failed`。
- DNS、TLS、连接失败、超时或取消：没有响应时 `response=null`，不得伪造 HTTP 状态或完成正文。
- trace 不包含 Authorization/Header、代理 URL、临时 Codex 环境、命令行或密钥；不写入磁盘、事件、通知、备份或 localStorage。
- 所有 Rust 测试使用 `tempfile`/`AppPaths::for_test` 和明确的非真实测试 key；不触及真实 `.codex` 或 `CodexRelay` 目录。

## 兼容性与文档同步

- 无磁盘格式、配置、备份或事务迁移；回滚只需还原本任务代码和文档。
- 现有 `stream=false`、超时、重定向、重试、代理选择和 Codex SSE 行为保持不变。
- 更新 Provider 可用性后端规范、前端测试契约、README/About 页面中“不会展示原始响应正文”的过时描述，使其改为“可在详情弹窗查看有界正文，但不展示 Header/密钥/代理信息”。

## 测试策略与 mock 边界

- `provider_http`：回环 TCP 服务 + 真实 reqwest，覆盖请求 payload、成功正文、HTTP 错误正文、无效 JSON、超限和取消；不 mock HTTP 客户端。
- `provider_availability_service`：临时 Provider/设置文件与 fake Codex runtime，只验证 API trace 合并和错误映射；不访问生产路径。
- Rust DTO：序列化 camelCase、自定义 Debug 不含正文/密钥。
- Vitest：mock typed Tauri client/composable；组件测试验证用户可见按钮、弹窗、错误和无响应状态，不验证私有函数调用顺序。
- 跨层测试：以公开 `ProviderAvailabilityResult` JSON 为边界，确认 Rust → IPC → TypeScript 字段完整往返。

## 回滚点

- 行为切片 1/2：只涉及 core API trace 与错误包装，可单独还原。
- 行为切片 3：DTO/前端组件可在不改变磁盘数据的情况下还原。
- 文档/规范更新与代码无迁移耦合；任何失败均可回退到原有“仅摘要”显示。
