# Codex Relay 发布控制台实施计划

## 1. 当前状态

- 状态：用户已于 2026-07-31 批准规划，任务已进入最终检查；切片 1–12 的主体实现、独立 EXE 构建和窗口观察已完成，正在重跑修复后的全量门禁并整理提交证据。
- 实施模式：Codex inline；主会话直接执行 TDD、实现、检查、规范更新和提交，不派发写入型子 Agent。
- 任务范围：独立便携 EXE、发布 workflow 契约、候选事务、Git/GitHub 编排、Draft 审计、Vue 可视化和实际构建。
- 明确不做：Sandbox、真实安装/升级、UAC、AuthentiCode、自动删除 Draft、无人值守公开。

本轮恢复检查点：最终 `npm run check` 首次重跑在根 Vitest 中发现发布控制台
`App.test.ts` 依赖 workspace cwd，错误读取主应用 `src/App.vue`。原失败入口已作为 RED 证据；
测试改为通过 Vite `?raw` 导入同目录 `App.vue`，根入口专项 2/2、控制台 `vue-tsc` 和前端
10 文件/15 项随后通过。下一步使用成对 Relay 临时路径覆盖重新运行完整 `npm run check`。

## 2. TDD 总体门禁

每个切片严格执行：新增一个公开行为测试 → 运行并确认因目标行为缺失而 RED → 最小实现 GREEN → 专项回归 → 必要重构。不得先搭完整内部实现再补测试。

权威公开行为、输入/结果和 mock 边界见 `prd.md` 的 B1–B8。关键原则：

- Git 提交/推送使用真实临时仓库与 bare remote，不 mock Git 语义。
- GitHub Run/Release/tag/asset 写操作全部 mock，不在自动化测试中触及真实仓库远端。
- 文件与状态全部使用 `tempfile` 或临时 Git dir；不得访问真实 Codex/Relay 数据目录。
- Vue 只 mock typed Tauri client/Channel，不 mock 私有组件实现。
- Windows 进程树、取消和输出上限通过现有 Job Object 测试边界验证。

## 3. 实施切片

### 切片 1：发布说明与 workflow 长期契约

- [x] RED：更新 `src/release-config.test.ts`，要求存在 `.github/release-notes.md`，workflow 具有 `expected_version` / `expected_sha` 输入、SHA/版本验证步骤，并从步骤 output 提供 `releaseBody`；删除针对 `0.4.0` 功能文案的固定断言。
- [x] 运行 `npx vitest run src/release-config.test.ts`，确认因新契约缺失失败。
- [x] GREEN：新增权威发布说明文件，修改 `.github/workflows/release.yml`，保留手动触发、固定 Action SHA、Draft、updater JSON 和两个 Secret 名称。
- [x] 增加工作流校验脚本或内联 PowerShell 的失败契约：Run SHA、版本、跨文件版本和说明文件不一致时在 `tauri-action` 前失败。
- [x] 重跑发布结构测试和 `src/release-retention.test.ts`。

证据：首次专项测试 14 项中 2 项按预期失败，分别缺少 workflow 输入和说明文件；实现后 `release-request`、`release-config`、`release-retention` 共 3 个文件/19 项通过。实际使用 PowerShell 7 对当前 `0.4.0`/HEAD 运行验证脚本退出 0，生成 1446 字节临时 GitHub output 后已删除。调试期间确认 workflow 使用 `pwsh`，Windows PowerShell 5.1 不适用于无 BOM UTF-8 中文脚本；脚本显式要求 PowerShell 7，并用 `ConvertFrom-Json -AsHashtable` 读取 package-lock 的空键。

### 切片 2：独立 npm/Tauri workspace 与构建隔离

- [x] RED：新增 `src/release-console-structure.test.ts`，断言独立 package/config/capability、唯一 identifier/binary、`bundle.active=false`、根 workspace/scripts，以及正式主 bundle 不引用控制台。
- [x] 运行专项 Vitest，确认脚手架缺失导致失败。
- [x] GREEN：创建 `tools/release-console` 的 Vue/Vite/Tauri 最小应用；加入 npm/Cargo workspace；新增 `dev:release-console`、`test:release-console`、`typecheck:release-console`、`build:release-console`。
- [x] 用最小窗口和安全 CSP 启动，不注册 updater、autostart、notification、shell 或 single-instance 插件。
- [x] 运行结构测试、console typecheck、最小 Rust test 和 `tauri build --no-bundle` 冒烟；此处只证明脚手架可构建。

