# 服务边界

## Crate 边界

- `codex-relay-core` 拥有 Provider、设置、事务、备份、配置/密钥存储及其网络/路径基础设施。
- Tauri 根 crate 只拥有 command、AppState、tray/window/plugin 组合，以及与桌面生命周期直接相关的
  autostart、file watch、自检和日志适配。
- `TauriAutostartBackend` 在根 crate 实现 core-neutral `AutostartBackend`；不得把 `AppHandle` 或
  Tauri plugin 类型传入 core。
- 根 crate re-export core 的稳定模块路径以保持 command/测试调用兼容；不得复制 Provider、事务、
  路径或日志脱敏逻辑形成第二套实现。

## 基础设施

- `path_service`：解析生产/测试根目录，并在测试模式拒绝真实用户目录。
- `atomic_file`：同目录临时文件、flush、解析验证、替换和写后读取；不包含 Provider 规则。
- `file_fingerprint`：用存在状态、长度、修改时间和 SHA-256 表示编辑基线。
- core `safe_log`：只能在 core 内读取 `AppError` 内部详情并完成秘密脱敏/安全日志格式化。
- 根 crate `safe_log`：滚动日志初始化与保留数量；复用 core 脱敏函数，不公开原始错误详情。

## 领域服务

- `config_service`：Provider 读取、校验和 TOML 局部修改；Provider ID 创建后不可改；Fast 投影只管理顶层 `service_tier`，开启时单向确保 `[features].fast_mode=true`。
- `provider_secret_service`：版本化 `providers.json` v1/v2，拥有命名密钥、稳定 ID、顺序、
  唯一性和 `selectedApiKeyId` 校验；损坏时保留副本并返回安全错误。
- `auth_service`：读取当前密钥并渲染只含 `OPENAI_API_KEY` 的规范 JSON。
- `backup_service`：事务快照、列表、加载和最多 20 份清理。
- `transaction_service`：锁、指纹、备份、临时写、验证和回滚。
- `provider_service`：组合配置、密钥、认证与事务，作为主界面和托盘的唯一业务入口。
- `provider_preference_service`：版本化 `provider-preferences.json` v1/v2/v3/v4，保存 Provider 显示
  顺序、命名 Base URL、Relay 私有模型/Fast 偏好与可选无密 `connectionOverride`；不得保存第二份
  URL 选择游标、URL/Key 值副本，或把该元数据写入 Codex Provider 块。
- `settings_service` / `autostart_service`：保存偏好并核对 Windows 实际自启状态。
- `self_check_service`：关键自检和扩展自检。
- `file_watch_service`：防抖、写入抑制和脱敏变化事件。

## Command 契约

```typescript
interface CommandResult<T> {
  success: boolean
  data?: T
  error?: { code: string; message: string }
}
```

command 只能：解析参数 → 调用一次服务 → 映射 `CommandResult<T>` → 触发必要的安全刷新。不得直接读写四个受管文件，不得返回堆栈、内部路径错误、文件全文或普通列表中的密钥。只有 `get_provider_api_keys_for_management` 可在显式管理边界返回完整密钥，并必须使用脱敏 `Debug`。

## Scenario：Tauri 异步命令的传输结果

### 1. 范围/触发条件

主应用与发布控制台中使用 `State<'_, AppState>` 等引用或生命周期参数的异步 Tauri command。

### 2. 签名

```rust
use tauri::ipc::InvokeError;

pub async fn run_extended_self_check(
    state: tauri::State<'_, AppState>,
) -> Result<CommandResult<HealthReport>, InvokeError>;
```

### 3. 契约

- Tauri 宏要求此类 async command 保留外层 `Result`；错误参数统一使用框架的 `InvokeError`。
- 业务成功和失败继续返回 `Ok(CommandResult<T>)`。业务错误保留安全 `code/message`，不得改成外层 `Err`，以免改变前端 Promise 与 DTO 契约。
- 外层只满足 IPC 传输接口；Tauri 拆开 Result 后序列化 DTO，因此不能向前端新增 `Ok`/`Err` JSON 包装。
- 不使用 `Result<_, ()>` 或抑制 `clippy::result_unit_err`；不靠降低工具链绕过严格检查。
- 当前锁定 serde 1.0.229 未实现 `Infallible: Serialize`，Tauri 2.11.5 也没有相应 `Into<InvokeError>` 转换，不能直接替换为 `Infallible`。

