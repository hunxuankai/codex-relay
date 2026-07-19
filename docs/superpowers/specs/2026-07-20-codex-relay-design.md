# Codex Relay 设计规格

日期：2026-07-20  
状态：总体架构已确认，等待用户审阅书面规格

## 1. 目标与范围

Codex Relay 是一款面向 Windows 10/11 的轻量 Tauri 2 桌面应用，用于管理当前 Windows 用户的 Codex Provider 配置与 API Key。应用读取和修改 `config.toml`、`auth.json`，并在自己的应用数据目录维护 `providers.json`、设置、事务备份和脱敏日志。

本项目直接在当前名为 `codex-relay` 的目录初始化，不再创建同名嵌套目录。最终交付包括可运行的 `CodexRelay.exe`、Tauri NSIS 安装程序、自动化测试、中文文档和实际构建结果。

首版明确不包含网络连通性测试、模型请求、密钥加密、Windows Credential Manager、Keyring、DPAPI、Stronghold、Provider ID 修改或云端同步。API Key 按需求以明文保存在用户本机文件中，但不得进入日志、错误、前端持久化或 Git。

## 2. 核心设计原则

1. 配置安全、切换正确和失败回滚优先于界面装饰。
2. Vue 前端不直接访问文件系统，也不生成 TOML 或认证 JSON。
3. 主窗口与托盘调用同一个 Rust Provider 服务，避免两套切换逻辑。
4. 所有配置修改经统一事务服务串行执行，不允许 Command 各自写文件。
5. `config.toml` 使用 `toml_edit` 局部修改，保留注释、空行、顺序、未知字段和无关配置。
6. JSON 使用 `serde_json`，统一采用 UTF-8、两个空格缩进和末尾换行。
7. 启动阶段不发起任何网络请求；Codex CLI 缺失只产生警告。
8. 测试只使用临时目录或仓库测试数据，并用路径保护阻止真实目录访问。

## 3. 技术方案选择

采用分层模块化 Tauri 架构：

- Vue 3、TypeScript、Vite、Composition API、`<script setup>` 和原生 CSS 负责界面与交互状态。
- Tauri Commands 只做参数转换、调用服务和统一结果封装。
- Rust Services 实现业务规则、事务、备份、自检、托盘同步和文件监控。
- Infrastructure 实现路径、原子文件、文件指纹和日志脱敏等基础能力。
- Models 定义前后端共享语义的数据结构和安全错误码。

没有采用单一大型后端服务，因为切换、回滚、备份、自检和监控会高度耦合；也没有完全按功能纵向复制基础设施，因为事务、路径和脱敏必须只有一套可信实现。

## 4. 项目结构

```text
codex-relay/
├─ src/
│  ├─ components/
│  ├─ composables/
│  ├─ services/
│  ├─ types/
│  ├─ views/
│  ├─ App.vue
│  └─ main.ts
├─ src-tauri/
│  ├─ src/
│  │  ├─ commands/
│  │  ├─ infrastructure/
│  │  ├─ models/
│  │  ├─ services/
│  │  ├─ error.rs
│  │  ├─ lib.rs
│  │  ├─ main.rs
│  │  └─ tray.rs
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ fixtures/
├─ dev-data/
├─ docs/
├─ README.md
├─ AGENTS.md
└─ package.json
```

`dev-data` 仅提供被 Git 忽略的本地安全开发目录；固定测试样本放入 `fixtures`，其中只允许 `test-key-a-not-real`、`test-key-b-not-real` 或明确的等价虚假密钥。

## 5. 路径与数据职责

### 5.1 Codex 配置目录

路径解析优先级固定为：

1. `CODEX_RELAY_CODEX_HOME`；
2. `CODEX_HOME`；
3. `%USERPROFILE%\.codex`。

`CODEX_RELAY_CODEX_HOME` 用于开发和测试。正式应用无需用户设置。解析结果提供给 `config.toml` 和 `auth.json`，不得写死用户名。

### 5.2 应用数据目录

应用数据路径默认为 `%LOCALAPPDATA%\CodexRelay`，开发与测试可由 `CODEX_RELAY_APP_DATA_DIR` 覆盖。目录中保存：

