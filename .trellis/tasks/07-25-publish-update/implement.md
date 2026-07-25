# Codex Relay v0.2.0 发布实施计划

## 当前状态

任务已激活。版本与最终发布说明切片、typecheck、全量检查、普通构建、候选提交和推送已完成。两轮 GitHub Actions 均在完整检查阶段失败：第一轮已修复 Cargo 正常 stderr 被 PowerShell 误判的问题，第二轮根因已收窄为冷 Windows runner 上进程树测试的 5 秒测试预算不足；测试级 30 秒预算、前端延迟焦点生命周期修复、本地完整检查和无签名普通构建均已完成，尚待提交推送并重新运行工作流。Draft 审计、公开 Release 和隔离升级尚未执行。

## 当前进度与证据

- RED：更新 `src/release-config.test.ts` 后运行 `npx vitest run src/release-config.test.ts`，13 项中 2 项按预期失败；实际版本仍为 `0.1.2`，工作流仍缺少 `v0.1.2` 用户升级文案。
- GREEN：将 npm、主 Tauri crate、内部 `codex-relay-core` crate 与 Cargo lock 版本统一为 `0.2.0`，并把 `releaseBody` 改为最终中文说明；同一专项测试 13/13 通过。
- 本切片未修改产品运行时逻辑、updater endpoint、公钥、安装范围或数据保留行为。
- `npm run typecheck`：退出 0。
- `npm run check`：退出 0，用时约 321.3 秒；Trellis 8 项、前端 37 个测试文件/175 项、Rust 主 crate 40 项、core crate 172 项、路径安全 3 项、Provider 工作流 1 项均通过，Rust 依赖图检查通过。
- 无签名环境的 `npm run build`：退出 0，用时约 270.6 秒。实际 `v0.2.0` 产物：
  - `src-tauri/target/release/CodexRelay.exe`：18,903,552 字节，SHA-256 `49F9AAD416BFBA75519DC300188D03F346F95779E42C0C0994FCC98094C39AFC`，写入时间 `2026-07-25T20:17:40.1294840+08:00`。
  - `src-tauri/target/release/bundle/nsis/Codex Relay_0.2.0_x64-setup.exe`：4,573,954 字节，SHA-256 `3EC5E6FE5FD53B2C17D1A5B31A203576DFB5BE346C1E85108482661FE55868DD`，写入时间 `2026-07-25T20:17:40.0368039+08:00`。
  - 当前进程的两个 Tauri 签名环境变量均不存在；Release 目录未生成 `.sig` 或 `latest.json`。同目录中的 `0.1.0`、`0.1.1`、`0.1.2` 安装器是历史构建残留，未作为本候选产物。