证据：结构测试首次 3 项中 2 项因 workspace/目录缺失按预期失败，实现后 3/3 通过；console Vue 1/1、typecheck、Cargo package test 均退出 0。首次 Cargo test 暴露 Tauri Windows 资源仍要求图标，显式复用现有 `icon.ico` 后通过。`npm run build:release-console` 退出 0，Release 构建耗时 7m08s，生成 `src-tauri/target/release/CodexRelayReleaseConsole.exe`：8,550,912 字节，时间 `2026-07-31T19:33:41.0527623+08:00`，SHA-256 `5B6911496A62DC5421D270230942DEAF0D1A7137930091A793BCD46078B6B566`。该证据只证明独立便携骨架已构建，不证明发布流程、安装或签名。

### 切片 3：共享安全进程运行器与脱敏

- [x] RED：为通用 process runner 增加直接 argv、环境白名单、实时事件、退出码、输出上限、超时、取消和后代终止测试；为 GitHub/signing 环境赋值增加脱敏测试。
- [x] 确认新增测试因通用接口/脱敏缺失失败。
- [x] GREEN：从现有 `codex_process` 的 Windows Job Object 边界提取 `SafeProcessRunner`，保留 Codex 适配和现有行为；不得复制第二套 Job Object 实现。
- [x] console 只接受固定 executable kind 和结构化 args，禁止通用 shell。
- [x] 运行 process runner 专项测试；后代取消与输出上限各连续运行至少 3 次，再运行 `npm run test:rust:lib -- codex_process`。

证据：通用流式测试首次因 `SafeProcessRunner`/`ProcessInvocation`/event 接口缺失编译失败；实现后在进程完成前收到 stdout 并保留最终有界输出。调试确认通用调用方必须显式提供安全环境，空环境会让 Windows PowerShell 自身向 stderr 报启动错误；测试改为使用既有最小白名单。GitHub/signing 脱敏测试首次失败后变绿。控制台环境测试首次因模块缺失编译失败，首次 GREEN 运行在外层 180 秒预算到期但已生成测试 EXE，无残留 Cargo 进程；缓存后以 300 秒预算重跑退出 0。最终联合回归：process 6/6、safe_log 1/1、console environment 1/1；后代取消和输出上限各连续 3 次通过。`check:rust:deps` 退出 0，core 仍不依赖 Tauri 且无 `aws-lc-sys`。

### 切片 4：领域模型、SemVer 和发布说明生成

- [x] RED：为 `ReleasePhase` 合法转换、SemVer 严格递增、Conventional Commit 分类、内部提交过滤、模板必需段落、稳定输出、占位/秘密拒绝编写 Rust 测试。
- [x] GREEN：实现 typed models、`ReleaseNotesService` 和稳定错误码。
- [x] 验证相同输入字节一致；没有用户可见提交时阻止发布但允许 UI 编辑补充。
- [x] 运行 console Rust 专项测试。

证据：说明生成测试首次因 `ReleaseNotesService` 模块缺失失败；占位/秘密校验首次因 `validate`、`ManualContentRequired` 和 `SecretDetected` 缺失编译失败；必需段落与稳定错误码分别先因对应接口缺失失败。状态机测试首次因 `models` 模块缺失失败，camelCase DTO 测试在临时移除未测试的 serde 实现后因缺少 `Serialize`/`Deserialize` 按预期失败。最终 `cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console` 退出 0：环境 1 项、发布说明 4 项、发布阶段 3 项全部通过；`cargo fmt --manifest-path src-tauri/Cargo.toml --all` 退出 0。相同输入生成字节一致的中文说明；没有用户可见提交时保留可编辑占位并禁止继续，编辑后仍强制保留版本、未知发布者和数据保留提示，疑似凭据命中复用 core 脱敏规则并停止。

### 切片 5：版本文件计划与候选事务

- [x] RED：临时 fixture 覆盖六个计划文件，测试版本生成、未知 JSON/TOML 保留、`Cargo.lock` 两个本地包定位、新说明写入、指纹冲突、写入失败、写后验证失败和精确回滚。
- [x] GREEN：实现 `ReleaseCandidateTransaction`，复用原子写/指纹基础设施，使用 `toml_edit` 局部修改 Cargo 文件。
- [x] 增加 crash marker 和恢复测试；状态损坏或外部修改时不覆盖源码。
- [x] 运行候选事务专项测试和格式检查。

