# 技术设计：应用内网络代理

## 1. 总体方案

采用 Tauri updater 已有的 `check({ proxy, timeout })` 能力，不替换官方更新检查、签名校验、下载或安装流程。代理配置作为应用设置持久化；前端在每次显式更新检查、手动代理测试或本机候选检测时，把对应代理传给 typed updater service。

当前只有 updater 发起网络请求，因此本次建立统一的“有效代理地址”读取与参数传递约定，而不增加空泛的通用 HTTP 客户端。未来新增应用内网络请求时，必须从同一设置 DTO 取得有效代理并在其客户端边界显式应用。

## 2. 设置模型与持久化

在 Rust `Settings` 和 TypeScript `Settings` 中新增嵌套结构：

```text
networkProxy.enabled: boolean
networkProxy.url: string
```

默认值为关闭和空字符串。Rust 保存前统一 trim 并校验：

- 仅允许 `http`、`https`；
- 必须有主机；
- 禁止用户名、密码、query 和 fragment；
- 保存规范化后的 URL，避免前后端各自解释。

前端可做即时提示，但 Rust 是最终验证边界。代理不含认证信息，可通过普通设置 DTO 返回。

`settings.json` 写入继续由 `SettingsService` 负责业务语义，但写操作必须接入 `TransactionService` 的共享锁、指纹、备份、临时解析、原子替换、写后读取验证和失败回滚。为避免形成第二套事务实现，新增 settings 单文件事务入口，并让 `AppState` 向 Provider 与 Settings 服务共享同一事务锁。现有窗口、托盘和自启设置保存也沿用该入口。

## 3. 更新与代理测试数据流

### 正常更新

```text
已保存 Settings.networkProxy
→ SettingsView 计算有效代理（关闭时为 undefined）
→ UpdatePanel / useUpdater
→ checkForUpdate(proxy?)
→ updater.check({ proxy })
→ 返回的 Update 会话沿用同一代理下载和安装
```

未启用时不传 `proxy`，保留 updater 当前的默认网络行为。设置保存不会修改已创建的 Update 会话；下一次检查才读取新值。

### 手动测试

`testUpdateProxy(url)` 使用 `check({ proxy: url, timeout: 5000 })`。调用成功即表示代理能够取得并解析固定更新清单；无论是否发现更高版本，都关闭临时 Update 资源并返回成功。异常统一映射为稳定的代理测试错误，不暴露底层 URL 响应或内部错误。

### 本机候选检测

候选列表是代码内固定只读常量。确认弹窗通过后，composable 用 `Promise.all` 并行调用 `testUpdateProxy`，把成功地址聚合为结果列表。单个失败不终止其他候选；组件只接收成功列表和“全部失败”状态，不显示底层逐项错误。

## 4. 前端组件与状态

- `SettingsView.vue`：编排代理设置草稿、保存、测试和发现入口；保存期间禁用相关操作。
- `useSettings.ts`：继续作为设置权威状态，新增“一键保存并启用检测结果”的显式动作，成功后以后端返回状态刷新页面。
- 新增代理检测 composable：管理 `idle/testing/discovering/result/error`，使用请求序列防止旧结果覆盖新操作。
- 复用 `ConfirmDialog.vue` 展示检测前说明。
- 新增结果选择对话框：原生 radio 选择、取消/确认、焦点管理、Esc、Tab 焦点约束；多个地址全部可见，默认选中第一个但不自动提交。
- 手动测试结果使用现有应用内通知；本机未发现代理或发现结果使用对话框，符合用户要求的显式提示。
- “一键设置本机代理”只在 trim 后地址为空时显示。

## 5. 错误与安全

稳定错误至少区分：

- `INVALID_PROXY_URL`：地址格式或协议不符合要求；
- `PROXY_TEST_FAILED`：代理无法访问或解析更新源；
- `PROXY_TEST_TIMEOUT`：5 秒内未完成；若插件不能稳定区分超时，公开层统一使用测试失败消息，内部测试仍覆盖 timeout 参数；
- 现有设置事务错误和回滚不完整错误保持原语义。

不记录 updater 底层响应正文，不把代理测试错误详情放入通知。候选固定为 loopback 白名单，用户不能借发现入口扫描任意地址或端口。手动输入代理仍允许非 loopback HTTP(S) 主机，但必须通过 URL 校验。

## 6. 兼容性与回滚

- serde 默认字段保证旧 `settings.json` 自动得到关闭状态，不需要迁移脚本。
- 新版本写出的 `networkProxy` 对旧版本属于未知字段；旧版本可能丢弃该字段，因此降级后需要重新配置代理，不承诺向后保留。
- 功能回滚只需移除 UI 和 updater 参数传递；保留在设置文件中的未知字段由兼容读取策略处理。
- 不修改 updater endpoint、公钥、签名或 Release 资产格式。

## 7. 测试边界

- Rust 测试使用 `tempfile` / `AppPaths::for_test`，验证默认值、URL 校验、规范化、事务成功与回滚；不读取真实应用数据。
- 前端 service 测试 mock 官方 updater，断言 `proxy`、`timeout`、临时资源关闭和安全错误映射。
- composable 测试 mock typed service，验证六候选并发聚合、旧响应防护、全失败与多结果。
- Vue 测试 mock composable，验证按钮显示条件、确认前无请求、取消、测试反馈、radio 选择和立即保存。
- 不通过真实 GitHub 或真实本机代理完成自动化测试；真实代理连通性仅能作为后续人工观察证据单独报告。

