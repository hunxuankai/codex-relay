# 服务边界

## 基础设施

- `path_service`：解析生产/测试根目录，并在测试模式拒绝真实用户目录。
- `atomic_file`：同目录临时文件、flush、解析验证、替换和写后读取；不包含 Provider 规则。
- `file_fingerprint`：用存在状态、长度、修改时间和 SHA-256 表示编辑基线。
- `safe_log`：滚动日志、保留数量和秘密脱敏。

## 领域服务

- `config_service`：Provider 读取、校验和 TOML 局部修改；Provider ID 创建后不可改。
- `provider_secret_service`：版本化 `providers.json`；损坏时保留副本并返回安全错误。
- `auth_service`：读取当前密钥并渲染只含 `OPENAI_API_KEY` 的规范 JSON。
- `backup_service`：事务快照、列表、加载和最多 20 份清理。
- `transaction_service`：锁、指纹、备份、临时写、验证和回滚。
- `provider_service`：组合配置、密钥、认证与事务，作为主界面和托盘的唯一业务入口。
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

command 只能：解析参数 → 调用一次服务 → 映射 `CommandResult<T>` → 触发必要的安全刷新。不得直接读写三个受管文件，不得返回堆栈、内部路径错误、文件全文或普通列表中的密钥。

## Provider 写操作矩阵

| 操作 | 必须经过 | 关键验证 |
|---|---|---|
| 创建/编辑 | `ProviderService` → `TransactionService` | 字段、指纹、目标密钥动作、未知内容保留 |
| 删除 | 同上 | 当前 Provider 禁止删除；其他 Provider/密钥保留 |
| 切换/同步 | 同上 | 目标存在且有密钥；顶层 Provider/模型/认证一致 |
| 恢复 | `BackupService` + `TransactionService` | 路径合法、恢复前备份、原字节与存在状态一致 |

## 扩展规则

新增受管文件或新写操作时，先扩展事务快照、备份、解析器、写后验证器、回滚验证和路径安全测试；不得只在 command 中增加一次文件写入。
