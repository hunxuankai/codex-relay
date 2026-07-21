# 自动更新检查与页头提醒实施计划

## 当前状态

- 阶段：实施、规范同步和全量质量门禁完成；待 Phase 3.4 补充提交并归档。
- 实施模式：Codex inline；不创建重复的子代理、TDD 或分支收尾流程。

## TDD 规划门禁

### 行为切片 1：更新状态机支持静默检查

- 可观察行为：静默检查发现更新时暴露 release；无更新时不产生提示；失败时不暴露错误且可再次检查；忙碌或已有可安装更新时跳过。
- 被测公开接口：`useUpdater` 返回的只读 `status`、`release`、`error`，以及新增的静默检查动作、现有显式检查动作和 `reset`。
- Mock 边界：注入 fake `UpdateClient` 和 fake `UpdateSession`；不调用真实 Tauri updater 或网络。
- RED：先扩展 `src/composables/useUpdater.test.ts`，证明现有接口无法表达静默失败和保留 session。
- GREEN：以最小状态机改动增加静默动作/模式，并保持旧响应关闭、单飞和安装行为。

### 行为切片 2：应用级启动与小时调度

- 可观察行为：设置首次加载结束后只检查一次；每小时检查一次；代理变化后读取新值；卸载清理；忙碌时不并发。
- 被测公开接口：`App.vue` 的挂载行为、共享 updater 调用和卸载行为。
- Mock 边界：mock `useUpdater`、`useSettings` 和其他既有 composable；使用 Vitest fake timers；不启动真实应用或访问用户目录。
- RED：扩展 `src/App.test.ts` 覆盖启动、引导、周期、代理变化和卸载。
- GREEN：在 `App.vue` 中创建唯一 updater，等待设置加载完成后静默检查并管理定时器。

### 行为切片 3：页头更新提醒

- 可观察行为：有 release 时显示版本和“前往更新”；无 release 时不显示；点击发出事件；窄屏样式存在。
- 被测公开接口：新组件 props、emit、可访问名称和 DOM 文案。
- Mock 边界：纯 Vue 组件测试，无 Tauri mock。
- RED：新增 `src/components/UpdateAvailableBanner.test.ts`。
- GREEN：新增专注展示的 `UpdateAvailableBanner.vue`。

### 行为切片 4：横幅顺序与导航

- 可观察行为：引导期间不展示更新横幅；进入主界面后展示；更新与自检横幅同时存在且顺序正确；点击更新切换设置页。
- 被测公开接口：`App.vue` 可见 DOM 和导航按钮的 `aria-current`。
- Mock 边界：共享 updater controller stub、现有视图 stub 和健康状态 stub。
- RED：扩展 `src/App.test.ts`。
- GREEN：组合新横幅并调整应用网格行布局。

### 行为切片 5：设置页复用共享 updater

- 可观察行为：自动检查已发现更新后进入设置页直接显示版本；不会再次检查；仍可请求确认和安装；手动检查错误仍可见。
- 被测公开接口：`SettingsView`/`UpdatePanel` props 和用户点击行为。
- Mock 边界：传入 updater controller stub；不 mock组件内部新建的 composable，因为组件不再创建它。
- RED：更新 `src/components/UpdatePanel.test.ts` 与 `src/views/SettingsView.test.ts`。
- GREEN：让 `SettingsView` 和 `UpdatePanel` 接收共享控制器，移除局部 `useUpdater` 实例。

### 行为切片 6：文档契约

- 可观察行为：README 和项目/发布规范准确描述启动检查、小时检查、静默失败和用户确认安装。
- 被测公开接口：文档文本与既有文档检查测试（如适用）。
- Mock 边界：无。
- RED/GREEN：先定位会因旧描述失败或需要扩展的文档测试，再更新文档与断言。

## 有序实施清单

1. Phase 2 开始前运行 `trellis-before-dev`，按索引读取前端状态、Vue、可访问性、测试隔离和 updater 发布规范。
2. 完成行为切片 1：先测试，后扩展 `useUpdater` 静默检查契约。
3. 完成行为切片 2：先测试，后在 `App.vue` 添加应用级调度和最新代理读取。
4. 完成行为切片 3：先测试，后新增更新横幅组件及响应式样式。
5. 完成行为切片 4：先测试，后组合横幅顺序、引导可见性和设置页导航。
6. 完成行为切片 5：先测试，后重构 `SettingsView`/`UpdatePanel` 共享 updater。
7. 完成行为切片 6：更新 README、产品契约和必要的 updater 规范。
8. 运行定向测试、完整前端测试、类型检查、lint/build 及 Trellis check；保留真实失败和限制。
9. 根据实施中形成的稳定契约判断并执行规范更新，然后请求提交/收尾。

