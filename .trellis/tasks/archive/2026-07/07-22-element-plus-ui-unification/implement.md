# Element Plus 页面组件统一实施计划

## 当前状态

- 阶段：根据用户截图完成首轮视觉缺陷修复，等待用户刷新安全开发版后复核最终窗口表现。
- 基线证据：
  - `npm run typecheck`：通过，2026-07-22 本轮执行。
  - `npm run test`：通过，24 个测试文件、113 个测试。
  - `npm run build:frontend`：通过；gzip CSS 9.52 kB、JS 100.44 kB。
- 已核对 Element Plus 官方文档与 2.14.3 本地类型声明，确认 `ElConfigProvider`、`ElDialog`、`ElFormItem.error`、`ElInput.showPassword`、`ElSwitch`、`ElAlert`、`ElProgress` 等 API。
- 实施结果：源码中已无原生 `<button>`、`<input>`、`<progress>` 或自制焦点陷阱；Element Plus 标签由 4 个增至 90 个，原生页面语义结构保留。
- 新鲜验证：第二轮 Provider 对齐修复后，`npm run check:frontend` 通过，28 个测试文件、126 个测试；`npm run build:frontend` 通过；`git diff --check` 通过。
- 构建对比：gzip CSS 9.52 → 17.14 kB（+7.62 kB），gzip JS 100.44 → 128.12 kB（+27.68 kB），仍为按需导入。
- 人工验证限制：`npm run dev:safe` 已使用成对安全覆盖启动，但 Windows 自动化原生管道不可用，未取得可信的窗口截图、明暗主题或窄窗口人工观察证据。

## 视觉缺陷复盘

- 用户截图证据：Provider 卡片名称与 ID 连在一起、详情与按钮区域间距失效、自检按钮 hover 出现白字浅底。
- 用户第二张截图证据：Provider ID 未保持原有右对齐；红色“查看自检详情”按钮需确认组件来源。
- 根因 1：旧的全局 `button/input/checkbox` 选择器命中了 Element Plus 内部节点；其中 `button:hover:not(:disabled)` 的优先级高于 `.el-button:hover`，覆盖背景但保留白色 hover 文字。
- 根因 2：迁移到 `ElCard` 后，原 grid/gap/padding 仍写在卡片根节点，未作用于实际承载插槽内容的 `.el-card__body`。
- 根因 3：`ElButton` 自动增加内容 `<span>`，Provider 名称和 ID 不再是按钮直接 flex 子项。
- 根因 4：虽然 `.provider-select-content` 已设置 `width: 100%`，但它的直属 Element Plus 插槽包装 `<span>` 仍按内容宽度收缩，导致内部两端对齐没有可分配空间。
- 修复：删除旧原生控件外观规则；补充 primary/danger 禁用态对比度；统一六类卡片 body 布局；为 Provider 选择按钮和主导航增加显式内容层布局；把剩余原生节点选择器改为 `.el-*` 边界。
- 第二轮修复：让 `.provider-select` 的直属 Element Plus 内容包装层占满按钮宽度，恢复 Provider ID 靠右；确认自检横幅使用 `ElAlert type="error"`，动作使用 `ElButton type="danger"`，仅外层保留语义与布局容器。
- 回归测试：新增 `src/element-plus-layout.test.ts`，扩展 `src/style.test.ts`，覆盖全局选择器隔离、卡片 body、复合按钮内容和禁用态对比度。

## 实施顺序

### 1. 主题与全局配置

1. 先在 `src/style.test.ts` 增加 Element Plus 主题映射、暗色和 44px 交互目标的失败断言。
2. 在 `src/App.vue` 增加 `ElConfigProvider` 与中文 locale。
3. 在 `src/style.css` 建立项目变量到 `--el-*` 的桥接，处理按钮间距、组件高度、焦点和窄窗口。
4. 运行 `npm run test -- src/style.test.ts src/App.test.ts` 与 `npm run typecheck`。

