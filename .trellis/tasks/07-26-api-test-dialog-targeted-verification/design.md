# 技术设计：API 测试结果生命周期与分层验证规则

## 设计摘要

保留现有 `ProviderAvailabilityResult` 和 trace DTO，不新增 IPC 命令或持久化状态。统一在
`useProviderAvailability.test` 的测试开始边界清除当前 `providerId + kind` 的旧结果；面板在
用户点击 API 按钮时关闭旧详情并登记一次待自动打开标记，只有新的带 trace 结果到达时才打开
`ProviderAvailabilityTraceDialog`。这样旧请求/响应不会被短暂复用，Codex 结果和既有失效隔离
仍由 composable 负责。

## 方案比较

| 方案 | 做法 | 权衡 |
|---|---|---|
| A：由 `ProvidersView` 传入受控弹窗状态 | 视图新增 open/request token，面板只展示 | 会把一次性 UI 状态提升到组合层，增加 props/emits 和视图测试耦合。 |
| B：composable 清结果 + 面板待打开标记（推荐） | 结果生命周期留在 composable；面板只管理本地弹窗意图，trace 到达后消费标记 | 改动最小，职责清晰；需要精确处理清空、无 trace 和 Provider 切换 watcher。 |
| C：新增“测试中”结果 DTO/占位 trace | 后端或 composable 返回 loading 结果让弹窗立即显示 | 扩大跨层契约，容易把不存在的请求/响应误呈现为真实数据，不符合 trace 安全边界。 |

选择 B，因为它满足“先清旧结果、再展示新结果”，不改后端协议，也不会伪造尚未形成的请求。

## 数据流与状态边界

1. 用户点击 API 按钮。
2. `ProviderAvailabilityPanel` 将 `traceDialogPending` 设为 `true`、关闭现有详情，然后
   通过既有 `testApi(useProxy)` emit；`ProvidersView` 仍只转发到 availability composable。
3. `useProviderAvailability.test` 在确认没有其他活动测试后，先删除当前测试类型的结果，
   再设置 running/token/generation 并调用 typed client。
4. 新结果通过既有 token/generation/provider/kind 校验后写入结果表。
5. 面板观察到新的 `apiResult`：有 trace 时打开详情并消费 pending 标记；无 trace 时消费标记
   且不渲染详情。命令级异常或取消结束时也清理 pending。
6. Provider ID 变化、结果失效或组件卸载时关闭详情并清除 pending；弹窗自身只发出 close，
   不调用测试服务。

不修改：Codex 结果、trace 结构、Rust 网络边界、代理解析、密钥/路径处理、结果持久化。

## 组件边界

| 单元 | 单一职责 | 公共契约 |
|---|---|---|
| `useProviderAvailability` | 管理测试活动、结果替换和异步隔离 | 现有 `testApi`/`testCodex`/`resultFor` 等 API 不变；按 kind 清旧结果。 |
| `ProviderAvailabilityPanel` | 展示按钮/结果并编排详情弹窗的短期 UI 状态 | 现有 props/emits 不变；API 点击消费一次 pending trace。 |
| `ProviderAvailabilityTraceDialog` | 只读展示已形成的 trace | 现有 `open/providerName/trace/durationMs` 和 `close` 事件不变。 |
| `ProvidersView` | 组合 Provider 与 availability 状态 | 不新增网络或结果解析逻辑。 |

## 错误与边界处理

- 已有测试运行时 `test` 立即返回，不清掉正在显示的结果，也不登记新的 pending。
- 请求构造前失败返回无 trace：结果（若有）仍按既有安全摘要展示，详情不打开。
- 取消、指纹变化或晚响应：沿用 token/generation 保护；pending 不得被无关旧结果触发。
- 新结果没有 trace 时，pending 必须消费，避免下次无关结果意外打开弹窗。
- 只清除当前测试类型，保证 API/Codex 结果独立。

## 测试设计

- composable：用未完成 Promise 先存入旧 API 结果，再启动新测试，断言 Promise 未完成前旧 API
  结果已为空、Codex 结果保留，随后新结果仍能保存；覆盖已有活动测试不误清除。
- 面板：以旧 trace 挂载，点击 API 按钮后模拟父层清空结果，再注入带新 trace 的结果，断言
  详情自动打开且只出现新 URL/body；无 trace、Provider 变化和手动关闭路径不打开/不残留。
- 视图：保留现有事件透传断言，确认不新增 Tauri 调用或改变代理参数。
- 规范：测试验证规则以文档结构/关键命令引用检查，不虚构运行时单元测试。

## 兼容性、回滚与安全

- 只改前端内存状态和测试规范，无磁盘格式/配置迁移；可按文件独立回滚。
- trace 仍由 Rust 实际网络请求生成，前端不重建 URL、body 或 Header；不引入密钥。
- 既有手动查看入口、Escape/关闭焦点行为和 760px 布局保持不变。
