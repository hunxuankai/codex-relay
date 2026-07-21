# 无服务器应用内更新实施计划

## 状态

规划已完成并获得用户明确实施授权，当前任务处于 `in_progress`。

## 前置输入

开始实施前必须具备：

- 公开 GitHub 仓库 `hunxuankai/codex-relay` 已存在；SSH 地址为 `git@github.com:hunxuankai/codex-relay.git`，HTTPS 更新清单地址为 `https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json`。本轮只读 API 已确认该仓库为公开空仓库，尚无提交或 Release。
- Tauri 更新公钥，以及由用户自行放入 GitHub Actions Secrets 的私钥和可选密码。私钥不得出现在命令输出、Git、任务材料或聊天中。
- Windows Sandbox 可用，或一个可恢复快照的 Windows 虚拟机。
- 用户对 `prd.md`、`design.md` 和本计划的最终批准。

客户端适配和纯自动化测试可以在公钥生成前编写，但带公钥的最终配置、真实 Release 和系统级验收必须等待密钥与 Actions Secret 配置。一个任务内按顺序完成客户端与发布切片，因为任一部分单独存在都不能交付可用更新链路，不拆分父子任务。

## 当前进度

Phase 1 规划已完成并获得用户明确实施授权，任务已进入 `in_progress`。切片 1 至切片 7 已完成：updater 客户端、Draft 发布工作流、`v0.1.0` 公开发布、资产/签名/API 下载兼容性均已核对。切片 8 已完成 Sandbox 功能确认、安全 staging/guest 脚本、dry-run、真实 guest bootstrap/升级前快照及 `v0.1.0` 基线安装；基线应用启动后暴露的 health 初始化竞态已按 RED→GREEN 修复。Run #3 暴露的 Windows 8.3 路径测试假失败已修复并推送到 `f6200b0`；Run #4 成功生成的 `v0.1.1` 已核对并公开发布。用户再次通过安全入口启动 `v0.1.0` 后仍永久停留在启动页，验收改用人工安装 `v0.1.1` 恢复基线，再以 `v0.1.1 → v0.1.2` 完成真实应用内升级。`v0.1.2` 的版本/说明切片已按 RED→GREEN 完成，并通过完整检查和无私钥普通构建；Sandbox 人工恢复 `v0.1.1` 已完成。候选已提交推送，Run #5 与 Draft 三项资产均已核对，当前等待用户确认公开 `v0.1.2`。

## 已完成

- 完成现状、Tauri updater、静态 GitHub Releases、NSIS 参数、签名边界和本机工具调研。
- 用户逐节确认手动检查、应用内安装、公开 GitHub Releases、Draft 发布、半自动 Sandbox/VM 验收等设计。
- 创建并通过 `task.py validate` 的 `prd.md`、`design.md`、`implement.md` 和 `research/tauri-updater.md`。
- 已确认公开仓库 `hunxuankai/codex-relay` 存在且当前为空仓库；未自动添加 Git remote。
- 切片 1 首个结构测试已按 RED→GREEN 执行；普通构建入口保持不读取签名私钥，发布入口使用 `src-tauri/tauri.updater.conf.json`。
- typed updater 服务把官方 `Update` 句柄封装为应用自有 session，规范化累计进度、无更新结果和安全错误。
- `useUpdater` 只在创建时读取本地版本，远端检查仅由用户显式动作触发；已覆盖单飞、旧响应、确认、取消、进度和 launching 状态。
- `UpdatePanel` 位于设置表单外，Release notes 只按纯文本渲染；`ConfirmDialog` 默认危险样式保持不变并增加中性变体。
- `.github/workflows/release.yml` 使用手动触发、完整检查、固定 SHA Actions、Draft Release 和 NSIS updater JSON 优先；尚未在 GitHub 执行。
- 用户已在 GitHub Actions Secrets 配置更新私钥与密码，并仅提供可公开提交的公钥；基础配置已锁定固定 HTTPS endpoint 与该公钥。
- 已更新产品、架构和发布规范，并新增 `.trellis/spec/release/updater.md`，沉淀 endpoint、公钥、Secrets、Draft、构建分离和系统验收契约。
- 新增 `scripts/windows-sandbox/` 主机准备、guest bootstrap、安全重启和升级后核验脚本；只读输入/可写结果映射、成对 Relay 覆盖、报告脱敏、白名单唯一性和 reparse point 防护均有自动化测试。
- Windows 10 Enterprise 22H2 上确认 Sandbox 功能已启用；开始菜单注册名为英文 `Windows Sandbox`，直接启动后实际窗口标题为“Windows 沙盒”。配置化 Sandbox 已完成 guest bootstrap 并写回升级前报告。
- `package.json`、package lock、Cargo 元数据和发布说明已统一为 `0.1.1` 本地候选，用于从 `v0.1.0` 执行首条真实应用内更新链路。

## 关键决策

- 使用官方 `tauri-plugin-updater`，不自写下载器或安装器。
- 保留 Tauri 更新包签名校验；不使用 Windows Authenticode。
- 仅用户手动检查，一条 `windows-x86_64` 稳定通道，NSIS 被动安装，人工恢复而非自动回滚。
- `createUpdaterArtifacts` 只放在发布配置覆盖中，普通本地构建不依赖私钥。
- 发布工作流 `workflow_dispatch` → 完整检查 → Windows 构建 → Draft Release → 人工发布。
- 人工安装 `v0.1.1` 只恢复可操作的 updater 客户端，不计为应用内升级成功；首条端到端 updater 证据改由更高版本 `v0.1.2` 提供。