### 2. 共享反馈、状态和对话框

1. 更新 `AppNotification`、`ConfirmDialog`、`ProviderStatus` 及横幅测试，先验证新组件契约缺失而失败。
2. 迁移到 `ElAlert`、`ElDialog`、`ElButton`、`ElTag`，保持现有 props/emits 和 ARIA。
3. 验证焦点恢复、Escape、默认取消焦点、危险/中性按钮、状态文字。
4. 运行对应组件测试和 `src/App.test.ts`。

### 3. Provider 行为切片

1. `ProviderEditor`：迁移表单、输入、复选和按钮；保留业务验证、首错聚焦、ID 归一化、API Key 三态。
2. `ApiKeyInput`：迁移密码显示/隐藏与按钮，保留清空确认。
3. `ProviderList`、`ProviderStatus`、`ProviderPreferenceControls`、`ProvidersView`：统一卡片、空状态、详情和动作。
4. 逐项运行：
   - `npm run test -- src/components/ApiKeyInput.test.ts src/components/ProviderEditor.test.ts`
   - `npm run test -- src/components/ProviderList.test.ts src/components/ProviderPreferenceControls.test.ts src/views/ProvidersView.test.ts`

### 4. 设置与代理行为切片

1. `SettingsView` 和 `ProxySettingsPanel` 迁移开关、输入、按钮和设置卡片。
2. `ProxyDiscoveryDialog` 迁移到 Element Plus Dialog/Radio/Empty，删除重复焦点陷阱。
3. 保持自启动立即动作、其他设置显式保存、代理检测/应用时机不变。
4. 运行：
   - `npm run test -- src/components/ProxySettingsPanel.test.ts src/components/ProxyDiscoveryDialog.test.ts src/views/SettingsView.test.ts`

### 5. 备份、自检、更新、引导、关于和应用壳

1. `BackupCard`/`BackupsView`：卡片、动作、加载和空状态。
2. `HealthStatus`：状态标签、检查卡片和重跑动作；保持错误定位聚焦。
3. `UpdatePanel`：卡片、按钮、进度和错误反馈；保持状态机不变。
4. `OnboardingView`、`AboutView`、`App.vue`：卡片、详情、导航与状态栏动作统一。
5. 运行所有相关视图/组件测试与 `src/App.test.ts`。

### 6. 全范围检查与人工体验验证

1. 搜索剩余 `<button>`、`<input>`、`<progress>` 和自制 dialog；逐个记录保留理由或完成迁移。
2. 执行：
   - `npm run typecheck`
   - `npm run test`
   - `npm run check:frontend`
   - `npm run build:frontend`
   - `git diff --check`
3. 对比构建 gzip 体积与基线，确认仍为按需导入。
4. 使用 `npm run dev:safe` 的成对安全覆盖进行 900×620、窄窗口、浅色和深色人工检查；确认导航、表单、对话框、滚动、状态和焦点。
5. 运行 Trellis check，更新需要长期保留的前端规范，再提交和收尾。

## 风险文件与回滚点

- `src/style.css`：主题桥接影响全局；单独提交前保持可逆，发现暗色或尺寸退化时优先回滚变量覆盖。
- `src/components/ConfirmDialog.vue`：所有危险操作共用；必须先通过焦点和事件专项测试再迁移调用方。
- `src/components/ProviderEditor.vue`：密钥与同步语义风险最高；不同时重写业务验证逻辑。
- `src/App.vue`：根包裹和导航影响全部页面；放在主题切片并用 App 测试保护。
- `src/views/SettingsView.vue`：`ElSwitch` 的即时更新与表单保存语义不同，必须保持自启动和普通设置的原行为边界。

## 暂停/恢复说明

- 下一步：在 Windows 自动化连接可用时补做安全开发版的 900×620、窄窗口、浅色和深色人工检查；如发现视觉问题，回到对应行为切片修复并重跑检查。
- 不使用子 Agent；Codex inline 模式由主会话直接实施和检查。
