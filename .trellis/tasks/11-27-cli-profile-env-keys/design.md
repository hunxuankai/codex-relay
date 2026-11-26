# 设计：CLI Profiles 与环境变量 Key

## 建议

让 Codex 继续管理 Profile，环境变量仍是唯一凭据来源，Relay 提供发现、状态和应用内更换入口。已有 `codex -p agentrouter` 和 `codex -p qiandao` 不需要改用 Relay 启动。默认供应商仍是全局选择；Profile 页没有「使用」「设为当前」或托盘切换动作。

## 用户路径

在「供应商」页切换 `默认供应商 / CLI Profiles`，初始保留现有默认页。Profile 行显示名称、目标供应商/主机、变量名与状态，主操作为「复制启动命令」，辅助操作为「管理 Key」。示例如下，变量名仅用于演示：

| Profile | 凭据 | 操作 |
| --- | --- | --- |
| `agentrouter` | `AGENTROUTER_API_KEY` · 用户变量已设置 | 复制 `codex -p agentrouter` · 更换 Key |
| `qiandao` | `QIANDAO_API_KEY` · 未检测到 | 复制 `codex -p qiandao` · 设置 Key |

- 「设置/更换 Key」只展示目标供应商、变量名、保存范围和一个默认遮挡的新 Key 输入。按钮明确命名为「保存到 Windows 用户变量」。不展示旧值；不生成带 Key 的命令。Windows 用户环境变量不是加密密钥库。同名变量不适合直接编辑时，说明原因并提供「复制变量名」与 Windows 管理入口。
- 区分「Windows 用户变量已设置」和「当前 Relay 进程可见」；两者可同时为真，也可只满足其一。都未查到时用「未检测到」，查询失败时用「无法检测」。不读取其他终端的环境，也不声称 Key 有效或网络已连通。
- 回读确认后显示「已保存到 Windows 用户环境变量」，而不是「Key 已生效」。影响之后启动且继承到新环境的进程；如果新标签页仍读到旧值，完全退出终端宿主后重开。现有 Codex/终端不热更新，PowerShell 启动脚本或当前会话的 `$env:` 仍可覆盖用户变量。仅在进程中看到的变量不会被导入；允许显式设置一个新的用户变量，并提前提示此差异。
- 缺少变量引用时说明需要在对应供应商配置中设置 `env_key`；损坏或不支持时只展示安全原因与文件名。不显示 TOML 原文、解析错误中的源行、含凭据的 URL 或响应头。

## 数据与兼容

- 使用现有 `AppPaths` 的 Codex 根目录解析，从该根一级发现 `<name>.config.toml`，不递归扫描或跟随指向外部的链接/重解析点。验证普通文件与名称；启动命令只使用明确支持的名称。页面提示终端也需指向同一个 `CODEX_HOME`。
- Profile 名、Provider ID、环境变量名是不同字段。按 Codex 支持版本的用户配置叠加语义解析 `config.toml` 和所选 Profile，取 `model_provider` 指定的 `model_providers.<id>`，再读取其中的 `env_key`。不要假设名称相同，也不要把简单字符串搜索当成合并。只承诺用户配置层静态结果，不冒充特定终端带 CLI 参数和受管策略时的最终解析。
- 记录路由字段与变量名来自默认层还是 Profile。继承的供应商、地址、模型或 `env_key` 可能随默认配置变化，要标明依赖而非宣称完全隔离。需要稳定独立的路由时，建议 Profile 显式固定其供应商及凭据引用，不复制整份基础配置。
- 对同名变量按 Windows 的大小写不敏感规则分组，标出当前配置目录内可确认的引用者（含默认配置所选供应商）。仅允许规则明确、单一引用的凭据变量直接编辑：如合法的专用 `*_API_KEY` 或 `*_TOKEN`，排除通用/保留名（如 `OPENAI_API_KEY`、`CODEX_API_KEY`）和同名系统变量。仅支持不存在或普通字符串型的用户变量；可展开的特殊值保留外部管理。Relay 无法穷尽外部程序或终端的引用；需要两把独立 Key 时应使用两个专用变量。
- 首版只解释可确定的 `env_key` 认证。其他认证、旧式 `[profiles.*]` 和未确认为兼容的配置不自动转换或回退到 `auth.json`。Codex 0.134.0 起 `--profile` 使用独立文件；版本未知或语义无法确定时按具体问题降级。
- Vue 列表只消费安全的 typed DTO；Tauri command 调用独立 Profile 服务。Rust 只依照解析出的变量名查询指定范围，不枚举或返回环境值；测试使用注入的环境来源。复用 `AppPaths`，不调用 `ProviderService::switch_provider`、默认 Key 管理器或网络测试。

