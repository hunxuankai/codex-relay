# 实施计划

## 行为切片

1. **危险纯按钮 hover 可读**：先在 `src/style.test.ts` 写回归断言，确认旧的
   `.danger-link { color: ... }` 会使测试失败；移除静态颜色并运行 `ApiKeyInput` 与样式专项
   测试。
2. **全局按钮状态审计**：逐一复核所有 `ElButton` 的 `type/plain/text/link` 及关联样式，
   记录结果；若发现同类覆盖，在同一切片中按最小改动修复并补断言。
3. **规范沉淀**：把“按钮装饰类只负责布局，颜色交给语义组件状态”的规则写入前端规范，
   在任务材料记录根因、审计范围和验证证据。

## 验证命令

```powershell
npm run test -- src/style.test.ts src/components/ApiKeyInput.test.ts
npm run typecheck
npm run check:frontend
git diff --check
```

若专项检查通过，再运行 `npm run test` 作为全量前端回归；不需要运行真实配置目录或真实
API Key。

## 风险与回滚点

- 风险文件：`src/components/ApiKeyInput.vue`、`src/style.test.ts`、
  `.trellis/spec/frontend/vue-guidelines.md`。
- 回滚点：移除 `danger-link` 静态颜色及新增契约（不恢复该覆盖），或针对验证发现的
  具体状态选择器单独修正。

## 当前进度与证据

- [x] 行为切片 1：新增样式回归后在旧实现上失败（`src/style.test.ts`，1 failed / 8
  passed），移除 `.danger-link` 的静态 `color` 后专项测试通过。
- [x] 行为切片 2：扫描所有 `ElButton` 的静态装饰类；当前无其它裸类颜色/背景/边框颜色
  覆盖。受控恢复旧行时全局扫描报告 `src/components/ApiKeyInput.vue: .danger-link`，
  证明契约能捕获同类回归。
- [x] 行为切片 3：已将规则写入 `.trellis/spec/frontend/vue-guidelines.md`。
- [x] 专项测试：`src/style.test.ts` 9/9、`src/components/ApiKeyInput.test.ts` 2/2；
  类型检查通过；`npm run check:frontend` 通过（28 个文件、127 个测试）；前端构建通过。
- [x] `npm run test:trellis` 通过；`cargo fmt --check`、Clippy、Rust 单元测试通过（124/124）。
- [ ] `npm run check` 的 Rust 集成测试仍有既有失败：
  `provider_workflow_preserves_unknown_config_and_restores_original_bytes` 在
  `switch_provider` 因缺少 `provider-preferences.json` 返回 `PROVIDER_PREFERENCE_MISSING`。
  该测试与偏好功能同一历史提交 `3a12680`，本任务未修改后端；需另开后端测试修复任务。
- [x] `git diff --check` 与最终全范围审查通过；用户确认提交计划后生成工作提交
  `2728fcf fix(frontend): 修复危险按钮悬停文字不可见`。提交前再次运行专项测试，
  2 个文件、11 个测试全部通过。
