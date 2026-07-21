# 备份文件列表与记事本打开设计

## 架构边界

功能沿用现有调用链：

```text
BackupsView
→ useBackups
→ src/services/tauri.ts
→ backup_commands
→ AppState / ProviderService
→ BackupService 路径解析
→ notepad.exe
```

前端只接收固定文件名枚举，不接收文件内容或绝对路径。Rust 负责将备份目录名与文件枚举
解析为经过验证的本地路径，再启动记事本。

## DTO 与接口

新增固定备份文件枚举，序列化值为：

- `config.toml`
- `auth.json`
- `providers.json`
- `metadata.json`

`BackupSummary` 增加 `files` 字段。后端根据 `BackupMetadata` 的三个存在状态生成列表，
并始终追加 `metadata.json`。前端直接渲染该列表，不自行拼接未知文件名。

新增 typed command：

```text
open_backup_file(directoryName, fileName) -> CommandResult<()>
```

`fileName` 在 Rust 侧反序列化为固定枚举，因此不能传入任意相对或绝对路径。

## 路径验证与打开行为

`BackupService` 提供备份文件路径解析：

1. 复用备份目录名校验，拒绝空值、`.`、`..`、斜杠和反斜杠。
2. 规范化备份根目录与目标备份目录，确认目标目录仍是备份根目录的直接子目录，拒绝
   目录联接、符号链接或其他逃逸到根目录外的情况。
3. 读取并解析 `metadata.json`。
4. 对三个快照文件核对元数据中的存在状态；不存在时返回稳定错误。
5. 规范化目标文件，确认它是普通文件且仍位于目标备份目录内。
6. 只把验证后的路径作为单个参数交给 `notepad.exe`。

打开成功不显示成功通知。启动失败、文件缺失、元数据无效或路径非法时返回安全中文错误，
内部错误细节不进入前端。

## Vue 交互与状态

`BackupsView` 保持页面级编排；新增 `BackupCard` 负责单条备份的元数据、动作区和文件
列表，通过 typed props 接收 `backup`、`expanded`、`busy`，并通过 typed emits 上报
`toggle`、`open-file` 和 `restore`。展开状态是纯局部 UI 状态，使用一个
`expandedDirectoryName: string | null` 表示，因此天然保证同一时间只展开一个备份。

`useBackups` 增加显式 `openFile(directoryName, fileName)` 动作，复用现有 `busy` 与
`error` 状态。打开期间禁用文件按钮和恢复入口，失败后展示 `AppNotification`，不刷新
备份列表，也不产生成功消息。

`BackupCard` 的文件项使用原生 `button`，可通过键盘访问；展开按钮
提供 `aria-expanded` 和指向列表的 `aria-controls`。

## 兼容性与范围

- 仅支持 Windows，使用系统 `notepad.exe`，符合当前产品平台边界。
- 旧备份无需迁移，文件列表由现有元数据计算。
- 不修改备份目录结构、恢复行为、保留数量和密钥存储。
- 不支持打开 `settings-*.json` 或 `.corrupt-*` 文件。

## 测试策略

- Rust：先为文件枚举列表、目录穿越、元数据不存在状态、缺失文件和规范化路径逃逸编写
  失败测试；为记事本命令构造验证程序名和唯一文件参数。
- command：验证命令返回统一安全结果，不返回路径或内容。
- TypeScript service：验证精确 command 名和 camelCase 参数。
- composable：验证打开动作的 busy/error 行为。
- Vue：验证展开/收起、单卡片展开、实际文件列表、打开动作和错误消息。
