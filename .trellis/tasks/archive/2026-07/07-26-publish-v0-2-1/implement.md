# Codex Relay v0.2.1 发布实施计划

任务已完成：产品修复、版本候选、CI 冷 runner 修复、Draft 审计、公开 Release、公开端点复核、
`v0.2.0 → v0.2.1` 隔离应用内升级与最终质量门禁均已取得证据；取消/断网/错误签名等未执行场景已明确记录。

## 当前进度与证据

- 产品修复已提交为 `b05991c`（`fix(update): 优化更新信息展示`）；提交前已通过前端检查、生产前端构建和生产依赖审计。
- 发布结构测试先按预期取得 RED：版本/说明未同步时 13 项中 2 项失败；同步后 `npx vitest run src/release-config.test.ts` 为 13/13 通过。
- 版本来源已统一为 `0.2.1`：`package.json`、`package-lock.json` 根包、`codex-relay` 和 `codex-relay-core` Cargo 包及 `Cargo.lock`。
- `npm run check` 于本轮退出 0，用时约 261 秒：Trellis 8 项、前端 38 个测试文件/178 项、Rust 主 crate 40 项、core crate 172 项、路径安全 3 项、Provider workflow 1 项，依赖图、fmt、Clippy 和 workspace 测试均通过。
- 已在当前构建进程移除 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（两者均不存在），`npm run build` 于本轮退出 0，用时约 321 秒。普通构建仅生成：
  - `src-tauri/target/release/CodexRelay.exe`：18,928,128 字节，写入时间 `2026-07-26T01:30:00.1290626+08:00`，SHA-256 `357A34ADA71FABAA297B7FC71847355DDCB4D84DAFDA1A694390ACA06F6247BF`。
  - `src-tauri/target/release/bundle/nsis/Codex Relay_0.2.1_x64-setup.exe`：4,597,020 字节，写入时间 `2026-07-26T01:29:59.9775178+08:00`，SHA-256 `613B63C9C03276992A309DEE1696476BBD367F0A70E88FB10E52BAB7603D43FF`。
- 普通构建输出未生成 `v0.2.1` 的 `.sig` 或 `latest.json`；构建日志的 `@vueuse/core` `#__PURE__` 注释为既有警告，不影响退出码。

## 发布 Run 30167838439 失败与修复

- 候选提交 `3207b5ade83a246c156fbf237783ef42ce6de974` 已推送并触发唯一发布 Run
  `30167838439`（`https://github.com/hunxuankai/codex-relay/actions/runs/30167838439`）。Run 于
  `2026-07-25T17:32:40Z` 创建、`17:32:40Z` 启动、`17:41:37Z` 结束，结论为 `failure`；Draft 构建步骤
  被跳过，远端没有生成或公开 `v0.2.1`。
- 失败发生在“运行完整检查”：前端 38 个测试文件/178 项通过，`codex-relay-core` 172 项中 171 项通过；
  唯一失败为 `infrastructure::codex_process::tests::cancellation_terminates_descendant_processes_in_the_job`，
  在 30 秒 deadline 后报 `child process id was not written in time`。输出上限测试通过，未观察到产品逻辑失败。
- 根因类别为 **D 测试覆盖缺口 + E 隐式时序/启动边界假设**：前一轮修复已把 PID 观察点移到父进程，
  但仍通过 PowerShell `Start-Process` 的 ShellExecute 包装创建后代；该边界在本机缓存环境通过，却可能在
  冷 Windows runner 的挂起/Job Object 嵌套链中阻塞。继续扩大 timeout 不能修复错误的启动边界。
- 修复仅修改 Windows 测试夹具：改用 `.NET System.Diagnostics.ProcessStartInfo`，设置
  `UseShellExecute = false`、`CreateNoWindow = true`，父进程在返回后立即写入子 PID；增加临时状态/异常文件，
  deadline 失败时把非秘密诊断带入消息。生产 Job Object、取消、超时、输出上限逻辑未改动。
- 修复后的本地证据：后代取消专项连续 3 次通过（每次约 0.88–1.07 秒），输出上限专项连续 3 次通过
  （每次约 0.69–0.83 秒）；随后 `codex_process` 5 项专项全部通过（3.65 秒）。完整 `npm run check`
  和新的候选提交/Actions Run 仍待执行，不能据此提前声称 Draft 或公开发布成功。
