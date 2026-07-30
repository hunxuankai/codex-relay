# 前端状态管理

## 基本模式

项目不引入 Pinia。`useProviders`、`useHealth`、`useBackups`、`useSettings` 等 composable 持有资源状态，对组件暴露 `readonly` 状态、计算值和显式动作。

```ts
return {
  providers: readonly(providers),
  loading: readonly(loading),
  refresh,
  switchProvider,
}
```

组件不得直接写 composable 内部 ref，也不得复制一份长期失真的 Provider 真相。

## 异步一致性

- 刷新请求使用递增序列号或同等机制；旧请求晚返回时不能覆盖更新的事件结果。
- Provider CRUD、切换、恢复或外部文件变化后，从后端重新加载权威状态。
- 操作状态区分 loading、busy、error 和最近成功消息；切换期间禁用所有 Provider 切换入口。
- 当前 Provider、密钥配置状态和健康结果来自后端 DTO，不由组件猜测。

## Provider 命名地址与密钥状态契约

- `useProviders` 持有普通脱敏 Provider 真相，负责 Base URL 批量保存、URL/密钥独立选择、
  命名导入和 mutation 后权威刷新；不得加入完整密钥字段。
- `useProviderApiKeyManager` 只在管理对话框生命周期内持有完整密钥，暴露只读状态和
  `load`、`replaceEntries`、`save`、`clear` 动作。
- `load`、`save` 使用请求序列；关闭、scope dispose 或新请求使旧响应失效，晚响应不得重新填充密钥。
- API Key 对话框默认明文展示，不设置单独“查看”步骤；统一隐藏/显示只改变显示方式，复制反馈不得包含值。
- URL 与 Key 分段选择分别发出显式事件；当前项名称必须有可见文本，选中状态不能只靠颜色。

完整跨层契约见 [Provider 多命名地址与密钥契约](../project/provider-multi-credentials.md)。

## Provider Fast 状态契约

- `ProviderProfile.fastEnabled` 与 `ModelCatalogItem.supportsFast` 都来自后端权威 DTO；组件不得硬编码
  支持模型 ID，也不得直接修改 Provider 对象。
- `useProviders.updateFast(providerId, enabled)` 使用当前 `FileSetFingerprint` 调用 typed
  `update_provider_fast`，与其他 mutation 共用 busy 防重、稳定错误和成功后权威刷新。
- `ProviderPreferenceControls` 只接收 props 并发出 `update-fast`；不支持模型时展示值必须为 false、
  控件禁用并关联可见原因。`ProvidersView` 只转发事件，不持有第二份 Fast 状态。
- `ProviderEditor` 的草稿可持有 Fast 表单值：create 默认 false，edit 从 Provider 回填；能力依据
  实际提交后偏好模型（保留的 `selectedModel`，否则模型数组第一项）派生。不支持时只单向关闭，
  Fast-only 变化也必须触发当前 Provider 的现有 `syncIfActive` 选项。
- `MODEL_FAST_UNSUPPORTED` 原码和安全消息必须保留；mutation 后刷新失败时不能显示 Fast 成功。

## Provider 列表排序状态契约

- `ProviderList` 只负责拖动/方向键手势并发出完整 Provider ID 排列；不得直接访问 Tauri、
  localStorage 或复制长期 Provider 列表。
- `useProviders.reorder` 可在已加载指纹时同步做短期乐观投影，再通过 typed
  `reorderProviders` mutation 持久化；失败且没有更新事件覆盖时恢复先前数组，并保留安全错误。
- mutation 成功后仍从 `list_providers` 重新加载权威顺序；`providers-changed` 和请求序列规则
  继续阻止晚刷新覆盖更新事件。
- 排序 busy 与其他 Provider mutation 共用同一防重状态；排序不改变 `selectedProviderId`、
  活动 Provider 或 Provider 内部 URL/Key 顺序。

## 事件处理

- `providers-changed`：刷新 Provider 数据和选中状态。
- `config-files-changed`：提示外部变化并触发必要刷新。
- `self-check-completed`：更新健康结果。
- `settings-changed`：只更新设置/自启状态。
- `app-notification`：只展示安全消息，不承载文件或秘密。

订阅必须在组件/应用生命周期结束时解除，避免重复事件处理。

## 失败行为

