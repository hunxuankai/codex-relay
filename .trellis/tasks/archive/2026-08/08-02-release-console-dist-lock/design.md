# 发布控制台交付目录隔离设计

## 1. 设计目标

在不改变主应用 Vite/Tauri 构建契约和发布门禁编排的前提下，把便携发布控制台移出主前端
`dist` 输出树，从结构上消除“运行中的控制台 EXE 阻止 Vite 清空 `dist`”的自锁故障。

## 2. 现有数据流与故障点

```text
npm run build:release-console
  → 构建 src-tauri/target/release/CodexRelayReleaseConsole.exe
  → postbuild 复制到 dist/release-console/CodexRelayReleaseConsole.exe
  → 维护者从该路径运行控制台
  → ordinary-build 执行 npm run build
  → tauri build 的 beforeBuildCommand 执行 npm run build:frontend
  → Vite 清空 dist
  → Windows 拒绝删除正在运行的控制台 EXE
  → Vite EBUSY，npm 退出 1，候选回滚
```

故障来自交付目录所有权冲突：`dist` 同时被当作主应用可整体重建的前端输出目录和控制台的稳定运行目录。

## 3. 目标数据流

```text
npm run build:release-console
  → 构建 src-tauri/target/release/CodexRelayReleaseConsole.exe
  → postbuild 复制到 artifacts/release-console/CodexRelayReleaseConsole.exe
  → 维护者从 artifacts 运行控制台

ordinary-build
  → Vite 只清理 dist
  → artifacts 不受影响
  → 主应用继续编译并生成 NSIS
```

`artifacts/` 是仓库本地、Git 忽略的维护者产物命名空间，不属于主应用 Tauri 的 `frontendDist`、资源、
bundle 或 updater 输入。

## 4. 文件与契约变化

### 4.1 包装脚本

`scripts/package-release-console.ps1` 的默认 `DestinationDirectory` 改为
`artifacts/release-console`。显式传入 `-SourcePath` 或 `-DestinationDirectory` 时行为不变；脚本继续
使用真实文件复制、枚举大小/时间并计算 SHA-256。

### 4.2 Git 忽略

`.gitignore` 新增 `artifacts/`。不删除旧 `dist/release-console`，也不迁移历史本地二进制；下一次构建
自然生成新路径，维护者按文档启动新副本。

### 4.3 文档与规范

README 和 `.trellis/spec/release/publishing.md` 的当前操作入口统一改为新路径。历史归档任务保留当时
真实证据，不做追溯改写。

### 4.4 结构测试

`src/release-console-structure.test.ts` 继续通过临时目录执行真实 PowerShell 包装脚本，并新增/调整
以下公开契约：

- 文档必须包含新路径；
- 包装脚本默认文本必须指向 `artifacts/release-console`；
- Git 忽略规则必须覆盖 `artifacts/`；
- 默认控制台交付路径不得位于根 `dist` 树内。

测试不读取或启动真实用户目录中的 Codex/Relay 数据，不 mock 文件复制或 SHA-256 计算。

## 5. 可观察行为切片与 TDD 边界

### 切片 1：默认交付路径与主前端输出分离

- 公开接口：根 package 的 `postbuild:release-console` 命令、包装脚本默认输出、README/发布规范路径。
- 输入/操作：运行结构测试并检查仓库配置与文档。
- 预期结果：默认路径为 `artifacts/release-console/...`，`artifacts/` 被忽略，且路径不属于 `dist`。
- RED：先修改测试期望，现有脚本/文档仍使用旧路径，测试因预期差异失败。
- GREEN：最小修改脚本、忽略规则和文档，使专项测试通过。

### 切片 2：运行控制台期间主构建不再自锁

- 公开接口：`npm run build:release-console`、`npm run build:frontend`、`npm run build`。
- 输入/操作：从新交付路径启动本轮构建的控制台，保持进程运行，再执行主前端和普通构建。
- 预期结果：两个构建命令退出 0，控制台进程仍存活；构建结束后只关闭本轮明确启动的 PID。
- mock 边界：不 mock Windows 文件锁、Vite 清理、PowerShell 复制或 Tauri 构建；进程启动使用成对安全
  Relay 路径覆盖，不接触真实 `.codex` 或 `%LOCALAPPDATA%\CodexRelay`。

## 6. 兼容性与风险

- 命令名、控制台 identity、session、发布门禁和 GitHub 流程不变。
- 唯一用户可见兼容变化是便携 EXE 的启动路径；旧副本不会自动删除。
- 选择移出 `dist` 而不是 `emptyOutDir=false`，避免主前端残留旧哈希资产。
- 不选择 `dist/app`，避免改变正式主应用的 `frontendDist` 与生产 bundle 输入。

## 7. 回滚

若新路径产生未预期问题，可同时回滚包装脚本、`.gitignore`、测试和两处文档。回滚不涉及数据迁移、
Git 历史重写、用户配置或远端发布对象；不得删除维护者已有的本地控制台副本。
