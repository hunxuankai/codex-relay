# Codex Relay

Codex Relay 是一款面向 Windows 10/11 的轻量桌面工具，用于管理当前 Windows 用户的 Codex Provider 配置、API Key、事务备份、自检、系统托盘和开机启动。应用基于 Tauri 2、Vue 3 与 Rust，主程序名为 `CodexRelay.exe`。

## 主要功能

- 读取并管理 `config.toml` 中已有的 `[model_providers.<id>]`。
- 新增、编辑、删除 Provider，保留无关 TOML 注释、未知字段和功能开关。
- 为每个 Provider 单独保存 API Key，并快速切换当前 Provider。
- 切换时以同一事务更新 `config.toml` 与 `auth.json`，失败时验证回滚结果。
- 关键自检与后台扩展自检，包括配置、密钥一致性、Codex CLI、开机启动、备份和外部修改。
- 系统托盘切换、单实例、Windows 通知、窗口位置恢复和关闭到托盘。
- 当前用户级开机启动，无需 Windows 服务或管理员权限。
- 修改前自动备份，最多保留最近 20 份事务备份，并支持手动恢复。
- 简体中文界面、跟随系统的明暗主题、键盘焦点态和窄窗口响应式布局。

## 技术栈

- Tauri 2、Rust 2024、Tokio
- Vue 3.5、Composition API、`<script setup lang="ts">`
- TypeScript、Vite、Vitest、Vue Test Utils
- `toml_edit`、`serde`、`serde_json`
- Tauri Single Instance、Autostart、Notification 与 Tray API
- NSIS Windows 安装包

## 目录结构

```text
src/                         Vue 界面、类型、composables 与 Tauri 命令边界
src-tauri/src/               Rust 服务、事务、路径、自检、托盘与命令适配
src-tauri/tests/             临时目录集成测试与真实路径安全门禁
src-tauri/icons/             Tauri/Windows 图标
src-tauri/installer/         自定义 NSIS 安装模板
fixtures/                    仅含假密钥的测试样例
scripts/prepare-dev-data.ps1 安全开发数据与环境覆盖脚本
dev-data/                    被 Git 忽略的本地开发配置
docs/                        架构、事务、安全与验证说明
AGENTS.md                    仓库级安全、测试与文档规则
```

## 环境要求

1. Windows 10 或 Windows 11。
2. Node.js 20.19+ 或 22.12+ 与 npm。
3. Rust stable 与 Cargo。
4. Microsoft C++ Build Tools（Desktop development with C++）。
5. Microsoft Edge WebView2 Runtime。Windows 10/11 通常已安装。
6. 构建 NSIS 安装包时需要 Tauri CLI 能下载或找到所需打包工具。

## 安装与首次使用

Release 构建完成后，运行 `src-tauri/target/release/bundle/nsis/` 下实际生成的 `.exe` 安装器。安装模式是所有用户（per-machine），需要管理员权限。首次安装时，如果 `D:` 是固定磁盘，默认目录为 `D:\Program Files\Codex Relay`；否则使用与构建目标架构匹配的系统 Program Files 目录，当前交付的 x64 安装包通常回退到 `C:\Program Files\Codex Relay`。后续 per-machine 版本升级优先沿用上次目录，安装界面也允许用户修改目录。开始菜单项位于“Codex Relay”目录。若电脑上安装过旧的 current-user 版本，请先从 Windows“已安装的应用”卸载旧版，再运行新的 per-machine 安装器，避免 AppData 与 Program Files 中同时保留两套程序；卸载不会删除 Codex 配置和应用数据。

首次启动且 `config.toml` 不存在或没有 Provider 时，会出现引导页。可打开配置目录、新增第一个 Provider、稍后设置或退出。应用不会自动创建带虚假地址的 Provider。

## 开发

安装依赖：

```powershell
npm install
```

只启动前端：

```powershell
npm run dev:frontend
```

推荐使用安全开发模式启动完整 Tauri 应用：

```powershell
npm run dev:safe
```

`dev:safe` 会在仓库的 `dev-data` 下写入明确的假配置和假密钥 `test-key-provider-a-not-real`、`test-key-b-not-real`，设置 `CODEX_RELAY_CODEX_HOME`、`CODEX_RELAY_APP_DATA_DIR` 后再启动 Tauri。用户始终通过 `npm run dev:safe` 进入；脚本内部使用 `npm.cmd run dev`，避免 Windows PowerShell 把 `& npm run dev` 错误解析为 `pm`。安全模式不会读取或修改真实 `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay`。

不要在没有路径覆盖的情况下直接运行 `npm run dev`，因为普通开发入口不会自动隔离真实 Codex 配置。只有当前终端已经同时设置两个 Relay 覆盖变量时，才可以手动启动：

```powershell
$env:CODEX_RELAY_CODEX_HOME = "$PWD\dev-data\codex"
$env:CODEX_RELAY_APP_DATA_DIR = "$PWD\dev-data\app-data"
npm run dev
```