## 验证证据

- `python ./.trellis/scripts/task.py validate .trellis/tasks/07-21-serverless-update-research`：通过。
- `python ./.trellis/scripts/get_context.py`：当前 session 精确指向本任务，状态 `planning`。
- GitHub API `GET https://api.github.com/repos/hunxuankai/codex-relay`：返回 `private=False`、`size=0`、默认分支 `main`；`git ls-remote ... HEAD` 无引用且退出码 0，符合空仓库状态。
- 规划材料占位符/行尾空白扫描：未发现未决占位标记或尾随空格。
- 尚未运行产品测试、构建、签名、GitHub Actions、安装、升级或 Sandbox 验收；不得据此宣称它们成功。
- `npx vitest run src/release-config.test.ts`：8 项通过。
- 公钥配置切片 RED：`npx vitest run src/release-config.test.ts` 的 10 项中仅新增 endpoint/pubkey 契约因 `tauri.plugins.updater` 缺失失败。
- 公钥配置切片 GREEN：补齐基础配置后同一命令 10 项全部通过。
- `npm run typecheck`：退出码 0。
- `cargo check --manifest-path src-tauri/Cargo.toml`：退出码 0。
- `npm run test`：17 个测试文件、77 项测试通过。
- `npm run check`：退出码 0；包含 77 项前端测试、107 项 Rust 单元测试、2 项路径安全测试、1 项 Provider 工作流测试和 8 项 Trellis 测试。
- 本轮首次 `npm run check` 在工具 124 秒超时后失去父进程输出；遗留的 `cargo test` 子进程完成后，使用 300 秒上限原样重跑，退出码 0：8 项 Trellis 测试、17 个前端测试文件共 78 项、107 项 Rust 单元测试、2 项路径安全测试和 1 项 Provider 工作流测试通过。
- 在移除 `TAURI_SIGNING_PRIVATE_KEY` 与密码环境变量后运行 `npm run build`：退出码 0，证明普通构建不依赖更新私钥。
- Release 主程序：`src-tauri/target/release/CodexRelay.exe`，16,630,272 字节，SHA-256 `4A4979110A70204157B081C4B6914CD0C00CB1624DC02A369E5109C6931CB941`。
- NSIS 安装器：`src-tauri/target/release/bundle/nsis/Codex Relay_0.1.0_x64-setup.exe`，3,974,662 字节，SHA-256 `BF1ED6487F9F1881E277BC64731D4C8DE928358D77FE808020526170F7984729`。
- 本轮公钥配置完成后再次显式移除两个签名环境变量并运行普通 `npm run build`：退出码 0；Release 主程序 16,633,344 字节，SHA-256 `2EE4172FBBB7C1D609AA745C4DFD3502E75E72156A8C4F4A6CD5A1F9D8FDE374`；NSIS 安装器 3,979,177 字节，SHA-256 `CF89DD3A37C0AC53B64FF8F1EDDD911CF6370F6B15D894E50993FC10C5E4E6A5`。
- `git diff --check` 未报告空白错误；仓库内未发现私钥扩展名文件或 minisign/PEM 私钥标记。扫描命令因 `rg` 无匹配返回 1，此退出码表示未发现命中，不是扫描执行失败。
- 规范更新后的提交前复验：`npm run check` 再次退出 0，17 个前端测试文件共 78 项、107 项 Rust 单元测试、2 项路径安全、1 项 Provider 工作流和 8 项 Trellis 测试通过；`task.py validate` 通过，跟踪敏感文件名与私钥标记命中数均为 0。
- 本地功能提交：`a98ab51 feat(updater): 增加手动检查与应用内更新`，包含 updater 运行时、发布工作流、公开公钥、测试、README、规范和任务材料；尚未推送时不宣称远端已有该提交。
- HTTPS remote 已配置为 `https://github.com/hunxuankai/codex-relay.git`；`master` 已成功推送到远端 `main`，远端运行目标提交为 `8883da75b794bbc7898cb8143afba2ee9c4532bd`。
- GitHub Actions `发布 Windows 更新 #1`（`https://github.com/hunxuankai/codex-relay/actions/runs/29802000663`）完成且结论为 `success`；检出、Node、Rust、依赖安装、完整检查和 Draft 构建步骤均成功。
- Draft Release `Codex Relay v0.1.0` 已生成且未发布，目标提交为 `8883da7`。资产：NSIS 3.79 MB，SHA-256 `227a650b205442c0cfff2dee201df71c49a80759eb56f025581a0c6cd85d1c75`；`.sig` 424 字节，SHA-256 `9d67394d4b92ded5bb2ab80e1a7f2eb85e286737978e21b99eab2177268d0be7`；`latest.json` 1.31 KB，SHA-256 `5d8f2eded14e7f9919b8d79e93cc054ccab6b99d801a8a3ac9cf587c66dbb328`。
- 用户提供的 Draft `latest.json` 可解析：版本 `0.1.0`，`windows-x86_64` 与 `windows-x86_64-nsis` 均指向实际 NSIS asset ID `484266111`，两个键使用同一签名。签名 envelope 与公开公钥的 minisign key ID 均为 `7DC2AE5ACEA0D00A`；本机 Tauri CLI 没有 `signer verify` 子命令，因此完整二进制签名验证仍等待可读取的实际资产。
- 清单核对发现 `notes` 仍为发布工作流占位文案。新增结构测试后，10 项中仅“最终发布说明”契约失败；将 `releaseBody` 改为正式多行简体中文说明后，同一专项 10 项全部通过。当前 Draft 资产仍是旧清单，不得发布，必须重新生成。
- 正式说明修复后的完整复验：`npm run check` 退出 0，17 个前端测试文件共 78 项、107 项 Rust 单元测试、2 项路径安全、1 项 Provider 工作流和 8 项 Trellis 测试通过；任务材料验证通过，私钥标记命中数为 0。
- 发布说明修复提交 `e3809f0 fix(release): 避免占位说明进入更新清单` 已推送到远端 `main`；修复推送后曾确认旧 Draft 仍引用提交 `8883da7` 和旧清单，随后按用户确认删除并重建。
- 用户明确确认后，旧占位 Draft 及其 3 个资产已删除；GitHub 页面显示 “Your release was removed” 且没有剩余 Release。
- GitHub Actions `发布 Windows 更新 #2`（`https://github.com/hunxuankai/codex-relay/actions/runs/29803671969`）以最新提交 `194a9b54fc691e212a3ed437723866e71bcde992` 运行完成，结论为 `success`；完整检查和 Draft 构建步骤均成功。
- 新 Draft `Codex Relay v0.1.0` 已生成且未发布，页面显示正式“更新内容 / 首次安装 / 注意事项”中文说明。资产：NSIS 3.79 MB，SHA-256 `e9dcb19419a198ac00244a579d5b81874cc847200fc216d1797fd837a693dad9`；`.sig` 424 字节，SHA-256 `6dfb4765df2d507e48f507603f1c84bdc396c8458c57c5bf28c1e13da0ac3500`；`latest.json` 1.85 KB，SHA-256 `c96bb5862372d881b81edd069d579eda80f1852c0a4ec554b1e78d91cd77c50f`。
- 用户提供的新 `latest.json` 可解析：版本 `0.1.0`，`notes` 与 Draft 页面正式中文说明一致；`windows-x86_64` 和 `windows-x86_64-nsis` 使用同一签名并指向新 NSIS asset ID `484291952`。签名 envelope 的 key ID 与客户端公开公钥均为 `7DC2AE5ACEA0D00A`，可信注释文件名为 `Codex Relay_0.1.0_x64-setup.exe`。
- `v0.1.0` 已公开发布：Release ID `357130493`，`draft=false`、`prerelease=false`，Tag 目标为 `194a9b54fc691e212a3ed437723866e71bcde992`，发布时间为 `2026-07-21T05:45:48Z`。
- GitHub API 公布的公开资产为：NSIS asset ID `484291952`，3,977,417 字节，SHA-256 `e9dcb19419a198ac00244a579d5b81874cc847200fc216d1797fd837a693dad9`；`.sig` asset ID `484291955`，424 字节，SHA-256 `6dfb4765df2d507e48f507603f1c84bdc396c8458c57c5bf28c1e13da0ac3500`；`latest.json` asset ID `484291961`，1,899 字节，SHA-256 `c96bb5862372d881b81edd069d579eda80f1852c0a4ec554b1e78d91cd77c50f`。
- 公开 `releases/latest/download/latest.json` 可解析且无占位说明；两个 Windows 平台键均指向 asset ID `484291952`，签名相同，公开 `.sig` 文本与清单内联签名完全一致。
- 当前依赖树确认使用 `tauri-plugin-updater v2.10.1`。该版本 `Update::download` 在请求头缺少 `Accept` 时自动插入 `application/octet-stream`；对清单中的 GitHub API asset URL 实测，默认请求得到 1,871 字节元数据，加入该 Header 后下载 3,977,417 字节安装器，SHA-256 为 `e9dcb19419a198ac00244a579d5b81874cc847200fc216d1797fd837a693dad9`，与公开资产一致，因此该 URL 形态与当前 updater 版本兼容。
- 记录公开发布与 API asset 兼容性证据后运行 `npm run check`：退出码 0；8 项 Trellis 测试、17 个前端测试文件共 78 项、107 项 Rust 单元测试、2 项路径安全测试和 1 项 Provider 工作流测试通过。
- 尚未执行 Windows Sandbox/VM 中的首次安装、应用内升级、UAC、安装器启动后的真实签名校验和数据保留验收；上述发布与源码证据不得用于声明这些系统场景成功。
- Sandbox 只读诊断：Windows 10 Enterprise 22H2 build `19045.6456`；`Containers-DisposableClientVM` 的 `Win32_OptionalFeature.InstallState=1`，`WindowsSandbox.exe`/`WindowsSandboxClient.exe` 存在，`Get-StartApps` 注册名为 `Windows Sandbox`，`vmcompute`/`hns` 正在运行。当前非提升 shell 调用 `Get-WindowsOptionalFeature` 仍按预期返回“请求的操作需要提升”。
- 直接启动 Sandbox 后观察到 `WindowsSandboxClient` 响应正常、窗口标题为“Windows 沙盒”并出现独立 `vmwp`/`vmmem`；随后关闭该空白探测实例并用配置化 `.wsb` 重新启动。
- `src/sandbox-update.test.ts` 按 RED→GREEN 增加 7 项：临时 staging/映射、guest fixture/脱敏报告、升级后版本与哈希、成对覆盖安全重启、重复报告项拒绝、junction 拒绝和真实 Relay 路径源文件拒绝。专项负例曾分别真实观察到脚本错误接受重复报告项、reparse staging 和伪 `.codex` 下的安装器，修复后对应测试通过。
- 配置化 Sandbox staging：公开 `v0.1.0` 安装器复制后 SHA-256 为 `E9DCB19419A198AC00244A579D5B81874CC847200FC216D1797FD837A693DAD9`，与 Release 一致；guest 写回的 `before.json` 仅含三项相对路径、长度和 SHA-256，覆盖指向 `C:\CodexRelaySandbox\dev-data\codex` 与 `C:\CodexRelaySandbox\dev-data\app-data`，未包含 fixture 密钥文本。
- 首次 `npm run check` 在 Trellis 8 项通过后因 `src/sandbox-update.test.ts` 数组索引未显式收窄而在 typecheck 失败，Rust 未运行；修复测试类型后 `npm run typecheck` 退出 0。随后从头运行 `npm run check` 退出 0：Trellis 8 项、18 个前端测试文件共 84 项、107 项 Rust 单元测试、2 项路径安全和 1 项 Provider 工作流通过。
- 显式移除 `TAURI_SIGNING_PRIVATE_KEY` 与密码后运行 `npm run build`：退出码 0。`src-tauri/target/release/CodexRelay.exe` 为 16,633,344 字节，SHA-256 `28ED9C6268AAF9D297E50D87AB697CD9EAC12AFAAE503B0894075AD484923F4C`；`Codex Relay_0.1.1_x64-setup.exe` 为 3,976,911 字节，SHA-256 `F4294EF64AC6EC2100C02DF4512DFB424E43452BFC588BAFCDEA35598DBDBAFF`。该普通构建不包含 updater `.sig`/`latest.json`，不能作为签名或升级成功证据。
- Sandbox 诊断确认 HKLM 安装项版本为 `0.1.0`、安装目录为 `C:\Program Files\Codex Relay`，`CodexRelay.exe` 已从该目录启动；进程级和用户级两项 Relay 覆盖均指向 `C:\CodexRelaySandbox\dev-data` 下的隔离目录，日志文件存在但为空。应用界面持续停留在“正在加载本机配置…”，因此不能把进程存在描述为界面初始化成功。
- health 竞态 RED：新增测试模拟初始 critical 自检未完成时 extended 自检先完成，`npx vitest run src/composables/resourceComposables.test.ts` 的 9 项中仅新增用例失败，实际 `loading=true`、预期为 `false`。
- health 竞态 GREEN：`runExtended()` 仅在自身仍为当前请求时结束初始 loading；同一专项 9 项通过，迟到的 critical 结果不覆盖 extended 报告。App/health 组合专项 12 项、Sandbox 专项 7 项和 `npm run typecheck` 均退出 0。
- 修复及最后两项 Sandbox 路径安全补丁后的完整复验：`npm run check` 退出 0；8 项 Trellis 测试、18 个前端测试文件共 87 项、107 项 Rust 单元测试、2 项路径安全和 1 项 Provider 工作流测试通过。
- Windows 自动化控制通道本轮不可用，未对 Sandbox 发送不可靠的全局按键。临时诊断版 `guest-start-app.ps1` 已从 staging 内备份恢复，源/目标 SHA-256 均为 `D0BB33A7722F43C417A78C71DC031C1B9328ADD4555A37834689F6467EF0CA64`；`WindowsSandboxClient`、`vmwp` 和 `vmmem` 仍在运行。
- 提交前差异审查发现诊断报告中的 NSIS `InstallLocation` 为带外层双引号的 `"C:\Program Files\Codex Relay"`，而 start/verify 直接路径规范化。将两个 dry-run 改为同形输入后，Sandbox 专项 7 项中对应 2 项因非法路径字符失败，证明真实 guest 入口会找错安装目录。
- 增加共享 `Get-CanonicalInstallLocation`，剥离一对完整外层双引号并拒绝空值/残留引号；start/verify 共同使用后 Sandbox 专项恢复 7/7。当前 Sandbox staging 的 `common.ps1`、start（含备份）和 verify 已从仓库同步，四个目标与各自源文件 SHA-256 一致。
- health 修复后再次显式移除两个签名环境变量并运行 `npm run build`：退出码 0。`src-tauri/target/release/CodexRelay.exe` 为 16,633,344 字节，SHA-256 `44D43468620F995C57B892A8017B58D6774AFA80B198544C04E8EE2B3C1E6CD9`；`Codex Relay_0.1.1_x64-setup.exe` 为 3,976,359 字节，SHA-256 `B378799603EBF3DD83ED39DC08ADA2BAF78044D61A4E5BFC84B96D5162D32BFB`。该普通构建仍不能作为 updater 签名或真实升级成功证据。
- `22e890a`、`e8c3c55`、`6d5d66d`、`9473b04` 四个候选提交已推送到远端 `main`；`git ls-remote` 与本地 HEAD 均为 `9473b04eca49e28d6ef2f573b1359960d1b21fce`。
- GitHub Actions `发布 Windows 更新 #3`（`https://github.com/hunxuankai/codex-relay/actions/runs/29835205616`）精确检出 `9473b04`，但在“运行完整检查”步骤失败；Draft 构建步骤被跳过，因此未生成、签名或发布 `v0.1.1` 资产。4 个测试失败均来自同一 Windows 路径别名：Node 预期使用 `C:\Users\RUNNER~1`，PowerShell 实际输出等价的 `C:\Users\runneradmin` 长路径。
- CI 路径别名修复：Sandbox 测试先要求实际值为绝对路径，再用 `realpathSync.native` 比较现有路径身份，避免把 8.3 短路径与长路径误判为不同目录。本地 Sandbox 专项 7/7、typecheck 和完整 `npm run check` 退出 0；跨会话恢复后再次运行专项 7/7 与完整 `npm run check`，完整检查包含 8 项 Trellis、18 个前端文件共 87 项、107 项 Rust 单元测试、2 项路径安全和 1 项 Provider 工作流。`task.py validate` 与 `git diff --check` 同样通过，318 个跟踪文件中没有禁用的认证/私钥文件名。修复提交为 `ad62084 fix(testing): 兼容 Windows 8.3 路径别名`，仍需推送和新的 GitHub run 证明 Windows runner 恢复。
- `ad62084` 与任务证据提交 `f6200b0` 已推送；远端 `main` 与 GitHub Actions `发布 Windows 更新 #4`（`https://github.com/hunxuankai/codex-relay/actions/runs/29838852791`）的 `head_sha` 均为 `f6200b027f1ca9294f43e260c328c047146202a5`。Run #4 于 `2026-07-21T14:36:33Z` 完成且结论为 `success`，检出、依赖安装、完整检查、Draft 构建和收尾步骤全部成功。状态轮询在最终成功响应后出现一次传输 EOF；新的独立 API 请求再次确认总状态和全部步骤成功。
- 新 Draft `Codex Relay v0.1.1` 保持未发布，tag 为 `v0.1.1`，目标提交为 `f6200b0`，Release 说明与工作流正式中文说明一致。公开 `releases/latest` 仍为 `v0.1.0`，公开 tag API 查询 `v0.1.1` 返回 404，证明 Draft 尚未被客户端发现。
- Draft 三项资产已按实际字节核对：NSIS 为 3,976,351 字节，SHA-256 `17F775BF0DBD61E568F81FD06EB66DA1AFFB3CD56B522128859D43CFE92BD405`；`.sig` 为 424 字节，SHA-256 `78545C7D23EE1A3EFC78FC0069246FEB0BFCED382C758138BFF166013BCCF6DA`；`latest.json` 为 1,991 字节，SHA-256 `EC5337BEBF22D8F1974A3457778EE29B4B44FC8AB1FDA039E11024186A3164AD`。
- `latest.json` 版本为 `0.1.1`，说明与 Draft 页面一致；`windows-x86_64` 与 `windows-x86_64-nsis` 均指向 installer asset ID `484785887`，两项内联签名与 `.sig` 文本完全一致。签名 envelope 的 key ID 与客户端公开公钥均为 `7DC2AE5ACEA0D00A`；本轮未使用独立 minisign verifier 对安装器字节执行密码学校验，真实签名拒绝/接受仍等待 Sandbox 内 updater 路径验证。
- 用户明确回复“发布”后，`v0.1.1` 已公开：Release ID `357413189`，`draft=false`、`prerelease=false`，发布时间 `2026-07-21T15:01:49Z`，tag 与公开 `releases/latest` 均为 `v0.1.1`，远端 tag 和 Release 目标都精确指向 `f6200b027f1ca9294f43e260c328c047146202a5`。
- 公开发布后通过无登录下载重新取得三项资产；实际大小和 SHA-256 与 Draft 核对值完全一致。公开 `latest.json` 仍为 `0.1.1`，两个 Windows 平台 URL 均指向 installer asset ID `484785887`，内联签名保持一致；验证临时目录包含 3 个文件，核对后已从系统临时目录安全删除。
- Windows Sandbox 客户端、`vmwp`、安全 staging 与 `before.json` 仍存在。`computer-use` 连接仍因本机控制管道不存在而不可用，本轮未向 Sandbox 发送全局按键或执行其它非受控 UI 自动化；guest 桌面入口名称已从脚本确认是 `Start Codex Relay Safely.cmd` 和 `Verify Codex Relay Update.cmd`。
- 用户在 Sandbox 再次双击 `Start Codex Relay Safely.cmd`，明确观察到 `v0.1.0` 仍停在“正在加载本机配置…”。这与已复现的 health 初始化竞态一致，证明 `v0.1.0 → v0.1.1` 无法通过可见 UI 执行；不得把后续人工安装 `v0.1.1` 记作 updater 成功。
- `v0.1.2` 恢复链路 RED：先把 `src/release-config.test.ts` 改为要求 package/Cargo 版本 `0.1.2` 及 `v0.1.1 → v0.1.2` 正式说明；专项 11 项中恢复链路用例因实际版本仍为 `0.1.1` 失败。
- `v0.1.2` 恢复链路 GREEN：最小提升 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml` 与 `Cargo.lock` 版本，并更新发布说明。首次 GREEN 尝试暴露说明遗漏既有契约“首次带 updater 的版本需要手动安装”和“Windows Sandbox”；补回后专项 11/11 与 `npm run typecheck` 通过。
- `v0.1.2` 完整本地检查：`npm run check` 退出 0；8 项 Trellis 测试、18 个前端测试文件共 87 项、107 项 Rust 单元测试、2 项路径安全和 1 项 Provider 工作流测试通过。
- 提交前再次显式移除 `TAURI_SIGNING_PRIVATE_KEY` 与密码并运行 `npm run build`，命令于 185.2 秒后退出 0。`src-tauri/target/release/CodexRelay.exe` 为 16,633,344 字节，SHA-256 `48CAB991840DBBB97A08E035FB5EA2E46B1CEBF6C71909801E8F99BCC32EF89A`；`Codex Relay_0.1.2_x64-setup.exe` 为 3,975,782 字节，SHA-256 `A3E2B8F72B7C78C65BE415A5932DB8287F40FADBCACFC45A59E3614132F6B06C`。本轮普通构建未生成 `v0.1.2` `.sig` 或 `latest.json`，符合无私钥边界，但不能作为签名或 updater 成功证据。
- 已把公开 `v0.1.1` NSIS 写入安全 staging 的只读 input；重新核对结果为 3,976,351 字节、SHA-256 `17F775BF0DBD61E568F81FD06EB66DA1AFFB3CD56B522128859D43CFE92BD405`，等待用户在 guest 内人工安装。`results/before.json` 已存在，`after.json` 尚未生成。
- 用户在 Sandbox 内人工覆盖安装 `v0.1.1` 后，通过安全入口确认应用已进入主界面。该观察只证明已知良好基线恢复成功，不计为 updater、Tauri 签名或 `v0.1.1 → v0.1.2` 升级成功；Sandbox 保持开启。
- 用户确认提交分组后，候选提交依次为 `5832198 chore(release): 准备 v0.1.2 恢复升级候选`、`f634be4 docs(release): 固化不可启动基线恢复契约`、`e387adb chore(task): 记录 v0.1.2 恢复链路与验证证据`；`git push origin HEAD:main` 成功，远端 `main` 精确为 `e387adb4ab4617ee3f3c42317ed3599179ada5ae`。
- 新安装的 GitHub CLI `2.96.0` 通过 API 确认登录账户 `hunxuankai`，未输出 Token。手动触发的 Run #5（`https://github.com/hunxuankai/codex-relay/actions/runs/29845939529`）精确检出 `e387adb4ab4617ee3f3c42317ed3599179ada5ae`，于 `2026-07-21T16:04:08Z` 完成且结论为 `success`；完整检查、Draft 构建和全部收尾步骤均成功。唯一 annotation 是 GitHub 将使用 Node.js 20 的固定 Actions 强制运行在 Node.js 24 的弃用提醒，本次未因此失败。
- `v0.1.2` Draft Release ID 为 `357470117`，`draft=true`、`prerelease=false`，目标提交精确为 `e387adb4ab4617ee3f3c42317ed3599179ada5ae`。未登录 `releases/latest` 仍返回公开 `v0.1.1`，未登录 tag API 查询 `v0.1.2` 返回 404，证明 Draft 尚未被客户端发现。
- Draft 三项资产的下载字节与 GitHub digest 完全一致：NSIS 为 3,976,952 字节，SHA-256 `944F55AABACD1615ECEDF95A1D715F11A15DFD6FD8C8CA344341139FFD203D70`；`.sig` 为 424 字节，SHA-256 `0D506B4535CCEBC892EE5C28CCBE3FF5EEEF086EF3BC51B92824C6018C2C5C39`；`latest.json` 为 2,082 字节，SHA-256 `F2DEA8211954E66A251127BF1E32DB17ECF0A355EF98BA3A1A333BDF01073DEB`。
- Draft `latest.json` 版本为 `0.1.2`，说明与 Release body 一致且包含人工恢复边界；平台恰为 `windows-x86_64` 与 `windows-x86_64-nsis`，两者 URL 均指向 installer asset ID `484875016`，内联签名相同且与独立 `.sig` 文本一致。公钥与签名 payload 的原始 key-id 字节均为 `7DC2AE5ACEA0D00A`；minisign 注释按反向显示为 `0AD0A0CE5AAEC27D`（省略前导零时为 `AD0A0CE5AAEC27D`）。本轮未执行独立密码学验签，真实接受仍等待 Sandbox updater。
- Draft 审计下载目录在核对后已从系统临时目录安全删除。当前 staging 的 `input/Verify Codex Relay v0.1.2 Update.cmd` 为 195 字节，SHA-256 `3D209E4BD465EA3414ABA962BBB5B9D61DB675F07EDAEB70083599E99C8F10F2`；它只调用现有 `guest-verify.ps1`、固定期望版本 `0.1.2`，且不引用真实默认目录。

