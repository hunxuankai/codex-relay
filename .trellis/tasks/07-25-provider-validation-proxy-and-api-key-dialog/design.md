# 技术设计：Provider 验证代理选项与密钥弹窗保存收尾

## 边界与组件职责

| 层 | 所有者 | 责任 |
|---|---|---|
| 应用根 | `App.vue` | 保持唯一 `useSettings` 状态，并把 `networkProxy.enabled` 作为只读 prop 传给 Provider 页面。 |
| 页面 | `ProvidersView.vue` | 编排 Provider 测试、保存 API Key、Codex 确认对话框和安全成功提示。 |
| 面板 | `ProviderAvailabilityPanel.vue` | 持有短生命周期的“不使用代理”复选框状态，显示代理前置条件，并向上发出显式 `useProxy` 载荷。 |
| composable | `useProviderAvailability` | 以显式参数启动 API/Codex 测试，继续维护单活动会话、取消和结果隔离。 |
| typed IPC | `src/services/tauri.ts` | 使用 camelCase `useProxy` 调用两个固定 command。 |
| command | `provider_availability_commands.rs` | 仅转换参数并单次委托服务。 |
| core service | `ProviderAvailabilityService` | 在只读设置边界解析是否允许使用配置代理，并把同一解析结果传给 API HTTP 探针或 Codex 网关。 |
| 密钥管理 | `useProviderApiKeyManager` + `ProvidersView` | `save()` 成功返回后才关闭；关闭动作清空完整密钥。 |

## 代理数据流

```text
App.useSettings().settings.networkProxy.enabled
  -> ProvidersView.networkProxyEnabled
  -> ProviderAvailabilityPanel.skipProxy (默认 true)
  -> emit testApi/requestCodexTest(useProxy = !skipProxy)
  -> useProviderAvailability.test*(providerId, useProxy)
  -> tauri test_provider_*(providerId, requestId, useProxy)
  -> ProviderAvailabilityService.resolve_test_proxy(useProxy)
  -> provider_http::probe_api(..., proxy) / CodexCompatibilityGateway::start(..., proxy)
```

面板在 `skipProxy=false && networkProxyEnabled=false` 时不发出测试事件，改为显示禁用原因。Codex 的确认状态保存 `{ providerId, useProxy }`，避免确认弹窗打开期间复选框变化造成请求模式漂移。

## IPC 与后端契约

两个 command 增加必需的 `use_proxy: bool` 参数，对应前端 `useProxy`：

```text
test_provider_api(providerId, requestId, useProxy)
test_provider_codex_compatibility(providerId, requestId, useProxy)
```

`ProviderAvailabilityService` 新增单一代理解析辅助逻辑：

- `useProxy=false` 返回 `None`，不加载设置，确保测试客户端继续使用 `.no_proxy()`。
- `useProxy=true` 使用 `SettingsService::load_read_only()` 读取设置；只有 `network_proxy.enabled=true` 且 URL 非空时返回 URL。
- 若代理未启用或地址为空，返回 `PROVIDER_TEST_PROXY_DISABLED` 和安全中文消息。该服务端门禁保护直接 IPC 调用，不能只依赖 UI。
- API 探针与 Codex 网关都接收该 `Option<&str>`；不新增配置写入、环境变量继承或敏感日志。

现有 `SettingsService::save` 已验证启用代理必须拥有无认证 HTTP(S) 地址。只读路径遇到损坏设置仍沿用既有安全 command 错误，不伪造成功或自行修复文件。

## API Key 保存流程

```text
点击保存
  -> apiKeyManager.save()
  -> save_provider_api_keys
  -> 管理查询权威重载
  -> onSaved: providerState.refresh()
  -> outcome 成功：缓存安全消息 -> closeApiKeyManager() -> clear()
  -> outcome 缺失：弹窗保持打开，显示 manager 的安全错误
```

成功提示移到对话框外的 `AppNotification`，因此关闭对话框后仍可看见“API Key 已保存。”，但不携带任何密钥值。不会改变 `useProviderApiKeyManager` 的密钥生命周期，也不会把完整密钥交给普通 Provider 状态。

## 兼容性、错误与回滚

- `useProxy` 是新增的固定 IPC 参数，前后端在同一版本内同步升级；没有磁盘格式迁移。
- 禁用代理的 UI 门禁防止普通操作发出请求；服务端稳定错误防止过期前端或直接 IPC 绕过。
- `useProxy=false` 的默认值改变为明确直连，符合本任务目标；用户可取消勾选恢复对已启用 Relay 代理的使用。
- 代码回滚仅需还原本任务文件；没有新配置字段、写入事务、备份格式或用户数据迁移需要回滚。

## 测试策略

- Vue 面板：默认勾选、代理未启用的禁用提示、事件载荷和可访问标签。
- 页面：App 传入代理开关、API/Codex 载荷及确认后的模式保留；成功保存关闭和失败不关闭。
- composable / typed service：`useProxy` 透传与精确 IPC args。
- Rust command：新参数仍验证 UUID 并单次调用服务。
- Rust core：临时路径上的禁用代理拒绝、无代理不读写设置、API/Codex 使用同一代理选择，以及既有路径安全和密钥脱敏测试不回归。
