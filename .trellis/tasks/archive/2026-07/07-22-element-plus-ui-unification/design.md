# Element Plus 页面组件统一设计

## 结论

Element Plus 会改善整体体验，但前提是“选择性全面迁移”，不是把所有 HTML 标签机械替换成组件。真正高收益的部分是交互控件、表单校验、对话框、反馈、状态、空状态、进度和重复卡片表面；`main`、`nav`、标题、说明、列表和定义列表等语义结构应继续使用原生 HTML。

## 方案比较

### 方案 A：最小补丁

只补主题变量，并迁移两个对话框和通知。风险最低，但 46 个按钮、14 个输入和多套卡片/状态样式仍然割裂，不能实现用户要求的全局体验统一。

### 方案 B：选择性全面迁移（采用）

迁移所有有明确交互收益的控件，保留语义和布局结构；通过现有共享组件封装 Element Plus，逐页面落地。收益、风险和可回滚性最平衡。

### 方案 C：Element Plus 后台模板化重写

连导航、布局和内容结构都改成 `ElContainer`、`ElMenu`、`ElTabs` 等。虽然组件覆盖率最高，但会增加包体、破坏窄窗口布局和现有语义，并让桌面工具变成通用后台模板，不采用。

## 架构边界

```text
App.vue / views（页面编排）
  ├─ 共享交互组件（AppNotification、ConfirmDialog、ProviderStatus）
  ├─ 业务组件（ProviderEditor、ProviderList、UpdatePanel 等）
  └─ composables（业务状态与显式动作，不变）
       └─ typed Tauri service（不变）

Element Plus 只进入视图/组件层；不进入 composable、service、DTO 或 Rust。
```

## 全局主题设计

- `App.vue` 根部增加 `ElConfigProvider`，配置简体中文、统一尺寸与 z-index。
- 继续由 `src/style.css` 的项目变量作为唯一视觉事实来源，并映射：
  - `--el-color-primary/success/warning/danger`
  - `--el-bg-color`、`--el-bg-color-page`、`--el-fill-color-*`
  - `--el-text-color-*`、`--el-border-color-*`
  - `--el-border-radius-*`、`--el-mask-color`
- 暗色主题仍由 `prefers-color-scheme` 驱动，Element Plus 变量引用同一组项目变量，不额外引入第二套主题状态。
- 不全量 `app.use(ElementPlus)`；继续显式导入组件，由 `unplugin-element-plus` 注入样式，保持 tree-shaking。

## 组件迁移矩阵

