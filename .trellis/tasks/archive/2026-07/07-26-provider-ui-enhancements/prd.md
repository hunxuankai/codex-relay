# 完善版本标题、Provider排序与API测试详情交互

## 目标

让用户在应用主界面立即识别当前版本，能够按自己的工作习惯排列 Provider，并在发起
API 可用性测试时马上看到请求/响应详情上下文和明确的进行中状态。

排序属于 Relay 的界面偏好：拖动后的顺序跨刷新和重启保留，但不得写入 Codex 官方
`config.toml` 的私有字段，也不得改变当前 Provider 的生效语义。

## 已确认事实

- `App.vue` 已通过 `getCurrentVersion()` 获取当前版本，并将版本传给“关于”页，但左上角
  主标题没有显示它。
- `ProviderAvailabilityTraceDialog` 当前只在 API 结果带 trace 后挂载，遮罩点击被禁用；
  `ProviderAvailabilityPanel` 在测试完成后才有可展示的请求/响应数据。
- Provider 列表顺序目前直接沿用后端 `config.toml` 的 `model_providers` 表顺序，前端没有
  拖放状态或排序 command。
- `provider-preferences.json` 已由 `TransactionService` 统一保护，适合保存不属于 Codex
  官方配置的 Relay 私有排序元数据；所有新写入必须继续经过指纹、备份、原子替换、写后
  验证和回滚流程。

## 需求

### VERSION-1：主标题显示版本

- 应用主界面左上角的品牌/标题区域显示当前运行版本，格式为 `Codex Relay v<version>`。
- 版本来源继续使用现有 `getCurrentVersion()` typed service；加载失败时保留无版本的安全
  标题，不阻塞 Provider 管理。
- “关于”页现有版本显示和启动加载行为保持不变。

### ORDER-1：Provider 拖动排序

- 左侧 Provider 列表中的每个 Provider 都提供可识别的拖动手柄，用户可通过鼠标/触控板将
  Provider 拖到其他位置，放开后列表立即按新顺序显示。
- 拖动期间不触发选择、编辑、使用或删除；Provider 正在加载、保存、切换、删除或排序时，
  拖动入口禁用，重复提交被阻止。
- 排序成功后顺序通过 Relay 私有偏好持久化，刷新、文件监控事件和应用重启后保持；未曾
  排序的 Provider 继续按现有配置顺序展示，新建 Provider 追加到末尾，删除 Provider 同步
  移除其排序记录。
- 排序命令必须提交当前完整 Provider ID 排列和 `expectedFiles`，只修改
  `provider-preferences.json`；无效、重复、缺失或未知 ID 返回稳定安全错误且不写任何文件。
- 排序失败时恢复拖动前的权威列表和安全错误消息，不改变选中 Provider、活动 Provider 或
  Codex 官方配置。
- 排序数据不包含密钥，不进入日志、通知或普通 DTO 之外的秘密边界；旧版缺少排序字段时
  只读兼容并将未记录 Provider 追加在配置顺序末尾。

### TRACE-1：API 测试详情弹窗生命周期

- 点击每个 Provider 详情中的“测试 API 可用性”后，详情弹窗立即打开，即使请求尚未返回。
- 测试进行中，弹窗的“请求”和“响应”区域分别显示可访问的 loading 状态；不得伪造请求 URL、
  请求正文、响应状态或响应正文。
- API 结果带 trace 后，弹窗自动更新为本次测试的请求、响应和耗时；旧测试的 trace 不得短暂
  或最终显示。结果不带 trace、请求前置失败或取消时，不展示虚假的 trace 内容。
- 弹窗支持点击遮罩、按 Escape、页脚“关闭”按钮关闭；关闭不取消测试、不发起额外请求。
- 测试完成后，结果卡片上的“查看请求与响应”按钮可以再次打开同一结果详情；测试运行中若
  用户关闭弹窗，也可以通过同一入口重新打开当前 loading 详情（若入口仍可见）。
- Provider 切换、指纹失效、组件卸载或结果作废时，弹窗不会残留旧详情；Codex 测试结果和
  现有取消/并发门禁保持不变。

## 验收标准

- [x] 主界面左上角显示真实当前版本；`getCurrentVersion()` 失败时标题仍可用，关于页版本
      行不回归。
- [x] 拖动两个以上 Provider 后，列表顺序立即改变并通过排序 command 保存；刷新和重新挂载
      按保存顺序展示，新增追加、删除移除，排序错误不留下部分写入。
- [x] 排序写入只触及 Relay 私有偏好，保留未知 Provider 偏好、其他配置字节和安全边界；
      Rust 临时目录测试覆盖旧字段兼容、无效排列、指纹冲突和事务回滚。
- [x] 点击 API 测试按钮后同一事件循环内弹窗可见，且请求/响应区域均有 loading 文本或
      `aria-busy` 状态；返回 trace 后显示本次 URL/body、HTTP 状态/body 和耗时。
- [x] 遮罩点击、Escape、关闭按钮和“查看请求与响应”重复打开均有效；关闭期间测试仍继续，
      不产生额外 IPC。
- [x] 无 trace、取消、切换 Provider、指纹失效、晚响应和组件卸载路径不显示旧请求/响应；
      API/Codex 结果独立保存，现有专项测试与可访问性行为不回归。
- [x] 本轮相关前端专项、Rust Provider/事务专项、类型检查、`npm run check`（或如实记录的
      等价拆分门禁）和 `git diff --check` 均有新鲜命令证据。

> 验证边界：本轮完成了自动化组件、跨层、事务、路径安全、秘密扫描和整合门禁；未进行真实
> Provider 网络请求、人工桌面 UI 观察或安装/升级/卸载/签名流程验证。

## 范围外

- 不修改 Provider 网络请求格式、trace 脱敏/上限、API Key 存储或 Codex 测试协议。
- 不提供自动轮询、负载均衡、批量导入或拖动 Base URL/API Key 条目排序。
- 不把排序顺序写入 Codex 官方 `config.toml`，不改变活动 Provider、模型、认证或托盘切换语义。

## 开放决定

- 排序按持久化顺序实现；如果产品后来明确只需要当前窗口临时排序，可在不改变 API 弹窗
  契约的情况下移除排序 command 和私有字段写入。
