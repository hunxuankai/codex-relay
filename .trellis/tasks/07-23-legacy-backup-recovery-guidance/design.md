# 旧版备份元数据兼容与保留设计

## 决策

采用“版本化写入、读时规范化、异常目录隔离”的策略。已知历史格式在内存中转换为
当前备份语义，绝不原地修改 `metadata.json`；真正损坏或版本未知的目录保留在磁盘，
但不再阻塞其他备份、清理或配置事务。

不根据快照文件推断并修复元数据。元数据决定文件存在状态和恢复边界，推断会把无法
证明的状态变成可恢复状态，违背事务恢复的精确快照契约。

## 历史格式

| 原始形态 | 识别方式 | 规范化结果 | 用户状态 |
| --- | --- | --- | --- |
| v1 无版本 | 无 `schemaVersion` 且无 `preferencesExisted` | `preferencesExisted=false` | 旧版可恢复 |
| v2 无版本 | 无 `schemaVersion` 但有 `preferencesExisted` | 保留字段值 | 正常可恢复 |
| v2 已版本化 | `schemaVersion=2` | 严格解析当前结构 | 正常可恢复 |
| 未知版本 | `schemaVersion` 不是已支持值 | 不生成快照 | 不可安全恢复 |
| 损坏/缺字段 JSON | 不能满足对应结构 | 不生成快照 | 不可安全恢复 |

`appVersion` 只用于展示，不参与 schema 判定。新版创建备份写入
`schemaVersion: 2`。现有无版本 v2 仍可读取，避免升级后的过渡备份被误当作 v1。

## 后端边界

```text
metadata.json
  -> BackupService::read_metadata
  -> ParsedBackupMetadata { metadata, compatibility }
  -> BackupInventory { backups, unavailableBackups }
  -> Tauri list_backups command
  -> typed Tauri service / useBackups
  -> BackupsView
```

`BackupService` 是唯一解析和归一化元数据的所有者：

- `BackupMetadata` 保存新写入格式的版本和快照存在状态。
- `BackupCompatibility` 仅表达用户需要知道的恢复差异：`current` 或
  `legacyWithoutPreferences`。
- `BackupInventory` 返回可恢复摘要及 `UnavailableBackup` 列表。后者仅含目录名、稳定
  错误码和安全中文消息，不含绝对路径、原始 JSON 或快照内容。
- `read_metadata` 仍在直接恢复非兼容备份时返回稳定错误；扫描目录时将该错误转换为
  `UnavailableBackup`，而不是终止整个扫描。

目录扫描只能吞掉某一目录的元数据读取/版本错误。备份根目录无法读取仍然返回错误，
避免把基础设施故障伪装成空列表。

`cleanup_old_backups` 只对已成功规范化的摘要排序和清理。不可读取目录既不计入 20
份可验证备份，也不自动删除。这样保留优先于固定数量上限，也不会因遗留目录触发
事务回滚。

## 恢复与查看

已知 v1 解析为 `preferencesExisted=false`。恢复继续经过
`TransactionService::restore_backup`，因此会：

1. 创建当前四个受管文件的事务备份；
2. 按历史快照精确恢复 `config.toml`、`auth.json`、`providers.json`；
3. 删除当前的 `provider-preferences.json`，因为历史快照记录该文件当时不存在；
4. 重新读取并逐字节/存在状态验证；失败则走现有可验证回滚。

这不是数据丢失捷径：当前状态已先备份，且确认界面会说明命名地址、模型与推理偏好
不在旧快照中。

`open_backup_file` 对 `metadata.json` 先完成固定文件名、目录名、规范化根目录和普通
文件验证，再直接允许打开；它不依赖元数据能否解析。其他快照仍必须先解析元数据并
检查其存在状态。前端不读取文件内容或绝对路径。

## 前端体验

- `useBackups` 从单一数组改为持有 `BackupInventory` 的可恢复列表与不可用列表；传输级
  失败仍使用原有错误状态。
- `BackupCard` 对 `legacyWithoutPreferences` 显示“旧版备份”状态，保持恢复可用。
- 确认恢复旧版备份时，增加明确说明：该快照不含命名地址、模型与推理偏好，恢复会回到旧版
  状态，且当前配置已先备份。
- `BackupsView` 在有效卡片之外显示不可恢复备份区：说明备份未被修改或删除、当前
  版本无法安全恢复，并提供“打开元数据”按钮。该按钮只发出固定
  `metadata.json` 枚举值。
- 无可恢复备份但存在不可用备份时，不显示“暂无可恢复的事务备份”作为唯一内容。

## 错误与安全

未知版本使用专门的稳定错误码，例如 `UNSUPPORTED_BACKUP_METADATA_VERSION`；损坏和缺
字段使用 `INVALID_BACKUP_METADATA`。不可用 DTO 只向前端传安全 message/code。

所有 Rust 文件测试使用 `tempfile`；前端通过 typed Tauri client/composable mock 验证
行为。fixture 不能包含真实认证文件或密钥，且不得访问真实 Codex 或 Relay 数据目录。

## 取舍

仅给 `preferencesExisted` 添加 serde 默认值能修复 v1，但不能区分无版本 v2、未知未来
版本和损坏 JSON，也仍可能让一份坏目录影响清理。采用显式 schema reader 增加少量模型
和 DTO 代码，却将未来兼容路径、用户提示与数据保留行为放到一个明确边界中。

## 不变量

- 任何历史 `metadata.json` 都不被自动写回、迁移、删除或伪造。
- 不可安全恢复的备份没有恢复按钮。
- 已知 v1 的恢复语义保持精确快照，不隐式保留新版 Relay 私有偏好文件（命名地址、模型与推理偏好）。
- 任何恢复写入仍只通过 `TransactionService`。
- 备份页、DTO、事件、日志和测试输出不包含快照内容、绝对路径或 API Key。
