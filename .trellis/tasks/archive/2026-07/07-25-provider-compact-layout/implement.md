# Provider 页面紧凑布局实施计划

## 实施顺序

1. 记录当前工作区状态，搜索现有 Element Plus 尺寸覆盖和 Provider 布局契约，避免遗漏重复样式。
2. 调整应用级控件密度和共享间距基线，保持焦点、禁用态和主题变量。
3. 将 Provider 详情摘要改为宽屏横向布局，并为窄窗口保留单列回退。
4. 将 Base URL/API Key 控制区改为宽屏两列，压缩卡片留白和管理按钮，收紧 Segmented 宽度。
5. 更新或补充前端布局契约测试，覆盖 900×620 目标、窄窗口类名、焦点/标签和长选项保护。
6. 运行类型检查、相关 Vitest、完整前端检查；发现失败时先定位根因再修复。
7. 执行 Trellis 综合检查，判断是否需要把新的布局约定沉淀到 `.trellis/spec/`，再提交并收尾。

## 计划验证命令

- `npm run typecheck`
- `npx vitest run src/style.test.ts src/element-plus-layout.test.ts src/components/ProviderEndpointControls.test.ts src/components/ProviderCredentialControls.test.ts src/views/ProvidersView.test.ts`
- `npm run test`
- `npm run check:frontend`

## 风险文件与回滚点

- `src/App.vue`、`src/style.css`：全局尺寸改变可能影响其他页面；若回归，优先缩小覆盖范围而不是继续降低全局尺寸。
- `src/views/ProvidersView.vue`：摘要/双栏布局需在窄窗口回退；保留现有 DOM 语义和事件绑定。
- `src/components/ProviderEndpointControls.vue`、`ProviderCredentialControls.vue`：Segmented 样式改变不能影响选择事件；回滚只需恢复组件样式。
- 测试文件：只更新布局契约，不删除业务行为断言。

## 完成门禁

- 用户批准已获得（本轮“可以，做吧”）。
- 任务材料已完成并在 `task.py start` 前审阅。
- 代码修改后必须有新鲜命令输出作为类型检查和测试证据，不以推测代替成功声明。

## 实施进度

- 2026-07-25：已完成任务规划、规范加载和 Element Plus 2.14.3 类型声明核对；确认
  `ElConfigProvider` 与 `ElSegmented` 支持 `default` / `small` 尺寸。
- 2026-07-25：已新增紧凑密度、三列摘要、两列切换区、620px 窄窗口回退及 Segmented
  内容宽度契约测试。专项 Vitest 结果为 4 个测试文件中 6 项按预期失败，失败原因均为
  目标样式/props 尚未实现；其余 18 项通过。下一步进入最小实现。
- 2026-07-25：已完成最小实现。专项布局测试 4 个文件、24 项全部通过；Provider 详情、
  模型偏好与可用性相关回归 3 个文件、22 项全部通过；`npm run typecheck` 退出码为 0。
- 2026-07-25：使用仅含明确测试数据的本地浏览器样例进行布局观察。900×620 视口下，
  Provider 摘要为三列，Base URL 与 API Key 为两列并进入首屏，模型和推理强度可见，
  页面横向溢出为 false；600×620 下摘要和切换区回退为单列，页面横向溢出仍为 false。
  Windows 桌面自动化连接不可用，因此没有把真实 Tauri 窗口观察记为已完成。
- 2026-07-25：`npm run test` 通过 36 个测试文件、171 项测试；`npm run build:frontend`
  成功生成前端产物（Rollup 仅报告第三方 `@vueuse/core` PURE 注释位置警告）；随后
  `npm run check:frontend` 再次通过类型检查及 36 个文件、171 项测试。
- 2026-07-25：已同步 `.trellis/spec/frontend/vue-guidelines.md` 与 `accessibility.md`，记录
  普通控件 36px、显式紧凑控件 32px、900×620 首屏优先级和窄窗口无横向溢出契约。
- 2026-07-25：最终审查发现全局 720px 规则会把控制卡片中的小号“管理”按钮拉满宽度；
  先新增布局契约并确认 1 项按预期失败，再增加组件级 `width: auto` 覆盖，专项 3 个文件、
  14 项测试全部通过。
- 2026-07-25：最后一次新鲜验证中，`npm run check:frontend` 通过类型检查及 36 个测试文件、
  171 项测试，`npm run build:frontend` 成功（仍仅有第三方 PURE 注释位置警告）。
  `git diff --check`、Trellis 上下文校验、任务 JSON 校验和高置信度密钥扫描均通过；临时
  `compact-preview.html` 已删除且未被 Git 跟踪。