证据：六文件计划测试首次因 `release_candidate` 模块缺失失败；后续指纹冲突、候选写入、前向写失败、写后篡改、回滚失败、崩溃恢复、恢复前源码漂移、备份损坏、写前二次指纹检查和重复事务分别先以缺少接口或错误结果进入 RED。实现使用 core `FileFingerprint` 与 `atomic_write`，JSON 只更新根版本并保留未知值，两个 Cargo manifest 和 `Cargo.lock` 使用 `toml_edit` 局部修改，恢复 marker/备份只写所选 Git dir。最终 `release_candidate` 11 项通过；控制台 Rust 全包共环境 1 项、候选事务 11 项、发布说明 4 项、阶段 3 项通过。`cargo fmt --manifest-path src-tauri/Cargo.toml --all` 退出 0。首次控制台 Clippy 因切片 3 的 `SystemCodexProcessBackend` 手写 `Default` 可派生而失败；核对仅有一个已实现 `Default` 的 `SafeProcessRunner` 字段后改为 `#[derive(Default)]`，相同 Clippy 命令重跑退出 0。

### 切片 6：仓库预检与真实临时 Git 流程

- [x] RED：真实临时 repo + bare remote 覆盖正确预检、remote 错误、脏工作区、HEAD 与 origin/main 漂移、计划外文件、精确暂存、提交、push、非快进和远端 SHA 验证。
- [x] GREEN：实现 `GitBackend`、`RepositoryInspectionService` 和 `GitReleaseService`；本地分支名可不同，但 remote/default branch/目标 repo 固定。
- [x] 工具探测和 GitHub 状态使用 mock process backend；Git 文件语义使用真实 git.exe。
- [x] 运行 Git 专项集成测试，确认不引用真实 origin 的写操作。

证据：Git 模块首次因 `infrastructure::git` / `services::git_release` 缺失失败；随后 remote 身份、脏工作区、HEAD 漂移、GitHub URL 规范化、精确提交推送、计划文件内容漂移、计划外文件、真实 pre-push non-fast-forward 竞态、稳定错误码和外部预检阻断分别先进入 RED。测试全部使用系统临时 repo 与本地 bare remote；产品 Git 调用通过 `SafeProcessRunner` 的结构化 argv、环境白名单、超时和 Job Object，未引用当前仓库 `origin`。最终 `git_release` 12 项通过，覆盖本地分支 `master` → 远端 `main`、固定目标 GitHub 仓库、精确六文件提交、远端 SHA 复核和 mock 工具链/活动 Run/冲突 Draft；`cargo fmt` 与控制台包 Clippy 均退出 0。

### 切片 7：状态存储、会话锁与本地验证编排

- [x] RED：测试版本化 session 原子保存、损坏状态、单仓库互斥、阶段持久化、推送前失败回滚、推送后禁止伪回滚、取消和重启恢复。
- [x] GREEN：实现 `ReleaseStateStore`、repo lock、`LocalVerificationService` 和 orchestrator 到 `pushed` 阶段。
- [x] 本地验证固定运行发布专项、`npm run check` 和清除签名环境后的普通 `npm run build`；子进程沿用有界输出、脱敏环境、超时与 Job Object，事件日志接入留在 typed IPC 切片。
- [x] 枚举普通主 EXE/NSIS 路径、大小、时间和 SHA-256；不把它描述为 updater 签名或安装证据。

证据：状态模型/存储首次因 `ReleaseSession` 与 `ReleaseStateStore` 缺失失败；语义损坏、文件锁、阶段原子推进、固定命令顺序、非零退出、产物枚举、编排回滚、推送后 marker 清理、取消分类和重启后取消分别先进入 RED。集成测试发现候选事务清理曾错误地把共享状态目录当成独占目录，导致 `session.json`/`session.lock` 存在时把已恢复源码误报为回滚不完整；修复为只删除候选 marker/backup，目录非空时保留。最终控制台 Rust 全包 43 项通过：Git 12、local verification 4、环境 1、候选事务 11、说明 4、orchestrator 4、阶段 3、状态 4。普通产物 fixture 实际枚举相对路径、长度、mtime 和 SHA-256。首次 Clippy 因锁文件 `OpenOptions` 未显式声明 truncate 语义失败；确认锁文件只作 OS 互斥句柄后增加 `.truncate(false)`，相同 Clippy 重跑退出 0，锁专项重跑通过。

