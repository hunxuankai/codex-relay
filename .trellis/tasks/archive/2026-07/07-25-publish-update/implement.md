# Codex Relay v0.2.0 发布实施计划

## 当前状态

任务已激活。版本与最终发布说明切片、typecheck、全量检查、普通构建、候选提交和推送已完成。三轮 GitHub Actions 均在完整检查阶段失败：第一轮已修复 Cargo 正常 stderr 被 PowerShell 误判的问题；第二轮暴露冷 Windows runner 上进程树测试的 5 秒预算不足；第三轮证明只扩大到 30 秒仍不足以解决“由子 PowerShell 自行报告 PID”的错误等待条件。父进程 PID 修复后的第四轮工作流已成功，Draft 审计、公开 Release、公开端点复核、隔离 `v0.1.2 → v0.2.0` 应用内升级验证、最终全范围检查和规范更新判断均已完成；当前只剩证据提交、推送和任务归档。

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
- 修复提交 `fe6ed317f2a375aade4123bf7f7f8d8569c10ba5`（`fix(release): 稳定发布候选检查`）已创建，
  精确包含 Windows 冷启动进程测试预算、延迟焦点生命周期回归与实现、两项长期规范和本任务证据；
  当前本地 `master` 领先 `origin/main` 1 个提交，尚未推送。
- 任务证据提交 `b065764afb66e6b5ebc34feeacbe428626314a9d` 已连同修复提交推送到远端 `main`；
  本地 `HEAD`、`origin/main` 与远端引用精确一致，推送后工作区干净。触发前 GitHub Status 为
  All Systems Operational，且不存在 `v0.2.0` Release 或 Tag。
- 第三轮 GitHub Actions Run `30161343392`（`https://github.com/hunxuankai/codex-relay/actions/runs/30161343392`）
  已于 `2026-07-25T14:17:16Z` 通过 `workflow_dispatch` 创建，head SHA 精确为
  `b065764afb66e6b5ebc34feeacbe428626314a9d`。Run 于 `2026-07-25T14:26:27Z` 失败：检出、Node、
  Rust、依赖安装、Trellis、前端、Rust 依赖图、fmt、Clippy、主 crate 40 项均通过；core 172 项中
  171 项通过，只有 `cancellation_terminates_descendant_processes_in_the_job` 在 30 秒后仍未看到 PID。
  `output_limit_terminates_the_process_tree` 本轮通过，Draft 构建被跳过，`v0.2.0` Release 仍不存在。

## 第三轮 Actions 失败与假设修正

- 第三轮日志推翻了“只把 5 秒扩大为 30 秒即可”的完整性假设。失败测试等待的是子 PowerShell 完成
  冷启动、开始执行 `child.ps1` 并自行写入 `$PID`；但 Job Object 取消契约只需要确认后代进程已经由
  父进程创建并取得真实 PID。
- 父脚本的 `Start-Process -PassThru` 返回值已经提供 `$child.Id`。最小根因修复改为由父进程在
  `Start-Process` 返回后立即写 PID，再等待子进程；子进程只执行 120 秒挂起命令，确保取消前不会自然
  退出。测试继续保留 30 秒有界条件轮询、严格 `Cancelled` 结果和“子 PID 不再运行”断言。
- 该调整不改变生产 Job Object、进程启动、取消、超时或输出上限逻辑，只纠正测试观察点；未来不得
  通过继续扩大任意 timeout 掩盖错误条件。
- 修复后的本地验证：`cargo fmt --all --check` 通过；`codex_process` 5 项专项通过；取消后代进程用例
  连续重复 3 次均通过（每次约 0.7–0.9 秒）。下一步运行完整 `npm run check`，再重新生成候选提交。
- 最新完整 `npm run check` 已在父进程 PID 修复后退出 0，用时约 192.4 秒：Trellis 8 项、前端 37/176
  项、主 Rust crate 40 项、`codex-relay-core` 172 项、路径安全 3 项、Provider workflow 1 项全部通过；
  依赖图、fmt、Clippy 和 workspace 测试均通过。下一步重新执行无签名普通构建，再提交并推送新候选。
- 父进程 PID 修复后的无签名 `npm run build` 已退出 0，用时约 215 秒；两个 Tauri 签名环境变量均为
  `False`，且 `UPDATER_ARTIFACT_COUNT=0`。最终候选普通构建产物：
  - `src-tauri/target/release/CodexRelay.exe`：18,903,552 字节，写入时间
    `2026-07-25T22:38:51.0466447+08:00`，SHA-256
    `6CA203EA5042F7970C677AF00A78018EFC00D21CF3CA4D66FC6ED289895B24AC`。
  - `src-tauri/target/release/bundle/nsis/Codex Relay_0.2.0_x64-setup.exe`：4,573,253 字节，写入时间
    `2026-07-25T22:38:50.9765553+08:00`，SHA-256
    `5417BC319FDCDE4579BE89DE9150A6503EACABC095EAF8D303E744E1E7240289`。
