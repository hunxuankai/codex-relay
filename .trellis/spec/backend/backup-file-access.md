# 备份文件查看契约

## 1. 范围与触发条件

当界面需要列出或打开事务备份中的文件时适用。该能力只读，不属于配置事务写入，
但必须保持备份路径和密钥边界：前端只能接收固定文件名，不得接收文件内容或绝对路径。

## 2. 签名

```rust
enum BackupFileName { Config, Auth, Providers, Preferences, Metadata }

fn BackupService::resolve_backup_file(
    &self,
    directory_name: &str,
    file_name: BackupFileName,
) -> Result<PathBuf, AppError>;

#[tauri::command]
fn open_backup_file(
    state: State<'_, AppState>,
    directory_name: String,
    file_name: BackupFileName,
) -> CommandResult<()>;
```

前端对应接口：

```typescript
type BackupFileName = 'config.toml' | 'auth.json' | 'providers.json' | 'provider-preferences.json' | 'metadata.json'
openBackupFile(directoryName: string, fileName: BackupFileName): Promise<void>
```

## 3. 契约

- `BackupSummary.files` 只包含元数据允许且当前磁盘实际存在的固定文件名。
- `metadata.json` 对有效备份通常存在；其余文件由 `configExisted`、`authExisted`、
  `providersExisted`、`preferencesExisted` 限定。
- command 只把后端验证后的规范化路径作为 `notepad.exe` 的单个参数。
- 前端、事件、通知和日志不得包含备份文件内容、API Key 或绝对路径。
- 不支持 `settings-*.json`、`.corrupt-*` 或备份目录中的其他文件。

## 4. 验证与错误矩阵

| 条件 | 错误码 |
|---|---|
| 目录名为空、`.`、`..` 或含路径分隔符 | `INVALID_BACKUP_NAME` |
| 备份根目录不可访问 | `BACKUP_DIRECTORY_NOT_FOUND` |
| 所选备份目录不可访问 | `BACKUP_NOT_FOUND` |
| 元数据无法读取或解析 | `BACKUP_METADATA_READ_FAILED` / `INVALID_BACKUP_METADATA` |
| 元数据记录所选快照原先不存在 | `BACKUP_FILE_NOT_FOUND` |
| 元数据允许但文件当前缺失 | `BACKUP_FILE_MISSING` |
| 规范化后逃逸目录或目标不是普通文件 | `INVALID_BACKUP_PATH` |
| `notepad.exe` 启动失败 | `OPEN_BACKUP_FILE_FAILED` |

未知 `fileName` 不能反序列化为 `BackupFileName`，不得回退为字符串路径。

## 5. 良好、基线与错误用例

- 良好：有效目录 + `auth.json` 枚举，元数据记录存在且文件在该目录中，记事本打开文件。
- 基线：备份只包含 `metadata.json`，摘要只返回一个文件名，界面仍可展开查看。
- 错误：`..\\outside`、未知文件名、缺失快照、目录联接逃逸或符号链接到目录外，均拒绝打开。

## 6. 必需测试

- DTO：未知文件枚举反序列化失败；摘要序列化不包含内容或路径。
- `BackupService`：断言实际文件列表、元数据不存在状态、磁盘缺失、目录穿越和规范化路径。
- 系统边界：断言程序名为 `notepad.exe`，参数只有验证后的文件路径。
- command：断言安全错误码和消息不泄漏请求路径。
- 前端：断言精确 command 名、camelCase 参数、单卡片展开、文件按钮和失败消息。

所有文件测试必须使用 `tempfile` / `AppPaths::for_test`，不得触及真实用户目录。

## 7. 错误与正确做法

错误：

```typescript
await invoke('open_backup_file', { path: backupDirectory + '/' + clickedName })
```

正确：

```typescript
await openBackupFile(backup.directoryName, 'auth.json')
```

Rust 侧必须再次用固定枚举、元数据、规范化根目录和普通文件检查解析路径，不能信任前端
拼接结果，也不能把解析后的路径返回前端。