### 切片 8：GitHub Run、Draft 下载与审计

- [x] RED：使用固定 `gh` JSON/二进制 fixture 测试 dispatch stdin、Run URL/唯一 Run 发现、Job/Step 计时、Run 失败、Draft 身份、三项资产、asset API 下载、size/hash、manifest 平台/URL/notes 和签名关联。
- [x] GREEN：实现 `GhBackend`、`GithubReleaseService`、`ArtifactWorkspace` 和 `DraftAuditService`。
- [x] `gh` 调用不继承 Token/signing/Codex secrets；下载 stdout 直接写临时文件，不进入内存日志。
- [x] 运行 GitHub/Draft 专项测试和泄漏扫描。

证据：`SystemGhBackend` 环境纵深过滤测试先因原始 `GH_TOKEN` 被继承而失败，修复后只保留安全白名单；真实本机 `gh workflow run --help` 证明 `--json` 是结构化 stdin，Run URL 仅“可用时”返回，因此实现 URL 解析以及按 workflow、main、候选 SHA、触发时间轮询唯一 Run 的回退。Run 失败、远端 digest 漂移和 Run 暂不可见分别先进入 RED。Draft 审计固定下载三项资产到系统临时目录，stdout 直接写新文件，安装器使用 64 KiB 流式 SHA-256，不整体进入内存，并对照 GitHub `sha256:` digest、size、tag SHA、说明、manifest 平台/URL和独立签名。负向矩阵覆盖资产缺失/多余、Release/manifest 说明漂移、size/digest、URL、签名与 tag SHA。最终 `github_release` 10 项通过；控制台 Rust 全包 53 项通过，测试执行约 37.6 秒、总墙钟 4m36.7s（主要为 Windows 编译/链接）；控制台 Clippy 退出 0；core 通用 runner 的结构化 stdin、实时输出和大文件直写 3 项通过。

### 切片 9：公开、在线复核、清理与恢复

- [x] RED：测试公开前身份漂移、按 Release ID PATCH、Latest/tag/manifest 复核、cleanup 成功/失败分离，以及从 pushed/run/draft/已公开状态恢复。
- [x] GREEN：完成 orchestrator 远端阶段、`publish_release` 和 resume 逻辑。
- [x] 错误 Draft 不自动删除；清理失败返回 `completedWithWarnings` 而非覆盖公开成功事实。
- [x] 运行状态机与 GitHub service 全部专项测试。

证据：公开前 Draft Release ID 漂移测试先因缺少 `publish_release`/身份错误进入 RED；固定 `gh api --method PATCH repos/.../releases/<id> --input -` 只接受结构化 `{"draft":false}`。公开后通过 `releases/latest`、同一 tag SHA、三项资产 size/digest/SHA-256、manifest notes/URL 和签名重新下载复核；cleanup Run 独立发现和轮询，`success` 与 `failure` 均返回真实证据，后者不覆盖 Release 已公开事实。会话新增 Run、Draft、Published、cleanup 检查点和阶段不变量；orchestrator 可从 pushed、workflowQueued/running、auditingDraft、awaitingPublishApproval、publishing、verifyingPublishedRelease 与 monitoringCleanup 继续。专项覆盖 Run 恢复不重复 dispatch、PATCH 后崩溃时识别同一 Release 已公开而不重复 PATCH、已公开恢复不重复 publish、cleanup 失败进入 `completedWithWarnings`。最终 GitHub 生命周期 15 项、orchestrator 8 项、状态 5 项通过；控制台 Clippy 退出 0。

### 切片 10：typed IPC 与前端发布会话

- [x] RED：在 console Vitest 中 mock typed client/Channel，测试预检、计划、开始、事件序列、旧事件丢弃、取消、恢复、公开和错误映射。
- [x] GREEN：实现 Rust commands/AppState、TypeScript DTO、唯一 `services/tauri.ts` 和 `useReleaseSession`；对外只读状态 + 显式动作。
- [x] 运行 console frontend 专项、typecheck 和 Rust command 测试。