## 保存与故障处理

- 专用保存命令仅接受 Profile 标识、短期不透明编辑基线和本次新值。后端复核生效路由、变量资格和同名引用，不接受前端指定注册表路径或作用域。新值只在对话框与写入所需的后端内存短暂存在，不进入普通 DTO、日志、事件、命令行或通用备份。取消、提交及对话框卸载清空输入。
- 在 `TransactionService` 的共享锁下新增单一用户变量资源，不能直接绕开原文件事务。重新验证配置指纹与变量基线（存在状态、类型和值在后端比较，不输出密钥指纹）。保留精确旧态用于恢复，校验候选的类型和编码；作用域固定为 `HKCU\Environment`，不执行 `setx`，不修改其他进程环境。
- 事务首选验证 Windows 原生注册表事务：在未提交事务中暂存候选并回读，提交前再次核对路由基线，再原子提交并从已提交状态回读。将注册表原生暂存/原态保护和文件事务的临时文件/备份契约明确区分；审批并通过专用资源的事务门禁前不得启用直接写入。没有验证时不能以普通注册表写入代替。任何事务标记只记录非敏感元数据，不把它称为密钥备份。
- 提交前失败须取消并验证旧态。提交后如需补偿，只在确认当前状态仍是本次写入时尝试事务恢复并验证原值/存在状态；发现第三方改动则不覆盖，无法确认则明确报错，不宣称恢复。进程在提交边界崩溃后先检测并安全报告未知结果，不承诺长期保存旧 Key 或自动回写。
- 持久回读成功后，尽力发出 Windows `WM_SETTINGCHANGE` 的 `Environment` 通知。通知失败不会撤销已验证的持久化，只提示终端需重启。应用不修改自己的进程变量，保存结果和进程可见状态分别展示。

## 方案权衡

首版直接写专用用户变量能减少切换窗口，既不引入 Relay 私有密钥副本，也不要求改变启动方式。系统变量、共享变量和终端自设值仍由各自来源管理。运行时注入不是首版替代方案：普通终端的现有命令不会自动获得 Relay 注入的值。

## 依据

- [Codex Profiles](https://learn.chatgpt.com/docs/config-file/config-advanced#profiles)、[配置优先级](https://learn.chatgpt.com/docs/config-file/config-basic#configuration-precedence)、[环境变量凭据](https://learn.chatgpt.com/docs/config-file/environment-variables#authentication-and-network)。
- [Windows 环境继承](https://learn.microsoft.com/en-us/windows/win32/procthread/environment-variables)、[设置变更通知](https://learn.microsoft.com/en-us/windows/win32/winmsg/wm-settingchange)、[注册表原生事务](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regopenkeytransactedw)。隔离原生验证观察到非事务的竞争写可以成功，但会取消竞争中的原生事务；不能假设外部写入被锁住。
- 当前默认切换见 `src-tauri/crates/codex-relay-core/src/services/provider_service.rs:1052`；现有事务入口见 `src-tauri/crates/codex-relay-core/src/services/transaction_service.rs:272`。