## 尚未解决的问题

- Windows Sandbox 已人工恢复为能进入主界面的 `v0.1.1`；`v0.1.2` Draft 已核对但尚未公开，仍需用户确认发布，再在 guest 内执行应用点击和可能出现的 UAC。
- GitHub Secrets 的存在由用户确认；本机未安装 GitHub CLI，尚未通过 CLI 独立枚举，但不会读取或输出 Secret 值。
- `v0.1.1` 已公开发布并完成公开资产复核，但完整端到端签名校验、应用内升级、重启和 `after.json` 数据保留证据仍未完成。基线安装未出现 UAC，UAC 成功与取消路径均未验证。

## 行为切片与 TDD 顺序

### 切片 1：发布配置契约

**公开行为**：普通构建不需要更新私钥；发布构建显式生成 updater 资产；客户端只拥有固定 HTTPS endpoint 和公钥。

1. 在 `src/release-config.test.ts` 先增加失败断言，覆盖 updater 依赖、版本来源、基础配置、release 配置覆盖、权限和普通构建脚本。
2. 运行专项 Vitest，确认因配置缺失失败。
3. 添加 `@tauri-apps/plugin-updater`、`tauri-plugin-updater`、基础 updater 配置、`updater:default` 权限和发布专用 config；注册 Rust 插件。
4. 保持 `npm run build` 不读取更新私钥；新增或配置发布专用构建入口。
5. 运行专项测试、`cargo check`/Clippy 相关检查和 typecheck。

