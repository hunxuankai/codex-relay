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

Phase 1 规划已完成并获得用户明确实施授权，任务已进入 `in_progress`。切片 1 至切片 6 的本地实现已完成并推送到公开仓库 `main`：updater 依赖/插件/权限/版本来源/发布覆盖、固定 endpoint/公开公钥、typed 服务边界、手动检查与安装状态机、设置页 UI、Draft 发布工作流和 README 已落地。旧占位 Draft 已删除，GitHub Actions 第二次真实运行成功，并从最新提交生成含正式说明、NSIS、`.sig` 与 `latest.json` 的新 Draft；新清单内容核对、公开发布和 Windows 升级尚未完成。

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

## 关键决策

- 使用官方 `tauri-plugin-updater`，不自写下载器或安装器。
- 保留 Tauri 更新包签名校验；不使用 Windows Authenticode。
- 仅用户手动检查，一条 `windows-x86_64` 稳定通道，NSIS 被动安装，人工恢复而非自动回滚。
- `createUpdaterArtifacts` 只放在发布配置覆盖中，普通本地构建不依赖私钥。
- 发布工作流 `workflow_dispatch` → 完整检查 → Windows 构建 → Draft Release → 人工发布。

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
- 未执行 Tauri updater 签名、GitHub Actions、Draft Release、安装、升级、UAC 或 Sandbox/VM 验收；上述构建证据不得用于声明这些项目成功。

## 尚未解决的问题

- Windows Sandbox 功能是否启用尚未用管理员权限确认；必要时需准备快照虚拟机。
- GitHub Secrets 的存在由用户确认；本机未安装 GitHub CLI，尚未通过 CLI 独立枚举，但不会读取或输出 Secret 值。
- 新 Draft 的 Release 描述已核对为正式文案，但新 `latest.json` 文件内容尚未由用户提供，仍需确认 `notes`、版本、平台 URL 和内联签名后才能发布。

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

1. 用户从新 Draft 下载 `latest.json` 并提供完整公开 JSON 内容。
2. 核对新清单的正式说明、版本、平台 URL 和内联签名与实际资产一致。
3. 在用户确认发布动作后公开首个 Release，并手动安装首个带 updater 的版本。
4. 发布更高 SemVer 版本，在 Sandbox/VM 中完成真实应用内升级与失败场景验收。
