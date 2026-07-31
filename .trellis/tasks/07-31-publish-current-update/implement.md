# v0.4.0 发布实施计划

> 本任务按 Trellis `tdd` inline 模式执行；主会话直接实施和检查，不派发实现、检查或
> 分支收尾子代理，也不创建重复的外部计划。

**目标：** 把已完成的保持 Provider 身份连接同步能力发布为经本地验证、Draft 审计和
公开复核的 Windows `v0.4.0` 更新。

**架构：** 先用发布结构测试固定版本与说明契约，再同步 npm/Cargo 版本和 GitHub Actions
最终文案；候选提交推送到 `main` 后，由现有 Draft 工作流生成 NSIS、`.sig` 和
`latest.json`，经 API 与下载字节审计后公开并核对历史清理。

**技术栈：** npm、Vitest、Rust/Cargo、Tauri 2、Git、GitHub CLI、PowerShell、Windows
Sandbox 准备脚本。

---

## Task 1：用结构测试固定 v0.4.0 候选

**文件：**

- 修改：`src/release-config.test.ts`

- [x] 把发布用例改为 `0.4.0`，基线改为 `v0.3.0`，并断言最终说明包含“保持
  `model_provider` 身份”“仅应用连接”“恢复自身连接”“首次覆盖”“v4”“降级前”以及
  标准未知发布者和数据保留文案。
- [x] 运行：

  ```powershell
  npx vitest run src/release-config.test.ts src/release-retention.test.ts
  ```

  结果：2 个文件共 17 项中 15 项通过、2 项按预期失败；失败分别是工作流仍引用
  `v0.2.1` 基线，以及 `package.json.version` 仍为 `0.3.0`。

## Task 2：同步版本与最终发布说明

**文件：**

- 修改：`package.json`
- 修改：`package-lock.json`
- 修改：`src-tauri/Cargo.toml`
- 修改：`src-tauri/crates/codex-relay-core/Cargo.toml`
- 修改：`src-tauri/Cargo.lock`
- 修改：`.github/workflows/release.yml`

- [x] 使用等价的精确文件修改，把 npm、两个 Cargo package 和 Cargo lock 根包版本同步为
  `0.4.0`。
- [x] 把 `releaseBody` 改为本次最终简体中文说明；保留 `workflow_dispatch:`、固定 Action
  SHA、两个 Secret 名称、`releaseDraft: true`、`uploadUpdaterJson: true` 和
  `updaterJsonPreferNsis: true`。
- [x] 搜索并核对当前配置中的新旧版本：

  ```powershell
  rg -n -S '0\.4\.0|0\.3\.0|0\.2\.1' package.json package-lock.json src-tauri `.github/workflows/release.yml` src/release-config.test.ts README.md
  ```

  历史任务与说明性历史记录可以保留旧版本；当前版本、测试和工作流正文不得漂移。
- [x] 重新运行 Task 1 的两个专项测试：2 个文件、17 项测试全部通过。

## Task 3：运行本地完整检查与普通构建

**文件：**

- 只写构建目录与当前任务验证记录，不写真实用户目录。

- [x] 在系统临时目录创建本次专用根，确认它位于 temp 真子路径且不含 reparse point；在
  其中创建 `codex-home` 与 `app-data`，成对设置
  `CODEX_RELAY_CODEX_HOME` / `CODEX_RELAY_APP_DATA_DIR`。
- [x] 修复测试并发争用后重新运行 `npm run check`，退出 0：Trellis 8 项、前端 40 个
  文件/231 项、Rust 工作区 47 + 243 + 3 + 1 项通过，依赖图、类型检查、格式与 Clippy
  完成；真实首次失败保留在下方调试记录中。
- [x] 从当前 PowerShell 进程移除 `TAURI_SIGNING_PRIVATE_KEY` 和
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，记录构建开始时间后运行 `npm run build`。
- [x] 枚举实际 `src-tauri/target/release/CodexRelay.exe` 和
  `src-tauri/target/release/bundle/nsis/*.exe` 的路径、大小、时间、SHA-256 与
  Authenticode 状态；确认本次普通构建没有生成或更新 `*.sig` / `latest.json`。
- [x] 普通构建退出 0，且本轮没有创建或更新 `.sig` / `latest.json`。实际产物：
  - `src-tauri/target/release/CodexRelay.exe`：19,418,112 字节，SHA-256
    `178A0E09C7E1082B2A1C920048B88CFB9DCC754BF6EFF4A46F4FF33AD4EDC997`，
    Authenticode `NotSigned`；
  - `src-tauri/target/release/bundle/nsis/Codex Relay_0.4.0_x64-setup.exe`：
    4,685,904 字节，SHA-256
    `13D692DF169A1FC35A1CD78754325F519DE61E7EE4D776BC6208996249E64F5B`，
    Authenticode `NotSigned`。
  Vite 对 `@vueuse/core` 的两个 `/* #__PURE__ */` 注释位置给出移除警告，但构建退出 0。
  这些证据只支持普通未签名 Release/NSIS 已生成；updater 签名、安装与升级尚未执行。