**风险文件**：`package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json`、发布覆盖文件、`src-tauri/capabilities/default.json`、`src-tauri/src/lib.rs`。

### 切片 2：Typed updater 服务边界

**公开接口**：`UpdateClient.getCurrentVersion()`、`checkForUpdate()`、`UpdateSession.downloadAndInstall()` 和 `close()`，以及 `UpdateReleaseInfo`/`UpdateProgress` DTO。

1. 在 `src/services/tauri.test.ts` 或独立 updater 服务测试中 mock `@tauri-apps/plugin-updater` 与本地版本 API，先描述无更新、可更新、进度累计、资源释放和阶段错误。
2. 运行专项测试，确认因接口缺失失败。
3. 在 `src/types/update.ts` 定义稳定应用类型；只在 `src/services/tauri.ts` 导入官方 Tauri/updater API 并完成规范化。
4. 将未知异常映射为 `UPDATE_CHECK_FAILED` 或 `UPDATE_INSTALL_FAILED` 的安全简体中文消息；不把原始载荷暴露给调用方。
5. 运行服务专项测试和 typecheck。

**mock 边界**：只 mock 官方插件和 `@tauri-apps/api/app`；不 mock 适配层自己的进度累计、错误映射或资源释放逻辑。

### 切片 3：手动检查状态机