当前检查点（2026-07-31）：Rust `AppState`/typed commands、TypeScript DTO、唯一 Tauri service、typed Channel 与 `useReleaseSession` 已实现。为避免后台长流程只在结束时刷新 UI，新增原子 `session.json` watcher；首次专项因 `watch_session_state` 缺失按预期编译失败，实现后应用层 4 项专项通过。watcher 只在 session 内容变化时发送 `SessionUpdated`，并在初始、恢复、公开三条后台流程结束时停止并等待退出。

Git 检查点补强：首次 RED 证明原 `ReleasePushBackend` 无法表达“commit 成功、push 失败”；现已拆分 `commit`/`push`，orchestrator 在 push 前持久化 `Committed` 与候选 SHA。push 失败保持可恢复检查点，不再被应用层覆盖为 `Failed`；新增 `push_committed` 只重试推送，成功后持久化 `Pushed` 并清理候选事务标记。对应专项分别通过，下一步运行 Git/orchestrator 全套与前后端完整回归。

最终补强：根 Vitest 首次重跑发现 `App.test.ts` 依赖 workspace cwd、误读主应用 `src/App.vue`；改用 Vite `?raw` 后根入口与独立 workspace 均通过。活动 session 初始化与后台管线注册现在分别原子拒绝重复启动；`idle/inspected/planned` 崩溃检查点可安全取消。最终控制台前端 typecheck 通过，完整门禁覆盖 10 个控制台测试文件；Rust command 应用层、状态和 watcher 均纳入控制台 80 项 Rust 测试。

### 切片 11：可视化界面与可访问性

- [x] RED：逐组件测试仓库预检、版本/说明编辑、步骤与耗时、日志详情、Draft 审计、公开确认、完成/警告分离、按钮禁用原因、Escape、焦点恢复和窄布局类。
- [x] GREEN：按 `design.md` 组件图实现单窗口 UI；Element Plus API 以当前安装类型/官方文档为准，类名只负责布局，所有不可逆动作使用明确对话框。
- [x] 确保 `App.vue` 为组合面，组件 props/emits 类型化，说明正文使用 textarea/文本插值，不使用不可信 `v-html`。
- [x] 运行 console Vitest、typecheck，并人工观察 1120×760、900×620、窄窗口、浅色/暗色和键盘流程。

证据：1120×760 与 900×620 为双列，760×560 在 820px 断点前切换单列；Tab 顺序为仓库路径、目标版本、重新生成计划、发布说明、脱敏日志。浅色根背景为 `rgb(238, 242, 247)`；DevTools 模拟 dark 后媒体查询为 true、根背景为 `rgb(16, 23, 34)`、文字为 `rgb(236, 242, 250)`，并实际观察暗色画面。旧暗色截图因模拟在截图前恢复 light 而无效，已保留这一真实诊断而未计作暗色证据。

### 切片 12：集成、文档、规范和实际产物

- [x] 更新根 README 的开发者发布入口和 `.trellis/spec/release/{index,publishing,updater}.md`，准确区分控制台自动证据与未执行 Sandbox/安装证据。
- [x] 核对 `src/views/AboutView.vue`：发布控制台不进入产品能力，因此不修改；对应 About 测试在完整前端套件中通过。
- [x] 增加发布控制台构建脚本，把便携 EXE 复制到忽略目录并枚举 SHA-256；同时忽略控制台 `src-tauri/gen/` 生成 schema，并以结构测试固化。
- [x] 运行结构测试、console 全套、Rust workspace、根前端、Trellis 与完整 `npm run check`。
- [x] 显式移除两个 updater 签名环境变量后运行正式 `npm run build`，确认主应用普通构建仍独立。
- [x] 运行 `npm run build:release-console`，枚举控制台 EXE 实际路径、大小、时间和 SHA-256，并启动观察/烟测。
- [x] 执行 Git/秘密/路径安全审计、`git diff --check` 和精确暂存审查。

最终检查证据：

