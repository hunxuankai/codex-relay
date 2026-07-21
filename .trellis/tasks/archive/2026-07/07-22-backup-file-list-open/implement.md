# 备份文件列表与记事本打开实施计划

## 当前状态

- 阶段：实施、全量验证和工作提交完成，准备归档。
- 已确认：卡片内展开、同时只展开一个、四类文件直接用记事本打开、无敏感信息确认。
- 已加载：backend、frontend、security、testing 规范，跨层指南与 Vue 核心参考。

## 实施步骤

1. 加载实施前规范与 Vue 核心参考，确认工作树和安全路径。
2. 行为切片一：备份摘要公开固定文件列表。
   - [x] 先添加 Rust 失败测试，确认因缺少文件枚举与摘要字段失败。
   - [x] 实现 `BackupFileName` 与 `BackupSummary.files`，同步 TypeScript DTO。
   - [x] `cargo test ... backup` 与 `npm run typecheck` 通过。
3. 行为切片二：后端安全解析并用记事本打开备份文件。
   - [x] 先添加路径解析与 typed IPC 红色测试，确认因接口缺失失败。
   - [x] 覆盖元数据不存在状态、磁盘缺失、目录穿越、未知文件枚举和安全 command 错误。
   - [x] 实现 `BackupService` 路径解析、`AppState` 记事本启动、command 注册和 typed service。
   - [x] Rust 备份/命令专项测试与 `src/services/tauri.test.ts` 通过。
4. 行为切片三：备份页卡片内展开和打开交互。
   - [x] 先添加 composable 与 Vue 红色测试，确认因公开动作和按钮缺失失败。
   - [x] 实现 `useBackups.openFile`、`BackupCard`、单卡片展开、可访问文件按钮和错误状态。
   - [x] 3 个前端专项测试文件共 24 项测试通过。
5. [x] 运行 Trellis 综合检查：格式、类型、前端测试、Rust 测试和构建级检查。
6. [x] 评估并更新项目规范/README，记录验证证据。
7. [x] 工作提交已创建；任务由 Trellis 收尾流程归档。

## 预计修改文件

- `src-tauri/src/models/backup.rs`
- `src-tauri/src/services/backup_service.rs`
- `src-tauri/src/services/provider_service.rs`
- `src-tauri/src/app_state.rs`
- `src-tauri/src/commands/backup_commands.rs`
- `src-tauri/src/lib.rs`
- `src/types/backup.ts`
- `src/services/tauri.ts`
- `src/composables/useBackups.ts`
- `src/components/BackupCard.vue`
- `src/views/BackupsView.vue`
- 对应 Rust、service、composable 和 Vue 测试

## 验证命令

```powershell
npm test -- --run src/services/tauri.test.ts src/composables/useBackups.test.ts src/views/BackupsView.test.ts
npm run typecheck
npm run lint
cargo test --manifest-path src-tauri/Cargo.toml backup
cargo test --manifest-path src-tauri/Cargo.toml
npm test -- --run
npm run build
```

## 风险与回滚点

- 测试和开发不得访问真实 `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay`；所有
  Rust 文件测试使用 `tempfile` / `AppPaths::for_test`。
- 路径校验必须在启动 `notepad.exe` 前完成，前端参数不能直接成为路径。
- 如果 DTO 扩展破坏现有测试，先同步所有固定 fixture，再继续下一切片。
- 每个行为切片保持独立绿色；出现回归时只回退当前切片的局部改动。

## 验证证据

- 红色测试：依次确认缺少 `BackupFileName/files`、`resolve_backup_file`、
  `openBackupFile`、`useBackups.openFile` 和“查看文件”按钮时按预期失败。
- 调试回归：删除已备份的 `auth.json` 后，列表最初错误显示 `[Auth, Metadata]`；修复后
  只显示 `[Metadata]`。
- 首轮检查真实发现并修复：`rustfmt --check` 格式差异、仓库不存在 `npm run lint`、
  Vue readonly 数组类型不兼容。
- `npm run check`（最终新鲜运行）：Trellis 8 项、前端 23 个文件/110 项、Rust 118 项、
  路径安全 2 项、Provider 工作流 1 项全部通过；同时通过 vue-tsc、rustfmt、Clippy。
- `npm run build`：退出码 0，生成 Release 主程序与 Windows x64 NSIS 安装器。
- `CodexRelay.exe`：16,679,424 字节，SHA-256
  `87EA9568A760296E7F583FC2E26EA5CE0E3FC4301DAB74DA3161ED015D469650`。
- `Codex Relay_0.1.2_x64-setup.exe`：3,992,099 字节，SHA-256
  `C6E6A514F0E2D4549FCC6C8B70DEAF2A2A2F53B1F9269F0DEF0B1A3133F16F20`。
- `git diff --check` 通过；受管文件中未跟踪真实 `auth.json`、`providers.json`、备份或
  `transaction.json`。高置信度密钥扫描仅命中 `safe_log.rs` 既有脱敏测试字符串。
- 未执行真实用户备份或记事本人工点击验证；自动化文件测试全部使用临时目录。