**公开行为**：创建 composable 不检查远端；用户调用 `check()` 后得到最新、可更新或失败状态；重复动作与旧响应受控。

1. 新建 `src/composables/useUpdater.test.ts`，通过 fake `UpdateClient` 先覆盖 idle、checking、upToDate、available、error、单飞和旧响应。
2. 运行专项测试，确认因 composable 缺失失败。
3. 实现 `useUpdater({ client })`，使用 `shallowRef` 保存不透明句柄、`computed` 派生 UI 状态、递增序列保护响应，并向外导出 `readonly` 状态和显式动作。
4. 在重新检查、失败复位和作用域销毁时关闭旧资源；不使用 mount/watch 自动检查。
5. 运行 composable 专项测试和 typecheck。

**不应 mock**：Vue reactivity、状态转换、序列号和资源生命周期。

### 切片 4：确认、下载和安装进度

**公开行为**：可更新状态经用户确认后进入下载；进度可观察；重复安装被拒绝；失败不报告成功。

1. 扩展 composable 测试，先覆盖 confirming、取消确认、Started/Progress/Finished、未知总长度、下载失败和 launching 状态。
2. 运行测试确认红色结果。
3. 实现 `requestInstall()`、`cancelInstall()`、`confirmInstall()` 和进度归一化；下载开始后锁定其它动作。
4. Windows 安装调用返回前不写成功消息；若插件抛错则进入安全 error 状态。
5. 运行专项测试和 typecheck。

