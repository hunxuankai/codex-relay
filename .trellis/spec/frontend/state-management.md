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