- `trellis-check` 发布前审查：改动仅涉及版本、发布说明、结构测试和任务材料；`cargo metadata --locked` 返回 `codex-relay` 与 `codex-relay-core` 均为 `0.2.0`；任务校验通过；未发现需要新增长期规范的模式或缺陷。
- 安全审计：`git diff --check` 通过；Git 跟踪文件只保留 `dev-data/.gitkeep`，没有真实 `auth.json`、`providers.json`、备份、签名资产或 target；高置信度 `OPENAI_API_KEY`/Authorization/Bearer 命中均位于既有测试或脱敏实现，未发现凭据值。
- 候选提交 `930225a1a6ff7dd5b57a4b6026111d94863868bf` 已推送到远端 `main`，远端引用精确一致。
- 已触发 GitHub Actions Run `30157828371`（`https://github.com/hunxuankai/codex-relay/actions/runs/30157828371`），head SHA 精确为候选提交。Run 当前保持 `queued`，未分配 `windows-latest` runner；GitHub Status 同期将 Actions 标记为 `major_outage`。因此 Draft 尚未生成，不能报告 Actions、签名或发布成功，也不重复触发 Run。
- `gh run watch 30157828371 --exit-status` 等待约 30 分钟后因命令超时退出（退出码 124）；随后只读复核仍显示 Run/Job 为 `queued`、无 conclusion，GitHub Status 事故“Actions run failures and delays”仍为 `critical / investigating`（短链 `https://stspg.io/448g37mrq066`）。这是真实外部阻塞，不是构建失败。
- 外部服务恢复后，同一 Run 于 `2026-07-25T13:15:59Z` 开始执行；检出、Node、Rust 和依赖安装均通过。Run `30157828371` 首次实际执行在 `2026-07-25T13:18:18Z` 的 `check-rust:deps` 因 Windows PowerShell 把 `cargo tree` 的 `Updating crates.io index` stderr 误判为 `NativeCommandError` 而失败；前端 37/175 项和 Trellis 8 项已先通过，Draft 构建未执行。
- 根因调查：`scripts/check-rust-dependency-graph.ps1` 在全局 `Stop` 偏好下直接调用原生 Cargo；CI 冷缓存会产生正常 stderr，而本机缓存命中未暴露该边界。新增 fake `.cmd` 回归用例先复现退出 1，再以 `Invoke-CargoTree` 暂时使用 `Continue`、捕获 `$LASTEXITCODE`、`finally` 恢复偏好；`npm run test:rust-deps` 现为 6/6 通过。
- 修复后的本地 `npm run check` 于本轮退出 0，用时约 205.2 秒：Trellis 8 项、前端 37/175 项、Rust 主 crate 40 项、core crate 172 项、路径安全 3 项、Provider 工作流 1 项均通过。
- Cargo stderr 修复提交 `478a094eb3db5f25e6587d804a2e1839f4e27d16` 已推送到远端 `main`，本地 `HEAD`、`origin/main` 与第二轮工作流 head SHA 精确一致。
- 第二轮 GitHub Actions Run `30159956379`（`https://github.com/hunxuankai/codex-relay/actions/runs/30159956379`）在 `2026-07-25T13:33:24Z` 启动；检出、Node、Rust、依赖安装、Trellis 8 项和前端 37/175 项均通过。`codex-relay-core` 运行 172 项时有 170 项通过、2 项失败，Draft 构建因此跳过且 `v0.2.0` Release 仍不存在：
  - `cancellation_terminates_descendant_processes_in_the_job` 在原 5 秒条件轮询预算内未等到子进程写入 PID。
  - `output_limit_terminates_the_process_tree` 在原 5 秒运行预算内先返回 `Timeout`，未到达预期的 `OutputTooLarge`。
- 根因不是生产进程终止语义退化，而是两项测试把冷 runner 上 PowerShell 启动、子进程创建与超过输出上限的数据写入错误地约束为 5 秒。最小调整仅在 Windows 测试模块新增 `PROCESS_TREE_TEST_TIMEOUT = 30s`：PID 仍按 20 毫秒条件轮询但使用有界 deadline，输出上限和后代取消用例的测试运行预算统一为 30 秒；生产调用方传入的超时、Job Object 终止逻辑和错误映射均未修改。
- 调整后的 `codex_process` 5 项专项测试全部通过；两个 CI 失败用例各连续重复 3 次均通过。最新完整 `npm run check` 退出 0，用时约 196.1 秒：Trellis 8 项、前端 37/175 项、Rust 主 crate 40 项、core crate 172 项、路径安全 3 项、Provider 工作流 1 项均通过。

## 第二轮失败复盘

### 根因类别

- **D. 测试覆盖缺口 + E. 隐式时序假设**：测试覆盖了 Job Object 的真实行为，但把 Windows PowerShell
  启动、后代创建和大输出管道吞吐隐含假设为 5 秒；本地缓存命中没有暴露该假设，冷 runner 暴露后才到达
  这些断言。

### 修复为何失败

1. 第一轮只修复 Cargo stderr 在 PowerShell `Stop` 偏好下被提升为异常的问题；这是独立的脚本边界，修复
   后 CI 才能继续执行 Rust 测试，并非修复不完整或生产逻辑回归。
2. 第二轮首次暴露两个测试的冷启动预算不足；直接把 `Timeout` 接受为成功会掩盖输出上限行为，因此只
   调整测试预算并保留严格错误断言。

### 预防机制

| 优先级 | 机制 | 具体动作 | 状态 |
|---|---|---|---|
| P0 | 测试规范 | 固化条件轮询、30 秒有界测试预算和“不得放宽 `OutputTooLarge` 断言”规则 | 已完成 |
| P0 | 回归验证 | 两个失败用例各连续重复 3 次，并在完整 `npm run check` 中覆盖 172 项 core 测试 | 已完成 |
| P1 | CI 门禁 | Draft 构建必须等待完整 Rust 检查成功；任何冷 runner 失败都停止发布 | 已有 |
| P1 | 搜索审查 | 继续区分生产超时（如 100ms/10s）与测试冷启动预算，避免批量替换产品 SLA | 已完成 |

### 系统性扩展