### 切片 5：更新界面与可访问性

**公开行为**：设置页能通过键盘检查、确认和安装；Release notes 为纯文本；进度与错误不只靠颜色表达。

1. 新建 `UpdatePanel.test.ts`，先覆盖各状态的可见文本、按钮启停、确认焦点、纯文本说明、确定/不确定进度和错误 `role`。
2. 为 `ConfirmDialog` 先增加保持默认危险样式兼容、同时支持中性确认变体的失败测试。
3. 实现 `UpdatePanel.vue` 和中性确认样式；保持 `<script setup lang="ts">`、原生按钮、`progress`/`aria-live` 与窄窗口稳定布局。
4. 扩展 `SettingsView.test.ts`，确认 UpdatePanel 位于设置 form 外且不会触发设置提交。
5. 在 `SettingsView.vue` 仅组合组件，不复制更新状态。
6. 运行组件/视图专项测试、style tests 和 typecheck。

**共享回归风险**：`ConfirmDialog` 已被 Provider、备份和引导流程使用；默认 props 和现有危险操作外观必须保持。

### 切片 6：GitHub Draft 发布工作流

**公开行为**：只有手动工作流且完整检查通过时才生成 Draft；发布资产包含 NSIS updater、`.sig` 和 `latest.json`，并优先 NSIS。