- 知识已沉淀到 `.trellis/spec/testing/rust-build-feedback.md` 的“Windows 冷 runner 进程树测试”场景，
  明确无 ShellExecute 启动、诊断文件、根因类别和防复发门禁。

## 修复后的本地完整门禁

- `cargo fmt --all --check --manifest-path src-tauri/Cargo.toml` 退出 0。
- `npm run test:rust:lib -- codex_process -- --nocapture` 退出 0：5 项全部通过。
- 后代取消和输出上限两个高风险测试在修复后各连续运行 3 次，均为 3/3 通过。
- 新鲜 `npm run check` 于本轮退出 0，用时约 240.7 秒：Trellis 8 项、前端 38 个测试文件/178 项、
  Rust 主 crate 40 项、`codex-relay-core` 172 项、路径安全 3 项、Provider workflow 1 项全部通过；
  Rust 依赖图、fmt、Clippy 和 workspace 测试均通过。
- 该门禁只证明本地候选质量；远端冷 runner、Draft、公开 Release 和隔离升级仍未验证，不能提前标记验收完成。

## 新候选、成功 Run 与 Draft 审计

- 修复提交 `7b3d5f2c793b21b8a5aea01f3fc482bbd9de3ffe`（`fix(test): 稳定 Windows 后代进程测试`）
  已推送到远端 `main`；推送后本地 `HEAD` 与 `origin/main` 精确一致。
- 发布 Run `30168932882`（`https://github.com/hunxuankai/codex-relay/actions/runs/30168932882`）于
  `2026-07-25T18:05:25Z` 创建并启动，`2026-07-25T18:18:29Z` 完成，结论为 `success`；Job 实际执行
  约 12 分 59 秒。检出、Node、Rust、依赖安装、完整检查和 Draft 构建全部成功。远端完整检查从
  `18:06:35Z` 到 `18:12:47Z`，Draft 构建从 `18:12:47Z` 到 `18:18:22Z`。
- Draft Release ID 为 `359826042`：`tag_name=v0.2.1`、名称 `Codex Relay v0.2.1`、
  `target_commitish=7b3d5f2c793b21b8a5aea01f3fc482bbd9de3ffe`、`draft=true`、
  `prerelease=false`，说明与工作流最终中文说明一致。
- Draft 恰有 3 个资产，并已通过 GitHub asset API（`Accept: application/octet-stream`）下载到系统临时
  审计目录核对：
  - `Codex.Relay_0.2.1_x64-setup.exe`：4,597,899 字节，SHA-256
    `C4FCCB44FE104F12355B745D99CF2E79CA15D58BE46D497DB59BF7BC51CEB60A`。
  - `Codex.Relay_0.2.1_x64-setup.exe.sig`：424 字节，SHA-256
    `C50508F2535F15B544A43EA58891AE7842D84C477F74B9C3AC534B4B84FE69E0`。
  - `latest.json`：2,011 字节，SHA-256
    `E28E4E990E319778C0444F3BF36D76E3AFDF984189BE04107B5AFF9C8F3B949A`。
- `latest.json.version=0.2.1`、`notes` 与 Release body 完全一致，平台恰为 `windows-x86_64` 与
  `windows-x86_64-nsis`，两者都指向安装器资产 API `489645366`；两个内联签名与独立 `.sig` 去除
  末尾空白后的内容完全一致。
- 安装器 PE 头有效，`FileVersion` 与 `ProductVersion` 均为 `0.2.1`；`Get-AuthenticodeSignature`
  返回 `NotSigned`，与发布说明“未知发布者”边界一致。Draft 审计脚本退出 0 并输出 `DraftAudit=PASS`。

## 公开 Release 与公开端点复核

- Draft 审计通过后，通过 Release API 将 ID `359826042` 公开并标记 Latest；公开时间为
  `2026-07-25T18:22:18Z`，URL 为 `https://github.com/hunxuankai/codex-relay/releases/tag/v0.2.1`。
- `releases/latest` 返回同一 Release ID、`v0.2.1`、`draft=false`、`prerelease=false` 和候选提交
  `7b3d5f2c793b21b8a5aea01f3fc482bbd9de3ffe`；远端 Tag `v0.2.1` 直接指向同一提交。
