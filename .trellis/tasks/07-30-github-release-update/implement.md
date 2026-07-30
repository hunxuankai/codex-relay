# v0.3.0 发布实施计划

## 当前状态

- 2026-07-30：已确认公开 Latest 为 `v0.2.1`、远端默认分支为 `main`、本地候选领先
  `origin/main` 21 个提交，两个 updater Secret 名称存在。
- 2026-07-30：发布结构测试先因旧版本和旧说明出现 2 个预期失败；同步 `0.3.0`
  及最终说明后，`npx vitest run src/release-config.test.ts src/release-retention.test.ts`
  通过（2 个文件、17 个测试）。
- 2026-07-30：带系统临时目录成对 Relay 覆盖的 `npm run check` 退出 0；Trellis
  8 项、前端 39 个文件共 207 项、Rust 工作区 252 项通过，类型检查、Rust 依赖图、
  `cargo fmt --check` 和 Clippy 完成。
- 2026-07-30：显式移除两个 updater 签名环境变量后，`npm run build` 退出 0；普通
  构建没有生成 `.sig` 或 `latest.json`。实际产物：
  - `src-tauri/target/release/CodexRelay.exe`：19,210,752 字节，SHA-256
    `564C128C9E80F1A07966DD32A50ED8DCD73482CB809A9077F477B7B3652E5B87`；
  - `src-tauri/target/release/bundle/nsis/Codex Relay_0.3.0_x64-setup.exe`：
    4,642,139 字节，SHA-256
    `76CA8B1D2D32EB74826D3BF08BC0F145D66FA758ED1207E6FD6DFABF5CDE2B2E`；
  - 两个文件的 Authenticode 状态均为 `NotSigned`，没有签名成功声明。
- 2026-07-30：`v0.2.1` 基线安装器已保存到
  `D:\Users\23869\AppData\Local\Temp\codex-relay-v0.3.0-baseline-259318cd3d864396bc2f0bbd311525a2`；
  目录是系统临时目录真子路径、创建前为空且不含 reparse point。安装器为
  4,597,899 字节，SHA-256
  `c4fccb44fe104f12355b745d99cf2e79ca15d58be46d497db59bf7bc51ceb60a`，
  与 GitHub `v0.2.1` Release API 一致。
- 2026-07-30：规范同步判断完成；本任务没有改变 updater、NSIS、签名、数据保留或
  发布接口契约，现有 release/testing/security 规范已完整覆盖，没有新增项目专属
  规则，因此不修改 `.trellis/spec/`。
- 当前阶段：提交并推送本地候选，尚未触发发布工作流。

## 执行清单

1. [x] 同步 `package.json`、`package-lock.json`、根与 core Cargo manifest、
   `src-tauri/Cargo.lock` 到 `0.3.0`。
2. [x] 把 `.github/workflows/release.yml` 的最终说明改为本次用户变化，并同步
   `src/release-config.test.ts` 的版本与说明契约。
3. [x] 运行专项发布结构测试，再运行 `npm run check`；修复所有真实失败。
4. [x] 从当前 PowerShell 进程移除两个签名环境变量，运行 `npm run build`，枚举 EXE/NSIS
   路径、大小、时间和 SHA-256，并确认普通构建没有本次 updater `.sig`/`latest.json`。
5. 运行版本搜索、`git diff --check`、状态/忽略文件/跟踪文件和高置信度秘密扫描；精确
   暂存发布文件与任务材料，提交候选。
6. 推送 `HEAD:main` 并确认远端 SHA；记录 GitHub Actions 状态和活动 Run，确保没有
   重复候选。
7. [x] 下载公开 `v0.2.1` NSIS 到安全临时目录，核对 Release API 大小与 SHA-256，保留给
   后续隔离升级。
8. 触发“发布 Windows 更新”，记录 Run URL、候选 SHA、创建/开始/完成时间、排队和
   执行时长；等待成功或如实处理失败。
9. 审计 `v0.3.0` Draft：Tag、标题、目标提交、说明、状态、三项资产、大小、SHA-256、
   清单版本/说明/平台 URL、内联签名与独立 `.sig`。
10. 公开 Release，复核 Latest 与公开资产；等待并核对历史 Release/tag 清理 Run。
11. 尝试在 Windows Sandbox/隔离 VM 执行 `v0.2.1 → v0.3.0` 应用内升级，读取脱敏的
    `before.json`/`after.json`；保留所有实际失败和未验证项目。
12. 完成 Trellis check、规范更新判断、发布证据提交、任务归档和会话记录，并把收尾提交
    推送到 `main`。任务归档提交不改变已经发布的 Tag 目标。

## 验证命令

```powershell
npx vitest run src/release-config.test.ts src/release-retention.test.ts
npm run check
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
npm run build
git diff --check
git status --short --ignored
```

## 发布门禁

- Draft 目标提交不等于候选提交、说明或 `latest.json.notes` 漂移、资产缺失、版本/签名
  不一致、秘密扫描出现真实凭据时，停止公开。
- GitHub Actions 状态为重大故障或已有活动候选时，不重复触发。
- 只有本轮命令或 API 观察能支撑对应的测试、构建、签名、公开、清理和升级声明。

## 真实失败与修正

- 首次任务校验误用不存在的 `task.py check` 子命令并退出 1；读取帮助后改为
  `task.py validate .trellis/tasks/07-30-github-release-update`，随后验证通过。
- 首次基线准备使用当前 PowerShell 不支持的 `New-Item -LiteralPath`，产生两个非终止
  错误；虽然随后下载的字节与公开哈希一致，但该目录不作为合格证据。检查命令参数确认
  根因后，启用 `$ErrorActionPreference = 'Stop'`，使用 `New-Item -Path` 在新的临时
  目录完成预写入边界/reparse/空目录检查和重新下载，以上只记录第二次结果。
