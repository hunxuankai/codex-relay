# 优化 API 可用性测试交互与验证规则

## 目标与用户价值

用户重新点击“测试 API 可用性”时，应立即进入本次测试的上下文：上一次 API 结果和
trace 不再干扰判断；本次请求/响应 trace 返回后，详情弹窗自动打开，用户无需先看结果卡片
再点一次“查看请求与响应”。同时把“局部、低风险改动不必每次运行全量测试”的判断边界
沉淀到项目测试规范，缩短反馈时间而不降低跨层、高风险和提交前的质量门禁。

## 已确认事实与根因证据

- `ProviderAvailabilityPanel` 当前只在用户点击旧结果卡片时打开详情；`traceDialogOpen` 在
  `src/components/ProviderAvailabilityPanel.vue:33-48` 管理，且 `apiResult` 变化时现有 watcher
  会无条件关闭弹窗（`src/components/ProviderAvailabilityPanel.vue:37-41`）。
- `useProviderAvailability.test` 先设置运行状态，等异步 command 返回后才调用 `storeResult`
  （`src/composables/useProviderAvailability.ts:91-128`），因此重新测试期间仍会展示旧 API 结果。
- `ProvidersView.startApiTest` 目前只转发 `testApi`（`src/views/ProvidersView.vue:205-209`），
  面板不应直接访问 Tauri 或复制测试状态。
- API trace 只在请求已构造时存在；请求构造前失败时 `trace` 必须保持 `null`，这是既有
  `provider-availability-testing` 与 trace 任务的安全契约。Codex 结果不携带 API trace。
- 现有专项基线：`ProviderAvailabilityPanel.test.ts` 7 项、`useProviderAvailability.test.ts` 5 项、
  `ProvidersView.test.ts` 13 项均通过（2026-07-26 本轮命令证据）。

## 需求

### API-UI-1：重新测试先清除旧结果

用户点击可用且空闲的“测试 API 可用性”按钮时，先清除该 Provider 的旧 API 结果，再开始
新的异步测试；只清除 API 类型，不清除 Codex 结果。重复点击在已有测试运行时仍被阻止，
Provider 切换、指纹失效和晚响应隔离契约保持不变。

### API-UI-2：新 trace 自动打开详情

点击 API 测试按钮时登记一次短生命周期的“待自动打开”状态，并关闭可能已打开的旧详情。
本次测试返回带 `trace` 的新结果后，自动打开“查看请求与响应”弹窗并展示该结果的请求、
响应和耗时；不能展示旧 trace。弹窗打开/关闭不发起请求，也不改变测试状态。

若本次测试在请求构造前失败、结果没有 trace 或被取消/作废，则不伪造请求或响应，保留既有
“无 trace 不显示详情入口”的契约，并清理待自动打开状态。Provider 切换、结果失效或组件
卸载时弹窗和待自动打开状态都必须清理。

### VERIFY-1：按风险分层验证

在 `.trellis/spec/testing/verification.md` 增加可执行规则：

1. 单层、局部、低风险且不涉及共享状态、跨层契约、安全边界、配置/构建/发布的改动，运行
   直接相关专项测试，并运行受影响层的类型/静态检查和 `git diff --check`；无需默认运行
   全量测试。
2. 涉及跨层数据流、共享 composable/基础设施、异步并发/取消、秘密或路径安全、配置迁移、
   构建/发布流程，或专项测试失败后修复的改动，运行受影响层测试并扩大到 `npm run check`
   （必要时再运行构建或安全专项）。
3. 提交、任务归档、发布前仍执行项目规定的完整检查；任何未执行项目必须在报告中明确写出，
   不能用旧报告或“改动很小”替代新鲜证据。

本次交互改动虽主要在 Vue，但同时触及共享 availability composable 的异步结果生命周期，
因此本轮至少运行面板、composable、视图专项测试、前端类型检查和一次与风险相称的完整检查。

## 验收标准

- [x] 点击 API 测试后，旧 API 结果卡片/旧 trace 在新请求期间不可见；Codex 结果仍保留。
- [x] 新 API 结果带 trace 时，详情弹窗自动打开，并显示新请求 URL/body、响应状态/body 和耗时；
      不会短暂或最终显示旧 trace。
- [x] 无 trace 的配置前置失败、取消、Provider 切换、指纹失效和组件卸载不会打开或残留详情弹窗。
- [x] 手动打开旧结果详情、关闭按钮、Escape 和既有可访问性行为不回归；弹窗不触发额外请求。
- [x] 现有 API/Codex 测试独立保存、晚响应隔离、代理参数透传和安全 trace 契约不变。
- [x] 测试规范明确区分专项验证、风险扩大验证和提交/归档前完整检查，并包含本轮可复用的判断标准。
- [x] 相关专项测试、类型检查、完整检查命令均以本轮真实输出记录；未执行的人工或发布验证如实说明。

## 范围外

- 不修改 Rust API trace 生成、请求格式、响应上限、密钥脱敏或任何受管配置写入。
- 不新增测试历史、持久化 trace、复制/导出功能，不改变 Codex 兼容性测试交互。
- 不为了满足本次局部改动而无条件重构所有测试或降低安全、跨层和发布门禁。

## 实施状态

本任务修改了 Vue 组件、共享 availability composable、对应 Vitest，以及项目测试验证规范；
两个行为切片均完成红—绿验证；首轮 `npm run check` 通过，最终代码状态又通过拆分的 Trellis、
前端和单线程 Rust 全量门禁。最终聚合命令有一次并行 Rust 挂起并被终止，限制已记录。未运行
真实 Provider 网络或人工 UI 观察，因此交付说明不把这两项称为已验证。
