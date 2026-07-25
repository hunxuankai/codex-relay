# 实施计划：展示 Provider API 验证请求与响应

## 前置门禁

- [x] `prd.md` 已完成收敛，需求和验收标准可观察、无未决产品问题。
- [x] `design.md` 已完成，跨层契约、错误矩阵、安全边界和回滚点已审查。
- [x] 用户已批准本规划；已运行 `task.py start` 后才编辑运行时代码。
- [x] 开始编辑前加载 `trellis-before-dev`，读取当前 frontend/backend/security/testing 规范。
- [x] 每个行为切片按“失败测试 → 最小实现 → 专项绿色 → 必要重构”执行。

## 有序实施步骤

### 1. Rust trace DTO 与探针失败测试

- [x] 修改 `models/provider_availability.rs` 的测试，先断言 `ProviderAvailabilityTrace` 的 camelCase 序列化、`trace=null` 兼容和安全 Debug；确认因字段/序列化缺失而失败。
- [x] 修改 `provider_http` 测试，先断言真实回环请求的 method/path/body、`stream=false`、成功响应正文和 HTTP 错误正文会进入 trace；覆盖无响应和正文截断。
- 仅使用回环 TCP、`test-key-*-not-real` 和临时路径；不得启动真实 Provider。

### 2. Rust HTTP 与服务实现

- [x] 在 `provider_http.rs` 增加内部 trace/失败包装，复用当前 endpoint、payload、取消、超时和 256 KiB 读取上限。
- [x] HTTP 非成功先读取有界正文再分类；网络/超时/取消携带已形成的 request trace；响应文本在公开前移除当前真实 key。
- [x] 在 `provider_availability_service.rs` 合并成功/失败 trace，保持现有稳定状态、错误码、代理选择和 `stream=false`；Codex 分支 `trace=null`。
- [x] 更新 service 专项测试和现有错误映射测试，确认没有配置写入或生产路径访问。

### 3. TypeScript DTO 与 composable 透传

- [x] 扩展 `src/types/providerAvailability.ts` 的共享类型。
- [x] 更新 `useProviderAvailability.test.ts`、`tauri.test.ts` 和相关 fixture，验证 trace 随 API 结果保存、Codex 结果为空、指纹失效清除；不改变 command 参数。
- [x] 保持 typed Tauri service API 不新增命令，确认旧结果无 trace 时仍可反序列化。

### 4. 详情弹窗组件与面板入口

- [x] 新增 `src/components/ProviderAvailabilityTraceDialog.vue`，遵循 Composition API、显式 props/emits、Element Plus `ElDialog`、键盘关闭、窄窗口和深色主题。
- [x] 先在 `ProviderAvailabilityTraceDialog.test.ts` 添加请求/响应、无响应、截断和关闭行为的红测。
- [x] 扩展 `ProviderAvailabilityPanel.vue`：只在 API trace 存在时显示入口，打开/关闭弹窗；Provider 或结果变化时关闭，不重新请求。
- [x] 扩展 `ProviderAvailabilityPanel.test.ts`，验证可访问名称、按钮条件、API/Codex 独立性和详情展示。

### 5. 契约文档与规范同步

- [x] 更新 README、About 页面及其测试中“不会展示原始响应正文”的过时说明。
- [x] 更新 `.trellis/spec/backend/provider-availability-testing.md`、相关 frontend/security/project 规范，记录 trace 边界、256 KiB 上限、无 Header/密钥/代理和同步 API / SSE Codex 分工。
- [x] 检查所有 `ProviderAvailabilityResult` fixture、快照和类型引用，避免遗漏字段或把 trace 误用于 Codex。

### 6. 质量门禁与审查

- [x] 先运行专项前端测试和 `npm run typecheck`。
- [x] 按 Rust 开发反馈规范运行 `npm run test:rust:lib -- provider_http`、`npm run test:rust:lib -- provider_availability`；两次均通过 watcher 安全门禁。
- [x] 运行 `cargo fmt --all --check --manifest-path src-tauri/Cargo.toml`。
- [x] 运行 `npm run check`（覆盖 Trellis、前端、workspace Rust、路径安全和 Provider workflow）。
- [x] 运行 `git diff --check`、`git status --short --ignored`、`git ls-files` 与高置信度密钥/Authorization/真实路径审计。
- [x] 逐项对照 PRD 验收标准，记录真实命令、退出码、测试数量和未执行限制。

## 建议验证命令

```powershell
npm run test -- src/components/ProviderAvailabilityTraceDialog.test.ts src/components/ProviderAvailabilityPanel.test.ts src/composables/useProviderAvailability.test.ts src/services/tauri.test.ts
npm run typecheck
npm run test:rust:lib -- provider_http
npm run test:rust:lib -- provider_availability_service
cargo fmt --all --check --manifest-path src-tauri/Cargo.toml
npm run check
git diff --check
```

