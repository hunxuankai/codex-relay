# 实施计划：版本标题、Provider 排序与 API 详情弹窗

## 前置检查

- [x] 已创建并收敛 `prd.md`，用户明确要求实施；排序持久化采用默认推荐方案。
- [x] 已读取 frontend、project、backend、security、testing、workflow 相关规范及 Vue 核心参考。
- [x] 已确认 `codex.dispatch_mode=inline`，不派发实现/检查子 Agent，不重复建立 Superpowers TDD 生命周期。
- [x] `task.py start` 前完成首个行为切片的失败测试与材料审查；任务已处于 `in_progress`，本轮继续进行最终质量检查。

## 行为切片（每项红 → 绿 → 重构）

### 1. 版本标题

- 在 `App.test.ts` 增加版本成功和获取失败仍渲染安全标题的公开行为断言。
- 修改 `App.vue` 标题区域，复用 `appVersion`，不新增 IPC。
- 运行 App 专项测试和类型检查。

### 2. API 测试详情弹窗立即打开与 loading

- 更新 `ProviderAvailabilityPanel.test.ts` / `ProviderAvailabilityTraceDialog.test.ts`：点击后
  立即存在打开弹窗、请求/响应 loading、遮罩/Escape/关闭和再次打开；先运行确认旧实现失败。
- 修改 trace dialog 的可空 trace/loading 契约、遮罩关闭、loading/trace 分区；面板在点击时打开，
  保持关闭后的显式再次打开语义，清理 Provider/失效边界。
- 保持 `useProviderAvailability` 的旧结果清除、取消、晚响应和 API/Codex 独立契约；只在需要时
  补回归测试。

### 3. Provider 私有顺序模型与事务服务

- 先为 preference service / ProviderService 添加 providerOrder 兼容、重排和失败路径测试，运行
  红测。
- 新增 `ReorderProvidersInput`、`TransactionOperation::ReorderProviders`、偏好字段规范化、
  ProviderService 重排及列表排序投影；创建/删除同步顺序。
- 新增 Tauri command、服务 typed wrapper 和 `useProviders.reorder`，补充命令注册及事件/托盘
  刷新路径；运行 Rust core 与 command 专项测试。

### 4. ProviderList 拖放交互

- 先补 `ProviderList.test.ts` 拖动手柄/drop/busy 失败断言。
- 实现原生 HTML5 拖动手柄、精确 ID 排列 emit、可见状态/aria-label 与窄窗口样式；ProvidersView
  连接 reorder action，不把排序状态复制到组件外。
- 运行列表、视图、composable 专项并重构重复 fixture。

### 5. 文档与质量检查

- 同步产品契约、Provider 多凭据规范、架构/README/About 页面中排序能力和私有字段说明，删除
  “拖拽排序”作为非目标的过时表述。
- 加载 `trellis-check`，覆盖规范符合性、跨层数据流、前端与 Rust 测试、类型/lint、diff 检查。
- 按风险执行：受影响专项 → `npm run typecheck` → `npm run check:frontend` → Rust core/command
  测试 → `npm run check`；任何超时或未执行项如实记录。

## 受影响文件与回滚点

- 前端：`src/App.vue`、`src/App.test.ts`、`src/components/ProviderList.vue/.test.ts`、
  `src/components/ProviderAvailabilityPanel.vue/.test.ts`、`src/components/ProviderAvailabilityTraceDialog.vue/.test.ts`、
  `src/views/ProvidersView.vue/.test.ts`、`src/composables/useProviders.ts/.test.ts`、
  `src/services/tauri.ts/.test.ts`、`src/types/provider.ts`。
- Rust：`models/provider.rs`、`models/transaction.rs`、`services/provider_preference_service.rs`、
  `services/provider_service.rs`、`commands/provider_commands.rs`、`lib.rs`及相关测试。
- 文档：产品/架构/Provider 规范、README、About 说明、任务验证记录。
- 回滚优先级：先回退拖放 UI，再回退排序 command/字段；API 弹窗与版本标题可独立保留。

