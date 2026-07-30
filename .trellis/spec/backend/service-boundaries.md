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
- `provider_preference_service`：版本化 `provider-preferences.json` v1/v2/v3，保存 Provider 显示
  顺序、命名 Base URL 与 Relay 私有模型/Fast 偏好；不得保存第二份 URL 选择游标或把该元数据写入
  Codex Provider 块。
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

## Provider 写操作矩阵

| 操作 | 必须经过 | 关键验证 |
|---|---|---|
| 创建 | `ProviderService` → `TransactionService` | 初始 URL/Key 名称和值、四文件指纹、未知内容保留 |
| 常规编辑 | 同上 | 只修改名称、Wire API、模型；不得替换或清空 URL/Key |
| URL/Key 批量管理 | 同上 | 稳定 ID、顺序、唯一性、当前项/最后项删除保护 |
| URL/Key 独立选择 | 同上 | 当前/非当前语义、`config.toml` / `auth.json` 单向同步 |
| Fast 独立更新 | 同上，`UpdateProviderFast` | 目录能力、当前/非当前语义、v3 偏好、tier/feature 单向投影与回滚 |
| Provider 列表排序 | 同上，只写 preferences | 完整 ID 排列、指纹冲突、创建追加/删除移除、其他三文件字节不变 |
| 删除 | 同上 | 当前 Provider 禁止删除；其他 Provider/密钥保留 |
| 切换/同步 | 同上 | 目标存在且有密钥；顶层 Provider/模型/认证一致 |
| 恢复 | `BackupService` + `TransactionService` | 路径合法、恢复前备份、原字节与存在状态一致 |

## 扩展规则

新增受管文件或新写操作时，先扩展事务快照、备份、解析器、写后验证器、回滚验证和路径安全测试；不得只在 command 中增加一次文件写入。