Rust 命令必须通过 watcher 安全门禁，并使用项目规定的 workspace/core target；不得设置随机 Cargo target 或回退真实用户路径。

## 风险与控制

| 风险 | 控制 |
|---|---|
| HTTP 错误正文泄漏密钥 | 只在 Rust 边界捕获；不记录 Header；返回前移除当前真实 key；自定义 Debug 不打印正文。 |
| trace 与测试结果错配 | trace 作为同一个 `ProviderAvailabilityResult` 字段返回，沿用现有 token/generation/fingerprint 隔离。 |
| 大正文撑爆 IPC/UI | 复用 256 KiB 协议上限，返回 `bodyTruncated`，弹窗使用滚动容器。 |
| 请求取消后伪造响应 | response 只有收到 HTTP 后才建立；取消路径保持 `response=null` 或已有真实片段。 |
| Codex 行为回归 | Codex 结果保持 `trace=null`，保留现有 SSE gateway 专项测试。 |
| 规范与实现不一致 | 完成代码后同步 README/About/`.trellis/spec`，再运行全量检查。 |

## 进度与验证记录

- 2026-07-25：完成需求探索；确认 API-only、详情弹窗、方案 1、同步 API probe、Codex SSE 不变及安全边界。
- 2026-07-25：已生成并自审 `prd.md`、`design.md`、`implement.md`，通过审查门禁后运行 `task.py start`，任务进入 `in_progress`。
- 2026-07-25：行为切片 1 红测：新增 DTO/HTTP trace 断言后，`npm run test:rust:lib -- provider_http` 首次因缺少 trace 类型和字段失败，原因符合预期。
- 2026-07-25：行为切片 1 绿色：实现基础 trace DTO、成功 API 探针请求/响应捕获和安全 Debug；`provider_http` 6 项、`provider_availability` 11 项通过。HTTP 错误正文和截断失败路径仍待下一切片。
- 2026-07-25：行为切片 2 红测/编译门禁：把 HTTP 失败、无效正文、超限和取消测试改为要求 `ApiProbeFailure.trace` 后，旧错误返回类型无法编译，证明失败路径尚未携带 trace。
- 2026-07-25：行为切片 2 绿色：新增显式 `ApiProbeFailure`、HTTP 错误正文读取、有界截断、无响应语义和 service 状态合并；复用 `safe_log::redact` 并额外移除当前真实 key。`provider_http` 6 项、`provider_availability` 12 项通过。
- 2026-07-25：行为切片 2 回归红测/绿色修复：补充断言 `Content-Length` 超限时仍保留 256 KiB 前缀；先观察到正文长度为 0 的失败，再改为读取有界前缀并标记 `bodyTruncated=true`，专项测试重新通过。
- 2026-07-25：安全回归红测/绿色修复：先证明 `ProviderAvailabilityTarget` Debug 会暴露完整 Base URL，再改为只输出 `base_url_configured`；同时新增 URL userinfo/敏感查询清理，以及最终 UTF-8/凭据清理文本仍不超过 256 KiB 的红绿测试。`provider_http` 增至 8 项，`provider_availability` 12 项通过。
- 2026-07-25：行为切片 3 绿色：新增 TypeScript trace DTO、详情弹窗和面板入口；API/Codex 结果保持独立，Provider/结果变化关闭旧弹窗。专项 Vitest 2 文件 10 项、全量 Vitest 37 文件 175 项、`npm run typecheck` 均通过。
- 2026-07-25：同步 README、About、产品/后端/前端/安全规范；`git diff --check` 通过（仅报告 Windows 行尾提示）。
- 2026-07-25：Escape 的真实 `KeyboardEvent` 在 jsdom 中未触发 Element Plus FocusTrap，保留该失败证据；随后显式设置 `closeOnPressEscape=true`，通过公开 prop、`update:modelValue` 契约和关闭按钮真实点击验证，专项 3 项通过。
- 2026-07-25：最终 `npm run check` 退出 0：Trellis 8 项、Vitest 37 文件 175 项、Rust 根 crate 40 项、core 172 项、路径安全 3 项、Provider workflow 1 项，以及 fmt、Clippy `-D warnings`、依赖图与 doc tests 全部通过。
- 2026-07-25：`npm run build:frontend` 退出 0，Vite 转换 1739 个模块并生成 `dist`；仅有第三方 `@vueuse/core` PURE 注释位置警告。`task.py validate` 通过。
- 2026-07-25：高置信度密钥扫描首次因 PowerShell/正则引号错误失败，拆分为 key 前缀、Authorization Bearer、`OPENAI_API_KEY` 赋值和真实用户路径规则后重新执行；前三项无命中，路径项仅命中既有安全拒绝 fixture，人工复核为测试数据。`git ls-files` 未跟踪认证存储或备份，`git status --short --ignored` 只显示本任务改动与既有忽略目录。
