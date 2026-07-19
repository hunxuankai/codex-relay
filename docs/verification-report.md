# Codex Relay 验证报告

验证日期：2026-07-20

工作目录：`D:\Kai\Project\unifyProject\codex-relay`

分支：`master`

本报告只记录命令输出或实际观察到的结果。所有启动和烟雾验证均同时设置
`CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR`，目标位于仓库的
`dev-data`；未使用真实 `%USERPROFILE%\.codex` 或
`%LOCALAPPDATA%\CodexRelay`。

## 一、最终报告 22 项

### 1. 已完成的功能

- Windows Tauri 2 桌面应用、Vue 3 + TypeScript 前端和 Rust 后端。
- Provider 读取、新增、编辑、删除、切换、API Key 状态展示和当前项保护。
- `config.toml` 保留式编辑、`auth.json`/`providers.json` 安全读写、事务备份、
  冲突检测、原子替换、写后校验和失败回滚。
- 启动关键自检、后台扩展自检、外部文件监控、备份恢复、应用内通知和安全日志。
- 系统托盘菜单、关闭窗口隐藏、单实例唤醒、当前用户开机启动配置。
- 首次启动引导、Provider、健康、备份、设置页面和 Windows 响应式样式。
- Debug、Release 和 NSIS current-user 安装包构建配置。

### 2. 项目实际目录

`D:\Kai\Project\unifyProject\codex-relay`

### 3. 主要文件结构

```text
codex-relay/
├─ src/                         Vue 页面、组件、composable、Tauri 类型化服务与测试
├─ src-tauri/src/               Rust 命令、模型、基础设施和业务服务
├─ src-tauri/tests/             路径安全与 Provider 工作流集成测试
├─ fixtures/                    明确标记为非真实密钥的测试 fixture
├─ scripts/prepare-dev-data.ps1 安全开发数据生成脚本
├─ docs/                        架构、事务、安全和验证文档
└─ dev-data/                    被 Git 忽略的安全启动数据
```

### 4. Provider 管理实现说明

`config_service` 使用 `toml_edit` 只修改目标 Provider 和活动字段，保留注释、未知字段、
其他 Provider、MCP、features、sandbox 和 profiles。`provider_secret_service` 单独管理
`providers.json` 中的 Provider ID 与 API Key。新增、编辑、删除和切换统一经过
`provider_service` 与事务服务；当前 Provider 禁止直接删除，缺少 API Key 时禁止切换。

### 5. API Key 保存方式

按项目明确接受的设计，API Key 以明文保存在应用数据目录的 `providers.json`，当前活动
密钥写入 Codex 目录的 `auth.json`。应用不使用 Credential Manager、Keyring、DPAPI 或
Stronghold。普通 Provider 列表和状态只接收是否已配置等安全信息；专用编辑命令
`getProviderApiKey(providerId)` 可以按 ID 把明文密钥返回前端编辑器。该值只应短暂存在编辑器
内存，不得持久化、写日志或进入通知/普通状态。备份快照可能包含明文密钥，文档和界面已
明确提示。

### 6. 切换事务实现说明

切换先获取应用级事务锁，再重新读取三个受管文件并核对外部修改指纹；随后创建同一事务
备份，在同目录写临时文件，解析并验证临时 TOML/JSON，原子替换正式文件，再重新读取验证。
任一步失败都会尝试恢复所有已触及文件，只有完整恢复并验证成功时才声明已恢复。

### 7. 自检实现说明

关键自检覆盖目录、文件、读写能力、设置和 Provider 加载；扩展自检覆盖 TOML/JSON、当前
Provider、字段、URL、`wire_api`、密钥一致性、Codex CLI、开机启动实际状态、未完成事务、
备份数量和外部修改。文件监控防抖后刷新相关前端/托盘状态并安排扩展自检。
后端依赖中没有 HTTP 客户端，关键启动自检测试也证明不会调用 Codex 子进程；启动阶段只做
本地文件、目录和 Windows 状态检查，不向 Provider 发起网络请求。

### 8. 托盘和开机启动实现说明

Rust 生命周期代码创建托盘和动态 Provider 菜单；主窗口关闭事件在非显式退出状态下隐藏
窗口，第二实例唤醒已有窗口，托盘“退出”设置显式退出标记后终止。开机启动使用 Tauri
autostart 插件的当前用户配置，并以 Windows 实际状态校正设置状态。

