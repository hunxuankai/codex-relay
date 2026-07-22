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

## Provider 测试会话状态契约

### 1. 范围/触发条件

`useProviderAvailability` 只管理用户显式启动的 API/Codex 测试会话；不把测试结果并入 Provider
DTO、localStorage 或应用数据。

### 2. 签名

composable 暴露只读 `results`、`runningKind`、`busy` 和显式 `testApi`、`testCodex`、
`cancel`、`invalidateAll`；组件通过 typed service 调用三个固定 command。

### 3. 请求/响应/环境契约

每个结果以 `providerId + kind` 为键保存，保留后端 `status/code/message`；请求序列号或 UUID
使取消、指纹变化后的晚响应无法覆盖新状态。Codex 测试只能从确认对话框进入。

### 4. 验证与错误矩阵

- 同一时间已有测试：按钮显示取消态，重复启动被阻止。
- Provider 指纹变化、CRUD 成功或 `providers-changed`：调用 `invalidateAll` 清除旧结果。
- 后端稳定错误码原样映射为安全中文消息；不得在组件中解析原始响应正文。

### 5. 良好/基线/错误用例

- 良好：API 通过不替换 Codex 结果，切换 Provider 仍可查看其他详情但不能编辑/删除/改偏好。
- 基线：缺少密钥、无模型或 busy 时按钮有可见禁用原因和 aria-label。
- 错误：把测试结果写入 `useProviders` 或让列表卡片再展示一份状态。

### 6. 必需测试

Vitest 断言两类结果独立、确认后才发起 Codex、取消态、晚响应丢弃、指纹失效、稳定错误消息、
键盘可达性和 760px 单列布局。

### 7. 错误与正确做法

#### 错误

```ts
provider.lastHealth = await testProviderApi(provider.id)
localStorage.setItem('provider-test', JSON.stringify(provider.lastHealth))
```

#### 正确

```ts
const availability = useProviderAvailability()
await availability.testApi(provider.id)
// 结果仅在 availability 会话内存中，Provider DTO 不变
```