- 新候选提交 `71bf86f115a543bcb4b969502585f7e17e3eaf48`（`test(rust): 修正后代进程测试观察点`）已推送，
  本地 `HEAD` 与 `origin/main` 精确一致，工作区干净；远端 `v0.2.0` Release/Tag 仍不存在，可以触发
  下一轮单一工作流。

## Draft 审计证据

- Run `30162150074`（`https://github.com/hunxuankai/codex-relay/actions/runs/30162150074`）于
  `2026-07-25T14:42:05Z` 触发，`headSha=d745f285e4d0d12301a31a5f318b30952b3fba33`，于
  `2026-07-25T14:57:41Z` 成功；检出、Node、Rust、依赖安装、完整检查和 Draft 构建每一步均为 success。
- Draft Release ID `359782094`，标题 `Codex Relay v0.2.0`，`tag_name=v0.2.0`、`draft=true`、
  `prerelease=false`，`target_commitish=d745f285e4d0d12301a31a5f318b30952b3fba33`，与 Run head 精确一致。
  Draft 页面暂用 `untagged-884ad2397451c7a69c40` 地址，公开前不把它描述为正式 Tag。
- Draft 恰有三个资产（API 返回大小/下载后大小/sha256 digest 均一致）：
  - `Codex Relay_0.2.0_x64-setup.exe`（实际文件名 `Codex.Relay_0.2.0_x64-setup.exe`）：4,573,718 字节，
    SHA-256 `d6a70c69b4e7e1c4f2621b905b2e433c05e4f272b80219b0ea6f689b286cb3d1`。
  - `Codex Relay_0.2.0_x64-setup.exe.sig`（实际文件名 `Codex.Relay_0.2.0_x64-setup.exe.sig`）：424 字节，
    SHA-256 `d499f14b01f52433d77b42d605f88f7c4676d50e2e76efda7b30aec718d44798`。
  - `latest.json`：2,332 字节，SHA-256 `948c27533335876c3e5b1fbd9084fa880539e74d30974faeae0a7fa4c206f557`。
- `latest.json` 的 `version=0.2.0`、`pub_date=2026-07-25T14:57:33.219Z`，平台恰为
  `windows-x86_64` 与 `windows-x86_64-nsis`；两者 URL 均指向
  `https://api.github.com/repos/hunxuankai/codex-relay/releases/assets/489516982`，且都与独立 `.sig`
  的签名内容一致。Release body 与 `latest.json.notes` 逐字一致，并包含未知发布者和数据保留提示。
- 使用 `Accept: application/octet-stream` 对该 API asset URL 做等价二进制请求，得到 4,573,718 字节、
  SHA-256 `d6a70c69b4e7e1c4f2621b905b2e433c05e4f272b80219b0ea6f689b286cb3d1`，与 Draft 资产一致。
- 本机未安装 `minisign`，未执行独立密码学验签；仅记录 Actions 成功、API digest、下载字节和内联/独立
  签名一致性，不把它们扩大为本地密码学验证声明。Draft 审计时尚未公开。

## 公开发布与复核证据

- 发布前再次确认 Release ID `359782094` 仍为 Draft、非 prerelease、目标提交为
  `d745f285e4d0d12301a31a5f318b30952b3fba33` 且资产数为 3；随后执行公开并显式标记 Latest，命令退出 0，
  正式页面为 `https://github.com/hunxuankai/codex-relay/releases/tag/v0.2.0`。
- 公开后 `releases/latest` 与 `releases/tags/v0.2.0` 均返回 `v0.2.0`、`draft=false`、
  `prerelease=false`、目标提交 `d745f285e4d0d12301a31a5f318b30952b3fba33`；公开资产仍恰为 3 项。
- 公开 `latest.json` 返回 `version=0.2.0`、`pub_date=2026-07-25T14:57:33.219Z`，Release body 与 notes
  仍逐字一致，平台仍为 `windows-x86_64` 和 `windows-x86_64-nsis`，两者均指向 API asset
  `489516982`。
- 公开 Tag 直链和带 `Accept: application/octet-stream` 的 API asset 请求都得到 4,573,718 字节，
  SHA-256 均为 `d6a70c69b4e7e1c4f2621b905b2e433c05e4f272b80219b0ea6f689b286cb3d1`，与 Draft digest 完全一致。
  公开 `latest.json` 仍为 2,332 字节，SHA-256
  `948c27533335876c3e5b1fbd9084fa880539e74d30974faeae0a7fa4c206f557`；独立 `.sig` 仍为 424 字节，
  SHA-256 `d499f14b01f52433d77b42d605f88f7c4676d50e2e76efda7b30aec718d44798`。未发现 Draft 到公开的资产漂移。

