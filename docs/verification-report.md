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

### 8. 托盘和开机启动实现说明

Rust 生命周期代码创建托盘和动态 Provider 菜单；主窗口关闭事件在非显式退出状态下隐藏
窗口，第二实例唤醒已有窗口，托盘“退出”设置显式退出标记后终止。开机启动使用 Tauri
autostart 插件的当前用户配置，并以 Windows 实际状态校正设置状态。

自动烟雾测试观察到“关闭隐藏”和“第二实例唤醒”。由于本次环境的 Windows 自动化连接
不可用，没有直接看到托盘图标、点击托盘退出或在 GUI 中切换开机启动；这些项目不作人工
成功声明。

### 9. 前端测试结果

2026-07-20T05:54:04.2359316+08:00 开始执行 `npm run check`，前端 Vitest 通过
15/15 个测试文件和 62/62 个测试，退出码为 0。

### 10. Rust 测试结果

同一次完整检查通过 107 个 Rust 单元测试、2 个路径安全集成测试和 1 个 Provider 工作流
集成测试，失败数为 0。

### 11. TypeScript 检查结果

`vue-tsc --noEmit -p tsconfig.app.json` 作为最终 `npm run check` 的第一阶段执行，退出码
为 0。

### 12. Clippy 结果

`cargo fmt --check --manifest-path src-tauri/Cargo.toml` 与
`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings`
均在最终 `npm run check` 中退出 0，Clippy 没有 warning。

### 13. Debug 构建结果

命令：`npm run build:debug`

开始：2026-07-20T05:57:58.9779851+08:00

结束：2026-07-20T05:58:29.9131889+08:00

退出码：0

产物：`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\debug\CodexRelay.exe`

大小：22,490,112 字节

最后写入：2026-07-20T05:58:27.4744935+08:00

SHA-256：`9F13E547B812D566696029A29CDC092A168D7294A80A0789A4B6C168B5A7B54A`

### 14. Release 构建结果

命令：`npm run build`

开始：2026-07-20T05:55:30.6347166+08:00

结束：2026-07-20T05:57:48.7253629+08:00

退出码：0

Release 应用：
`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\release\CodexRelay.exe`

大小：13,352,448 字节

最后写入：2026-07-20T05:57:48.6849047+08:00

SHA-256：`B91DD1AA8A3C840BAF1888FAB3839774E6BB8D4046AA42BBF8E0CF65F77B6D05`

早先有一次 Release 命令因工具等待超时而没有完成；该次不计为成功。以上是完成评审修正
后对当前工作树的最终完整运行。

### 15. NSIS 安装程序实际路径

`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\release\bundle\nsis\Codex Relay_0.1.0_x64-setup.exe`

大小：2,960,431 字节

最后写入：2026-07-20T05:57:48.6439040+08:00

SHA-256：`E253C4ED4B6B14A55F18823FB2339EDBBFCF8E5CCDBF7ED0405EC4878F68DE22`

### 16. 可执行文件实际路径

- Debug：`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\debug\CodexRelay.exe`
- Release：`D:\Kai\Project\unifyProject\codex-relay\src-tauri\target\release\CodexRelay.exe`

### 17. 已知限制

- 本次未直接观察托盘图标，也未点击托盘“退出”。
- 本次未在 GUI 中实际启用/禁用开机启动。
- NSIS 安装包已生成，但未执行真实安装、卸载或升级覆盖测试。
- 当前产物未进行 Windows 代码签名；正式发布前需要签名证书和签名流水线。
- `codex --version` 自检结果取决于目标机器是否安装并可执行 Codex CLI。

### 18. 人工验证步骤

1. 仅在测试目录设置两个覆盖变量后启动 Debug 或 Release 程序。
2. 确认主窗口打开、托盘图标可见、Provider 菜单与主界面一致。
3. 关闭主窗口，确认进程仍在且窗口隐藏；双击托盘或点击“打开 Codex Relay”恢复窗口。
4. 再次启动程序，确认没有第二常驻实例且已有窗口获得焦点。
5. 在设置页启用和禁用开机启动，并用 Windows 当前用户启动项实际状态交叉确认。
6. 点击托盘“退出”，确认唯一实例终止。
7. 在隔离 Windows 用户或虚拟机中安装、升级和卸载 NSIS 包，确认不会删除 Codex 配置、
   应用数据、密钥、日志或备份。

### 19. 是否检测到真实 API Key

否。2026-07-20T06:01:01.5088832+08:00 至 2026-07-20T06:01:02.3858581+08:00
对工作树（包括忽略文件但排除依赖、`target`、`dist` 和 `.git`）及全部 Git 历史执行
高置信度 `sk-...` 前缀扫描，结果均为 0。对
`OPENAI_API_KEY`、Authorization 和 Bearer 命中逐项复核，均来自明确非真实 fixture、
安全开发数据、文档示例或脱敏测试。未跟踪任何 `auth.json` 或 `providers.json`。

安全开发数据生成器的两个较短占位值在审计中被强化为
`test-key-provider-a-not-real`，并增加回归测试；生成后的三个密钥均通过非真实标记校验。

### 20. 是否确认测试没有操作真实 `%USERPROFILE%\.codex`

是。Rust 路径安全测试使用显式临时根目录、默认生产路径哨兵树和测试前后逐字节快照；测试
构造器在覆盖缺失时不会回退到生产路径。自动启动烟雾测试同时设置两个仓库内覆盖变量。
最终完整检查中的 2 个路径安全集成测试均通过。

### 21. 是否确认测试没有操作真实 `%LOCALAPPDATA%\CodexRelay`

是。应用数据路径同样使用 `AppPaths::for_test`、临时目录或
`CODEX_RELAY_APP_DATA_DIR`；路径安全测试对默认应用数据哨兵树执行前后快照比较。自动烟雾
测试指向 `dev-data\app-data`。最终完整检查中的路径安全和工作流集成测试均通过。

### 22. 尚未完成的项目及真实原因

- 托盘图标可见性、托盘“退出”和开机启动 GUI 切换未直接人工观察：本次环境的 Windows
  原生自动化连接不可用，系统托盘也未通过 UI Automation 暴露。
- 安装/卸载/升级未执行：本任务验证到安装包生成，未在隔离 Windows 用户或虚拟机中改动
  系统安装状态。
- 代码签名未完成：仓库没有发布证书或签名授权。

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

未直接观察托盘图标或点击托盘退出，因此这两项只保留自动化测试覆盖说明，不列为人工烟雾
成功项。

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
- `npm run check`：2026-07-20T05:54:04.2359316+08:00 至
  2026-07-20T05:55:20.5047648+08:00，退出码 0。
- `npm run build`：2026-07-20T05:55:30.6347166+08:00 至
  2026-07-20T05:57:48.7253629+08:00，退出码 0，生成 Release 与 NSIS。
- `npm run build:debug`：2026-07-20T05:57:58.9779851+08:00 至
  2026-07-20T05:58:29.9131889+08:00，退出码 0。
- 重新枚举 Debug、Release 和 NSIS 产物后，三个文件均存在；大小、最后写入时间和 SHA-256
  已记录在第 13 至 15 项。
- 最终高置信度密钥前缀复扫为 0；`git status --short --ignored` 仍确认开发数据、`dist`、
  `target` 和 Tauri 生成文件被忽略。
- `git diff --cached --check`：最终报告和源代码全部暂存后退出码 0，无空白错误。
- 最终独立复审：前述两个 Important 均已关闭，未发现新的 Critical、Important 或 Minor。
