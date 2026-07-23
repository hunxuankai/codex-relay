# 实施计划

## 工作流与范围

本任务是一个紧耦合的跨层交付物：两个私有存储版本、Provider 聚合投影、四文件事务、
Tauri DTO 与同一详情页交互必须共同满足验收，暂不拆父/子任务。Codex 使用 inline 模式，
每个行为切片在主会话按红色测试 → 最小实现 → 重构执行。

开始编码前加载 `trellis-before-dev`，按索引重新读取 backend、frontend、security、testing、
project 详细规范；再次核对当前安装版 Element Plus `Dialog`、`Segmented`、`Input`、`Button`
官方文档与 `.d.ts`。

## 实施进度

- [x] 1.1 命名 API Key 集合纯校验：稳定 ID、名称/值唯一、顺序和选中项。
- [x] 1.2 `providers.json` v1/v2 版本化读取、规范化与脱敏序列化。
- [x] 2.1 命名 Base URL 集合纯校验与共享 URL 规范化。
- [x] 2.2 `provider-preferences.json` v1/v2 版本化读取、规范化与稳定序列化。
- [x] 3. Provider 统一读取投影与 managed/external/missing 状态。
- [x] 4. 创建输入初始名称、常规编辑收窄与删除双私有记录。
- [x] 5. Base URL 批量管理、删除保护、独立选择与 Tauri 命令。
- [x] 6. API Key 查看器、批量管理、删除保护、独立选择与脱敏 Tauri 命令。
- [x] 7. Provider 切换门禁、命名导入与可用性测试目标解析。
- [x] 8. 事务解析、备份 operation、自检与既有四文件监控契约补齐。
- [x] 9. TypeScript DTO、typed service 与 composable。
- [x] 10. 创建表单与详情快速切换。
- [x] 11. 两个管理对话框与 Onboarding。
- [x] 12. 文档、规范与全范围检查。

## 当前检查点（2026-07-23，完成前）

- 当前进度：后端步骤 1–8、前端契约步骤 9、界面步骤 10–11、文档/规范与全范围检查步骤 12
  已完成；下一步为 Phase 3.4 提交。
- 关键决策：完整 API Key 只保存在 `useProviderApiKeyManager` 的短生命周期状态；普通
  `useProviders` 只处理脱敏 Provider、Base URL 管理、URL/Key 选择和命名导入。普通读取使用
  不创建缺失私有文件的版本化加载；外部 Provider 的常规编辑只保存模型偏好，不自动把实际地址
  纳管为“默认地址”。非密钥 mutation 宽松读取当前认证，仅在可读时用其同步内存中的密钥预选。
- 验证证据：本次恢复后已运行
  `npm run test:rust:lib -- provider_service::tests::list_does_not_create_missing_private_stores`
  （1/1）、`npm run test:rust:lib -- provider_service::tests::`（37/37）、
  `npm run check`（Trellis 8/8、前端 36 个文件/156 项、Rust workspace 160 项、路径安全 3 项、
  Provider workflow 1 项）、`npm run build:frontend`（1733 个模块）和 `git diff --check`。
  fixture 格式、Git 跟踪文件与高置信度密钥模式审计均为 0。
- 规范更新：新增项目级 `provider-multi-credentials.md` 七段式契约，并补充缺失私有文件只读和
  外部 Provider 常规编辑不自动纳管地址的回归规则。
- 下一步：在最终聚合门禁通过后，提交本任务全部代码、测试、文档、规范和任务材料。
- 尚未解决：未进行真实桌面窗口的人工交互观察或安装/签名验证；本任务未修改这些流程，且不能
  由自动化测试替代。

## 有序 TDD 行为切片

### 1. 命名条目纯逻辑与 `providers.json` v2

- 先写失败测试：稳定 ID、名称 trim/大小写唯一、密钥值唯一、空密钥、顺序、选中 ID、
  最后一项/当前项删除规则、v1 → v2 规范化、未知版本、损坏文件和脱敏 `Debug`。
- 实现 `NamedApiKey`、v2 `ProviderSecret`、版本感知解析结果与稳定序列化。
- 公开被测边界：`ProviderSecretService` 的 parse/validate/serialize/read-only load。
- 不 mock 纯逻辑；文件行为只使用 `tempfile`。

### 2. 命名 URL 与 `provider-preferences.json` v2

- 先写失败测试：有序命名 URL、URL 规范化/唯一性、可选模型偏好、v1 模型记录迁移、
  外部无记录 Provider 不被误认作旧受管 Provider、未知版本和损坏保护。
