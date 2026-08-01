# 发布控制台仓库路径显示规范化实施计划

> **执行约束：** Codex inline；使用 `trellis-before-dev`、`superpowers:test-driven-development` 与 `vue-best-practices`，不得派发写入或检查子 Agent。

## 行为切片 1：偏好边界规范化

- [x] 在 `useRepositoryPreference.test.ts` 写 RED：加载/记住扩展盘符路径时返回并保存普通盘符路径；扩展 UNC 正确转换；未知设备路径保持原样。
- [x] 运行目标测试，确认当前实现因只执行 `trim()` 而失败。
- [x] 在 `useRepositoryPreference.ts` 实现最小纯规范化函数，并仅用于 load/remember。
- [x] 重跑 composable 专项测试至绿色。

## 行为切片 2：App 用户可见流程

- [x] 修改 `App.test.ts`，让成功 inspection 返回 `\\?\D:\canonical\repository`，要求输入框和 localStorage 均为普通路径。
- [x] 运行 App 目标测试，确认修复前失败、修复后绿色。
- [x] 确认 App、RepositorySetupPanel 和 Rust backend 无需生产代码改动。

## 验证与交付

- [x] 运行：

  ```powershell
  npm run test --workspace @codex-relay/release-console -- --run src/composables/useRepositoryPreference.test.ts src/App.test.ts
  npm run typecheck:release-console
  npm run test:release-console
  npm run check
  ```

- [x] 重新打包：

  ```powershell
  npm run build:release-console
  ```

- [x] 核对源/交付 EXE 的路径、大小、最后写入时间和 SHA-256。
- [x] 运行差异、安全和真实路径审计。
- [x] 精确暂存并提交本任务改动。

## 回滚点

- 只修改前端偏好与测试，不改 session、Git 或 Rust canonical path。
- `dist/` 和 target 产物保持忽略，不进入 Git。

## 当前进度

Phase 3.4 已完成；工作提交为 `7b53fbd fix(release): 规范化仓库路径显示`，准备归档任务并记录会话。

## 已完成

- 仓库偏好加载与显式 `remember` 会把扩展盘符路径转换为普通盘符路径。
- 扩展 UNC 路径转换为标准 UNC；未知设备路径保持原样。
- App 启动恢复与成功预检流程通过同一 composable 边界显示并保存普通路径。
- `update()` 仍只更新内存，不写 localStorage；Rust、session schema 与现有 session 文件未修改。

## 关键决策

- 内部 Git/文件/会话继续使用 Rust canonical 路径，只在 Vue 仓库偏好边界做用户显示规范化。
- 不对所有 `\\?\` 盲目截断，只识别扩展盘符与扩展 UNC。

## 验证证据

- 首次专项命令因工作区缺少 Vitest 可执行文件而未进入断言；执行 `npm ci` 后恢复锁文件依赖。
- 盘符 RED：2 个测试文件中 3 个断言按预期收到 `\\?\D:\...` 而失败。
- 盘符 GREEN：2 个测试文件、17 个测试通过。
- UNC RED：1 个测试按预期收到 `\\?\UNC\...` 而失败。
- 最终专项 GREEN：2 个测试文件、18 个测试通过。
- `npm run typecheck:release-console` 退出码 0。
- `npm run test:release-console` 退出码 0：16 个测试文件、62 个测试通过。
- 成对安全临时路径覆盖下运行 `npm run check`，退出码 0；主 Vitest 59 个文件、310 个测试通过，
  发布控制台测试、Trellis 测试、Rust fmt/Clippy/全套测试均完成。
- 已更新 `.trellis/spec/release/publishing.md` 的 0.2 仓库偏好契约，明确 Windows 扩展路径仅在
  Vue 偏好显示边界转换。
- `npm run build:release-console` 退出码 0；Vite 转换 1661 个模块，Rust release 构建完成并执行
  `postbuild:release-console`。
- 源/交付 EXE 均为 12,648,448 字节，时间 `2026-08-02T01:44:33.9595372+08:00`，SHA-256 均为
  `D28F970B2A6025ABE8E76FA57484BACECD5DCC183D4F0FA295BC9B5A3A848C49`。
- 源 EXE：`src-tauri/target/release/CodexRelayReleaseConsole.exe`；交付 EXE：
  `dist/release-console/CodexRelayReleaseConsole.exe`；两者大小和 SHA-256 一致。
- `git diff --check` 退出码 0；高置信度密钥扫描无命中；Git 未跟踪真实认证/Provider/备份路径；
  `dist/`、`node_modules/` 与 `src-tauri/target/` 均保持忽略。
- 工作提交 `7b53fbd` 已精确包含本任务代码、测试、规范和任务材料，未 push。

## 下一步

- 归档任务并记录会话日志。

## 尚未解决的问题

- 无产品或验证遗留项；仅剩 Trellis 归档与会话日志记账。
