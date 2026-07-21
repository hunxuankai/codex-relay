# 无服务器应用内更新技术设计

## 状态

所有设计章节均已由用户逐节确认，任务已获得实施授权并处于 `in_progress`。

## 决策摘要

- 使用公开 GitHub 仓库同时托管源码与 Releases，不建设常驻更新服务。
- MVP 只允许用户在设置页手动检查一条稳定通道。
- 使用官方 Tauri updater 下载、校验并启动 NSIS 被动更新。
- 保留 Tauri 更新包签名；不使用 Windows Authenticode。
- 不做启动检查、后台轮询、强制更新、静默更新、自动回滚、断点续传、多通道或私有源认证。

## 运行时架构

```text
SettingsView
  -> UpdatePanel
  -> useUpdater
  -> src/services/tauri.ts 中的 updater 适配层
  -> @tauri-apps/plugin-updater
  -> 固定 GitHub Releases latest.json
  -> 公开 NSIS updater 资产
  -> Tauri 公钥校验
  -> NSIS /P /R /UPDATE
  -> 原安装目录升级并重启
```

前端不解析任意远端下载地址，也不允许用户配置更新源。应用级代码只消费适配层导出的带类型状态；官方插件内部负责 Tauri IPC、下载、签名校验和 Windows 安装器启动。Rust 侧只注册 updater 插件，不另写 HTTP 下载器或安装器执行逻辑。

现有 `src-tauri/installer/custom-installer.nsi` 已保留 `/P`、`/UPDATE`、更新后重启和已有安装目录恢复逻辑；官方 updater 在 Windows NSIS 路径传入 `/P /R /UPDATE`。实际兼容性仍须通过已安装旧版本到新版本的真实升级验证，不能仅凭模板检查宣称成功。

## 用户触发的数据流

1. 用户在设置页点击“检查更新”。
2. 适配层请求固定的 `https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json`。仓库地址是编译期发布配置，不来自运行时输入。
3. 没有更新时返回当前版本与“已是最新版本”状态；发现更新时返回版本、发布日期和发布说明。
4. 用户显式点击“下载并安装”，界面锁定重复操作并展示下载进度。
5. updater 下载 Windows updater 资产并使用内置公钥验证清单中的签名。
6. 校验通过后，界面明确提示应用将退出；updater 清理 Tauri 运行时、启动 NSIS 并退出当前进程。
7. NSIS 请求当前 per-machine 安装所需权限，沿用注册表记录的安装目录，更新程序文件并重启应用。

升级不得读取、移动或删除 `.codex`、`%LOCALAPPDATA%\CodexRelay`、API Key、日志或事务备份。更新组件不进入 Provider 配置事务边界，也不调用 `TransactionService`，因为它只替换安装目录中的应用程序产物。

## 失败边界

- 网络不可用、非 2xx 响应、无效 JSON、无效 SemVer 或缺失目标平台时，检查失败但本地 Provider 管理继续可用。
- 下载失败或签名校验失败时不启动安装器，不报告更新成功。
- UAC 被取消、安装器失败或更新后程序无法启动时不得伪造成功；MVP 不提供自动二进制回滚，保留旧 Release 供人工重新安装。
- 同一时刻只允许一个检查、下载或安装动作；旧响应不得覆盖较新的用户动作。

## 依据

- Tauri updater 官方文档：<https://v2.tauri.app/plugin/updater/>
- 现有插件注册与版本来源：`src-tauri/src/lib.rs`
- 现有唯一前端 Tauri 服务边界：`src/services/tauri.ts`
- 现有 per-machine NSIS 配置：`src-tauri/tauri.conf.json`
- 现有升级参数与安装目录恢复：`src-tauri/installer/custom-installer.nsi`

## 发布与托管设计（已确认）

```text
提交版本变更
  -> workflow_dispatch
  -> npm run check
  -> tauri-apps/tauri-action action-v1.0.0（工作流按提交 SHA 固定）
  -> Tauri updater artifacts + .sig + latest.json
  -> GitHub Draft Release
  -> 人工检查并发布
```

- 工作流只响应 `workflow_dispatch`，不因普通提交自动发布。
- 发布 job 需要 `contents: write`，使用 GitHub 自动提供的 `GITHUB_TOKEN` 上传 Release 资产。
- 工作流在构建前运行仓库完整检查；检查或构建失败时不得发布成功状态。
- 基础 Tauri 配置只包含 updater 公钥、HTTPS endpoint 和插件运行配置；`bundle.createUpdaterArtifacts` 放在仅由发布工作流通过 `--config` 合并的 release 覆盖文件中，避免普通本地构建因缺少私钥而失败。
- Action 显式开启 updater JSON、签名上传，并优先使用 NSIS 资产；其构建参数使用上述 release 配置覆盖。
- Release 先创建为 Draft。Draft 不应成为客户端 `releases/latest/download/latest.json` 的可见最新版本，人工检查资产和说明后再发布。
- `TAURI_SIGNING_PRIVATE_KEY` 与可选密码只存在于 GitHub Actions Secrets，并在开发者控制范围内离线备份；公钥可以进入公开 `tauri.conf.json`。任何私钥、密码、令牌都不进入 Git、任务材料、日志或前端状态。
- 客户端固定使用 `https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json`；地址不是用户输入，也不需要 GitHub 访问令牌。
- `package.json` 作为应用发布版本来源，`tauri.conf.json` 使用指向该文件的版本配置；发布检查需验证 Cargo 元数据或构建产物没有产生与应用版本不一致的意外漂移。
- MVP 不启用 Windows Authenticode、证书、时间戳或 SmartScreen 声誉流程。