## 验证命令

```powershell
npx vitest run src/App.test.ts src/components/ProviderList.test.ts src/components/ProviderAvailabilityPanel.test.ts src/components/ProviderAvailabilityTraceDialog.test.ts src/views/ProvidersView.test.ts src/composables/useProviders.test.ts src/services/tauri.test.ts src/views/AboutView.test.ts
npm run typecheck
npm run check:frontend
cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay-core --lib
cargo test --manifest-path src-tauri/Cargo.toml --target-dir src-tauri/target -p codex-relay --lib
npm run check:rust
npm run check
git diff --check
```

命令实际执行时按 Cargo/Vitest 支持的筛选语法调整，并在任务材料中记录真实退出码与测试数；
不得把未运行的人工 UI、真实 Provider 网络、安装或发布行为写成成功。

## 当前进度

- 阶段：实施和 2.2 全范围质量检查已完成，进入 Phase 3.3/3.4 收尾；Rust 格式化已通过。
- 已完成：版本标题、API 测试详情弹窗即时打开/loading/关闭与重开、Provider 原生拖放及方向键排序、
  Relay 私有排序持久化、事务/指纹/回滚边界、前端与 Rust 专项测试，以及相关产品/架构/安全规范同步。
- 关键决策：Provider 顺序只写 `provider-preferences.json.providerOrder`，不重排 Codex 官方
  `config.toml`；弹窗关闭只改变局部显示状态，不取消正在进行的测试。
- 当前证据：
  - `cargo fmt --all --manifest-path src-tauri/Cargo.toml` 与对应 `--check`：退出码 0；
    `git diff --check` 无空白错误。
  - `npx vitest run`（App、Provider 列表/视图、API 详情面板/弹窗、composable、typed service、关于页）：
    8 个测试文件、66 项测试全部通过。
  - `npm run typecheck`：退出码 0。
  - `cargo test ... -p codex-relay-core --lib`：177/177 通过；
    `cargo test ... -p codex-relay --lib`：41/41 通过。
  - `npm run check:frontend`：类型检查通过；39 个测试文件、197 项测试全部通过。
  - `npm run check:rust`：依赖图（ring 已启用、无 `aws-lc-sys`、core 无 Tauri）、fmt、Clippy、
    workspace 测试全部通过；workspace 测试包含 core 177、Tauri 根 crate 41、path safety 3、
    provider workflow 1 项及无测试目标的正常退出。
  - `npm run check`（非法 ID 修正后最终运行）：退出码 0；Trellis 脚本 8/8、前端 39 个文件/197 项、Rust workspace 全部通过；同一命令后的 `git diff --check` 也退出码 0。
  - 最终人工审查发现非法 Provider ID 会透出 `INVALID_PROVIDER_ID`：新增回归用例后先得到预期红测，
    随后把所有非精确排列统一映射为 `INVALID_PROVIDER_ORDER`；`cargo test ... --lib reorder` 3/3 通过，
    并补充断言排序成功或失败时 `config.toml`、`auth.json`、`providers.json` 均保持原字节。
  - 仓库秘密审计：高置信度前缀扫描只命中明确的 `test-key-wrong-not-real` 假密钥；
    `OPENAI_API_KEY` / Authorization / Bearer / `apiKey` 命中均为代码契约、文档或
    `test-key-*-not-real` fixture。`git status --short --ignored` 仅显示预期忽略目录和本任务改动。
- 工作提交：
  - `995d96f feat(provider): 完善版本标题与 API 测试详情交互`
  - `da14243 feat(provider): 支持 Provider 拖动排序与私有持久化`
  - `743a1f5 docs(provider): 同步排序与测试交互契约`
- 下一步：提交本任务材料，然后运行 `trellis-finish-work` 归档任务并记录会话日志（不 push）。
- 尚未解决的问题：未进行真实 Provider 网络、人工 UI、安装/发布/卸载验证，
  这些不属于本次自动化证据范围。