1. 为 `.github/workflows/release.yml` 增加可结构验证的失败测试或校验脚本，覆盖 `workflow_dispatch`、`contents: write`、完整检查、Windows runner、Draft、NSIS 优先和 Secrets 引用。
2. 运行校验确认工作流尚不存在而失败。
3. 添加工作流，使用经核验且固定版本的第三方 Actions；只在 Tauri 构建步骤注入更新私钥变量。
4. 更新 README，说明公开仓库、密钥、手动触发、Draft 审查、首个 updater 版本手动安装和不使用 Authenticode。
5. 本地运行结构测试和完整 `npm run check`；没有真实 GitHub run 时只报告“工作流配置通过本地检查”。

### 切片 7：构建与资产证据

**公开行为**：普通 Release/NSIS 构建仍成功且无需更新私钥；发布工作流能使用 Secret 生成可验证更新资产。

1. 在无 `TAURI_SIGNING_PRIVATE_KEY` 环境运行普通 `npm run build`，记录真实退出码和 NSIS 资产；失败不得归为成功。
2. 枚举 Release exe 与 NSIS 的实际路径、大小、时间和 SHA-256。
3. 在公开仓库配置好 Secrets 后运行一次 GitHub Actions，记录 run URL/状态和 Draft Release 的实际资产。
4. 验证 `latest.json` 的版本、`windows-x86_64` URL 和内联签名与实际 Release 一致，不打印私钥。
5. 未执行 GitHub run 时，本切片保持未完成，不以本地 mock 替代。

### 切片 8：隔离 Windows 真实升级