- 首次最终 `npm run check` 在根 Vitest 因控制台测试 cwd 假设失败；修复后又在审查中新增长 Run、commit 前回滚、Git 取消、重复 session/管线和启动崩溃恢复门禁。
- 最终成对 Relay 临时路径覆盖的 `npm run check` 退出 0，用时 339.9 秒，临时目录已删除：Trellis 8 项；前端 52 文件与控制台 10 文件均通过；Rust workspace 包含主应用 47、core 247、路径/工作流 3+1，以及控制台 unit 9、commands 1、Git 14、GitHub 15、本地验证 4、环境 1、候选事务 11、说明 4、orchestrator 11、阶段 4、状态 6，全部 0 失败。
- 发布 Run 发现预算由约 10 秒提高到 2 分钟，Run/cleanup 监控由约 30 分钟提高到 4 小时、每 5 秒轮询；专项 RED/GREEN 与 GitHub/orchestrator 全套通过，可覆盖本次已观测的 1 小时 09 分冷构建。
- 普通主构建产物：`CodexRelay.exe` 19,434,496 字节，SHA-256 `4F5C147356AA55938848A64CC44335EF20D512E7E7DBB133E873F96414A7D2F1`；NSIS 4,692,299 字节，SHA-256 `82A60F488DD8EFA153B9E86F24E32380507067096D5071877FB121BD999769D8`；普通构建目录中 `.sig` / `latest.json` 数量为 0。
- 最终 `npm run build:release-console` 退出 0，用时 144.5 秒；`dist/release-console/CodexRelayReleaseConsole.exe` 为 12,405,760 字节，时间 `2026-08-01T03:49:25.6284313+08:00`，SHA-256 `83D9854712E0B8E7EB57F913621B135AC8138BE906C2BE2C42AC72B4464F2410`。使用成对临时路径隐藏启动 4 秒，进程保持运行，随后已关闭并删除临时目录。
- 只读公开契约核对：当前 Latest `v0.4.0` 的 `target_commitish` 与 tag 均为提交 SHA；资产恰为 NSIS、`.sig`、`latest.json` 三项且 digest 均存在；manifest 平台为 `windows-x86_64` 与 `windows-x86_64-nsis`，共同指向 REST asset API，内联签名一致。
- 未执行：真实 GitHub workflow dispatch、Draft 创建/下载审计、Release 公开、cleanup 写操作、Sandbox、真实安装、UAC、应用内升级、重启、卸载或数据保留验证。

## 4. 验证命令

专项命令会随切片补充，最终至少运行：

```powershell
npx vitest run src/release-config.test.ts src/release-retention.test.ts src/release-console-structure.test.ts
npm run test:release-console
npm run typecheck:release-console
npm run check:frontend
npm run check:rust
npm run check
```

构建命令：

```powershell
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
npm run build
npm run build:release-console
```

提交前：

```powershell
git diff --check
git status --short --ignored
git diff --name-only
git diff --cached --name-only
```

所有命令必须记录退出码、测试数量、实际失败与未执行项。自动化测试不得触发真实 GitHub workflow、创建/公开 Release、删除 tag 或写真实 Codex/Relay 数据。

## 5. 高风险文件与回滚点

- `.github/workflows/release.yml`：可能创建错误 Draft；结构测试必须证明校验在 `tauri-action` 前。
- `src/release-config.test.ts`：长期发布契约；不能因移除版本硬编码而降低 Draft/Secrets/updater 断言。
- `src-tauri/crates/codex-relay-core/src/infrastructure/codex_process.rs` 及新通用 runner：取消/进程树安全；必须保持现有专项和冷 runner 契约。
- `package.json` / `package-lock.json` / `src-tauri/Cargo.toml`：workspace 与依赖图；不得让 core 引入 Tauri。
- `ReleaseCandidateTransaction`：源码回滚风险；没有原字节验证不得宣称恢复。
- `GithubReleaseService`：远端不可逆风险；测试只 mock，正式公开只由用户在控制台明确确认。

回滚形态：独立 console workspace、根 scripts/workspace 配置和 workflow 说明文件契约可按提交整体回退；不涉及正式 Codex Relay 用户数据迁移。已经公开的 Release 不能靠代码回滚，必须发布更高 SemVer 修复。

## 6. 完成门禁

- [x] `prd.md`、`design.md`、`implement.md` 与实际实现一致且没有未解决占位。
- [x] 全部 AC 有本轮测试、构建、只读公开契约或明确人工观察证据；真实远端写操作与安装类场景明确标为未执行。
- [x] 没有真实 GitHub Token、updater 私钥、API Key、认证文件或用户数据进入 Git/日志/任务材料；8 处高置信命中均为显式测试假值或脱敏断言。
- [x] 正式 Codex Relay 普通 build 与独立控制台 build 都有实际证据，且没有误报签名/安装/升级。
- [x] 运行 `trellis-check` 并完成规范更新判断。
- [ ] 精确提交本任务改动并记录提交哈希。