除“关闭隐藏”和“第二实例唤醒”外，本次还通过 Windows 原生通知区域工具栏直接读取到
`Codex Relay` 图标，打开并截图核对真实托盘菜单，点击托盘“退出”后观察目标进程终止。
托盘菜单中的开机启动项也完成启用、禁用和恢复原状态验证；HKCU Run 项与安全
`settings.json` 始终保持一致。

### 9. 前端测试结果

2026-07-20T06:33:25.3614103+08:00 开始执行最终 `npm run check`，前端 Vitest 通过
15/15 个测试文件和 63/63 个测试，退出码为 0。新增的第 63 个测试锁定 Release 使用
Windows GUI 子系统。

### 10. Rust 测试结果

同一次最终完整检查通过 107 个 Rust 单元测试、2 个路径安全集成测试和 1 个 Provider
工作流集成测试，失败数为 0。

### 11. TypeScript 检查结果

`vue-tsc --noEmit -p tsconfig.app.json` 作为 2026-07-20T06:33:25.3614103+08:00 开始的
最终 `npm run check` 第一阶段执行，退出码为 0。

### 12. Clippy 结果

`cargo fmt --check --manifest-path src-tauri/Cargo.toml` 与
`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
均在 2026-07-20T06:34:35.7590727+08:00 结束的最终 `npm run check` 中退出 0，Clippy
没有 warning。

### 13. Debug 构建结果

命令：`npm run build:debug`

开始：2026-07-20T06:37:09.4407740+08:00

结束：2026-07-20T06:37:40.5159146+08:00

退出码：0

产物：`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\debug\CodexRelay.exe`

大小：22,490,112 字节

最后写入：2026-07-20T06:37:38.1304033+08:00

SHA-256：`BDF875636EE87DE52246EB12903FB0EAAB591900CB1F94A13EFDF255564564E9`

PE Subsystem：3（Console，Debug 保留诊断控制台）

### 14. Release 构建结果

命令：`npm run build`

开始：2026-07-20T06:34:46.3597116+08:00

结束：2026-07-20T06:36:55.3059086+08:00

退出码：0

Release 应用：
`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\release\CodexRelay.exe`

大小：13,352,448 字节

最后写入：2026-07-20T06:36:55.2713838+08:00

SHA-256：`970A6B808FEA8A0905D15825E1A282ED8F8D8204BD34695DEE1D17B624581AF7`

PE Subsystem：2（Windows GUI，Release 启动不创建控制台窗口）

早先有一次 Release 命令因工具等待超时而没有完成；该次不计为成功。以上是完成评审修正
后对当前工作树的最终完整运行。

### 15. NSIS 安装程序实际路径

`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\release\bundle\nsis\Codex Relay_0.1.0_x64-setup.exe`

大小：2,959,293 字节

最后写入：2026-07-20T06:36:55.2303800+08:00

SHA-256：`08CCF4F449157CD944D72A274228EBBC7BE8301975B14266B9CB48DC507789AE`

### 16. 可执行文件实际路径

- Debug：`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\debug\CodexRelay.exe`
- Release：`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\release\CodexRelay.exe`

### 17. 已知限制

- NSIS 安装包已生成，但未执行真实安装、卸载或升级覆盖测试。
- 当前产物未进行 Windows 代码签名；正式发布前需要签名证书和签名流水线。
- `codex --version` 自检结果取决于目标机器是否安装并可执行 Codex CLI。

### 18. 人工验证步骤

1. 已在测试目录同时设置两个覆盖变量后启动 Release 程序。
2. 已确认主窗口完整渲染、托盘图标可见、Provider 菜单与主界面一致。
3. 已确认关闭主窗口后进程仍在且所有大尺寸窗口隐藏。
4. 已确认第二实例退出码为 0，并唤醒已有窗口。
5. 已从托盘启用和禁用开机启动，并与 Windows 当前用户 HKCU Run 项交叉确认。
6. 已点击托盘“退出”，确认唯一实例终止。
7. 发布前仍建议在隔离 Windows 用户或虚拟机中安装、升级和卸载 NSIS 包，确认不会删除
   Codex 配置、
   应用数据、密钥、日志或备份。

### 19. 是否检测到真实 API Key

否。最终复扫于 2026-07-20T06:39:17.4416968+08:00 至
2026-07-20T06:39:18.4247022+08:00 执行，
对工作树（包括忽略文件但排除依赖、`target`、`dist` 和 `.git`）及全部 Git 历史执行
高置信度 `sk-...` 前缀扫描，结果均为 0。对
`OPENAI_API_KEY`、Authorization 和 Bearer 命中逐项复核，均来自明确非真实 fixture、
安全开发数据、文档示例或脱敏测试。未跟踪任何 `auth.json` 或 `providers.json`。

安全开发数据生成器的两个较短占位值在审计中被强化为
`test-key-provider-a-not-real`，并增加回归测试；当前数据与烟雾测试生成的备份共命中 9 个
`test-key-...` 字面值，全部通过非真实标记校验。

### 20. 是否确认测试没有操作真实 `%USERPROFILE%\.codex`

是。Rust 路径安全测试使用显式临时根目录、默认生产路径哨兵树和测试前后逐字节快照；测试
构造器在覆盖缺失时不会回退到生产路径。自动启动烟雾测试同时设置两个仓库内覆盖变量。
最终完整检查中的 2 个路径安全集成测试均通过。

### 21. 是否确认测试没有操作真实 `%LOCALAPPDATA%\CodexRelay`

是。应用数据路径同样使用 `AppPaths::for_test`、临时目录或
`CODEX_RELAY_APP_DATA_DIR`；路径安全测试对默认应用数据哨兵树执行前后快照比较。自动烟雾
测试指向 `dev-data\app-data`。最终完整检查中的路径安全和工作流集成测试均通过。

### 22. 尚未完成的项目及真实原因

- 安装/卸载/升级未执行：本任务验证到安装包生成，未在隔离 Windows 用户或虚拟机中改动
  系统安装状态；最终验收标准要求的是 NSIS 安装程序生成成功。
- 代码签名未完成：仓库没有发布证书或签名授权；项目提示词要求在正式发布前说明签名，
  未把实际签名列为本次最终验收条件。

## 二、自动烟雾测试证据

### 手动启动与单实例

时间：2026-07-20T05:32:46.1186306+08:00

- 找到标题为 `Codex Relay` 的可见主窗口。
- 第二次启动在 15 秒内以退出码 0 结束。
- 只剩 1 个匹配进程。

### 关闭隐藏与二次唤醒

时间：2026-07-20T05:33:19.4816450+08:00

- 主窗口初始可见。
- 向实际 `Codex Relay` 窗口发送 `WM_CLOSE` 后，进程仍存活且主窗口变为隐藏。
- 第二次启动以退出码 0 结束，已有主窗口重新可见。
- 只剩 1 个匹配进程。

### 系统托盘菜单与真正退出

时间：2026-07-20T06:09+08:00 至 2026-07-20T06:10:18.9600745+08:00

- 从 Windows `NotifyIconOverflowWindow` 的原生通知区域工具栏读取到 tooltip 为
  `Codex Relay` 的图标。
- 右键菜单实际显示“打开 Codex Relay”、当前 Provider、Provider A、Provider B、
  “运行自检”、“打开 Codex 配置目录”、“开机自动启动”和“退出”。
- Provider A 有选中标记。
- 2026-07-20T06:10:18.6712952+08:00 点击“退出”后，目标进程在
  2026-07-20T06:10:18.9600745+08:00 前终止。

### Release 自启、单实例和关闭隐藏

- 初次 Release `--autostart` 验证发现主窗口隐藏但控制台窗口可见。根因是 Rust 入口缺少
  Windows GUI 子系统属性；先增加失败回归测试，再添加
  `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`。
- 修复后的 Release 构建于 2026-07-20T06:17:44.6209270+08:00 至
  2026-07-20T06:19:54.9552782+08:00 完成，退出码 0。
- 2026-07-20T06:20:33.8440699+08:00 以 `--autostart` 启动修复后的 Release：进程存活，
  Tauri 主窗口和 WebView 宿主均隐藏，没有控制台，面积大于 10,000 像素的可见窗口为 0；
  托盘菜单仍可打开。
- 2026-07-20T06:23:09.6941596+08:00 手动再次启动同一 Release，第二实例退出码为 0，
  已有实例的 `Codex Relay` 窗口恢复为可见。
- 2026-07-20T06:27:51.9184028+08:00 向 Release 主窗口发送关闭消息后，进程保持存活，
  主窗口和 WebView 宿主均隐藏，大尺寸可见窗口为 0。

### 开机启动启停与状态恢复

- 从真实托盘菜单启用后，HKCU
  `Software\Microsoft\Windows\CurrentVersion\Run` 出现名为 `Codex Relay` 的值，内容精确为
  `D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\release\CodexRelay.exe --autostart`；
  安全 `settings.json` 的 `autostartEnabled` 同时为 `true`。
- 从托盘菜单禁用后，该 Run 值消失，安全设置同时恢复为 `false`。
- 测试开始前开机启动未启用；结束时再次确认 Run 匹配项为 0、设置为 `false`，没有遗留
  Windows 启动项。

### 托盘 Provider 切换

- 使用安全 `dev-data` 从真实托盘菜单选择 Provider B 后，`config.toml` 顶层
  `model_provider` 变为 `provider-b`，`auth.json` 密钥与 `providers.json` 中 Provider B 的
  虚假密钥一致。
- 随后从托盘菜单切回 Provider A，顶层 `model_provider` 与活动密钥都恢复为 Provider A。
- 全过程未读取或修改真实用户 Codex 配置和应用数据。

## 三、忽略规则与路径审计

`git status --short --ignored` 显示以下目录为忽略状态：

- `dev-data/app-data/`
- `dev-data/codex/`
- `dist/`
- `node_modules/`
- `src-tauri/gen/`
- `src-tauri/target/`

`git check-ignore -v` 另外确认 `*.log`、`.env`、根目录 `auth.json`、根目录
`providers.json`、Debug/Release 产物和开发数据中的密钥文件均命中 `.gitignore`。`git
ls-files` 未发现受跟踪的 `auth.json` 或 `providers.json`。

## 四、最终复验

- 第一次 `npm run check`：2026-07-20T05:53:31.2007813+08:00 至
  2026-07-20T05:53:39.2829523+08:00，退出码 2。`vue-tsc` 报告新回归测试的正则捕获值
  类型为 `string | undefined`；将映射改为缺失捕获时返回空字符串，使类型安全且仍会触发
  密钥格式断言，然后从头重新执行完整检查。
- 运行验收于 2026-07-20T06:15:58.8360426+08:00 发现 Release `--autostart` 虽隐藏主
  窗口，却仍显示大尺寸控制台窗口。新增 GUI 子系统回归测试后，首次专项运行按预期失败
  1 项；添加最小入口属性后专项测试 5/5 通过。
- 最终 `npm run check`：2026-07-20T06:33:25.3614103+08:00 至
  2026-07-20T06:34:35.7590727+08:00，退出码 0；前端 63/63，Rust 107+2+1。
- 最终 `npm run build`：2026-07-20T06:34:46.3597116+08:00 至
  2026-07-20T06:36:55.3059086+08:00，退出码 0，生成 Release 与 NSIS。
- 最终 `npm run build:debug`：2026-07-20T06:37:09.4407740+08:00 至
  2026-07-20T06:37:40.5159146+08:00，退出码 0。
- 重新枚举 Debug、Release 和 NSIS 产物后，三个文件均存在；大小、最后写入时间和 SHA-256
  已记录在第 13 至 15 项。
- PE 头核对：Debug Subsystem 为 3（Console），Release Subsystem 为 2（Windows GUI）。
- 2026-07-20T06:39:17.4416968+08:00 开始的最终密钥复扫中，高置信度密钥前缀为 0；
  `git status --short --ignored` 仍确认开发数据、烟雾截图、`dist`、`target` 和 Tauri 生成
  文件被忽略。
- 运行验收结束时，匹配的 HKCU Run 项为 0、安全设置 `autostartEnabled` 为 `false`、安全
  `config.toml` 活动 Provider 为 Provider A、`CodexRelay` 进程数为 0，均恢复到测试前状态。
- `git diff --cached --check`：最终报告和源代码全部暂存后退出码 0，无空白错误。
- 最终独立复审未发现 Critical 或 Important。有 1 个不阻塞提交的 Minor：GUI 子系统回归
  测试是源码字符串断言，未来可改为构建后自动解析 PE；本轮已直接核对实际 Release PE
  Subsystem 为 2、Debug 为 3，因此当前产物证据不依赖该字符串测试。