### 已确认的首次失败与修复

- 首次 `npm run check`：Trellis 8 项和前端 40 个文件/230 项断言全部通过，但 Vitest
  出现 `[vitest-worker]: Timeout calling "onTaskUpdate"`，命令退出 1，Rust 阶段未开始。
- 不改配置的前端复现：Sandbox 重复条目负向用例耗时 20.313 秒，越过自身 20 秒预算，
  同时再次出现 `onTaskUpdate` 超时；单独运行同一用例为 2.977 秒并通过。
- 单变量验证 `npx vitest run --maxWorkers=4`：40 个文件/230 项全部通过，Sandbox 文件从
  80.319 秒降到 23.247 秒，目标用例为 4.818 秒，确认根因是默认 8 worker 与同步
  PowerShell 子进程争用。
- 已先添加结构测试并观察 1 项预期失败，再在 `vite.config.ts` 固定 `maxWorkers: 4`；
  发布专项测试随后 2 个文件/18 项全部通过。未放宽单测试或生产超时。

## Task 4：提交前审计、候选提交与推送

**文件：**

- 精确暂存 Task 1/2 的发布文件与当前 Trellis 任务材料。

- [x] 运行：

  ```powershell
  git diff --check
  git status --short --ignored
  git ls-files
  ```

  结果：`git diff --check` 与任务材料校验通过；高置信度秘密命中 0，受跟踪敏感路径命中
  0，受忽略文件被跟踪命中 0。构建产物、依赖和安全开发数据均保持 ignored。
- [x] 精确暂存 14 个发布、测试规范与任务文件，`git diff --cached --check` 通过，提交：

  ```text
  chore(release): 准备 v0.4.0 发布
  ```
- [x] 发布准备提交 `2bc2a89639c553ed4365ff8f9d057c24a048e300` 已推送到
  `origin/main`，推送前远端基线为 `0eb8fc34cd09b6e326b5c71293e35b3e415bd2f5`，推送后
  远端 SHA 与本地一致；未创建 Tag。发布证据提交后将再次固定最终候选 SHA。

## Task 5：保存 v0.3.0 升级基线

**文件：**

- 写入：系统临时目录中的独立只读基线目录。

- [x] 通过 GitHub API 读取 `v0.3.0` Release ID `362221700`；NSIS 资产 ID
  `494995775`、名称 `Codex.Relay_0.3.0_x64-setup.exe`、大小 4,644,968 字节、digest
  `sha256:1af72eaccbfec0b65716ad2334830000cb4e4bd623b61bcf66c212a87734fde1`。
- [x] 创建空的系统临时目录真子路径，检查目标及父路径没有 reparse point，按
  `Accept: application/octet-stream` 下载基线安装器。
- [x] 基线保存到
  `D:\Users\23869\AppData\Local\Temp\codex-relay-v0.4.0-baseline-2befab081d664d68928bf4a375f6d166\Codex.Relay_0.3.0_x64-setup.exe`；
  实际大小 4,644,968 字节、SHA-256
  `1af72eaccbfec0b65716ad2334830000cb4e4bd623b61bcf66c212a87734fde1`，与 API 一致，
  目录不含 reparse point且不纳入 Git。

## Task 6：触发并监控唯一 Draft 发布 Run

**文件：**

- 更新：当前任务的非秘密发布证据。