## 预计风险文件与回滚点

| 文件 | 风险 | 回滚点 |
| --- | --- | --- |
| `src/composables/useUpdater.ts` | 静默失败可能污染手动错误语义或泄漏 session | 保持显式/静默动作边界，逐个状态机测试 |
| `src/App.vue` | 启动竞态、重复定时器、引导状态和网格顺序 | 以 fake timers 和卸载测试锁定生命周期 |
| `src/views/SettingsView.vue` | props 透传后影响现有设置测试 | 保持设置表单逻辑不变，只替换 updater 来源 |
| `src/components/UpdatePanel.vue` | 共享控制器类型过宽或破坏安装确认 | 导出最小明确 controller 类型，保留现有 UI 状态测试 |
| `src/components/UpdateAvailableBanner.vue` | 可访问语义或窄屏布局回归 | 独立组件测试与样式契约检查 |
| `README.md` / `.trellis/spec/` | 新旧产品契约矛盾 | 全局搜索“启动检查/后台轮询/手动检查”并逐项核对 |

## 验证命令

实施时根据 `package.json` 实际脚本校准，至少执行：

```powershell
npx vitest run src/composables/useUpdater.test.ts
npx vitest run src/components/UpdateAvailableBanner.test.ts src/components/UpdatePanel.test.ts src/views/SettingsView.test.ts src/App.test.ts
npm test
npm run typecheck
npm run build:frontend
npm run check
python .\.trellis\scripts\task.py validate .trellis\tasks\07-22-auto-update-check-banner
```

若仓库不存在某脚本，必须记录真实结果并改用项目已有等价命令，不能声称未执行的检查通过。

## 安全边界

- 所有测试只注入 fake updater client/session，不访问真实 GitHub 更新源。
- 不启动未设置安全路径覆盖的 Tauri 开发环境。
- 不读取或写入真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`。
- 不修改或输出真实密钥、Authorization Header、认证文件或更新签名私钥。

## `task.py start` 前检查

- [x] PRD 具有可测试验收标准并完成收敛检查。
- [x] 技术设计记录架构、数据流、兼容性、错误和回滚。
- [x] 每个行为切片记录公开接口与 mock 边界。
- [x] Inline 模式，无 JSONL 门禁。
- [x] 用户审查并批准最终规划材料。
- [x] 已运行 `trellis-before-dev` 获取 Phase 2 细则。

## 实施结果

- `useUpdater` 新增静默检查动作，自动失败回到 `idle` 且不暴露错误；已有可安装更新时周期检查保留原 session。
- `App.vue` 持有唯一 updater，在设置首次加载结束后检查一次，并每小时调度；每次读取最新应用代理，卸载时清理定时器。
- 新增 `UpdateAvailableBanner`，位于页头下方并支持一键进入设置页；与自检错误横幅可同时展示。
- `SettingsView` 和 `UpdatePanel` 改为接收共享 updater，自动发现的更新无需再次检查即可继续安装。
- README、关于页、产品契约和 updater 发布规范已同步新的网络访问与安装边界。

## 验证记录

- TDD RED：`useUpdater` 测试先因 `checkSilently is not a function` 失败；更新横幅测试先因组件不存在失败；共享面板和应用编排测试先按预期失败。
- 核心定向复验：`npx vitest run src/composables/useUpdater.test.ts src/components/UpdateAvailableBanner.test.ts src/components/UpdatePanel.test.ts src/views/SettingsView.test.ts src/App.test.ts`，5 个测试文件、19 个测试全部通过。
- 全量质量门禁：`npm run check` 通过；Trellis 8 个测试、前端 23 个测试文件/110 个测试、Rust 118 个单元测试及 2 个路径安全测试、1 个 Provider 工作流测试均通过，类型检查、Rust fmt 和 Clippy 同步通过。
- 前端生产构建：`npm run build:frontend` 成功，85 个模块完成转换。
- 规范同步复核：补齐 `.trellis/spec/project/architecture.md` 的启动/小时自动检查、应用级唯一 updater、动态代理和共享 session 契约；全局搜索不再命中“更新网络请求只能由设置页显式触发”等旧描述。
- 历史阻塞：首次全量检查曾被尚未完成的 `backup-file-list-open` 工作区改动阻塞；该任务提交归档后，本轮重新执行完整门禁并通过，未用重试覆盖或删除原失败事实。