- 保留后端稳定错误码，向用户显示安全中文消息。
- `EXTERNAL_MODIFICATION_CONFLICT` 要求重新加载，不在前端强制覆盖。
- `ROLLBACK_INCOMPLETE` 必须提供备份恢复引导，不显示通用成功通知。
- `MODEL_FAST_UNSUPPORTED` 显示后端明确原因，不在前端猜测为通用配置错误。

## Provider 测试会话状态契约

### 1. 范围/触发条件

`useProviderAvailability` 只管理用户显式启动的 API/Codex 测试会话；不把测试结果并入 Provider
DTO、localStorage 或应用数据。

### 2. 签名

composable 暴露只读 `results`、`runningKind`、`busy` 和显式 `testApi`、`testCodex`、
`cancel`、`invalidateAll`；测试动作签名为 `testApi(providerId, useProxy)` 和
`testCodex(providerId, useProxy)`，组件通过 typed service 调用三个固定 command。

### 3. 请求/响应/环境契约

每个结果以 `providerId + kind` 为键保存，保留后端 `status/code/message`；请求序列号或 UUID
使取消、指纹变化后的晚响应无法覆盖新状态。Provider 可用性面板默认传入
`useProxy=false`，并且只能在应用根部唯一 `useSettings` 状态中的
`networkProxy.enabled=true` 时传入 `true`；不得在 Provider 页面重新加载或复制设置状态。
Codex 测试只能从确认对话框进入，确认状态必须保留本次 `useProxy` 值，不能在确认期间重新读取
复选框。
API 结果的 `trace` 与结果作为一个对象保存和失效，不建立第二套详情 store；前端不得根据 Provider
配置重新拼装请求。Codex 结果的 `trace` 必须为 `null`。

### 4. 验证与错误矩阵

- 同一时间已有测试：按钮显示取消态，重复启动被阻止。
- Provider 指纹变化、CRUD 成功或 `providers-changed`：调用 `invalidateAll` 清除旧结果。
- 同一类测试重新启动时，`useProviderAvailability` 必须在确认没有其他活动测试后先清除该
  `providerId + kind` 的旧结果，再发起异步 command；另一类测试结果保持不变。活动测试期间的
  重复启动仍直接拒绝，不能先清结果造成用户可见闪烁。
- API 面板的弹窗打开/loading 只是短生命周期局部 UI 状态：点击测试后立即清除旧详情并打开
  弹窗，请求和响应区域在新结果到达前分别显示 loading；`trace` 到达后原位更新。用户关闭只
  改变弹窗状态，不取消测试；运行中或结果卡片上的“查看请求与响应”可再次打开。无 `trace`、
  取消、Provider 切换、指纹失效或晚响应时必须关闭/清除局部状态，不能显示旧请求或伪造请求
  响应。结果生命周期仍由 composable 管理，面板不得建立第二份结果 store。
- 后端稳定错误码原样映射为安全中文消息；除专用只读详情组件按文本展示 `trace` 外，组件不得解析、
  重写或从响应正文推导状态，也不得使用 `v-html`。
- 用户取消“不使用代理”但网络代理未启用：显示可见原因、禁用两类测试按钮，且不发起 IPC；后端
  `PROVIDER_TEST_PROXY_DISABLED` 仍作为过期 UI 或直接 IPC 的防御门禁。

### 5. 良好/基线/错误用例

- 良好：API 通过不替换 Codex 结果，切换 Provider 仍可查看其他详情但不能编辑/删除/改偏好。
- 良好：默认直连的 API/Codex 操作都传递 `useProxy=false`；启用代理后，确认的 Codex 操作仍使用
  点击时保存的 `true`。
- 基线：缺少密钥、无模型或 busy 时按钮有可见禁用原因和 aria-label。
- 错误：把测试结果写入 `useProviders`、让列表卡片再展示一份状态，或由局部组件猜测/复制网络代理设置。

### 6. 必需测试

Vitest 断言两类结果独立、API trace 透传与指纹失效、默认直连和已启用代理的 `useProxy` 透传、
确认后才发起 Codex 且保留模式、代理未启用门禁、取消态、晚响应丢弃、稳定错误消息、键盘可达性
和 760px 单列布局。

### 7. 错误与正确做法

#### 错误

```ts
provider.lastHealth = await testProviderApi(provider.id)
localStorage.setItem('provider-test', JSON.stringify(provider.lastHealth))
```

#### 正确

```ts
const availability = useProviderAvailability()
await availability.testApi(provider.id, false)
// 结果仅在 availability 会话内存中，Provider DTO 不变
```