| 文件 | 责任 | 迁移决定 |
|---|---|---|
| `src/App.vue` | 应用壳、导航、状态栏、视图编排 | 增加 `ElConfigProvider`；导航和状态栏动作改用 `ElButton`，保留 `nav`/`footer` 语义 |
| `src/views/ProvidersView.vue` | Provider 页面编排 | 详情动作使用 `ElButton`；详情字段采用 `ElDescriptions`；警告保留语义并使用统一反馈样式 |
| `src/views/BackupsView.vue` | 备份列表编排 | 刷新使用 `ElButton`；加载/空列表使用 `ElSkeleton` 或 `ElEmpty`；恢复仍走共享确认框 |
| `src/views/SettingsView.vue` | 设置草稿和保存编排 | 刷新/保存/打开目录使用 `ElButton`；布尔项使用 `ElSwitch`/`ElCheckbox`；设置区采用 `ElCard` |
| `src/views/OnboardingView.vue` | 首次引导 | 主表面使用 `ElCard`，动作使用有层级的 `ElButton`，危险/次要动作区分明确 |
| `src/views/AboutView.vue` | 产品说明 | 当前信息使用 `ElDescriptions`，信息区采用 `ElCard`；正文列表和代码说明保持原生语义 |
| `src/components/AppNotification.vue` | 成功/错误反馈 | 内部改为不可关闭的 `ElAlert`，保留动态 `role` 和原 props |
| `src/components/ConfirmDialog.vue` | 危险/中性确认 | 内部改为 `ElDialog` + `ElButton`，删除自制遮罩和 Tab 陷阱，保留公共 API |
| `src/components/ProviderStatus.vue` | Provider 状态 | 使用 `ElTag` 的 success/warning/danger/info 类型，继续显示文字 |
| `src/components/SelfCheckErrorBanner.vue` | 全局错误横幅 | 使用 `ElAlert` 外观与 `ElButton` 动作，保留 `role=alert` |
| `src/components/UpdateAvailableBanner.vue` | 全局更新横幅 | 使用 `ElAlert` 外观与 `ElButton` 动作，保留 `role=status` |
| `src/components/ApiKeyInput.vue` | 密钥局部状态 | 使用 `ElInput type=password show-password` 和 `ElButton`；显式清空确认保留 |
| `src/components/BackupCard.vue` | 单个备份展示 | 使用 `ElCard`、`ElDescriptions`/紧凑元数据和 `ElButton`；文件列表语义保留 |
| `src/components/HealthStatus.vue` | 自检摘要和检查项 | 摘要/级别使用 `ElTag`，检查项使用 `ElCard`，重跑使用 `ElButton`；目标项聚焦逻辑不变 |
| `src/components/ProviderEditor.vue` | Provider 表单和验证 | 使用 `ElForm`、`ElFormItem`、`ElInput`、现有 `ElSelect`、`ElCheckbox`、`ElButton`；保留业务校验和首错聚焦 |
| `src/components/ProviderList.vue` | Provider 列表 | 使用 `ElCard`、`ElButton`、`ElEmpty`；选中状态和所有禁用条件不变 |
| `src/components/ProviderPreferenceControls.vue` | 模型偏好 | 保留两行 `ElSegmented`；缺失态按钮改为 `ElButton`，补主题与窄宽处理 |
| `src/components/ProxyDiscoveryDialog.vue` | 代理候选选择 | 使用 `ElDialog`、`ElRadioGroup`/`ElRadio`、`ElEmpty`、`ElButton`；删除重复焦点陷阱 |
| `src/components/ProxySettingsPanel.vue` | 代理设置字段 | 使用 `ElSwitch`、`ElInput`、`ElButton`，保留测试/检测动作切换 |
| `src/components/UpdatePanel.vue` | 更新状态机展示 | 使用 `ElCard`、`ElButton`、`ElProgress`、`ElAlert`，更新状态机不变 |

## 组件边界与数据流

- 所有 route/view 级组件继续只负责编排；不把 composable 状态复制进新的 UI store。
- 现有 props/emits 尽量保持不变，尤其是 `ConfirmDialog`、`AppNotification`、`ProviderStatus`，从而降低调用方迁移风险。
- Element Plus 的 `v-model` 只用于真实双向输入；Provider/设置最终提交仍调用现有显式动作。
- 对 `ElForm` 不重复实现第二套业务规则：现有验证函数继续是权威，`ElFormItem.error` 只负责展示；这样避免与 Rust 校验或现有错误文本漂移。

## 可访问性设计

- Element Plus FormItem 在单一输入时使用标签关联；组合字段继续提供显式 `aria-label`/`aria-describedby`。
- 危险确认默认聚焦取消按钮；Escape 取消；关闭后焦点返回触发元素。
- `ElSwitch` 必须同时有可见文字与 `aria-label`，不能只显示颜色。
- 状态标签继续包含“当前”“未配置密钥”“配置无效”等文字。
- 测试只依赖公开标签、可见文本、props/emits 和原生可访问节点，不锁定 `.el-*` 私有结构。

## 兼容性与风险

- Element Plus 的相邻按钮默认 margin、组件高度和浮层 Teleport 会影响现有 CSS/test，需要统一覆盖并让对话框测试 `attachTo: document.body`。
- `ElInput`、`ElSwitch`、`ElCheckbox` 的原生 input 位于组件内部，测试选择器需转为可访问标签或组件 API。
- `ElDialog` 默认允许点击遮罩关闭；危险确认必须显式关闭该行为，避免误操作。
- 扩大组件使用会增加前端包体；以改造前 gzip CSS 9.52 kB、JS 100.44 kB 为基线记录差异，不改成全量导入。

## 验证与回滚

- 每个行为切片先更新测试并观察预期失败，再完成最小迁移。
- 共享基础组件先迁移，页面按 Provider → 设置/代理 → 备份/自检/更新 → 引导/关于/App 壳顺序推进。
- 每个切片保留现有组件公共 API，若 Element Plus 行为无法满足焦点或语义要求，可在单个共享组件内部回退，不影响 composable 和后端。
- 最终使用安全的 `npm run dev:safe`，不得启动连接真实用户目录的开发实例。