- `providers.json`：Provider ID 到 API Key 的唯一持久映射；
- `settings.json`：界面、托盘和自启动设置；
- `backups/`：最多 20 份事务备份；
- `logs/`：滚动且脱敏的日志。

### 5.3 文件权威关系

- Provider 的名称、Base URL、Wire API、默认模型和未知字段以 `config.toml` 为准。
- 各 Provider 的 API Key 以 `providers.json` 为准。
- 当前生效的 API Key 以 `auth.json` 为准。
- 顶层 `model_provider` 决定当前 Provider。
- 普通列表接口只返回 `apiKeyConfigured`，不会返回密钥。

## 6. Rust 后端组件

### 6.1 Infrastructure

`path_service` 解析所有路径并执行测试路径保护。保护逻辑在测试模式或覆盖变量启用时拒绝解析到真实 `%USERPROFILE%\.codex` 和真实 `%LOCALAPPDATA%\CodexRelay`。

`atomic_file` 负责同目录临时文件、flush、必要的同步、重新解析、可靠替换和写入后读取验证。它不包含 Provider 业务规则。

`file_fingerprint` 使用 SHA-256，辅以文件存在状态和元数据，检测用户打开编辑页面后发生的外部修改。

`safe_log` 统一处理 `OPENAI_API_KEY`、`apiKey`、Authorization、Bearer Token、JSON 密钥字段以及查询字符串中的 token/key。任何可能带密钥的 Command 参数不使用 Debug 全量输出。

### 6.2 配置与密钥服务

`config_service` 使用 `toml_edit::DocumentMut` 读取 Provider、识别当前 Provider，并执行局部新增、编辑、删除和切换。Provider 表中的未知字段必须保留。Provider ID 创建后不可更改。

`provider_secret_service` 读取、创建和修改 `providers.json`。文件不存在时创建版本 1 的空结构；文件损坏时先复制到带时间和标识的损坏备份，返回安全错误并保留原文件，不静默覆盖。

`auth_service` 读取当前认证状态并生成仅包含 `OPENAI_API_KEY` 的规范 JSON。服务不向普通调用方暴露完整文件内容。

### 6.3 事务与备份服务

`transaction_service` 持有应用级异步互斥锁。每次写操作创建唯一事务 ID，重新读取最新文件，验证预期指纹，创建同一事务编号的备份，在内存生成全部目标内容，再逐个通过 `atomic_file` 写入和验证。

事务覆盖：创建、编辑、删除、切换、同步当前 Provider 和恢复备份。若任一步骤失败，服务按事务开始前的存在状态恢复所有已触及文件。只有全部恢复验证成功时，错误消息才可声明“原配置已恢复”；否则必须提示自动恢复未完全成功并引导备份恢复。

`backup_service` 保存 `config.toml`、`auth.json`、`providers.json` 及不含密钥的 `metadata.json`。不存在的源文件只在元数据中标记。手动恢复前先备份当前状态，恢复完成后重新自检并刷新 Provider 和托盘。

### 6.4 Provider 服务

`provider_service` 是主窗口和托盘共同调用的业务入口，负责：

- 验证 Provider ID、名称、HTTP/HTTPS Base URL、`responses` Wire API 和 API Key；
- 合并 `config.toml` Provider 信息与 `providers.json` 密钥状态；
- 创建、编辑和删除 Provider；
- 阻止删除当前 Provider；
- 执行 Provider 切换事务；
- 发出数据刷新事件和安全通知。

切换时修改顶层 `model_provider`，确保目标 Provider 必需字段正确，并确保 `cli_auth_credentials_store = "file"`。目标 Provider 有默认模型时更新顶层 `model`；没有默认模型时保留现有顶层 `model`。目标 API Key 写入 `auth.json`。切换结束后再次验证 TOML、JSON、目标 Provider 字段和两个密钥文件的一致性。

### 6.5 自检、监控和系统集成

`self_check_service` 分为关键自检和后台扩展自检。关键自检只做路径、目录、文件存在性、可读写性、设置加载、Provider 加载和当前 Provider 判断。托盘尽早创建，扩展自检在后台解析文件、验证 Provider、检查密钥一致性、执行带超时的 `codex --version`、核对自启动实际状态、事务残留、备份数量和外部修改。

