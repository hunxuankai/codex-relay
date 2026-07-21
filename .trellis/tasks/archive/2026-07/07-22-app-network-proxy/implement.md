# 实施计划：应用内网络代理

## 当前进度

- [x] 设置模型、URL 校验与事务持久化
- [x] updater 代理参数与 5 秒代理测试
- [x] 固定六候选并发检测状态
- [x] 设置页、确认弹窗与结果选择交互
- [x] 更新面板代理集成与回归
- [x] 前端与 Rust 全量质量检查
- [x] 项目规范同步

## 行为切片与 TDD 顺序

### 1. 设置模型、校验与事务持久化

- 公开接口：Rust `SettingsService::save/update`、Tauri `save_settings`、TypeScript `Settings` DTO。
- 输入/操作：保存关闭代理、合法 HTTP(S) 地址、非法协议、认证 URL、缺失主机、旧版无字段 JSON。
- 预期：默认兼容、合法值规范化持久化、非法值不写入、事务失败可验证回滚。
- 先写失败测试：Rust model/service 临时目录测试及前端 DTO fixture 编译失败点。
- 实现：新增代理模型和验证；扩展 `TransactionService` settings 单文件事务入口及共享锁装配；更新设置保存链路。
- mock 边界：Rust 文件失败使用可替换 `FileOps`；不 mock 纯 URL 校验；不得访问真实用户路径。

### 2. updater 代理参数与测试接口

- 公开接口：`checkForUpdate(proxy?)`、`testUpdateProxy(proxy)`。
- 输入/操作：无代理、合法代理、5 秒测试、updater 返回 `null`、返回 Update、抛出底层错误。
- 预期：正常更新只在启用时传代理；测试始终传 `proxy` 和 `timeout: 5000`；临时 Update 被关闭；错误稳定脱敏。
- 先写失败测试：`src/services/tauri.test.ts` 对官方 updater mock 的公开调用和资源释放断言。
- 实现：扩展 typed updater service，不改变下载进度与安装契约。
- mock 边界：只 mock `@tauri-apps/plugin-updater` 与版本 API，不 mock应用 service 自身。

### 3. 本机候选并发检测状态

- 公开接口：新增代理检测 composable 的 `testProxy`、`confirmDiscovery`、`selectCandidate`、`applyCandidate`、`reset`。
- 输入/操作：确认/取消、六候选部分成功、全部失败、重复点击、旧请求晚返回。
- 预期：取消无请求；六地址精确且并发开始；失败隔离；全部成功项保留；旧结果不覆盖新状态。
- 先写失败测试：composable 单元测试，以可控 Promise 证明并行和请求序列行为。
- 实现：固定白名单常量、状态机和成功聚合。
- mock 边界：mock typed `testUpdateProxy`；不访问真实网络或端口。

### 4. 设置页与对话框交互

- 公开接口：设置页可见控件和新结果选择组件 props/emits。
- 输入/操作：地址为空/非空、启用切换、测试、检测确认取消、多个结果选择、立即保存成功/失败。
- 预期：标签可访问；空地址显示一键入口；确认前无检测；结果可键盘选择；应用结果后以后端状态为准；失败保留原设置。
- 先写失败测试：`SettingsView.test.ts`、新对话框组件测试和必要的 composable 集成测试。
- 实现：Composition API + `<script setup lang="ts">`，复用通知和确认对话框样式/焦点模式。
- mock 边界：组件 mock composable/typed service，不 mock DOM 原生表单行为。

### 5. 更新面板集成与回归

- 公开接口：`UpdatePanel` / `useUpdater` 的检查动作。
- 输入/操作：已保存代理开启、关闭、代理变更后重新检查。
- 预期：下一次检查使用最新有效代理；同一 Update 会话下载时保持检查时代理；原有进度、确认和错误行为不变。
- 先写失败测试：扩展 `useUpdater.test.ts` 与 `UpdatePanel.test.ts` 的代理传递及回归断言。
- 实现：从 SettingsView 向更新面板传递已保存有效代理，或通过明确 typed 边界读取；不让组件解析远端 URL。

## 计划修改文件

- Rust：`src-tauri/src/models/settings.rs`、`services/settings_service.rs`、`services/transaction_service.rs`、`app_state.rs`、相关 command/测试。
- TypeScript：`src/types/settings.ts`、`src/services/tauri.ts`、`src/composables/useSettings.ts`、`src/composables/useUpdater.ts`、新增代理检测 composable。
- Vue：`src/views/SettingsView.vue`、`src/components/UpdatePanel.vue`、新增代理结果选择对话框及测试。
- 规范/结构测试：按实现发现更新 release updater、项目架构、安全事务和测试契约；不修改发布 endpoint 或密钥。

## 验证命令

按行为切片先运行对应专项测试，再执行：

```powershell
npm run typecheck
npm run test
npm run check:frontend
npm run check:rust
npm run check
git diff --check
```

完成声明前检查 `git status --short --ignored`、高置信度秘密模式和真实用户路径哨兵；真实代理与 GitHub 连通性未人工执行时必须明确标记为未验证。

## 风险与回滚点

- `TransactionService` 共享与 settings 事务扩展是最高风险点；先以失败注入测试锁定回滚，再接 UI。
- updater 临时检查返回 Update 资源时必须关闭，防止资源泄漏。
- 六候选并发不能因一个失败拒绝整个聚合。
- 如果 settings 事务改造影响现有自启/窗口保存，先回滚该切片，不继续叠加代理 UI。
- 不修改真实 `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay`，不运行需要真实安装或更新的自动化流程。
