# 实施计划

## 行为切片与测试边界

### 切片 1：旧版元数据可恢复

- 公开边界：`BackupService::list_backups`、`load_snapshot`、`resolve_backup_file`。
- 输入：隔离临时目录中的无 `schemaVersion`、无 `preferencesExisted` 的 v1
  `metadata.json` 与固定快照文件。
- 预期：返回可恢复摘要、`legacyWithoutPreferences` 兼容状态、偏好快照为缺失，历史
  元数据字节不变；打开/恢复所需元数据解析不再报 `INVALID_BACKUP_METADATA`。
- mock 边界：不 mock `BackupService` 文件读取；使用 `tempfile`。不启动 Tauri 或 Windows
  记事本。

### 切片 2：异常目录不阻塞其他行为

- 公开边界：`list_backups` 返回的库存 DTO、`cleanup_old_backups` 与
  `TransactionService::restore_backup`/事务执行。
- 输入：一个有效备份与一个损坏 JSON 或未知 `schemaVersion` 的备份目录。
- 预期：有效备份仍可列出和恢复；异常目录以安全问题 DTO 返回、不会被清理；配置事务
  不因清理该目录失败而回滚。
- mock 边界：Rust 使用临时路径与现有 `FileOps`/事务 helper；不使用生产路径。

### 切片 3：用户获得匹配的操作

- 公开边界：`list_backups` typed service、`useBackups`、`BackupsView`。
- 输入：mock inventory，分别含旧版可恢复摘要和不可用备份。
- 预期：旧版卡片标识为旧版，确认框说明命名地址、模型与推理偏好影响；不可用区保留安全原因与“打开元数据”
  操作，点击只调用 `openBackupFile(directoryName, 'metadata.json')`；有效卡片仍可恢复。
- mock 边界：Vitest mock composable/client，不访问 Tauri IPC 或文件系统。

## 实施顺序

1. 在 core 备份模型中加入 `schemaVersion`、兼容状态、库存/不可用 DTO，并确保 camelCase
   序列化且不带密钥字段。
2. 为 `BackupService` 写失败测试：v1 无版本、无版本 v2、未知版本、损坏 JSON、保留扫描
   和元数据直接打开。
3. 实现唯一的版本化读取器及目录扫描；新版写入 v2，旧元数据只在内存规范化。
4. 调整清理只处理已验证摘要，并添加事务回归测试证明异常目录不会引起写入回滚。
5. 适配 command 和 Rust command 测试使用库存 DTO；不新增文件系统写入路径。
6. 更新 TypeScript DTO、`tauri.ts` 客户端测试和 `useBackups` 状态，使其传播库存结果。
7. 添加/更新 Vue 测试，再实现旧版卡片标签、恢复确认说明、不可用备份区与受限打开动作。
8. 检查 `AboutView` 的备份说明是否仍与用户可见契约一致；仅在内容不再准确时更新页面和
   测试。
9. 运行专项红绿测试、类型检查、Rust 格式/Clippy/测试与 `npm run check`；记录实际结果。
10. 评估并更新长期备份文件查看、数据保留和跨层契约规范，然后提交和归档。

## 验证命令

```powershell
npm run test:rust:lib -- backup_service
npm run test:rust:lib -- transaction_service
npm run test -- src/views/BackupsView.test.ts
npm run test -- src/services/tauri.test.ts
npm run typecheck
npm run check:frontend
npm run check:rust
npm run test:trellis
```

命令实际可用参数以本轮输出为准；若 Vitest 或 Cargo 过滤参数不兼容，保留失败证据并改用
等价的项目脚本，不声称未执行的检查通过。

## 风险与回滚点

- 版本识别错误可能使无版本 v2 丢失偏好快照。先以缺字段与有字段两个回归测试固定判断，
  再实现读取器。
- 把异常目录静默忽略会掩盖用户问题。库存 DTO 必须把它交给前端，并保留安全 message。
- 调整 `list_backups` DTO 是跨层破坏性接口改动；同一切片内更新 command、typed service、
  composable、视图和全部相关测试。
- `metadata.json` 的直接打开只能放宽元数据解析前置条件，不能放宽目录规范化、固定枚举、
  普通文件和根目录边界。
- 任一实现验证失败时，停止在当前红绿切片，保留历史文件，不运行真实升级或恢复。Git
  回滚仅限本任务新增代码，不撤销用户已有工作。

## 启动前审查

- [x] PRD 已合并用户确认的保守处置边界。
- [x] 设计覆盖无版本 v1、无版本 v2、版本化 v2、未知版本和损坏 JSON。
- [x] 实施顺序先测试再实现，并列出 Rust/前端公开边界。
- [x] 所有测试均使用隔离临时路径或 mock。
- [x] 用户已明确授权实施。

## 执行记录（2026-07-24）

- 旧版无版本元数据缺少 `preferencesExisted` 时，在内存中规范化为
  `legacyWithoutPreferences`；历史字节不改写，打开固定快照文件、清理和事务恢复均可继续执行。
- 真正损坏、缺失或未知版本的元数据作为 `UnavailableBackup` 返回；有效备份仍可恢复，
  异常目录不会被清理，也不再使新事务回滚。
- 旧版恢复的确认文案说明其不含命名地址、模型与推理偏好；恢复前当前四文件会先创建事务备份。
- 专项验证：`backup_service` 16 项、`transaction_service` 12 项、备份前端专项 29 项均通过。
- 全量验证：`npm run check:frontend` 通过（36 个测试文件、162 项测试）；
  `npm run check:rust` 通过（依赖图、fmt、Clippy、workspace 单元与集成测试）。
- 首次聚合 `npm run check` 受执行环境 60 秒时限中断；其已完成的 Trellis 测试为 8 项通过，
  后续改以独立 `check:frontend`、`check:rust` 和 `test:trellis` 取得完整证据。
