# Provider 验证代理选项与密钥弹窗保存收尾

## 目标与用户价值

用户在 Provider 详情中验证 API 或 Codex 兼容性时，可以明确选择是否绕过代理，避免无意经过已保存的网络代理；需要使用代理时，界面会先确认设置页的“网络代理”已启用。管理命名 API Key 成功后，弹窗应自动关闭并清除本次查看的明文密钥。

## 已确认事实与约束

- Provider 可用性区域同时提供 API 与 Codex 两类用户显式测试；测试结果只保存在当前前端会话。
- 当前 API 测试会自动采用已启用的 `settings.json.networkProxy`，Codex 测试网关当前固定无代理。两条路径都经过 typed service、Tauri command 和 `ProviderAvailabilityService`。
- 设置在应用根部由 `useSettings` 持有；Provider 页面不能创建第二份长期设置真相。
- 所有 Provider 测试读取配置必须走只读路径，不能创建、备份或改写 `settings.json`、Codex 配置或密钥文件。
- 完整 API Key 只能暂存在 `useProviderApiKeyManager`；关闭对话框会调用 `clear()` 清空数组并使晚响应失效。
- 用户已明确要求实施本任务。

## 需求

### PXY-1 验证代理模式

- 在“验证当前 Provider 配置”区域增加“不使用代理”复选框，初始状态为已勾选。
- 此选项对 API 可用性测试和 Codex 兼容性测试一致生效：勾选时两类测试均绕过代理；取消勾选时两类测试均请求使用已保存的网络代理。
- 取消勾选但设置页“网络代理”未启用时，显示明确的中文原因，并阻止两个测试动作发起请求。

### PXY-2 跨层代理契约

- 前端向两条 Provider 测试 IPC 调用显式传递 `useProxy` 布尔值；默认“不使用代理”对应 `false`。
- 后端以只读方式检查 `useProxy=true` 的网络代理设置。若代理未启用或没有有效地址，必须拒绝该请求并返回稳定、安全的错误，不得静默回退直连。
- API HTTP 探针与 Codex 兼容性网关使用相同的代理选择结果；`useProxy=false` 不读取或使用设置的代理，也不得使用环境代理。
- IPC 参数、结果、错误、日志、通知和测试输出不得包含完整 API Key、Authorization Header 或代理 URL 中的敏感信息。

### KEY-1 密钥保存后的关闭

- 点击“管理与查看 Provider 的 API Key”对话框中的“保存”后，只有保存、权威重载及 Provider 刷新均成功才关闭对话框。
- 保存失败、刷新失败或重复提交时保持对话框打开并保留安全错误信息。
- 成功关闭时必须继续调用既有 `clear()`，使完整密钥不留在前端状态；成功提示必须不包含密钥值。

## 验收标准

- [x] Provider 可用性面板初始显示已勾选的“不使用代理”；两种测试动作都以 `useProxy=false` 发起。
- [x] 取消勾选且网络代理未启用时，面板显示可见原因、两个测试按钮不可用，且不触发 IPC。
- [x] 取消勾选且网络代理已启用时，API 与 Codex 测试均以 `useProxy=true` 发起；Codex 二次确认仍保留本次选择。
- [x] typed service 使用精确的 camelCase `useProxy` 参数；Tauri command 与 Rust service 将该参数传递到 API 探针和 Codex 网关。
- [x] 后端对 `useProxy=true` 的禁用/无地址代理返回稳定安全错误；`useProxy=false` 始终不使用代理，且不会写入设置或真实默认路径。
- [x] API Key 保存成功后对话框关闭、管理器清空完整密钥，并显示不含密钥的成功反馈；失败时对话框不关闭。
- [ ] 新增或调整的前端、Rust 和跨层测试均通过；类型检查、格式检查与本任务要求的质量检查有本轮证据。功能级测试、类型检查、格式检查、隔离 core/path-safety 测试和依赖图均有本轮通过证据；标准 `npm run check` 仍受运行中的 Rust watcher 门禁阻止。

## 范围外

- 不修改设置页网络代理的保存、检测或更新功能。
- 不新增代理协议、认证代理、代理自动发现或环境代理继承。
- 不改变 Provider 配置、密钥存储格式、事务流程或测试结果持久化规则。
- 不改变常规 Codex CLI 的网络环境。

## 开放问题

无。单个选项统一作用于同一可用性区域内的 API 和 Codex 测试，以避免两个测试对同一开关产生不一致语义。