- 从公开 Tag 下载 NSIS 与 `.sig`、从 `releases/latest/download/latest.json` 下载清单后，三项大小和
  SHA-256 与 Draft 审计完全一致：4,597,899 / 424 / 2,011 字节，摘要分别为
  `C4FCCB44FE104F12355B745D99CF2E79CA15D58BE46D497DB59BF7BC51CEB60A`、
  `C50508F2535F15B544A43EA58891AE7842D84C477F74B9C3AC534B4B84FE69E0`、
  `E28E4E990E319778C0444F3BF36D76E3AFDF984189BE04107B5AFF9C8F3B949A`。
- 公开 `latest.json.version=0.2.1` 且说明仍与 Release body 一致，公开复核脚本退出 0 并输出
  `PublicAudit=PASS`；未观察到资产、URL、签名或说明漂移。

## 隔离 Sandbox 应用内升级证据

- 使用公开 `v0.2.0` NSIS 安装器作为基线：4,573,718 字节，SHA-256
  `D6A70C69B4E7E1C4F2621B905B2E433C05E4F272B80219B0EA6F689B286CB3D1`。主机准备器在系统临时目录
  `D:\Users\23869\AppData\Local\Temp\codex-relay-sandbox-v0.2.0-to-v0.2.1-e332877684a64df2bddcfcac8cc316cf`
  创建 staging；复核显示没有 reparse point，输入映射 `ReadOnly=true`，结果映射 `ReadOnly=false`，
  未使用真实 `.codex`、真实 Relay 应用数据或仓库目录。
- `before.json` 于 `2026-07-26T02:26:48.5399208+08:00` 写回，覆盖为
  `C:\CodexRelaySandbox\dev-data\codex` 与 `C:\CodexRelaySandbox\dev-data\app-data`，三项白名单
  fixture 均存在且报告不含 `test-key-*`、`OPENAI_API_KEY`、Authorization 或 Bearer 内容。
- 在 Sandbox 中人工启动基线安装器并从 `v0.2.0` 设置页点击应用内“下载并安装”。基线界面实际显示原始
  ISO 日期和未渲染 Markdown；升级确认文案提示可能出现 UAC。下载/安装阶段首次观察到 NSIS 因
  `C:\Program Files\Codex Relay\CodexRelay.exe` 仍被占用而报错；等待后选择“重试”，安装器完成，应用
  自动重启。全过程未观察到 UAC 提示；该文件锁失败与重试事实保留，不把第一次尝试单独计为成功。
- `guest-verify.ps1` 于 `2026-07-26T03:02:24.4462268+08:00` 写出 `after.json`：
  `expectedVersion=0.2.1`、`installedVersion=0.2.1`、`versionMatched=true`、
  `executablePresent=true`、`dataPreserved=true`、`success=true`；安装目录为
  `C:\Program Files\Codex Relay`，可执行文件为 `C:\Program Files\Codex Relay\CodexRelay.exe`。
- 三项 fixture 前后完全保留：

  | 相对路径 | 长度 | SHA-256（前后相同） |
  |---|---:|---|
  | `codex/auth.json` | 56 | `2EAF2CA9850326AE844E2BE84455EAA246461C523A50A46D5CC97D7239355E81` |
  | `codex/config.toml` | 273 | `753D51E4C045937E1EDB169BA6D6088E15BFDC20B78968C72C668539F1173267` |
  | `app-data/providers.json` | 180 | `E7B0872EA3A97EB0740D9B53496FBA6B21329BBB41CDE6C12D17D6AD1725342A` |

- 升级后应用设置页显示当前版本 `0.2.1`，不再显示待更新提示；应用自动重启和数据保留均为实际观察/脚本
  证据。Sandbox 进程已退出，staging 在确认位于系统临时目录、非 reparse point 后清理成功且目录不存在。
- 本轮未执行取消下载、断网、错误签名、下载失败和 UAC 取消场景；这些场景保持“未验证”，不扩大为完整错误矩阵
  验收。未安装 `minisign`，也未声称完成独立本地密码学验签；签名证据限于 Actions 生成、资产摘要和内联/独立
  签名一致性。

## 最终全范围质量门禁

