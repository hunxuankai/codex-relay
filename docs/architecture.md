# Codex Relay 架构

## 1. 总体结构

Codex Relay 使用四层结构：

```text
Vue 视图与组件
  ↓ typed services / composables
Tauri command adapters
  ↓ AppState 共享服务与事务互斥
Rust domain services
  ↓ path / atomic file / backup / logging infrastructure
Windows 当前用户文件与系统集成
```

前端不直接访问文件系统。`src/services/tauri.ts` 是唯一 `invoke` 边界，负责精确命令名、camelCase DTO、`CommandResult<T>` 解包与安全错误。`useProviders`、`useHealth`、`useBackups`、`useSettings` 暴露只读状态和显式动作，并使用序列号防止旧请求覆盖新事件。

## 2. Rust 模块职责

- `path_service`：解析生产/测试根目录，拒绝测试回退到真实用户路径。
- `config_service`：使用 `toml_edit` 解析、校验并局部编辑 Provider。
- `provider_secret_service`：读取与序列化版本化 `providers.json`，损坏时保留原件。
- `auth_service`：读取当前密钥并生成固定格式 `auth.json`。
- `transaction_service`：应用级异步写锁、指纹冲突、快照、备份、原子写、验证和回滚。
- `backup_service`：创建、列出、加载与清理事务备份，最多 20 份。
- `provider_service`：组合配置、密钥、认证与事务服务，提供 CRUD、切换、导入和恢复。
- `settings_service` / `autostart_service`：持久化设置并核对 Windows 实际自启状态。
- `self_check_service`：关键自检和扩展自检。
- `file_watch_service`：监控三个文件，抑制应用自身写入，发出脱敏事件。
- `safe_log`：滚动日志、保留策略与秘密脱敏。
- `tray`：早期托盘、菜单模型、切换忙锁、窗口生命周期、通知和事件刷新。
- `commands`：一次委托到服务并统一映射 `CommandResult<T>`，不返回堆栈和秘密。

## 3. 配置与数据所有权

`config.toml` 是 Provider 非秘密配置的主要来源；`providers.json` 是每个 Provider 密钥的应用存储；`auth.json` 是 Codex 当前生效密钥。应用不能把 `providers.json` 当作 Provider 定义的唯一真相，也不能从普通列表返回密钥。

`settings.json` 只保存应用设置。窗口位置由生命周期处理器防抖更新；开机启动状态同时包含设置期望值和 Windows 实际值。

## 4. 启动流程

1. Single Instance 插件首先注册；第二实例只显示并聚焦已有窗口。
2. 立即创建托盘占位菜单。
3. 解析生产路径，初始化脱敏日志、设置、Provider 服务和自启动后端。
4. 安装共享 `AppState`、文件监控与日志守卫。
5. 从磁盘刷新托盘，恢复仍与显示器相交的窗口边界。
6. 根据 `--autostart` 参数与真实设置决定显示窗口还是仅托盘。
7. 同步运行关键自检，后台运行扩展自检并通过事件更新前端。

启动阶段不调用模型接口。Codex CLI 探测仅执行带超时的本地 `codex --version`。

## 5. Provider CRUD 数据流

1. Vue 编辑器完成即时校验并提交 typed DTO 与文件指纹。
2. command adapter 只委托一次到 `ProviderService`。
3. 服务重新验证 ID、名称、Base URL、Wire API、模型和密钥动作。
4. 事务服务获取全局写锁并读取最新快照。
5. 指纹不一致时返回外部修改冲突，不创建写入结果。
6. `toml_edit` 仅修改目标 Provider，密钥存储只修改目标 ID。
7. 创建备份、写临时文件、解析验证、替换、再读验证。
8. 成功后命令层刷新托盘、Provider 事件和安全通知。

当前 Provider 不允许删除。清除当前密钥时不能立即同步 `auth.json`，界面会明确说明现有生效密钥可能继续被 Codex 使用。

## 6. Provider 切换流程

切换入口包括主界面和托盘，两者调用同一个 `ProviderService::switch_provider`。托盘和服务层都有忙状态/事务锁，避免重复点击和并发写坏文件。

服务验证目标 Provider 合法且存在密钥，局部更新顶层 `model_provider`、`cli_auth_credentials_store = "file"`；目标有默认模型时更新顶层 `model`，否则保留现有模型。随后把目标密钥写入 `auth.json`。成功结果刷新界面、托盘和 Windows 通知，并提示重启 Codex。

## 7. 文件监控与事件

文件监控覆盖 `config.toml`、`auth.json`、`providers.json`，对突发事件防抖。应用事务开始时安装应用写入守卫，事务结束后记录最终指纹；监控器只抑制匹配的自身写入。外部修改会直接发出 `config-files-changed`，从磁盘重建托盘并发出 `providers-changed`，同时在后台运行扩展自检并发出 `self-check-completed`。

其他事件来源需要区分：

- `settings-changed` 只由保存设置或开机启动变更触发。
- `app-notification` 只用于显式 Provider 操作、托盘操作及其安全成功/失败消息。

事件 payload 只包含 DTO、指纹、状态或安全消息，不含文件全文和密钥。

## 8. 自检流程

关键自检覆盖目录可写性、必要文件存在性、设置加载、Provider 列表和当前 Provider，适合启动阶段。扩展自检增加 TOML/JSON 解析、Provider 校验、当前认证一致性、Codex CLI、自启一致性、事务标记、备份数量与外部修改。

等级为 `normal`、`warning`、`error`。缺少/超时的 CLI 是 warning；损坏配置、密钥不一致和残留事务是 error。

## 9. 备份恢复流程

每个事务用同一事务 ID 生成目录，保存三个受管文件的原始字节和无密钥元数据。恢复前先为当前状态创建恢复前备份，然后按原存在状态恢复或删除文件，再解析验证并刷新 Provider、托盘和自检。界面分别报告恢复本身与后续跨资源刷新结果。

## 10. 窗口、托盘与系统集成

主窗口关闭默认隐藏到托盘。只有显式退出会设置 `exit_requested`，允许 Tauri 进程退出；普通 `ExitRequested` 会被阻止。窗口移动/缩放采用短防抖保存，越出所有显示器的旧位置不恢复。

Autostart 使用当前用户注册，设置页始终显示插件查询的实际状态。应用不创建服务、不写所有用户启动项、不要求管理员权限。
