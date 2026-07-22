# 统一 Element Plus 页面组件与交互体验

## 目标

系统审计并改造全部 Vue 页面、共享组件、全局样式和相关组件测试，在不改变业务契约的前提下，把适合的交互控件迁移到当前已安装的 Element Plus 2.14.3，提升界面一致性、可访问性、状态反馈和维护性。

## 背景与证据

- 前端共有 20 个 Vue 文件：`src/App.vue`、5 个页面视图和 14 个共享/业务组件。
- 当前模板包含 46 个原生按钮、14 个原生输入、1 个原生进度条和 2 套自制对话框，仅使用了 2 类 Element Plus 组件、共 4 个组件标签（`src/components/ProviderEditor.vue:3`、`src/components/ProviderPreferenceControls.vue:3`）。
- 全局样式直接统一原生按钮和输入（`src/style.css:59`、`src/style.css:79`），但没有 Element Plus 主题变量或暗色模式桥接；已使用的 `ElSelect`、`ElSegmented` 因此可能与项目明暗主题脱节。
- 对话框各自维护焦点保存、Tab 陷阱和 Escape 处理（`src/components/ConfirmDialog.vue:22`、`src/components/ProxyDiscoveryDialog.vue:18`），重复且容易产生行为偏差。
- 卡片、状态、通知和页面标题样式在 Provider、备份、设置、自检、更新、关于等区域重复实现。
- 改造前基线为：`npm run typecheck` 通过；`npm run test` 24 个文件、113 个测试全部通过；`npm run build:frontend` 通过，产物 gzip 为 CSS 9.52 kB、JS 100.44 kB。

## 产品决定

- 采用“选择性全面迁移”：凡 Element Plus 能明显改善交互一致性、焦点/键盘行为、状态表达或复用性的控件均迁移；纯语义、纯布局或迁移收益不足的元素保留原生实现。
- 保留 Codex Relay 当前颜色、信息密度和桌面工具气质，Element Plus 作为交互基础设施，不把产品整体改成未经定制的默认后台模板。
- 不引入第二套 UI 库，不新增图标依赖，不进行后端、配置格式或 Tauri IPC 变更。
- 用户已明确要求“出计划后再干活”，规划门禁通过后可直接实施。

## 需求

### R1 主题与全局配置

- 使用 `ElConfigProvider` 建立中文、统一尺寸和浮层层级配置。
- 在 `src/style.css` 中把项目颜色、边框、圆角、背景和遮罩变量映射到 Element Plus 变量，并覆盖明暗主题。
- 所有交互目标继续满足约 44px 可点击高度，焦点清晰，窄窗口不溢出。

### R2 共享交互基础组件

- `AppNotification` 内部迁移到 `ElAlert`，保留原 props 和成功/错误语义。
- `ConfirmDialog` 内部迁移到 `ElDialog` + `ElButton`，保留危险/中性确认、Escape、焦点恢复和现有事件契约。
- `ProviderStatus` 迁移到 `ElTag`，状态文本不能只靠颜色表达。
- 更新提示和自检错误提示使用一致的 Element Plus 按钮与反馈视觉，不丢失现有 `role`、`aria-label` 和动作。

### R3 表单与选择控件

- Provider 编辑器使用 Element Plus 表单、输入、选择、复选和按钮组件；保持现有校验文本、首个错误聚焦、Provider ID 归一化和密钥三态语义。
- API Key 输入使用 Element Plus 密码输入能力，但保留显式清空确认和密钥不进入普通状态/日志的边界。
- 设置页和代理面板使用适合布尔设置的 `ElSwitch`/`ElCheckbox`、`ElInput` 和 `ElButton`；保留 Windows 实际状态、不一致提示和保存时机。
- 代理检测结果使用 `ElDialog`、`ElRadioGroup`/`ElRadio` 和明确的空状态。

### R4 页面级体验

- Provider、备份、自检、设置、更新、首次引导和关于页面按需使用 `ElCard`、`ElDescriptions`、`ElEmpty`、`ElProgress` 等组件统一表面、详情、空状态和进度。
- 主导航继续保留原生 `nav` 语义，可使用 `ElButton` 统一动作表现，但不强行改成不适合窄窗口的 `ElMenu`。
- `main`、`nav`、`section`、`header`、`dl`、`ul/ol` 等有意义的文档结构保留原生语义。

### R5 行为兼容与验证

- Provider CRUD/切换/偏好、备份恢复、设置、自启动、代理检测、更新、自检定位和首次引导的业务结果不变。
- 组件 props/emits 和 composable 状态边界保持显式类型化；视图不直接调用 Tauri `invoke`。
- 测试通过用户可见行为和公共组件边界验证，不依赖 Element Plus 私有 DOM。
- 按需导入必须继续生效，并记录改造前后前端构建体积差异。

## 验收标准

- [ ] 20 个 Vue 文件及其相关测试均已审计，每个交互元素都有迁移或保留结论。
- [ ] Element Plus 主题变量在浅色、深色和窄窗口下与项目视觉变量一致。
- [ ] 共享通知、确认对话框、状态标签不再重复实现组件库已有的基础交互能力。
- [ ] 46 个原生按钮、14 个原生输入、2 套自制对话框和 1 个原生进度条均被处理；只保留有明确语义或兼容理由的原生交互元素。
- [ ] 所有危险操作仍需确认；对话框支持 Escape、合理初始焦点和关闭后焦点恢复。
- [ ] Provider 表单校验、API Key 显示/隐藏与清空、设置保存、代理应用、更新下载进度等现有行为保持不变。
- [ ] 当前 Provider、禁用原因、警告/错误/成功状态不只靠颜色表达。
- [ ] 专项 Vitest、`npm run typecheck`、`npm run test`、`npm run check:frontend` 和 `npm run build:frontend` 通过，并附本轮命令证据。
- [ ] 使用成对安全覆盖的 `npm run dev:safe` 进行浅色/深色、900×620 和窄窗口人工检查；若环境限制无法完成，必须如实记录。
- [ ] 未读取、写入或删除真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`，未泄漏真实密钥。

## 范围外

- 后端命令、配置事务、配置文件格式、更新协议和数据迁移。
- 把纯排版内容机械替换成 Element Plus 容器，或为了“全量”而牺牲 HTML 语义。
- 引入 `@element-plus/icons-vue`、Pinia、路由器或新的设计系统依赖。