- 将现有模型偏好收进 `modelPreference` 子结构，保持目录和逐模型强度规则不变。
- 公开被测边界：`ProviderPreferenceService` 的 parse/validate/serialize/normalize。
- 不 mock URL/模型校验；使用内存 JSON 和 `tempfile`。

### 3. Provider 统一读取投影

- 先写服务测试：
  - v1 单值显示“默认地址/默认密钥”且只读不写；
  - v2 值匹配得到 managed ID；
  - 未知 config URL、未知 auth key、缺失 key 分别得到 external/missing；
  - 普通 DTO 序列化不含任何密钥；
  - 当前 auth 匹配已保存但非预选密钥时映射到实际名称。
- 重构 `DiskState`，携带规范存储、升级标记和外部选择状态。
- 公开被测边界：`ProviderService::list_providers`。
- 不 mock 存储服务；在 `AppPaths::for_test` 临时目录写四个 fixture 文件。

### 4. 创建、常规编辑与删除

- 先写失败测试：创建输入包含初始 URL/密钥名称；立即应用四文件一致；编辑常规字段不再
  清空/替换 URL 或密钥；删除目标 Provider 同时移除两个私有记录。
- 调整 `CreateProviderInput` / `UpdateProviderInput`、ConfigService 更新边界和写后验证。
- 公开被测边界：`ProviderService::{create_provider, update_provider, delete_provider}`。
- 使用现有 `FileOps` 故障注入验证任一文件失败后完整回滚。

### 5. Base URL 批量管理与独立选择

- 先写失败测试：批量新增/重命名/替换/删除、稳定顺序、重复名称/值、删除当前/最后一项、
  外部 URL 显式命名、指纹冲突、当前与非当前消息。
- 实现 `save_provider_base_urls` 与 `select_provider_base_url`；当前 URL ID 从 config 实际值
  推导，不持久化第二个 selected ID。
- 修改当前选中 URL 的值或点击选择时写 `config.toml`；auth/providers 保持不变。
- 公开被测边界：ProviderService 新方法及对应 command inner。
- mock 只限故障注入 FileOps；不 mock Config/Preference 业务校验。

### 6. API Key 查看器、批量管理与独立选择

- 先写失败测试：显式管理查询返回目标 Provider 全部密钥、普通列表不返回；批量新增、
  重命名、替换、删除非当前项、禁止删除当前/最后一项、重复名称/值、外部 auth 显式纳管、
  Rust Debug/错误/事件脱敏。
- 实现 `get_provider_api_keys_for_management`、`save_provider_api_keys`、
  `select_provider_api_key`；替代旧单密钥读取和 `ApiKeyChange::Clear`。
- 当前 Provider 的选中密钥或选中密钥值变化时同事务写 `auth.json`；非当前只写预选。
- 公开被测边界：ProviderService、Tauri command inner 与 JSON 序列化。
- 不在测试失败输出中打印 DTO；fixture 仅用 `test-key-*-not-real`。

### 7. Provider 切换、导入与可用性测试

- 先写失败测试：切换使用预选 URL/密钥/模型/强度；external/missing 状态拒绝切换；
  现有 auth 导入必须命名；API/Codex 测试解析目标当前选择且不改文件。
- 更新 `resolve_availability_target`、Onboarding 导入服务和切换写后验证。
- 公开被测边界：`switch_provider`、命名导入 command、ProviderAvailabilityService。
- 网络行为继续使用现有本地脚本服务器/网关测试，不访问真实 Provider。

### 8. 事务、备份、自检与监控契约

- 先写失败测试：TransactionService 临时解析器接受 v1/v2、拒绝未知版本；新增 operation
  名称进入备份元数据；旧备份恢复后可读；写后验证/回滚失败保持真实错误。
- 更新自检：受管列表非空、选中 ID、当前 auth 匹配、外部/缺失状态和安全中文消息。
- 文件集合不增加第五项；确认 watcher、指纹、备份白名单和恢复仍覆盖两个被升级文件。
- 公开被测边界：TransactionService、BackupService、SelfCheckService、path_safety。
- 使用 `tempfile` 和现有故障注入，不启动真实 watcher 写用户目录。

### 9. TypeScript DTO、typed service 与 composable

- 先写失败测试：新 command 名与 camelCase 参数、普通 DTO 脱敏、管理 DTO 只进入短生命周期
  composable、选择/保存后的权威刷新、晚响应丢弃、busy 防重。
- 扩展 `src/types/provider.ts`、`src/services/tauri.ts` 和 `useProviders`；新增
  `useProviderApiKeyManager`，在 close/dispose 时清空秘密数组。
- 公开被测边界：typed service 与 composable 返回 API。
- mock `src/services/tauri.ts` 或注入 client；不 mock Vue reactivity 内部实现。

