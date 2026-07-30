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
- 2026-07-30：发布配置提交 `651ca7d`、准备证据提交 `3585504` 已推送到 GitHub
  `main`；推送后 GitHub API 与本地 `HEAD` 均为
  `3585504acc76ffdf9f4d5da324fd582f0a433702`。GitHub 状态页为
  `All Systems Operational`，没有活动发布 Run。
- 2026-07-30：最终候选任务记录提交 `ce76f43` 已推送到 `main`。唯一发布 Run
  [30518089881](https://github.com/hunxuankai/codex-relay/actions/runs/30518089881)
  基于完整 SHA `ce76f43e25fd3ca2fa9b3a2dac4a8930d2d975a1` 成功：创建于
  `05:57:08Z`，Job 于 `05:57:12Z` 开始（排队约 4 秒），`06:13:14Z` 完成
  （Job 约 16 分 2 秒）；工作流内完整检查与 Draft 构建分别成功。Run 注解指出固定
  SHA 的 `actions/checkout` / `actions/setup-node` 声明 Node.js 20，Runner 已强制使用
  Node.js 24；该注解未使 Run 失败。
- 2026-07-30：Draft Release ID `362221700`，标题/Tag 为 `Codex Relay v0.3.0` /
  `v0.3.0`，目标提交精确为候选 SHA，`draft=true`、`prerelease=false`。Draft 资产：
  - `Codex.Relay_0.3.0_x64-setup.exe`：4,644,968 字节，SHA-256
    `1af72eaccbfec0b65716ad2334830000cb4e4bd623b61bcf66c212a87734fde1`；
  - `Codex.Relay_0.3.0_x64-setup.exe.sig`：424 字节，SHA-256
    `e24a53b3457ab4d02ec2a65eadaf3a3bccc7c8bd68c36df5308f08ac239ce177`；
  - `latest.json`：2,293 字节，SHA-256
    `1325670de9cff1fddc6f978ae37f44c14566e9554db003de378566f04f8cf8d2`。
- 2026-07-30：Draft 下载审计确认三个文件与 GitHub digest 一致；`latest.json.version`
  为 `0.3.0`，`pub_date` 为 `2026-07-30T06:13:00.415Z`，说明与 Release 正文完全
  一致；`windows-x86_64` 和 `windows-x86_64-nsis` 都指向资产 `494995775`，两项
  内联签名与独立 `.sig` 内容一致。公开说明/清单没有高置信度秘密命中，安装器
  Authenticode 为 `NotSigned`。
- 2026-07-30：Release 于 `06:39:33Z` 公开；Tag ref 与 `releases/latest` 均指向
  候选 SHA。公开后重新下载三项资产，哈希与 Draft 全部一致；固定 Latest 清单为
  `0.3.0`，按 `Accept: application/octet-stream` 请求清单中的 GitHub REST asset
  URL 得到 4,644,968 字节安装器，SHA-256 与 Release 资产一致。
- 2026-07-30：清理前存在 `v0.3.0` Draft（ID `362221700`）和 `v0.2.1` Release
  （ID `359826042`），公开 Tag 只有 `v0.2.1`。清理 Run
  [30520280747](https://github.com/hunxuankai/codex-relay/actions/runs/30520280747)
  成功；清理后只保留 `v0.3.0` Release、三项资产和指向候选 SHA 的 Tag，旧 Release
  查询返回 404。该远端清理不涉及 Codex/Relay 本机配置、日志、备份或密钥。
- 2026-07-30：Windows 自动化本机管道连续两次不可用；Sandbox 二进制存在，但功能
  状态查询要求管理员提升。未启动不可观察的 Sandbox。仓库准备器以 `-PrepareOnly`
  成功验证 `v0.2.1` 基线哈希，并生成
  `D:\Users\23869\AppData\Local\Temp\codex-relay-sandbox-dbe6338c783447a9adb6ed1214a97368`：
  staging 无 reparse point，输入映射只读、结果映射可写，结果目录为空。真实安装、
  UAC、应用重启、Tauri 验签、`v0.2.1 → v0.3.0` updater 升级和升级后数据保留均未
  执行，不能声明成功。
- 2026-07-30：最终专项检查再次通过（2 个发布测试文件、17 项测试），Trellis 材料
  校验、`git diff --check`、任务材料秘密扫描通过；实时公开门禁确认 Latest 与
  Manifest 均为 `v0.3.0`、Tag/Release 目标为候选 SHA、Release/Tag 各 1 个且清理
  Run 为 `completed/success`。
- 当前阶段：最终检查、提交发布证据、归档任务并记录会话。

## 执行清单

1. [x] 同步 `package.json`、`package-lock.json`、根与 core Cargo manifest、
   `src-tauri/Cargo.lock` 到 `0.3.0`。
2. [x] 把 `.github/workflows/release.yml` 的最终说明改为本次用户变化，并同步
   `src/release-config.test.ts` 的版本与说明契约。
3. [x] 运行专项发布结构测试，再运行 `npm run check`；修复所有真实失败。
4. [x] 从当前 PowerShell 进程移除两个签名环境变量，运行 `npm run build`，枚举 EXE/NSIS
   路径、大小、时间和 SHA-256，并确认普通构建没有本次 updater `.sig`/`latest.json`。
5. [x] 运行版本搜索、`git diff --check`、状态/忽略文件/跟踪文件和高置信度秘密扫描；精确
   暂存发布文件与任务材料，提交候选。
6. [x] 推送 `HEAD:main` 并确认远端 SHA；记录 GitHub Actions 状态和活动 Run，确保没有
   重复候选。
7. [x] 下载公开 `v0.2.1` NSIS 到安全临时目录，核对 Release API 大小与 SHA-256，保留给
   后续隔离升级。
8. [x] 触发“发布 Windows 更新”，记录 Run URL、候选 SHA、创建/开始/完成时间、排队和
   执行时长；等待成功或如实处理失败。
9. [x] 审计 `v0.3.0` Draft：Tag、标题、目标提交、说明、状态、三项资产、大小、SHA-256、
   清单版本/说明/平台 URL、内联签名与独立 `.sig`。
10. [x] 公开 Release，复核 Latest 与公开资产；等待并核对历史 Release/tag 清理 Run。
11. [x] 尝试在 Windows Sandbox/隔离 VM 执行 `v0.2.1 → v0.3.0` 应用内升级；自动化
    连接不可用，因此只完成安全 `PrepareOnly`，没有启动 Sandbox，也没有生成
    `before.json` / `after.json`。真实升级及所有相关人工场景如实记录为未执行。
12. [x] 完成 Trellis check、规范更新判断和发布证据整理；Phase 3.4 提交证据后进入
    `finish-work` 归档与会话记录。收尾提交不会改变已经发布的 Tag 目标。

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
- 尝试运行 `npx tauri signer verify --help` 时，Tauri CLI 2.11.4 因不存在 `verify`
  子命令退出 1；实际 signer 只提供 `sign` / `generate`。因此没有独立 CLI 密码学
  验签声明，只记录工作流签名成功、`.sig`/清单关联一致，以及真实 updater 验签未执行。
- Draft 阶段按 Tag 查询 Release 和 Git Tag ref 均返回 404；认证后的 Release 列表显示
  Draft 目标提交与资产，公开后 Tag 端点/ref 正常出现并精确指向候选 SHA。