`file_watch_service` 监控 `config.toml`、`auth.json` 和 `providers.json`，使用防抖与应用自身写入抑制。检测到外部修改时更新指纹、运行相关自检并发出前端/托盘刷新事件。编辑会话持有的旧指纹在保存时触发冲突，不静默覆盖。

`autostart_service` 封装 Tauri Autostart 插件，并始终以 Windows 实际注册状态为准。自动启动参数使应用默认只显示托盘；手动启动按设置显示主窗口。

`tray` 动态生成当前 Provider 和全部 Provider 菜单。切换期间防重复操作；新增、编辑、删除、恢复或外部修改后重建菜单。托盘“退出”是唯一正常结束后台进程的界面入口。

Single Instance 插件阻止第二实例；再次启动时聚焦或恢复已有主窗口。Notification 插件显示不含密钥的切换结果。

## 7. Tauri Command 边界

Commands 返回统一结构：

```typescript
interface CommandResult<T> {
  success: boolean
  data?: T
  error?: {
    code: string
    message: string
  }
}
```

Commands 不返回 Rust 堆栈、完整文件内容或普通列表中的 API Key。密钥只可通过专门的编辑接口读取或修改。编辑读取接口仅在用户明确打开编辑器时调用，前端不把返回值放入 localStorage、日志或测试快照。

错误码覆盖验证失败、文件损坏、缺少密钥、Provider 不存在、当前 Provider 不可删除、外部修改冲突、事务失败、回滚不完整、自启动失败和系统集成错误。用户消息保持简体中文且可理解。

## 8. 前端设计

界面采用约 900×620 的可缩放双栏布局，无重量级 UI 框架和复杂动画。颜色使用 CSS 变量适配系统明暗主题，控件保留清晰焦点态并支持键盘操作。

左侧包含 Provider 列表、当前标记、新增入口、自检、备份和设置导航。右侧显示选中 Provider 的名称、不可编辑 ID、Base URL、Wire API、默认模型、密钥状态以及编辑、启用和删除操作。底部状态栏显示 Codex 配置目录、当前 Provider、最近操作、自检概况和打开目录按钮。

`ProviderEditor` 同时支持新增和编辑。API Key 默认密码模式，并有可访问的显示/隐藏按钮。编辑时未触碰密钥则不提交覆盖；明确清空触发二次确认。当前 Provider 的影响生效字段改变后，询问是否立即同步到 Codex。

首次启动且不存在配置或 Provider 时显示引导页，只提供打开目录、新增首个 Provider、稍后设置和退出，不创建虚假 Provider。

Provider 列表展示有效性、当前状态和密钥配置状态。缺少密钥时禁用“使用此 Provider”；当前 Provider 禁用删除。成功切换必须同时显示应用内提示和 Windows 通知，并包含“请重启 Codex 后生效”。

备份页只显示元数据与恢复操作，不展示备份中的明文密钥。设置页显示自启动实际状态及窗口/托盘设置。

## 9. 主要数据流

### 9.1 启动

1. Single Instance 建立唯一实例。
2. 尽早构建托盘。
3. 解析安全路径并创建必要目录和默认应用数据文件。
4. 加载设置、Provider 摘要和当前 Provider。
5. 按启动来源决定显示主窗口或仅托盘。
6. 后台运行扩展自检、Codex CLI 探测和文件监控。
7. 将自检与 Provider 状态通过事件同步给前端和托盘。

### 9.2 创建或编辑 Provider

1. 前端完成基础即时校验并提交表单。
2. Rust 重新验证所有字段并获取事务锁。
3. 重新读取磁盘文件并检查编辑会话指纹。
4. 事务备份原文件。
5. 使用 `toml_edit` 局部更新 Provider 表，使用 JSON 服务更新目标密钥。
6. 原子写入、重新解析并验证。
7. 更新界面和托盘；如选择立即使用，则继续执行同一可信切换流程。

### 9.3 切换 Provider

