# 技术设计

## 边界

改动限定在 Vue 前端的按钮样式契约、`ApiKeyInput` 组件和前端回归测试；不改变
Provider 业务状态、Tauri 调用、文件事务或密钥数据流。

## 根因与方案

Element Plus `ElButton` 的语义类型通过 CSS 自定义属性驱动默认、hover、active 和 disabled
颜色。`ApiKeyInput` 的 `.danger-link` 同时直接声明 `color: var(--danger)`，在 scoped 样式
加载顺序和选择器优先级相同的情况下覆盖了 hover 的白色文字。移除该静态文字颜色，让
`type="danger" plain` 完全负责语义颜色；保留类名仅用于 `width: fit-content` 等布局。

全局审计以所有 `ElButton` 模板和其关联样式为清单，重点查找按钮装饰类中的直接
`color`、`background`、`border-color` 以及 `:hover/:active/:disabled` 覆盖。若某处确需
自定义状态，使用明确的状态选择器或 Element Plus 变量，避免与组件默认状态竞争。

## 测试契约

- 在 `src/style.test.ts` 增加静态回归：读取 `ApiKeyInput.vue`，验证危险按钮装饰类不再
  直接写 `color`，并保留 `type="danger"`/`plain` 语义。
- 继续运行现有 `ApiKeyInput` 交互测试，确认清空确认和取消流程不变。
- 运行前端类型检查与 Vitest；最后按任务风险运行全量前端检查和 `git diff --check`。

## 兼容性与回滚

这是 CSS 层最小改动。若视觉回归，回滚组件样式和回归测试即可，不触及配置文件或用户
数据；暗色主题继续沿用 `--el-color-danger*` 映射。