- 其他固定的 5 秒值仍需按用途区分：`PROCESS_TERMINATION_GRACE`、网络连接超时和“快速返回”断言
  属于产品契约，不应因本次 CI 失败放宽；只有启动/吞吐等待测试使用测试常量。
- 未来新增 Windows 子进程测试时，先等待可观察条件，再使用有界 deadline，并在冷 runner 与缓存命中
  两种环境下验证；不得用固定循环次数代表时间预算。

### 知识沉淀

- `.trellis/spec/testing/rust-build-feedback.md` 已新增“Windows 冷 runner 进程树测试”场景。
- 本任务保留两轮 Actions 失败、根因、修复边界和未生成 Draft 的事实，避免后续只看到最终绿色结果。

## 第三轮本地检查失败与修复

- `npm run check` 首次重跑于本轮退出 1：前端 37 个测试文件/175 个断言均通过，但 Vitest 捕获 1 个
  测试环境销毁后的未处理异常。`ProviderAvailabilityTraceDialog.vue` 的 `setTimeout(focusCloseButton, 0)`
  未保存或清理句柄，回调在 jsdom teardown 后访问 `HTMLButtonElement`；Rust 阶段尚未执行，不能把这次
  结果记作完整检查通过。
- 按 TDD 先在 `ProviderAvailabilityTraceDialog.test.ts` 增加卸载清理回归测试；旧实现下专项测试
  4 项中 1 项失败（未调用 `clearTimeout`），证明测试捕获的是目标行为而非同义断言。
- 最小修复仅限该组件生命周期：保存 `focusTimer`、在 `onBeforeUnmount` 清理、用 `componentActive` 阻止
  卸载后的 `nextTick` 回调，并在 `HTMLButtonElement` 不可用时安全短路；不改变对话框 props/emits、焦点
  交互契约或业务数据。
- 修复后 `npx vitest run src/components/ProviderAvailabilityTraceDialog.test.ts` 通过 4/4，触发异常的
  `ProviderAvailabilityPanel.test.ts` 通过 7/7；随后 `npm run check:frontend` 退出 0，typecheck 通过，
  前端 37 个测试文件/176 个断言全部通过。下一步重新运行完整 `npm run check`，保留首次未处理异常及
  修复证据。
- 最新完整 `npm run check` 已于本轮退出 0，用时约 100.5 秒：Trellis 8 项、前端 37/176 项、Rust
  主 crate 40 项、`codex-relay-core` 172 项、路径安全 3 项、Provider workflow 1 项均通过；依赖图、fmt、
  Clippy 和 workspace 测试均通过。
- 显式移除当前构建进程的两个 Tauri 签名环境变量后，最新 `npm run build` 退出 0，用时约 213 秒；
  未生成本次 `.sig` 或 `latest.json`（updater artifact count 为 0）。当前候选普通构建产物：
  - `src-tauri/target/release/CodexRelay.exe`：18,903,552 字节，写入时间
    `2026-07-25T22:12:33.7505341+08:00`，SHA-256
    `847D3AC3C5D0E5A00651DAC438B496BAAB4BE002F19DF4CBE6FACF37976CE447`。
  - `src-tauri/target/release/bundle/nsis/Codex Relay_0.2.0_x64-setup.exe`：4,573,176 字节，写入时间
    `2026-07-25T22:12:33.6745453+08:00`，SHA-256
    `4452E250B7F4908FBE23D026F59C90C5B5D9FFF13AE20AD19F41E98A86A2FE3D`。
- 最新提交前审计：`git diff --check` 与任务校验通过；忽略目录仍包含本地 `dev-data/`、`dist/`、
  `node_modules/` 和 Cargo target，但未纳入 Git，受跟踪开发数据只有 `dev-data/.gitkeep`；当前差异中的
  高置信度真实 OpenAI Key、Authorization 值和 Bearer 值命中均为 0。GitHub 不存在 `v0.2.0`
  Release，远端也不存在同名 Tag，可在新候选推送后重新触发工作流。

## 恢复步骤

1. 补齐第二轮 CI 失败与测试预算契约，运行 Trellis check、差异检查和任务校验。
2. 精确提交并推送测试预算调整到远端 `main`，确认远端引用与新候选提交一致。
3. 确认不存在 `v0.2.0` Draft/Tag 冲突后重新触发发布工作流，监控同一新 Run 直至结束；失败时保留真实步骤与日志边界。
4. 只有新 Run 成功生成 `v0.2.0` Draft 后，按步骤 9 审计版本、目标提交、说明、NSIS、`.sig` 与 `latest.json`。
5. Draft 审计通过后公开 Release，执行公开端点复核，再进入隔离 `v0.1.2 → v0.2.0` 升级验证。