公开仓库 `hunxuankai/codex-relay` 已由用户创建并确认为公开空仓库；在推送源码、配置 Secrets 和创建 Release 前，它不阻塞本地适配层、测试和静态清单 fixture 的开发。

## 前端组件与状态设计（已确认）

### 组件边界

| 单元 | 单一职责 |
|---|---|
| `src/services/tauri.ts` | 封装官方 updater API，规范化元数据、进度和阶段错误，不向界面暴露插件对象 |
| `src/composables/useUpdater.ts` | 管理单飞异步状态机、旧响应防护、资源释放和显式动作，导出只读状态 |
| `src/components/UpdatePanel.vue` | 展示当前版本、检查结果、Release 说明、进度和更新操作 |
| `src/views/SettingsView.vue` | 组合现有设置表单与 `UpdatePanel`，不拥有更新业务状态 |
| `src/components/ConfirmDialog.vue` | 在保留现有危险操作样式的同时，提供中性确认变体供更新退出提示使用 |

`UpdatePanel` 位于设置 `<form>` 之外，避免更新按钮触发设置提交。Release notes 只按纯文本展示，不使用 `v-html`。插件返回的 Update 资源句柄是外部不透明对象，只在 composable 内以 `shallowRef` 保存；模板和普通全局状态只接收应用自己的只读 DTO。

### 状态机

```text
idle
  -> checking
  -> upToDate | available | error
available
  -> confirming
  -> available (取消确认)
  -> downloading
  -> launching | error
error
  -> checking
```

- `idle`：显示当前版本和“检查更新”；创建 composable 或打开设置页不会发起更新网络请求。
- `checking`：锁定重复检查并显示状态。
- `upToDate`：显示当前版本已是最新版本。
- `available`：展示远端版本、发布日期、纯文本说明和“下载并安装”。
- `confirming`：明确告知下载完成后应用将退出，且 per-machine 安装可能触发 UAC。
- `downloading`：根据插件事件显示确定或不确定进度；MVP 不提供取消，但禁止重复触发。
- `launching`：只显示正在启动安装器，不提前显示“更新成功”。
- `error`：按检查阶段或下载/安装阶段显示固定安全文案，允许重新检查；不展示原始 URL、签名、异常堆栈或其它远端载荷。

composable 在重新检查、失败复位或作用域销毁时关闭旧 Update 资源。派生按钮状态和进度百分比使用纯 `computed`；网络与安装副作用只存在于显式动作中，不使用 watcher 或组件挂载钩子自动检查更新。

## 验收执行模式（半自动）

验收分为自动脚本和少量 Windows 交互，不要求用户手工执行整个测试流程。

### 自动部分

- 在仓库临时目录准备旧版/新版产物、Sandbox 启动脚本、假配置和结果目录。
- 只设置成对的 `CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR` 覆盖，所有 fixture 使用 `test-key-*-not-real`。
- 安装旧版前记录安装目录、程序版本、测试文件哈希和备份元数据；升级后由 PowerShell 校验版本、目录、程序启动和数据保留。
- 自动保存命令退出码、实际产物路径/大小/SHA-256 和不含密钥的验证报告。

### 交互部分

- 在应用中点击一次“检查更新”和一次“下载并安装”。
- 在 Windows UAC 窗口确认管理员权限；自动化无法控制安全桌面时由用户点击。
- 观察升级后应用是否重新出现；其余结果由脚本核对。

优先使用 Windows Sandbox。它是 Windows 的可选功能，不是项目运行时依赖；关闭 Sandbox 后测试文件、安装目录和注册表改动一起丢弃。若本机未启用或不支持 Sandbox，再使用带快照的 Windows 虚拟机。GitHub CLI、Docker、VirtualBox 等不是 MVP 必需软件；GitHub 仓库和 Actions 可通过浏览器配置。

首次端到端测试需要先手动安装一个能够正常进入设置页的 updater 客户端；后续更高版本才验证应用内升级链路。如果已发布的基线客户端因启动缺陷无法到达更新入口，必须人工安装包含修复的已知版本恢复可操作基线，再发布更高 SemVer 验证 updater。人工恢复安装本身不能计为应用内更新成功。