### 4. 验证与错误矩阵

| 条件 | 必需结果 |
| --- | --- |
| 带生命周期参数的 async command 去掉外层 Result | Tauri 宏拒绝编译 |
| 外层错误为 `()`，使用 Rust/Clippy 1.98 且 `-D warnings` | `result_unit_err` 阻止通过 |
| 外层错误为 `InvokeError`，业务失败仍在 DTO 中 | IPC 返回既有 `success=false` 和安全错误，不新增 Promise 拒绝路径 |
| 本地与 CI 工具链版本不同 | 记录 `rustc --version`、`cargo clippy --version`，用实际 CI 版本核验修复 |

### 5. 良好/基线/错误用例

- 良好：外层 `Result<CommandResult<T>, InvokeError>`，函数体只包装既有安全 DTO。
- 基线：同步 command 继续直接返回 `CommandResult<T>`，无需机械增加 Result。
- 错误：用 `Err(InvokeError::from_error(error))` 替代既有业务错误映射，泄漏内部错误并改变前端行为。

### 6. 必需测试

- 使用实际 CI 工具链运行 workspace `cargo fmt --check` 和 `cargo clippy --workspace --all-targets --all-features -- -D warnings`，覆盖两套应用的宏展开。
- 运行既有 command、typed service 与安全错误序列化测试，确认 DTO、参数与错误消息不变。
- 发布前运行完整 `npm run check` 和普通构建；旧工具链的通过记录不能替代新版 Clippy 的证据。

### 7. 错误与正确做法

```rust
// 错误：单位错误会触发严格 Clippy。
async fn command(...) -> Result<CommandResult<T>, ()> { ... }

// 正确：保留 Tauri 传输 Result 和既有业务响应。
async fn command(...) -> Result<CommandResult<T>, InvokeError> {
    Ok(existing_safe_command_result)
}
```

## Provider 写操作矩阵

| 操作 | 必须经过 | 关键验证 |
|---|---|---|
| 创建 | `ProviderService` → `TransactionService` | 初始 URL/Key 名称和值、四文件指纹、未知内容保留 |
| 常规编辑 | 同上 | 只修改名称、Wire API、模型；不得替换或清空 URL/Key |
| URL/Key 批量管理 | 同上 | 稳定 ID、顺序、唯一性、当前项/最后项删除保护 |
| URL/Key 独立选择 | 同上 | 当前/非当前语义、`config.toml` / `auth.json` 单向同步 |
| Fast 独立更新 | 同上，`UpdateProviderFast` | 目录能力、当前/非当前语义、v4 偏好、tier/feature 单向投影与回滚 |
| Provider 列表排序 | 同上，只写 preferences | 完整 ID 排列、指纹冲突、创建追加/删除移除、其他三文件字节不变 |
| 删除 | 同上 | 当前 Provider 禁止删除；其他 Provider/密钥保留 |
| 普通切换/创建后启用 | 同上 | 有覆盖时先恢复旧目标块并清除关系；再验证新顶层 Provider/模型/认证一致 |
| 应用/更新连接 | 同上，`SyncCurrentProvider` | 输入只有来源 ID/四文件指纹；顶层身份不变，目标 URL、当前认证与 v4 关系一致 |
| 恢复自身连接 | 同上，`RestoreCurrentProvider` | 恢复点完整；按当前顶层身份决定恢复 URL+认证或仅旧目标 URL；关系清除 |
| 恢复 | `BackupService` + `TransactionService` | 路径合法、恢复前备份、原字节与存在状态一致 |

## 扩展规则

新增受管文件或新写操作时，先扩展事务快照、备份、解析器、写后验证器、回滚验证和路径安全测试；不得只在 command 中增加一次文件写入。

连接 action/status/role、禁用原因和安全条目名称必须由 `ProviderService` 的单一 resolver 投影。
command、前端和自检不得分别按 URL/Key 值重新推导来源；失效关系必须保留并返回稳定错误，直到
显式恢复或经受管普通切换安全清除。关系来源和目标的删除分别返回
`PROVIDER_CONNECTION_SOURCE_DELETE_FORBIDDEN` 与 `PROVIDER_CONNECTION_TARGET_DELETE_FORBIDDEN`。