- Sandbox 清理后再次运行完整 `npm run check`，本轮退出 0，用时约 131.9 秒：Trellis 8 项、前端
  38 个测试文件/178 项、Rust 主 crate 40 项、`codex-relay-core` 172 项、路径安全 3 项和 Provider
  workflow 1 项全部通过；依赖图、typecheck、fmt、Clippy 与 workspace 测试均通过。
- `UpdatePanel` 回归测试明确通过“发布日期格式化和安全 Markdown 渲染”断言；Sandbox 中升级后的应用显示
  当前版本 `0.2.1`，公开端点复核仍为 `v0.2.1` Latest。
- `git diff --check` 与 Trellis 任务校验通过；任务材料秘密扫描无高置信度命中。工作区除当前 Trellis 任务材料外
  没有未提交代码改动，修复提交已在远端 `main`。

## 实施顺序

1. 复核并提交更新面板修复、依赖和回归测试，提交信息使用简体中文 Conventional Commit。
2. 先更新 `src/release-config.test.ts`，要求版本 `0.2.1`、`v0.2.0 → v0.2.1` 和最终发布说明，运行专项测试取得预期 RED。
3. 更新 `package.json`、`package-lock.json`、两个 Cargo package、Cargo lock 和 `.github/workflows/release.yml`，运行专项测试取得 GREEN。
4. 搜索当前版本、上一版本和说明文字，确认当前配置无漂移；历史任务和历史证据保留旧版本。
5. 运行完整 `npm run check`，保留首次失败、耗时、测试数量和修复证据。
6. 移除当前进程中的两个 Tauri 签名环境变量，运行普通 `npm run build`；枚举实际 EXE/NSIS 路径、大小、时间和 SHA-256，确认未生成本次 `.sig` / `latest.json`。
7. 执行 Trellis check、任务校验、`git diff --check`、忽略文件检查、生产依赖审计、秘密和真实用户路径扫描。
8. 精确提交发布候选并推送 `HEAD:main`；确认本地、`origin/main` 与远端引用一致。
9. 确认 GitHub Status 正常且没有活动候选 Run 后，触发唯一发布工作流并记录 Run ID、URL、head SHA、排队与执行时间。
10. Actions 成功后审计 Draft：Release ID、Tag、目标提交、状态、最终说明、三个资产大小/SHA-256、`latest.json` 版本/说明/平台 URL、内联签名与 `.sig` 一致性，以及 API asset 二进制下载摘要。
11. Draft 全部通过后公开并标记 Latest；立即复核公开 Release、Tag、`latest.json` 与三个资产无漂移。
12. 准备安全 Windows Sandbox/VM staging，从公开 `v0.2.0` 执行应用内升级到 `v0.2.1`，运行 verifier 并读取 `after.json`。
13. 记录 UAC、自动重启、版本、目录、可执行文件、数据保留和未执行场景；安全关闭隔离环境并清理 staging。
14. 运行最终质量检查，提交发布证据，推送，归档任务并记录会话日志。

## 验证命令

```powershell
npx vitest run src/release-config.test.ts
npm run check

Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
npm run build

git diff --check
git status --short --ignored
python ./.trellis/scripts/task.py validate .trellis/tasks/07-26-publish-v0-2-1
```

## 风险文件与回滚点

- 产品修复：`package.json`、`package-lock.json`、`src/components/UpdatePanel.vue`、`src/utils/updatePresentation.ts` 及测试。
- 版本：`package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/crates/codex-relay-core/Cargo.toml`、`src-tauri/Cargo.lock`。
- 发布：`src/release-config.test.ts`、`.github/workflows/release.yml`。
- 测试与规范：`src-tauri/crates/codex-relay-core/src/infrastructure/codex_process.rs`、
  `.trellis/spec/testing/rust-build-feedback.md`。
- 证据：本任务目录，只写入非秘密元数据。

版本或说明错误时在候选提交前修正；Draft 错误时删除 Draft 并用新提交重跑；公开后不得替换同版本资产。

## 安全门禁

- 不读取、写入或删除真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`。
- 不输出 updater 私钥、密码、GitHub Token、真实 API Key、Authorization Header 或完整认证文件。
- 不把普通构建描述为签名、安装或升级成功，不把 Draft 描述为已公开。
- 所有失败、超时、未执行场景和人工观察按实际状态保留。
