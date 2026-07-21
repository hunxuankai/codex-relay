# 自动更新检查与页头提醒设计

## 设计目标

在不改变现有签名校验、安装确认和发布链路的前提下，把设置页内的手动 updater 状态提升为应用级单一状态源，使启动检查、小时检查、页头提醒和设置页安装共享同一个 updater session。

## 方案选择

采用由 `App.vue` 创建唯一 `useUpdater` 控制器，并通过显式 props 向下传递的方案。

- 不使用模块级单例：避免测试污染、隐式常驻 session 和生命周期不清晰。
- 不使用 `provide/inject`：当前层级较浅，显式 props/events 更易理解和测试。
- `App.vue` 只承担应用级编排；更新状态机、展示组件和设置视图仍保持独立职责。

## 组件与职责

| 单元 | 单一职责 | 公开契约 |
| --- | --- | --- |
| `useUpdater.ts` | 管理检查、session、安装和错误状态机 | 只读状态；显式手动检查、静默检查、安装动作和释放动作 |
| `App.vue` | 创建唯一 updater，读取最新代理，调度启动/小时检查并控制导航 | 向横幅和 `SettingsView` 传递共享控制器 |
| `UpdateAvailableBanner.vue` | 展示已发现版本及跳转操作 | `version` prop；`viewUpdate` emit |
| `SettingsView.vue` | 组合设置表单、代理工具与共享更新面板 | 接收 updater 控制器，不创建新 updater |
| `UpdatePanel.vue` | 展示并操作共享 updater 状态 | 接收 updater 控制器；手动检查调用显式动作 |

## 状态与数据流

```text
settingsState.settings.networkProxy
              │ 每次检查时读取
              ▼
App.vue ──唯一 useUpdater──► Tauri updater service
  │              │
  │              ├── release/session ──► UpdateAvailableBanner
  │              └── 全部状态/动作 ───► SettingsView ─► UpdatePanel
  │
  └── activeView：横幅点击后切换为 settings
```

`UpdateSession` 继续只保存在 composable 的 `shallowRef` 中。横幅只接收版本字符串，不接触插件不透明对象；Release notes 仍只在设置页按纯文本渲染。

## 运行时序

1. `App.vue` 创建 updater，并通过函数动态读取应用级设置中的当前有效代理。
2. 设置首次加载结束后执行一次静默启动检查；不等待扩展自检完成。
3. 首次设置引导页期间也检查。发现更新只保存状态，不替换或打断引导页。
4. 应用挂载时建立一小时周期调度。每轮读取最新代理；检查、下载或启动安装忙碌时跳过。
5. 已处于 `available` 或 `confirming` 时保留现有 release/session，不重复访问更新源。
6. 无更新时可保留 `upToDate` 状态，但页头不展示；自动检查失败时清除自动错误并回到不打扰用户的状态。
7. 用户点击更新横幅后切换到设置页，`UpdatePanel` 直接展示现有 release 并允许继续下载与安装。
8. 设置页手动检查仍展示 checking、upToDate、available 或 error，并继续使用最新代理。
9. 应用作用域销毁时清除定时器并关闭 updater session。

## 横幅布局与可访问性

- 更新横幅位于页头正下方，自检错误横幅位于更新横幅下方；两者同时存在时均展示。
- 横幅使用 `role="status"` 和稳定可访问名称“软件更新提示”，文案包含新版本号。
- 操作按钮文案为“前往更新”，触发显式事件，由 `App.vue` 完成页面切换。
- 窄窗口下横幅纵向排列；详细 Release notes 不放入横幅。
- 自检错误仍保持 `role="alert"`，更新提示不是错误，不抢占错误语义。

## 错误与资源处理

- 静默自动检查失败不进入页头、`AppNotification`、状态栏或设置页错误展示，也不转换成 `upToDate`。
- 手动检查失败继续使用现有稳定安全错误文案。
- 静默失败后必须允许下一小时检查或用户手动检查重新执行。
- 新检查替换旧结果前关闭旧 session；过期响应到达后关闭其 session，不覆盖较新状态。
- 下载或启动安装期间不执行周期检查。

## 兼容性与文档

- 不新增设置字段，无配置迁移和受管文件写入。
- 不改变 Tauri updater endpoint、公钥、签名校验、安装确认、下载进度或 NSIS 行为。
- README 与 `.trellis/spec/project/product-contract.md` 必须删除“不提供启动检查、后台轮询”的旧契约，并说明自动网络访问、静默失败和仍需用户确认安装。
- 如 `.trellis/spec/release/updater.md` 仍限定为手动检查，应同步更新适用范围和客户端行为。

## 回滚考虑

本功能不含数据迁移。若需回滚，可移除应用级调度和横幅，并让 `UpdatePanel` 恢复自行创建 updater；不影响用户配置和已发布更新信任根。
