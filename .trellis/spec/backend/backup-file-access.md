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

## 场景：旧版元数据兼容与不可用备份库存

### 1. 范围与触发条件

当新增备份元数据字段、发布后仍需读取旧备份，或单个备份目录的元数据无法安全解析时适用。
升级不得原地迁移、删除或推断历史 `metadata.json`；一份异常目录也不得遮蔽可恢复备份或阻断
后续事务清理。

### 2. 签名

```rust
const BACKUP_METADATA_SCHEMA_VERSION: u32 = 2;

enum BackupCompatibility { Current, LegacyWithoutPreferences }

struct BackupInventory {
    backups: Vec<BackupSummary>,
    unavailable_backups: Vec<UnavailableBackup>,
}

fn BackupService::list_backups(&self) -> Result<BackupInventory, AppError>;

#[tauri::command]
fn list_backups(state: State<'_, AppState>) -> CommandResult<BackupInventory>;
```

前端对应接口：

```typescript
interface BackupInventory {
  backups: readonly BackupSummary[]
  unavailableBackups: readonly UnavailableBackup[]
}

type BackupCompatibility = 'current' | 'legacyWithoutPreferences'
```

### 3. 契约

- 新写入 `metadata.json` 必须包含 `schemaVersion: 2`。
- 无 `schemaVersion` 且无 `preferencesExisted` 的已知旧格式仅在内存中规范化为
  `preferencesExisted: false` 和 `legacyWithoutPreferences`；不得改写原元数据或从快照文件猜测该值。
- 无 `schemaVersion` 但已有 `preferencesExisted` 的过渡格式保留该字段，按 `current` 读取。
- 仅 `schemaVersion: 2` 可按当前结构严格反序列化；未知版本或损坏/缺字段 JSON 不生成可恢复快照。
- `UnavailableBackup` 只可返回目录名、稳定错误码、安全中文消息和
  `canOpenMetadata`；不得返回绝对路径、原始 JSON 或快照内容。
- 清理只对 `backups` 排序和删除；`unavailableBackups` 必须原样保留且不计入 20 份可验证备份。
- `metadata.json` 的受限打开可绕过内容解析，但仍必须经过固定文件枚举、备份根目录和普通文件校验。
- 前端只给 `legacyWithoutPreferences` 显示可恢复标签和恢复影响说明；不可用项只能打开元数据，不能出现恢复入口。

### 4. 验证与错误矩阵

| 条件 | 列表结果 | 直接恢复/快照读取 |
|---|---|---|
| 无版本且缺少 `preferencesExisted` | 可恢复，`legacyWithoutPreferences` | 按偏好文件不存在恢复 |
| 无版本且具有 `preferencesExisted` | 可恢复，`current` | 按记录值恢复 |
| `schemaVersion: 2` 且字段完整 | 可恢复，`current` | 正常恢复 |
| 未知 `schemaVersion` | `UNSUPPORTED_BACKUP_METADATA_VERSION` 不可用项 | 返回同一稳定错误码 |
| 损坏 JSON、错误类型或缺必填字段 | `INVALID_BACKUP_METADATA` 不可用项 | 返回同一稳定错误码 |
| 缺失/无法读取 `metadata.json` | `BACKUP_METADATA_READ_FAILED` 不可用项 | 返回同一稳定错误码 |

备份根目录本身无法枚举仍是基础设施错误，不能伪装成空库存。

### 5. 良好、基线与错误用例

- 良好：升级后无版本旧备份仍可列出和恢复；恢复前的当前四文件快照保留，旧快照缺失的
  `provider-preferences.json` 被精确恢复为不存在。
- 基线：一个有效目录和一个损坏 `metadata.json` 同时存在时，有效项继续可恢复，损坏项保留并只显示安全操作。
- 错误：为修复列表错误而给历史 JSON 自动补写字段、根据磁盘中碰巧存在的偏好文件推断状态，或让清理删除不可解析目录。

### 6. 必需测试

- `BackupService`：覆盖无版本 v1、无版本 v2、`schemaVersion: 2`、未知版本、损坏 JSON、直接打开损坏元数据、旧版打开固定快照文件和清理保留。
- `TransactionService`：旧版恢复前先备份当前偏好文件，恢复后按旧快照删除该文件；异常备份目录不阻断新事务。
- command/typed client/composable：断言 `BackupInventory` 使用 camelCase 传递，普通错误和 DTO 不含快照内容、密钥或绝对路径。
- 视图：断言旧版标签和完整恢复影响说明、不可用区无恢复按钮、只调用
  `openBackupFile(directoryName, 'metadata.json')`，并且刷新期间不同时渲染旧库存。

所有文件测试必须使用 `tempfile` / `AppPaths::for_test`，不得访问真实用户目录。

### 7. 错误与正确做法

错误：把单个解析错误直接从目录扫描向上传播，导致整个列表和后续事务失败。

```rust
let metadata = read_metadata(&metadata_path)?;
backups.push(summary_from(metadata));
```

正确：只在目录扫描边界把可识别的元数据错误投影为受限 DTO，其余基础设施错误仍向上传播。

```rust
match read_metadata(&directory.join(METADATA_FILE_NAME)) {
    Ok(parsed) => {
        let files = available_backup_files(&directory, &parsed.metadata);
        backups.push(BackupSummary {
            directory_name,
            metadata: parsed.metadata,
            files,
            compatibility: parsed.compatibility,
        });
    }
    Err(error) if is_unavailable_backup_metadata_error(&error) => {
        unavailable_backups.push(unavailable_backup(directory_name, &directory, &error));
    }
    Err(error) => return Err(error),
}
```

这样既保留历史目录，也不会把不可证明的快照伪装成可恢复备份。