- [x] 触发前 GitHub Status 为 `All Systems Operational`，仓库没有活动发布 Run。
- [x] 最终候选 `5c34bc6e4d840cb3775e0364aa0fce45554db78e` 精确等于远端
  `main`。首次 `gh workflow run` 因 GitHub API 连接 `EOF` 退出 1；只读 Run 列表确认没有
  产生候选后，使用直接 dispatch API 安全重试一次，创建唯一 Run
  [30621105452](https://github.com/hunxuankai/codex-relay/actions/runs/30621105452)。
- [x] Run 创建于 `2026-07-31T09:45:36Z`，Job 于 `09:45:44Z` 开始（排队 8 秒），
  `10:00:51Z` 完成（Job 907 秒/15 分 7 秒），结论 `success`。工作流内完整检查
  `09:46:38Z → 09:53:54Z` 成功，Draft 构建 `09:53:54Z → 10:00:45Z` 成功。注解指出
  固定 SHA 的 checkout/setup-node 声明 Node.js 20、runner 强制使用 Node.js 24；未使
  Run 失败。

## Task 7：审计并公开 v0.4.0

**文件：**

- 写入：系统临时目录中的 Draft 审计目录。
- 更新：当前任务的非秘密发布证据。

- [x] 从认证 Release 列表找到唯一 Draft Release ID `362978714`：标题/Tag 为
  `Codex Relay v0.4.0` / `v0.4.0`，目标提交精确为候选 SHA，
  `draft=true`、`prerelease=false` 和正文完全符合候选。
- [x] Draft 资产下载审计（首次下载 `.sig` 时 GitHub 重定向 `EOF`，生成 0 字节临时
  文件；有界重试后成功，资产本身无异常）：
  - `Codex.Relay_0.4.0_x64-setup.exe`：4,686,901 字节，SHA-256
    `261c56f552a8a126185165836743338b1b81b3f432f21e315c66a9d2d093ee8c`；
  - `Codex.Relay_0.4.0_x64-setup.exe.sig`：424 字节，SHA-256
    `33dfa3cac726ae5161a6bc868fcd35a04c9d1cc1b03f6c4af7467d2690480800`；
  - `latest.json`：2,600 字节，SHA-256
    `e0739694089ca734f97544e29dd0534e1e4c2aabef974510adef3a6ca80312a5`。
  三项均与 GitHub API digest 一致。解析清单并
  断言 `version=0.4.0`、notes 等于 Release 正文、两个 Windows 平台 URL 指向本次 NSIS、
  两项内联签名与独立 `.sig` 内容一致；`pub_date` 为 `2026-07-31T10:00:44.534Z`。
- [x] Release 正文与 `latest.json.notes` 完全一致，高置信度秘密命中 0；NSIS
  Authenticode 实际为 `NotSigned`，未宣称 Windows 发布者签名。
- [x] Release 于 `2026-07-31T10:04:28Z` 公开。公开复核确认 `releases/latest`、Tag ref、
  固定 `releases/latest/download/latest.json` 和三个 Tag 下载均指向 `v0.4.0` / 候选 SHA，
  大小与 SHA-256 相对 Draft 无漂移；清单中的 GitHub REST asset URL 按
  `Accept: application/octet-stream` 下载 4,686,901 字节，SHA-256 与 NSIS 一致。

## Task 8：核对清理并尝试隔离升级

**文件：**

- 写入：系统临时 Sandbox staging/result 目录。
- 更新：当前任务最终验证记录。

- [x] 公开前存在 `v0.4.0` Draft（ID `362978714`）与 `v0.3.0` Release（ID
  `362221700`），Tag 只有 `v0.3.0`。清理 Run
  [30622261267](https://github.com/hunxuankai/codex-relay/actions/runs/30622261267)
  于 `10:04:32Z → 10:04:46Z` 成功；清理后只保留 `v0.4.0` Release、三项资产和
  `v0.4.0` Tag，旧 `v0.3.0` Release 查询失败。该远端清理不涉及本机用户数据。
- [x] 用 Task 5 的基线安装器运行
  `scripts/windows-sandbox/prepare-update-test.ps1 -ExpectedTargetVersion 0.4.0 -PrepareOnly`，
  核对成功。staging 为
  `D:\Users\23869\AppData\Local\Temp\codex-relay-sandbox-819b023ddb85450baad8d268229f18de`：
  根目录无 reparse point，输入映射只读、结果映射可写且为空，剪贴板/打印机重定向关闭，
  目标版本为 `0.4.0`。
- [x] `WindowsSandbox.exe` 存在，但功能状态查询要求管理员提升；Windows 自动化连接失败，
  错误为“系统找不到指定的原生控制管道”。为避免启动无法观察和交互的 Sandbox，本轮没有
  启动真实 guest，也没有生成 `before.json` / `after.json`。真实安装、UAC、应用退出/重启、
  Tauri 验签、`v0.3.0 → v0.4.0` updater 升级及升级后数据保留均未执行，不能声明成功。

## Task 9：最终检查、提交证据与收尾

- [x] 最终新鲜门禁通过：发布专项 2 个文件/18 项测试，Trellis task validate、
  `git diff --check` 和任务材料高置信度秘密扫描（0 命中）通过；实时 Latest/Manifest 为
  `v0.4.0`，Release/Tag 各 1 个，清理 Run `completed/success`，活动发布/清理 Run 为 0。
- [x] PRD 六项验收已全部勾选，本计划保留本地 Vitest 首次失败、两次 GitHub `EOF`、
  Draft/公开/清理证据和 Sandbox 未执行边界；精确提交本次发布证据并在交付中报告哈希。
- [x] 使用 `trellis-check` 完成当前候选质量审查；将 Vitest 4-worker 与 Sandbox
  PowerShell 并发契约写入 `.trellis/spec/testing/tdd-and-isolation.md` 及测试索引。
- [ ] 使用 `trellis-finish-work` 归档任务并记录会话日志；收尾提交不得改变已公开 Tag 的
  目标提交。

## 发布停止条件

- 版本不一致、专项/完整检查失败、普通构建失败或秘密/真实路径审计不通过。
- 远端 `main` 不等于候选 SHA、已有活动发布 Run、GitHub 重大故障或工作流失败。
- Draft 目标提交、说明、资产、清单版本、平台 URL 或签名关联任一不一致。
- 安装器下载哈希不一致、审计目录不安全或任何命令可能访问真实 Codex/Relay 用户目录。
- 公开后发现问题时不得原地替换资产；只能停止并准备更高 SemVer 修复版本。