## 隔离应用内升级证据

- Windows Sandbox staging 位于系统临时目录的真子路径，宿主机复核 staging 及其现有父路径均不是
  reparse point；输入映射只读、结果映射可写，未使用真实 `.codex`、真实 Relay 应用数据或仓库目录。
- 基线安装器来自公开正式 Release `v0.1.2`，Release 与 staging 文件均为 3,976,952 字节，SHA-256
  `944F55AABACD1615ECEDF95A1D715F11A15DFD6FD8C8CA344341139FFD203D70`；`v0.1.2` 为
  `draft=false`、`prerelease=false`。
- `before.json` 在基线安装前写回；用户从 `v0.1.2` 设置页执行应用内更新并完成安装。用户在
  Windows Sandbox 中人工观察到：未出现 UAC，安装后应用自动重启。
- `guest-verify.ps1` 于 `2026-07-25T15:39:27.0167673Z` 写出 `after.json`，结果为：
  `expectedVersion=0.2.0`、`installedVersion=0.2.0`、`versionMatched=true`、
  `executablePresent=true`、`dataPreserved=true`、`success=true`。
- 升级后登记安装目录为 `C:\Program Files\Codex Relay`，可执行文件为
  `C:\Program Files\Codex Relay\CodexRelay.exe`；应用内升级沿用已登记目录，没有生成第二套安装。
- 三项白名单 fixture 均存在，长度与 SHA-256 前后一致：

| 相对路径 | 长度 | SHA-256 |
|---|---:|---|
| `codex/auth.json` | 56 | `2EAF2CA9850326AE844E2BE84455EAA246461C523A50A46D5CC97D7239355E81` |
| `codex/config.toml` | 273 | `753D51E4C045937E1EDB169BA6D6088E15BFDC20B78968C72C668539F1173267` |
| `app-data/providers.json` | 180 | `E7B0872EA3A97EB0740D9B53496FBA6B21329BBB41CDE6C12D17D6AD1725342A` |

- `before.json` / `after.json` 只包含相对路径、长度、SHA-256 和安装元数据；扫描未发现
  `test-key-*`、`OPENAI_API_KEY`、Authorization 或 Bearer 内容。
- 本轮未执行更新取消、断网、错误签名和下载失败场景，均明确标记为未验证；未安装本地
  `minisign`，仍不声称完成独立本地密码学验签。
- 关闭 Sandbox 后首次清理 staging 时，Sandbox 共享目录的异步卸载仍占用空 `input` 目录，
  `Remove-Item` 真实失败；确认 `WindowsSandbox` / `WindowsSandboxClient` 均已退出、目标仍为系统临时
  目录真子路径且无 reparse point 后，第二次受保护清理退出 0，staging 已不存在。

## 最终全范围检查与提交前审计

- 最终 `npm run check` 退出 0，用时约 106.7 秒：Trellis 8 项、前端 37 个测试文件/176 项、Rust
  主 crate 40 项、`codex-relay-core` 172 项、路径安全 3 项和 Provider workflow 1 项全部通过；依赖图、
  typecheck、fmt、Clippy 和 workspace 测试均通过。
- 版本复核通过：`package.json`、`package-lock.json`、lock 根包、`codex-relay` 和
  `codex-relay-core` 均为 `0.2.0`；任务校验通过。
- 首次聚合审计因 Windows PowerShell `ConvertFrom-Json` 默认不支持 `package-lock.json` 的空字符串属性而
  失败；改用 `-AsHashtable` 读取 lock 根包后，同一审计退出 0。该失败属于审计命令兼容性，不是项目检查失败。
- 最终公开端点复核仍返回 `v0.2.0`、`draft=false`、`prerelease=false`、目标提交
  `d745f285e4d0d12301a31a5f318b30952b3fba33`；`releases/latest`、Tag 和 `latest.json.version`
  一致，Release body 与 notes 一致，平台仍为 `windows-x86_64` 与 `windows-x86_64-nsis`，三个资产的
  数量、大小和 digest 与 Draft/公开初次审计相同。
- `git diff --check` 通过；提交前仅 `implement.md` 有差异，本地 `HEAD` 与 `origin/main` 均为
  `a15e310c8d5c0f851bc23894ee380e0dbfb07420`。差异中的高置信度 OpenAI Key、Bearer、Authorization、
  私钥块和真实宿主路径命中均为 0，Git 跟踪的真实 `auth.json`、`providers.json` 或备份路径数量为 0。