**公开行为**：已安装旧版从 GitHub 获取新版，经签名校验、UAC 和 NSIS 后沿用目录重启，测试数据不变。

1. 先编写 Sandbox/VM bootstrap 与验证脚本，并以临时目录 dry-run 测试路径保护、假密钥和报告脱敏。
2. 确认两个 Relay 覆盖成对设置且不指向真实默认目录；建立升级前递归快照和安装信息。
3. 手动安装首个 updater 版本，再发布更高 SemVer；在应用中执行检查与安装，必要时由用户确认 UAC。
4. 升级后检查进程、应用版本、安装目录、注册表、测试配置、应用数据、日志和备份快照。
5. 分别记录成功升级、断网、错误签名、下载失败和 UAC 取消的真实执行/未执行状态。
6. 关闭 Sandbox 或恢复 VM 快照；不得在主机真实 `.codex` 或 Codex Relay 数据目录运行。

### 切片 9：不可启动基线的恢复升级链路

**公开行为**：人工安装 `v0.1.1` 后应用能正常进入设置页；更高版本 `v0.1.2` 可由 `v0.1.1` 通过官方 updater 检查、签名校验和 NSIS 完成升级。

1. 先把发布结构测试改为要求包/Cargo 版本 `0.1.2`，以及说明中的 `v0.1.1 → v0.1.2`，运行专项测试确认因现有 `0.1.1` 元数据失败。
2. 最小更新 package、Cargo 与锁文件版本和发布说明，不改产品逻辑；运行专项测试、typecheck 和完整检查。
3. 从公开 Release 下载 `v0.1.1` NSIS 到安全 staging，核对 3,976,351 字节和 SHA-256 `17F775BF0DBD61E568F81FD06EB66DA1AFFB3CD56B522128859D43CFE92BD405` 后供 guest 人工安装；该步骤只记作恢复。
4. 构建、提交、推送并生成 `v0.1.2` Draft；核对资产与签名后经用户确认发布。
5. 在 Sandbox 的 `v0.1.1` 设置页执行应用内升级，随后运行 `Verify Codex Relay Update.cmd -ExpectedVersion 0.1.2` 对应入口并读取 `after.json`。

## 验证命令

专项命令按切片实际文件名调整，最低集合为：

```powershell
npx vitest run src/release-config.test.ts
npx vitest run src/services/tauri.test.ts
npx vitest run src/composables/useUpdater.test.ts
npx vitest run src/components/UpdatePanel.test.ts src/components/ConfirmDialog.test.ts src/views/SettingsView.test.ts
npm run typecheck
npm run test
npm run check:frontend
npm run check:rust
npm run check
npm run build
git diff --check
git status --short --ignored
git ls-files
```

发布和系统级证据不由上述命令替代：还需真实 GitHub Actions/Draft Release、资产枚举、签名内容核对，以及 Sandbox/VM 安装升级观察。

## 安全检查

- 所有自动化测试只使用临时路径或成对 Relay 覆盖；路径保护失败立即停止。
- fixture 只使用 `test-key-*-not-real`；不得把完整认证文件、Authorization Header 或任何真实私钥输出到报告。
- 扫描 Git 跟踪文件、差异、日志和工作流，确认没有 `TAURI_SIGNING_PRIVATE_KEY` 内容、API Key、`auth.json`、`providers.json` 或备份快照。
- endpoint 必须是固定 HTTPS GitHub Release URL；不接受运行时用户输入、HTTP 降级或无效证书。
- Release 发布前保持 Draft；资产、签名或版本不一致时删除/修复 Draft，不发布给客户端。

## 回滚点与限制

- 依赖、插件注册、权限和前端入口必须作为一个一致切片回退，避免保留可点击但不可用的更新 UI。
- 发布专用 config 与基础 config 分离；若 updater artifacts 破坏普通构建，先撤销发布覆盖引用而不改现有 NSIS 主流程。
- `ConfirmDialog` 新变体保持默认行为，发现共享回归时移除中性变体并为更新区块使用局部确认组件。
- GitHub Release 在人工确认前保持 Draft；错误 Draft 可以删除，不影响已安装客户端。
- 已发布坏版本不原地替换资产，发布更高版本修复；客户端无法启动时人工重装旧 Release。
- Tauri updater 当前 Windows 实现启动安装器后退出进程，UAC 取消可能导致应用关闭但未升级；MVP 接受该限制并如实说明。

## 文档与规范收尾

实施验证后，根据真实结果更新：

- `.trellis/spec/project/product-contract.md`：把显式手动更新加入产品能力，同时保留启动不联网。
- `.trellis/spec/project/architecture.md`：记录 updater 插件边界和手动网络数据流。
- `.trellis/spec/release/`：记录 updater artifacts、GitHub Draft 发布、密钥和首次迁移契约，并更新索引。
- `.trellis/spec/testing/verification.md`：仅在发现可长期复用的新证据规则时更新，避免复制一次性运行日志。

## 下一步

1. `v0.1.1` 人工恢复已完成且能进入主界面；该操作不计为 updater 成功。
2. `v0.1.2` 候选已提交推送，Actions Draft 与资产已核对；等待用户明确确认后公开发布。
3. 在 Sandbox 中从 `v0.1.1` 手动检查并应用 `v0.1.2`；升级重启后运行目标版本为 `0.1.2` 的核验入口，读取 `after.json` 并核对安装目录和三个 fixture 哈希。
4. 继续核对 UAC、安装目录沿用、重启、签名失败/断网/取消路径；未执行项如实保留，再完成全范围检查与任务收尾。