1. 获取全局写锁并重新读取三个文件。
2. 验证目标 Provider、目标密钥和外部修改状态。
3. 创建统一事务备份。
4. 生成新 `config.toml` 和 `auth.json`。
5. 写临时文件并解析验证。
6. 替换正式文件并再次读取验证。
7. 失败则恢复所有触及文件；成功则刷新状态、托盘和通知。

### 9.4 恢复备份

1. 获取写锁并验证备份元数据及文件。
2. 先为当前状态创建恢复前备份。
3. 按原存在状态恢复或移除相应文件。
4. 重新解析、验证、自检并刷新所有消费者。

## 10. 错误处理与日志

所有公开错误先映射为稳定错误码和不含秘密的中文消息。底层错误只以脱敏形式进入滚动日志。任何 panic、Rust backtrace、完整 `auth.json`、完整 `providers.json`、Authorization Header 或 API Key 都不得出现在前端、通知或日志中。

损坏的 `config.toml` 不被修改；损坏的 `providers.json` 先备份原件再引导重新设置密钥。外部修改冲突要求重新加载。Codex CLI 不存在属于警告，不阻止 Provider 管理。

事务错误明确区分“已验证恢复成功”和“恢复未完全成功”。后者提供备份入口，不做虚假成功声明。

## 11. 测试设计

### 11.1 Rust

Rust 测试覆盖提示词列出的至少 45 项：路径优先级与真实目录保护、TOML Provider 解析与局部编辑、Provider 校验、多个密钥的独立增删改、损坏 JSON 保护、认证 JSON 格式、切换模型规则、缺少目标或密钥、临时文件验证、各阶段失败回滚、备份创建恢复与 20 份限制、并发切换、外部修改冲突以及日志和错误脱敏。

服务构造函数接收路径上下文和可替换的文件操作故障注入点，使写入、替换和验证失败可确定性测试。所有测试使用 `tempfile`，并在测试入口显式设置测试路径；路径保护测试证明真实目录会被拒绝。

### 11.2 前端

Vitest 与 Vue Test Utils 覆盖至少 15 项界面要求：Provider 展示、当前标记、密钥状态、禁用规则、表单校验、密钥掩码切换、成功/失败提示、自检状态、冲突提示、CRUD 后刷新和自启动实际状态。

Tauri 调用集中在可 mock 的 `src/services/tauri.ts`，组件测试不访问真实文件系统。

### 11.3 构建验证

完成前实际执行：

1. `npm run typecheck`；
2. `npm run test`；
3. `cargo fmt --check`；
4. `cargo clippy --all-targets --all-features -- -D warnings`；
5. `cargo test`；
6. Tauri Debug 构建；
7. Tauri Release 构建；
8. NSIS bundle 构建；
9. 仓库密钥扫描和真实路径访问审计。

只有实际命令成功才报告通过。安装程序和可执行文件路径从构建产物中核实，不按约定猜测。

## 12. 发布与卸载

Tauri 配置的产品名为 `Codex Relay`，主程序名为 `CodexRelay.exe`，Windows bundle 使用 NSIS、当前用户安装和开始菜单入口。卸载不主动删除 Codex 配置、应用备份或其他用户数据。无代码签名证书时不声明已签名，并在 README 说明 Windows 可能显示“未知发布者”。

## 13. 文档交付

README 使用简体中文，覆盖用途、技术栈、目录、开发、测试、构建、三类配置文件、明文密钥风险、托盘、自启动、备份恢复、测试目录、退出、卸载、限制和签名说明。

`AGENTS.md` 固化真实目录保护、密钥禁入 Git/日志、事务服务唯一写入口、TOML 局部编辑、回滚与测试要求。`docs/architecture.md`、`docs/config-transaction.md` 和 `docs/security-notes.md` 分别说明架构数据流、配置事务细节和个人使用场景下的明文密钥风险。

## 14. 完成定义

完成必须同时满足项目提示词的全部验收项：Provider CRUD 与切换可用；主窗口和托盘共享后端；配置写入保留无关内容并可回滚；自检、监控、单实例、自启动、通知和托盘行为实现；测试不触碰真实数据；文档齐全；所有检查、Debug、Release 和 NSIS 构建都实际成功；最终报告逐项列出产物、命令结果、已知限制和人工验证步骤。