- `trellis-update-spec` 判断：本轮真实 Sandbox 结果验证了既有 updater、路径安全和完成证据契约，
  没有新增产品/API/环境变量行为；共享目录短暂占用属于一次性运行时序，现有规范已覆盖路径复核、如实记录
  清理失败和确认目录不存在。发布耗时复盘发现 Actions 外部排队、Job 执行和人工门禁仍需明确拆分，因此已在
  `release/publishing.md` 增加状态页检查、单一活动 Run、时间戳记录、失败分类和新候选重试规则。

## 发布耗时与 Actions 失败复盘

- 从任务规划材料于 `2026-07-25 20:02 +08:00` 写入，到隔离升级证据提交 `febc69d` 于
  `2026-07-26 00:27:59 +08:00` 创建，墙钟约 4 小时 26 分钟；正式 Release 在
  `2026-07-25 23:04:38 +08:00` 公开，之后仍有约 29 分钟 Sandbox 升级和最终检查/人工确认。
- 四个 Actions Run 从创建到结束合计约 90 分钟，其中第一轮因 GitHub Actions `major_outage` 从
  `20:21:00` 排队到 `21:15:59`，单独消耗约 55 分钟外部等待；第一轮 Job 实际只运行约 2 分 22 秒。
- 第二、三、四轮 Run 分别约 8 分 8 秒、9 分 11 秒和 15 分 36 秒。成功轮的完整检查约 7 分 24 秒，
  Draft 构建约 7 分 2 秒；这是发布门禁本身的真实成本，不能通过跳过检查或放宽断言缩短。
- 三次失败后的本地根因调查、专项回归、完整 `npm run check`、普通构建、提交和新候选推送分别占用约
  15 分钟、36 分钟和 16 分钟。第二次修复期间还由完整前端检查发现并修复了卸载后焦点 timer 的未处理异常，
  这不是 Actions 原始失败，但必须在下一候选前解决。

### 根因分类

1. **外部服务阻塞**：第一轮长时间 `queued` 来自 GitHub Actions 全局事故，不是仓库构建失败；没有重复触发
   相同提交，但等待本身占据了本次发布最大的一段不可控时间。
2. **D. 测试覆盖缺口 + E. 隐式假设**：PowerShell 在 `$ErrorActionPreference='Stop'` 下把退出 0 的
   Cargo 正常 stderr 提升为 `NativeCommandError`；本机缓存命中没有产生 `Updating crates.io index`，
   因而发布前未覆盖冷 runner 行为。
3. **D. 测试覆盖缺口 + E. 隐式时序假设**：Windows 进程树测试把 PowerShell 冷启动、后代创建和大量
   输出吞吐错误地限制在 5 秒内，第二轮因此出现一个 PID 等待失败和一个 `Timeout`。
4. **心智模型/观察点错误**：第一次把预算扩大到 30 秒只解决了输出上限用例，没有解决取消用例；测试仍要求
   子 PowerShell 完成冷启动后自行写 PID，而 Job Object 契约只需要父进程已创建后代。第三轮日志推翻该假设后，
   改为父进程从 `Start-Process -PassThru` 立即记录 `$child.Id`，第四轮才成功。

### 防复发记录

- `478a094` 新增原生命令包装和 fake Cargo 回归测试：成功命令即使向 stderr 输出进度，也以
  `$LASTEXITCODE` 判定；真实退出非 0 仍保留诊断并失败。
- `fe6ed31` / `71bf86f` 为 Windows 冷 runner 使用 30 秒测试级有界预算、20 毫秒条件轮询和父进程 PID
  观察点；生产超时与严格 `OutputTooLarge` / `Cancelled` 断言未放宽。
- `.trellis/spec/testing/rust-build-feedback.md` 已包含“Windows PowerShell 原生命令诊断”和
  “Windows 冷 runner 进程树测试”两个完整场景、错误矩阵、良好/错误示例和必需测试。
- `.trellis/spec/release/publishing.md` 已新增外部状态与运行时长门禁：触发/重试前检查 GitHub Status，
  只保留一个活动 Run，分开记录排队与执行时间，按外部服务、工具诊断、CI 时序和产品契约分类失败，并且只在
  专项回归、完整检查和新候选提交完成后重试。
- 这些机制能防止本次已知错误按相同方式复发，但不能消除 GitHub 服务事故或未来全新的冷环境差异；后者仍需
  通过真实失败证据、最小根因修复和新回归测试处理。

## 最终剩余步骤

1. 提交并推送最终发布证据。
2. 归档任务并记录开发日志。

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

提交并推送最终发布证据；随后归档任务并记录开发日志。