仅准备安全数据、不启动应用：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/prepare-dev-data.ps1 -PrepareOnly
```

`-PrepareOnly` 只创建或刷新安全数据，不启动应用。该命令会启动子 PowerShell，因此脚本内设置的环境变量不会回传到当前终端；若随后需要手动运行 `npm run dev`，必须按上面的示例在当前终端重新设置两个 Relay 覆盖变量。

## 测试与检查

```powershell
npm run typecheck
npm run test
npm run check:frontend
npm run check:rust
npm run check
```

Rust 单元与集成测试使用 `tempfile`，并通过 `AppPaths::for_test` 或测试模式双覆盖构造路径。`path_safety` 会在安全临时目录中建立默认路径哨兵，证明 Provider/备份工作流不触及默认用户目录。

禁止让测试回退到真实用户路径。禁止把真实 API Key 写入 fixture、日志、快照或 Git。

## Debug、Release 与 NSIS 构建

Debug 构建只生成主程序，不创建安装包：

```powershell
npm run build:debug
```

Release 构建会按照当前 `targets: ["nsis"]` 同时生成 Release 主程序和 NSIS 安装器；两个命令等价：

```powershell
npm run build:release
npm run build
```

也可以使用以下替代入口显式请求 NSIS bundle，但不需要在 `build:release` 后重复运行：

```powershell
npm run bundle:nsis
```

- Debug 可执行文件位于 `src-tauri/target/debug/`。
- Release 主程序通常位于 `src-tauri/target/release/CodexRelay.exe`。
- NSIS 安装器位于 `src-tauri/target/release/bundle/nsis/`。
- 最终报告必须枚举实际产物路径，不能只根据约定猜测文件名。

## 路径解析

Codex 配置目录按以下优先级解析：

1. `CODEX_RELAY_CODEX_HOME`：仅用于开发和测试的强制覆盖。
2. `CODEX_HOME`：用户现有 Codex 配置覆盖。
3. `%USERPROFILE%\.codex`：正式应用默认值。

应用数据目录按以下优先级解析：

1. `CODEX_RELAY_APP_DATA_DIR`：仅用于开发和测试。
2. `%LOCALAPPDATA%\CodexRelay`：正式应用默认值。

测试模式缺少任一 Relay 覆盖变量时会返回 `TEST_PATH_OVERRIDE_REQUIRED`，不会回退到真实目录。指向真实 `.codex` 或 CodexRelay 数据目录会返回 `UNSAFE_TEST_PATH`。

## 数据文件职责

### `config.toml`

Codex Provider 配置的主要数据源。Codex Relay 只局部修改目标 Provider、顶层 `model_provider`、`cli_auth_credentials_store`，以及目标 Provider 明确配置模型时的顶层 `model`。其他 Provider、注释、`[features]` 和未知字段必须保留。

### `auth.json`

保存当前生效认证：

```json
{
  "OPENAI_API_KEY": "当前 Provider 的 API Key"
}
```

切换成功后写入目标 Provider 的密钥。普通 Provider 列表、通知和日志都不会返回该明文。

### `providers.json`

位于 `%LOCALAPPDATA%\CodexRelay\providers.json`，按 Provider ID 保存独立 API Key。文件损坏时不会静默覆盖；应用先保存损坏副本，再返回安全错误并引导重新设置密钥。

### 其他应用数据

- `settings.json`：窗口、托盘、首次引导和开机启动配置。
- `backups/`：事务快照与不含密钥的 `metadata.json`。
- `logs/`：经过脱敏的滚动日志。

## API Key 保存方式与风险

这是面向个人本机使用的简化设计。API Key 以明文存在于 `providers.json`、当前 `auth.json`，并可能出现在事务备份的文件快照中。项目明确不使用 Windows Credential Manager、Keyring、DPAPI、Stronghold 或其他加密密钥库。

风险与建议：

- 能读取当前 Windows 用户文件的进程，可能读取这些密钥。
- 不要共享 `%LOCALAPPDATA%\CodexRelay`、`.codex`、备份目录或完整用户配置包。
- 不要把 `providers.json`、`auth.json`、备份或日志上传到公共仓库和工单。
- 不要在多人共用或不可信的 Windows 账户上保存高权限密钥。
- 怀疑泄漏时，应在 Provider 平台吊销并重新生成密钥。

详见 [docs/security-notes.md](docs/security-notes.md)。

## Provider 操作与切换事务

新增和编辑会先校验 ID、名称、HTTP(S) Base URL、固定 `responses` Wire API、模型和密钥规则。Provider ID 创建后不可修改。当前 Provider 的生效字段修改可选择立即同步；清除当前密钥时不能立即同步，现有 `auth.json` 可能继续保留当前生效密钥。

切换步骤包括：重新读取三个文件、验证目标与密钥、检查外部修改指纹、创建统一备份、生成内存结果、写入临时文件、解析验证、替换正式文件、再次验证、刷新托盘与界面。成功提示包含“请重启 Codex 后生效”。

当前 Provider 不能直接删除，必须先切换到其他 Provider。详见 [docs/config-transaction.md](docs/config-transaction.md)。

## 自检

关键自检只执行本地路径、目录、文件、设置、Provider 和当前 Provider 检查，不调用模型接口。托盘创建后运行扩展自检，包括 TOML/JSON、Provider 有效性、密钥一致性、`codex --version`、开机启动实际状态、事务残留、备份数量和外部修改。

Codex CLI 缺失或超时属于警告，不阻止 Provider 管理；配置损坏、密钥不一致等会显示错误。

## 托盘、窗口与退出

- 托盘尽早创建，Provider 变化后立即重建菜单。
- 双击托盘或选择“打开 Codex Relay”会显示、还原并聚焦主窗口。
- 再次启动不会创建第二实例，而是唤醒已有窗口。
- 关闭窗口默认只隐藏到托盘，不会结束进程。
- 只有托盘菜单“退出”或首次引导的显式“退出”会真正结束进程。
- 窗口位置与大小会保存；仅当保存位置仍与当前显示器相交时才恢复。

## 开机启动

设置页显示 Windows 实际注册状态，而不是只相信 `settings.json`。开机启动是当前用户级，自动启动时默认仅显示托盘，手动启动时默认显示主窗口。启用或禁用失败会显示插件返回的安全错误。

## 备份与恢复

每次 Provider 创建、编辑、删除、切换、同步或恢复前都会创建事务备份。备份包含原始 `config.toml`、`auth.json`、`providers.json` 文件快照，因此备份中可能包含明文 API Key；`metadata.json` 不含密钥。最多保留最近 20 份，并避免删除当前事务需要的备份。

手动恢复前会再次备份当前状态。恢复按原始存在状态写回或删除文件，完成后刷新 Provider、托盘和自检。若跨资源刷新未完全成功，界面会明确提示手动重新加载，不会虚报全部成功。

## 常见错误

- `TEST_PATH_OVERRIDE_REQUIRED`：测试模式缺少两个 Relay 路径覆盖。
- `UNSAFE_TEST_PATH`：测试路径指向真实用户配置目录。
- `INVALID_CONFIG_TOML`：`config.toml` 无法解析；应用不会修改该文件。
- `INVALID_PROVIDER_SECRETS`：`providers.json` 损坏；损坏副本已保留。
- `PROVIDER_API_KEY_MISSING`：目标 Provider 未保存密钥，不能启用。
- `ACTIVE_PROVIDER_DELETE_FORBIDDEN`：先切换到其他 Provider 再删除。
- `EXTERNAL_MODIFICATION_CONFLICT`：编辑期间文件被其他程序修改；请刷新后重试。
- `ROLLBACK_INCOMPLETE`：自动恢复未完全成功；立即从备份页恢复。
- `AUTOSTART_ENABLE_FAILED` / `AUTOSTART_DISABLE_FAILED` / `AUTOSTART_QUERY_FAILED`：Windows 开机启动注册或验证失败。
- 找不到 `codex`：仅为自检警告，可继续管理 Provider。
- `cargo metadata ... program not found`：启动当前终端或 VS Code 的父进程 PATH 中没有 Cargo。先运行 `cargo --version`；若 Windows Terminal 可以识别而 VS Code 不可以，应完全退出原启动器和所有 `Code.exe`，或从可识别 Cargo 的终端进入项目目录后运行 `code .`。

## 卸载与数据保留

NSIS 卸载器移除应用程序和快捷方式，但没有自定义卸载钩子去删除 `.codex`、`%LOCALAPPDATA%\CodexRelay`、API Key、日志或备份。需要彻底清理时，请先退出应用并在确认不再需要恢复后手动删除这些目录。卸载前建议保留必要配置副本。

## 当前限制

- 程序仅支持 Windows 10/11；安装器为所有用户（per-machine），但 Provider、Codex 配置、应用数据和开机启动均按当前登录用户管理。
- Wire API 当前只支持 `responses`。
- 不调用模型接口验证 Base URL 或 API Key 是否可用。
- API Key 和备份不加密，不适合共享计算机或高安全场景。
- 没有自动更新、云同步、团队权限或远程管理。
- 开发构建与发布构建都依赖本机 WebView2 与 Tauri/Rust 工具链。

## 代码签名与“未知发布者”

当前仓库不包含代码签名证书，也不虚假声明安装器已签名。自行构建的 `CodexRelay.exe` 和 NSIS 安装器可能被 Windows SmartScreen 显示为“未知发布者”。正式分发前应使用可信的 Windows 代码签名证书签署主程序和安装器，并记录实际签名验证结果。

## 进一步阅读

- [架构说明](docs/architecture.md)
- [配置事务与回滚](docs/config-transaction.md)
- [安全说明](docs/security-notes.md)
- [实施设计](docs/superpowers/specs/2026-07-20-codex-relay-design.md)