## 实施顺序

1. 更新 `src/release-config.test.ts`，先要求版本 `0.2.0` 与新的 `v0.1.2 → v0.2.0` 最终发布说明，运行专项测试取得预期 RED。
2. 最小更新 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 与 `.github/workflows/release.yml`，运行专项测试与 typecheck 取得 GREEN。
3. 搜索 `0.1.2`、`0.2.0`、上一版本说明和版本断言，确认当前配置没有遗漏；历史任务、历史 Release 证据和示例可以保留旧版本。
4. 运行 `npm run check`，记录首次结果、退出码和测试数量；失败时停止发布并按根因处理。
5. 从当前 PowerShell 进程移除 `TAURI_SIGNING_PRIVATE_KEY` 与密码，运行普通 `npm run build`；枚举实际 Release EXE 和 NSIS 的路径、大小、时间、SHA-256，并确认没有本次 `.sig` / `latest.json`。
6. 执行 Trellis check、任务校验、`git diff --check`、`git status --short --ignored`、跟踪文件与高置信度秘密/真实路径审计；把证据和未完成项更新到本文件。
7. 按发布文件和任务材料精确暂存，提交简体中文 Conventional Commit，并推送 `HEAD:main`；确认远端 `main` 精确包含候选提交。
8. 手动触发 `.github/workflows/release.yml`，监控工作流直至结束；记录 Run URL、候选提交、各步骤结论和真实失败。
9. Actions 成功后审计 `v0.2.0` Draft：Release ID、Tag、目标提交、说明、状态、三项资产的大小/SHA-256、`latest.json` 版本/说明/平台 URL、内联签名与 `.sig` 一致性。
10. Draft 全部通过后公开 Release；立即复核 `releases/latest`、Tag、公开 `latest.json`、公开资产大小/SHA-256 和 API asset 二进制下载语义。
11. 准备安全 Windows Sandbox/VM staging，从公开 `v0.1.2` 安装并运行安全基线，再通过应用内 updater 升级到 `v0.2.0`；运行 guest verifier 并读取 `after.json`。
12. 记录已执行/未执行的 UAC、重启、断网、错误签名、下载失败、安装目录和数据保留证据；安全关闭隔离环境并清理临时 staging。
13. 运行最终 Trellis check 和必要的全量验证，判断是否需要更新长期规范；提交最终发布证据、归档任务并记录会话日志。

## 验证命令

```powershell
npx vitest run src/release-config.test.ts
npm run typecheck
npm run check

Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
npm run build

git diff --check
git status --short --ignored
python ./.trellis/scripts/task.py validate .trellis/tasks/07-25-publish-update
```

发布与升级证据必须来自真实 GitHub Actions、Draft/公开 Release 和隔离 Windows 操作，不能由本地单元测试代替。

## 风险文件与回滚点

- `package.json`、`package-lock.json`：npm 版本权威与锁文件根版本。
- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`：Rust package 版本。
- `src-tauri/crates/codex-relay-core/Cargo.toml`：workspace 内部 core package 版本。
- `src/release-config.test.ts`：版本和发布结构契约。
- `.github/workflows/release.yml`：公开 Release 与 `latest.json.notes` 的最终说明来源。
- `.trellis/tasks/07-25-publish-update/`：非秘密发布计划、进度和证据。

版本或说明错误时在候选提交前回退对应文件；Draft 错误时删除 Draft 并用新提交重跑。公开 Release 后不得回退或替换同版本资产，只能发布更高 SemVer。

## 安全门禁

- 不读取、写入或删除真实 `%USERPROFILE%\.codex` 与 `%LOCALAPPDATA%\CodexRelay`。
- fixture 只使用明确的 `test-key-*-not-real`。
- 不输出或持久化 updater 私钥、密码、GitHub Token、真实 API Key、Authorization Header 或完整认证文件。
- 不把普通构建描述为签名、安装或升级成功；不把 Draft 描述为已公开。
- 每次失败、超时、未执行场景和人工观察都按实际状态保留。

## 下一步

重新运行无签名普通构建并记录产物元数据，完成提交前安全审计，提交并推送测试与焦点生命周期修复，然后重新触发并监控 `release.yml`。