## 数据契约

前端适配层向 composable 暴露应用自有接口，插件 `Update` 对象只作为私有资源句柄存在：

```ts
interface UpdateReleaseInfo {
  currentVersion: string
  version: string
  date: string | null
  notes: string | null
}

interface UpdateProgress {
  downloadedBytes: number
  totalBytes: number | null
  percent: number | null
}

interface UpdateSession {
  info: UpdateReleaseInfo
  downloadAndInstall(onProgress: (progress: UpdateProgress) => void): Promise<void>
  close(): Promise<void>
}

interface UpdateClient {
  getCurrentVersion(): Promise<string>
  checkForUpdate(): Promise<UpdateSession | null>
}
```

适配层把 `Started`、`Progress` 和 `Finished` 插件事件归一化为累计字节和可选百分比。Release notes、日期和版本都是不可信远端文本：版本由插件执行 SemVer 校验，日期无效时不展示，说明只转义为纯文本。

公开错误至少区分 `UPDATE_CHECK_FAILED` 与 `UPDATE_INSTALL_FAILED`，并提供固定简体中文消息。原始插件异常可在确认不含 URL 查询令牌、Header、签名和密钥后进入脱敏内部诊断，但不得进入界面、通知或测试快照。

`latest.json` 由 Tauri Action 生成，客户端不自行解析或重建下载 URL。静态契约的目标平台为 `windows-x86_64`，包含有效 SemVer、Updater 资产 URL 和 `.sig` 内容。

## 信任与兼容性

- 固定 HTTPS endpoint 和内置 Tauri 公钥共同建立更新信任；GitHub 托管本身不能替代更新包签名。
- updater 默认 SemVer 比较阻止普通降级；MVP 保留人工下载安装旧 Release 的恢复路径。
- 不提供自定义代理设置；使用官方插件网络栈。离线、受限网络或代理失败均作为非阻塞检查错误。
- 现有 per-machine、自定义安装目录、注册表范围、卸载数据保留和 NSIS 语言行为必须保持。
- 发布工作流只把签名私钥环境变量注入 Tauri 构建步骤，并将第三方 Actions 固定到经核验的不可变版本，降低发布供应链风险。

## 首次发布与恢复

1. 创建公开仓库并配置 endpoint、公钥和 Actions Secrets。
2. 手动安装首个带 updater 的版本，建立客户端信任根。
3. 发布更高 SemVer 的测试/正式版本，验证应用内升级链路。
4. 私钥丢失、信任根更换或新版本无法启动时，从公开 Releases 人工安装已知版本；MVP 不自动回滚或轮换密钥。

实际验收中，`v0.1.0` 虽包含 updater，但 health 初始化竞态使 Sandbox 持续停留在启动页，无法触发更新。恢复链路固定为人工安装已包含 health 修复的 `v0.1.1`，再发布仅提升版本与说明的 `v0.1.2`，以 `v0.1.1 → v0.1.2` 作为首条真实应用内升级证据。

不得替换已经发布版本的同名签名资产。任何修复都发布新的更高 SemVer 和独立资产，保证清单、签名和二进制的一致性可审计。

## 验证矩阵

| 层级 | 自动边界 | 主要证据 |
|---|---|---|
| 服务适配层 | mock 官方 updater 与本地版本 API | 元数据/进度归一化、资源释放、安全错误 |
| composable | 注入 fake `UpdateClient`，不 mock 状态机 | 无自动检查、单飞、状态转换、旧响应防护 |
| Vue 组件 | mock typed composable/service | 可见文案、确认、进度、键盘/焦点、纯文本说明 |
| Rust/Tauri | 真实编译和配置结构检查 | 插件注册、权限、版本和 release 覆盖 |
| GitHub 发布 | 真实 Actions run 与 Draft Release | 检查/构建退出码、实际资产、`.sig`、`latest.json` |
| Windows 升级 | Sandbox/VM 中的真实 NSIS 与 UAC | 安装目录、版本、重启、数据快照、失败观察 |

单元测试不访问真实网络、不启动安装器，也不读取或写入真实 `.codex` 和 Codex Relay 应用数据。真实网络、GitHub Release、UAC 和安装器行为只在隔离系统级验收中执行。

## 设计取舍

- 选择官方 updater 而非自写 HTTP 下载器/进程启动器，以减少签名、平台和安装参数的自维护代码。
- 选择静态 GitHub Releases 而非动态服务器，以换取最低运维成本；代价是不支持按用户、设备或灰度策略动态决策。
- 选择手动检查而非启动检查，保留当前启动无网络产品契约；代价是用户必须主动发现更新。
- 选择半自动系统验收而非完全 UI 自动化，因为 UAC 安全桌面和进程替换无法由 Vitest/Rust 测试真实覆盖。
- 选择人工恢复而非自动回滚，控制 MVP 范围；代价是失败版本需要从 Releases 手动重装。
