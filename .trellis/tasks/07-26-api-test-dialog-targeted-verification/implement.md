# 实施计划：API 测试结果生命周期与分层验证规则

## 前置检查

- [x] 已创建并收敛 `prd.md`，用户请求已明确授权实施。
- [x] 已读取 frontend/testing/workflow 相关规范、Vue 必读参考和 trace 归档设计。
- [x] 已确认 `codex.dispatch_mode=inline`，不创建子 Agent 或重复 Superpowers TDD 流程。
- [x] 已运行专项基线：面板 7、composable 5、视图 13 项通过。
- [x] `task.py start` 前运行 `trellis-before-dev` 并复核目标包规范。

## 行为切片与顺序

1. **清除旧 API 结果（红 → 绿）**
   - 先在 `useProviderAvailability.test` 增加未完成请求测试：旧 API 结果在新测试开始后
     立即为空，Codex 结果不变；现实现应因仍保留旧结果而失败。
   - 在统一测试入口按 `kind` 清除结果，保持运行 token/generation 与错误处理不变。
   - 运行 composable 专项测试并重构重复 fixture。

2. **新 trace 自动打开弹窗（红 → 绿）**
   - 先在 `ProviderAvailabilityPanel.test.ts` 增加旧 trace → 点击 → 清空 → 新 trace 的失败
     测试，以及无 trace/Provider 变化不打开的边界测试。
   - 增加面板一次性 pending 状态和精确 watcher；按钮事件契约、手动入口和弹窗组件 API 不变。
   - 运行面板专项测试，必要时补 `ProvidersView` 事件透传断言。

3. **验证规则沉淀**
   - 在 `.trellis/spec/testing/verification.md` 增加“按风险分层验证”章节，明确专项、扩大和
     提交/归档完整检查的触发条件与报告要求。
   - 在 `.trellis/spec/frontend/state-management.md` 记录同类重测清除顺序和详情 pending 的
     所有权，避免结果 store 与局部弹窗状态再次分叉。
   - 用 `trellis-update-spec` 规则检查文档位置、可执行性和与现有标准检查的兼容性。

4. **质量检查与收尾**
   - 受影响前端专项：面板、composable、视图；再运行 `npm run typecheck`、`npm run check:frontend`
     和 `git diff --check`。
   - 因涉及共享异步 composable，按风险规则运行 `npm run check`；若检查触发构建/安全限制，
     如实记录失败或未执行项。
   - 更新任务进度和验证证据，完成 Trellis check、规范更新、提交前审计与收尾。

## 风险文件与回滚点

- `src/composables/useProviderAvailability.ts` / `.test.ts`：结果清除和异步隔离；可独立回退。
- `src/components/ProviderAvailabilityPanel.vue` / `.test.ts`：弹窗 pending/watch；可独立回退。
- `.trellis/spec/testing/verification.md`：长期规则；若措辞与现有门禁冲突，先修正文档再提交。
- `.trellis/spec/frontend/state-management.md`：availability 状态契约；与 composable 行为保持同步。
- `.trellis/tasks/07-26-api-test-dialog-targeted-verification/*`：过程证据，不含秘密或真实路径。

## 验证命令

```powershell
npx vitest run src/composables/useProviderAvailability.test.ts
npx vitest run src/components/ProviderAvailabilityPanel.test.ts
npx vitest run src/views/ProvidersView.test.ts
npm run typecheck
npm run check:frontend
npm run check
git diff --check
```

## 当前进度

- 阶段：实施与质量检查完成，待提交前审计和 Trellis 收尾。
- 已完成：行为切片 1“按测试类型清除旧结果”红测 → 最小实现 → composable 专项绿色（6 项）。
- 已完成：行为切片 2“新 trace 自动打开详情”红测 → 最小实现 → 面板专项绿色（11 项，含无 trace、取消和 Provider 切换边界）。
- 已完成：受影响三组专项与 `npm run typecheck` 通过（28 项前端专项）。
- 已完成：将风险分层验证规则写入 `.trellis/spec/testing/verification.md`。
- 已完成：首轮 `npm run check` 通过（Trellis 8 项；前端 39 文件/185 项；Rust workspace
  172+40+3+1 项；Rust fmt、Clippy 和依赖图检查通过）。新增边界测试后的第二轮聚合命令在
  并行 `codex_relay_core` 测试进程中持续无输出约 6 分钟，已终止并保留该限制；随后拆分重跑
  `test:trellis` 8/8、`check:frontend` 39 文件/187 项，以及设置 `RUST_TEST_THREADS=1` 的
  `check:rust`（172+40+3+1 项）均退出 0。
- 下一步：完成安全/跟踪文件审计、最终 diff 检查，并按 Trellis 3.4/3.5 收尾。

## 红绿证据

- 切片 1 红测：新增断言在旧实现下失败，报告“expected … to be null”，旧 API 结果仍被读到。
- 切片 1 绿测：修复后 `npx vitest run src/composables/useProviderAvailability.test.ts` 为 6/6。
- 切片 2 红测：新增自动打开断言在旧实现下失败，报告缺少 `"status": "new-response"`。
- 切片 2 绿测：修复后 `npx vitest run src/components/ProviderAvailabilityPanel.test.ts` 为 9/9。
- 组合专项：面板 11 项、composable 6 项、视图 13 项共 30/30，`npm run typecheck` 退出 0。
- 完整检查：首轮 `npm run check` 退出 0；最终代码状态使用等价拆分门禁重新验证：前端
  39/39 文件、187/187 测试，Trellis 8/8，Rust 172+40+3+1 项及 fmt/Clippy/依赖图均无失败。
- 限制：最终聚合 `npm run check` 的一次并行 Rust 运行挂起并被终止；单线程 Rust 全量重跑通过，
  不能把被终止的那次聚合命令声称为成功。
- 提交前审计：`task.py validate`、`git diff --check` 通过；高置信度凭据模式无命中；本轮挂起的
  `npm run check` 进程树已核对命令行并终止，最终审计时无残留 cargo/core 测试进程。

## 尚未解决的问题

- 无产品范围阻塞问题；未运行真实 Provider 网络或人工 UI 观察，本轮不把它们声称为已验证。
