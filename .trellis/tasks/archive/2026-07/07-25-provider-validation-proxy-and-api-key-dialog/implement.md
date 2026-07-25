# 实施计划：Provider 验证代理选项与密钥弹窗保存收尾

## 实施步骤

1. [已完成] 为可用性面板补充失败的 Vue 测试：默认“不使用代理”、取消勾选后的代理未启用门禁，以及 API/Codex 事件携带的 `useProxy`。
2. [已完成] 为 Provider 页面补充失败测试：从应用传入代理状态、Codex 确认时保留请求模式，以及 API Key 保存成功自动关闭/清空、失败保持打开。
3. [已完成] 扩展 `ProviderAvailabilityPanel`、`ProvidersView` 与 `App` 的显式 props/emits；使用现有 `AppNotification` 提供无密钥成功反馈。
4. [已完成] 为 `useProviderAvailability` 和 typed Tauri service 补充 `useProxy` 透传测试，再更新其 TypeScript 签名和调用点。
5. [已完成] 为 Rust command 和 core service 写失败测试：代理未启用的稳定错误、无设置文件写入和 command 请求 ID 校验已完成；全量 core、路径安全与 Provider workflow 测试已通过。
6. [已完成] 更新 command、service、API HTTP/Codex 网关调用链；不添加任何配置写入或秘密日志。
7. [已完成] 运行专项测试、类型检查、Rust 格式/库测试和任务级质量检查；审查 diff、密钥命中、路径隔离与规范更新需求。watcher 阻塞解除后，标准 `npm run check` 与提交前 workspace Rust 门禁均已通过。

## 验证命令

```powershell
npm run test -- src/components/ProviderAvailabilityPanel.test.ts src/views/ProvidersView.test.ts src/composables/useProviderAvailability.test.ts src/services/tauri.test.ts
npm run typecheck
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
npm run test:rust:lib
npm run check
```

若专项 Rust 测试需要更快反馈，先在安全临时路径的 core crate 上运行目标测试；完成前仍执行上述任务级检查。任何失败、环境限制或未执行项必须记录在本文件的进度与验证记录中。

## 风险文件与回滚点

| 风险 | 文件/边界 | 控制与回滚 |
|---|---|---|
| UI 与 IPC 语义不同步 | `App.vue`、`ProvidersView.vue`、面板、composable、`tauri.ts` | 先写端到端参数断言；若失败，回滚这一链的同一提交。 |
| 代理被静默绕过 | `ProviderAvailabilityService`、`provider_http.rs`、`codex_gateway.rs` | 使用单一解析函数和测试；禁用代理返回稳定错误。 |
| 密钥关闭前后泄漏 | API Key manager 与页面 | 只保存安全消息，成功后调用现有 `clear()`；不改变普通 Provider DTO。 |
| 测试触及真实目录 | Rust 测试 | 仅使用 `AppPaths::for_test` / `tempfile`，保留路径哨兵测试。 |

## 开始实施前检查

- [x] 用户已明确要求实施。
- [x] PRD 已收敛，验收标准可测试。
- [x] 复杂任务的设计与实施计划已建立。
- [x] 已加载实施阶段细则和相关编码规范。
- [x] 红色测试已确认按预期失败。

## 进度与验证记录

- 2026-07-25：已创建任务，完成代码与规范探索，尚未编辑运行时代码或执行测试。
- 2026-07-25：行为切片 1 完成。`ProviderAvailabilityPanel` 以本地布尔状态默认绕过代理；代理未启用时阻止测试并显示原因。红色测试先失败，随后 `npm run test -- src/components/ProviderAvailabilityPanel.test.ts` 通过（6 项）。
- 2026-07-25：行为切片 2 完成。`ProvidersView` 仅在 API Key 保存、权威重载和 Provider 刷新全部成功后关闭管理器，关闭继续调用 `clear()`；安全成功提示移到对话框外。红色测试先失败，随后 `npm run test -- src/views/ProvidersView.test.ts` 通过（12 项）。
- 2026-07-25：行为切片 3 完成。`useProviderAvailability` 与 typed Tauri service 显式透传 `useProxy`，`App.vue` 复用唯一设置状态传给 Provider 页面，页面将模式快照保留到 Codex 确认完成。红色测试先失败，随后组合式（5 项）、typed service（7 项）和 App/Provider 页面（18 项）专项测试通过。
- 2026-07-25：行为切片 4 完成。Rust command 与 `ProviderAvailabilityService` 接受 `use_proxy`；共享只读解析函数让 API 探针与 Codex 网关使用同一代理结果。代理未启用时返回 `PROVIDER_TEST_PROXY_DISABLED`，不创建 `settings.json`。红色 Rust 编译失败先确认缺少参数，随后 core 专项测试（1 项）、Tauri command 专项测试（1 项）及 `npm run typecheck` 通过。
- 2026-07-25：长期规范已更新：产品契约、前端 Provider 测试会话契约和后端可用性测试契约均记录 `useProxy/use_proxy`、默认直连、代理未启用门禁与 API/Codex 共享解析结果。
- 2026-07-25：质量证据：`npm run test` 通过（36 文件、168 项）；`npm run typecheck` 通过；隔离临时 Cargo target 的 `codex-relay-core --lib` 通过（169 项）及 `path_safety` 通过（3 项）；`cargo fmt --all --check --manifest-path src-tauri/Cargo.toml`、`npm run test:trellis`（8 项）、`npm run test:rust-dev-guard`（12 项）和 `npm run check:rust:deps` 均通过。`npm run test:rust:lib` 被运行中的普通 Tauri dev watcher 按预期阻止，未把该结果计为测试通过；因此本轮未运行完整 `npm run check`，待 watcher 停止或改为 `npm run dev:safe:no-watch` 后执行。
- 2026-07-25：完成性复核时原 watcher 已不存在；重新运行标准 `npm run check`，退出码 0、耗时 215.9 秒。Trellis 单元测试 8 项通过；前端 typecheck 通过，Vitest 36 个文件、169 项通过；Rust 依赖图、fmt、Clippy 通过，根 crate 40 项、core 169 项、`path_safety` 3 项、`provider_workflow` 1 项全部通过。
- 2026-07-25：提交前审计通过：`git diff --check HEAD` 退出 0；改动与任务范围一致；高置信度密钥扫描无命中；未发现 `console.log`、`dbg!`、`println!` 等调试残留；`dev-data`、构建产物和依赖目录保持忽略，未发现真实认证存储被跟踪。