### 10. 创建表单与详情快速切换

- 先写组件失败测试：创建模式四个初始字段、编辑模式不再出现清空密钥入口、两行
  `ElSegmented`、managed/external/missing 状态、当前/非当前不同提示、键盘和 760px 布局。
- 实现 `ProviderEndpointControls`、`ProviderCredentialControls` 并让 `ProvidersView` 只编排。
- 公开被测边界：组件 props/emits、可见文本、ARIA 与用户触发事件。
- mock composable；不锁定 `.el-*` 私有 DOM 层级。

### 11. 两个管理对话框与 Onboarding

- 先写组件失败测试：有序草稿、统一保存/取消、即时校验、禁止删除规则、外部值命名、
  打开即明文密钥、隐藏/显示全部、逐项复制、关闭清空、复制失败安全提示、焦点恢复。
- 实现 Base URL 与 API Key 管理对话框；Onboarding 的现有 auth 导入增加名称输入并复用
  同一命名导入边界。
- 公开被测边界：对话框 props/emits、`navigator.clipboard.writeText` 调用和关闭事件。
- 仅 stub `navigator.clipboard` 与 typed service/composable；不把密钥写入测试快照。

### 12. 文档、规范与全范围检查

- 更新 README、AboutView 及测试、产品契约、架构、服务边界、路径/密钥安全和测试说明。
- 核对备份页、首次引导、Provider 状态、可用性禁用原因和所有旧“单个 API Key/Base URL”文案。
- 判断是否需要 `trellis-update-spec` 固化 v2 存储与“config 实际值是 URL 选择真相”规则。
- 运行 `trellis-check`，修复所有真实发现后再请求完成。

## 组件责任图

| 单元 | 单一职责 | 输入/输出 |
|---|---|---|
| `ProvidersView.vue` | 页面编排与对话框生命周期 | composable 状态 → 子组件 props；子事件 → actions |
| `ProviderEndpointControls.vue` | URL 快速选择与状态展示 | 脱敏 Provider → select/manage emits |
| `ProviderCredentialControls.vue` | 密钥快速选择与状态展示 | 脱敏 Provider → select/manage emits |
| `ProviderBaseUrlManagerDialog.vue` | URL 草稿编辑 | entries/fingerprint → save/cancel |
| `ProviderApiKeyManagerDialog.vue` | 秘密草稿查看、复制与编辑 | 短期 manager state → save/cancel |
| `useProviders` | 普通 Provider 权威状态 | readonly state + 显式 mutation |
| `useProviderApiKeyManager` | 对话框秘密生命周期 | load/save/clear，不共享全局状态 |

## 风险文件与回滚点

- `provider_secret_service.rs`、`provider_preference_service.rs`：版本迁移和秘密脱敏；先让纯逻辑
  与 round-trip 测试绿色，再接 ProviderService。
- `provider_service.rs`：统一读取与事务组合的最高风险点；每个新方法单独切片，不一次改完。
- `transaction_service.rs`：不得用直接 `serde_json::from_slice::<v2>` 破坏旧备份恢复；必须调用
  版本感知解析器。
- `ProviderEditor.vue` / `ProvidersView.vue`：避免继续膨胀；按组件图拆分，视图不持有完整密钥。
- `OnboardingView.vue`：命名导入改变公开交互，必须同步 App 编排和测试。
- 任一切片出现无法解释的重复失败时返回需求/设计，不通过放宽事务、安全或测试门禁绕过。

## 专项验证命令

实施过程中按切片选择过滤测试，例如：

```powershell
cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-core provider_secret_service::tests
cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-core provider_preference_service::tests
cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-core provider_service::tests
cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-core --test provider_workflow
cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-core --test path_safety
npx vitest run src/components/ProviderEditor.test.ts src/views/ProvidersView.test.ts
npx vitest run src/components/ProviderBaseUrlManagerDialog.test.ts src/components/ProviderApiKeyManagerDialog.test.ts
npm run typecheck
```

若默认 Cargo target 被用户进程占用，只能改用已验证位于系统临时目录内的独立
`CARGO_TARGET_DIR`；不得终止用户进程或清理默认 target。

## 完成前验证

```powershell
npm run test:trellis
npm run check:frontend
npm run check:rust
npm run check
npm run build:frontend
git diff --check
git status --short --branch
git ls-files
```

补充执行密钥与路径审计，确认 Git、任务材料、规范、日志 fixture 和测试输出没有真实密钥、
Authorization Header 或完整认证文件。若进行桌面人工验证，只允许 `npm run dev:safe` 或成对
Relay 覆盖；未执行时交付中明确说明，不能用单元测试代替人工观察。
