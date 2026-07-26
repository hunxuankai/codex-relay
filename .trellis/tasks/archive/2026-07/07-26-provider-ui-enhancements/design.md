# 技术设计：版本标题、Provider 排序与 API 详情弹窗

## 设计摘要

交付拆为三个行为切片：

1. `App.vue` 复用已有版本状态，在品牌标题旁显示 `v<version>`。
2. Provider 排序使用原生 HTML5 拖放作为列表交互，Vue 通过 typed service/composable 发出
   完整 ID 排列；Rust 将排列作为 `provider-preferences.json` 的 Relay 私有字段，使用现有
   `TransactionService` 写入并在列表投影时排序。
3. API 详情弹窗从“仅有 trace 才挂载”改为“点击即打开、trace 可为空”，面板持有短生命周期
   的打开状态，弹窗按 loading/trace 分区渲染；Element Plus 的遮罩和 Escape 关闭事件统一
   映射为 `close`。

## 跨层数据流

### Provider 顺序

```text
拖动手柄
  → ProviderList emits reorder(providerIds)
  → ProvidersView calls useProviders.reorder
  → typed tauri.reorderProviders({ input })
  → Tauri command reorder_providers
  → ProviderService::reorder_providers
  → TransactionService (preferences only)
  → provider-preferences.json.providerOrder
  → list_providers sorts config entries by providerOrder + config fallback
  → providers-changed / refresh → ProviderList
```

`providerOrder` 是稳定 Provider ID 数组，不包含密钥或 URL。排序输入必须是当前配置 Provider
ID 的精确排列；服务层验证 ID 规范化、无重复、无缺失和无未知项。旧文件缺字段按空数组读取，
列表先使用原 config 顺序，成功排序或其他已有偏好事务后才保存该字段。

### API 详情弹窗

```text
点击测试
  → panel.open=true, emit testApi
  → availability.testApi 清除旧 api result、设置 runningKind
  → TraceDialog(open=true, loading=true, trace=null)
  → command 返回 ProviderAvailabilityResult
  → availability.storeResult
  → panel props 更新
  → TraceDialog loading=false，展示本次 trace 或安全的无响应说明
```

弹窗不访问 command，也不重建请求。`trace` 为空时只显示“正在…”（测试进行中）或“未收到
请求/响应数据”（测试完成且无 trace），不伪造 URL、正文或 HTTP 状态。用户关闭后 `open=false`
保持到显式再次点击入口；Provider 变化/结果失效时由面板关闭并清理。

## 存储与兼容性

- `ProviderPreferenceStore` 保持版本 2，新增可选 `providerOrder` 字段以兼容旧文件；缺失
  字段反序列化为空数组，未知字段继续忽略。
- `normalize_store` 校验 `providerOrder` 中的 ID 合法且不重复，但允许暂时不在 preferences
  map 中的 Provider ID，以便与外部 `config.toml` 的 Provider 生命周期解耦。
- 创建 Provider 时将规范化后的当前配置顺序和新 ID 写入；删除时移除 ID；其他既有事务保留
  排序字段；排序命令只写 preferences 文件。
- 列表排序采用“先按 providerOrder 中出现的 ID，再按 config.toml 原顺序追加未记录 ID”，
  因而外部新增 Provider 不会被隐藏。
- 事务 operation 新增 `reorder_providers`，备份 metadata 仍使用既有统一格式；无新受管文件、
  无新密钥边界、无 config.toml 重排。

## 组件边界与契约

| 单元 | 职责 | 新/变更契约 |
|---|---|---|
| `App.vue` | 应用壳和版本展示 | `appVersion` 仍为只读状态，标题安全回退 |
| `ProviderList` | 列表渲染与拖放手势 | `reorder: [providerIds: string[]]`，不直接写状态 |
| `ProvidersView` | Provider 与排序动作编排 | 调用 `providerState.reorder`，合并 busy/error 展示 |
| `useProviders` | Provider 权威刷新与排序 mutation | 暴露只读列表和 `reorder(providerIds)` |
| `ProviderAvailabilityPanel` | 测试按钮与弹窗局部状态 | 既有 props/emits 保持；API 点击立即打开 |
| `ProviderAvailabilityTraceDialog` | 只读 trace/loading 展示 | `trace` 改为可空，新增 `loading`；`closeOnClickModal=true` |
| `src/services/tauri.ts` | typed IPC | `reorderProviders(input)` |
| Rust model/service/command | 校验、事务写入、排序投影 | `ReorderProvidersInput`、`reorder_providers` |

## 失败与并发处理

- `useProviders` 复用现有 `mutate`：busy 时直接拒绝，排序 command 成功后重新 list；刷新失败不
  展示成功文案。
- 外部文件指纹过期、非法排列或事务失败时，服务返回稳定错误，UI 使用后端刷新或保留原本地
  列表，不乐观地宣称持久化成功。
- 排序过程中 Provider 文件监控事件到达时，现有 state sequence/订阅规则决定最终权威状态；
  晚刷新不能覆盖更新事件。
- API 测试保持现有 operation token/generation；弹窗关闭不调用 cancel，取消仍由原按钮负责。

## 安全与回滚

- 排序只写 `provider-preferences.json`，继续经过锁、指纹、备份、临时文件解析、原子替换、
  写后验证和回滚；不访问真实用户目录的测试约束不变。
- `providerOrder` 只含 ID，不含 API Key、Authorization、trace 或完整配置正文。
- API trace 仍由后端产生并脱敏；前端只做文本插值，不使用 `v-html`。
- 可按三个行为切片独立回退；若排序存储发现兼容问题，可移除字段/command，旧 config 顺序
  仍可作为 fallback。

## 测试设计

1. App：版本成功/失败的标题文本及关于页回归。
2. ProviderList：拖动手柄可用、drop 发出精确排列、busy 禁用、按钮行为不被拖动干扰。
3. useProviders/typed service：排序参数、busy 防重、mutation 后刷新和安全错误。
4. Rust preference service：旧字段兼容、providerOrder 校验/序列化；ProviderService 临时目录
   测试创建/删除/重排/无效排列/指纹冲突/回滚和列表顺序。
5. API panel/dialog：点击即打开、请求/响应 loading、trace 更新、遮罩/Escape/关闭和再次打开、
   无 trace/取消/Provider 变化不泄漏旧数据。
