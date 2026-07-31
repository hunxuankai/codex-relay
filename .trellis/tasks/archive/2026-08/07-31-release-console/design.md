# Codex Relay 发布控制台技术设计

## 1. 设计结论

实现一个位于当前仓库内、但与正式 Codex Relay 完全隔离的第二个 Tauri 2 应用。它生成便携 Windows EXE，使用 Vue 3 + TypeScript + Element Plus 展示单窗口发布流程；Rust 后端拥有所有 Git、文件、进程和 GitHub 操作，前端只能调用固定 typed commands。

控制台不是新的发布后端。GitHub Actions 与 `tauri-action` 继续是唯一 updater 签名和 Draft 资产生成边界；控制台负责候选准备、状态编排、自动审计和最终明确确认。

## 2. 方案比较

### 方案 A：独立 Tauri 2 应用（采用）

- 优点：复用仓库现有 Vue/Tauri/Rust/Element Plus/测试体系；可生成体积较小的 EXE；Rust 能严格控制文件、进程和 GitHub CLI 边界；UI 与发布逻辑可分层测试。
- 代价：增加第二套 Tauri 配置和前端入口；完整 Rust workspace 检查会覆盖新应用。

### 方案 B：把发布页隐藏在正式 Codex Relay 中（拒绝）

- 优点：最少脚手架。
- 缺点：会把 Git、GitHub、提交和公开 Release 权限带入普通用户产品，破坏安装包、权限和安全边界，也产生“正在发布自身”的生命周期耦合。

### 方案 C：PowerShell/WebView 或 .NET/WPF 独立工具（拒绝）

- 优点：PowerShell 原型较快；WPF 是原生 Windows UI。
- 缺点：PowerShell 难以提供可靠的事务、恢复、类型化 IPC 和进程树管理；WPF 引入仓库没有的技术栈与测试工具。两者都会形成新的维护体系。

## 3. 仓库布局与构建隔离

```text
tools/release-console/
  package.json
  index.html
  vite.config.ts
  tsconfig*.json
  src/
    App.vue
    style.css
    types/release.ts
    services/tauri.ts
    composables/useReleaseSession.ts
    components/release/
      RepositorySetupPanel.vue
      ReleasePlanPanel.vue
      ReleaseTimeline.vue
      ReleaseStepDetails.vue
      DraftAuditPanel.vue
      ReleaseResultPanel.vue
      PublishConfirmDialog.vue
  src-tauri/
    Cargo.toml
    build.rs
    tauri.conf.json
    capabilities/default.json
    src/
      main.rs
      lib.rs
      app_state.rs
      commands.rs
      models.rs
      services/
      infrastructure/
```

- 根 `package.json` 增加 npm workspace `tools/release-console` 和独立 dev/test/typecheck/build 脚本。
- `tools/release-console/src-tauri` 通过 Cargo `package.workspace` 加入现有 `src-tauri/Cargo.toml` workspace，共享 `src-tauri/target` 和依赖缓存。
- 控制台 Tauri 配置使用独立 `productName`、`mainBinaryName=CodexRelayReleaseConsole`、`identifier=com.codexrelay.release-console`、窗口标签和 capability。
- 控制台配置 `bundle.active=false`，正式构建使用 `tauri build --no-bundle`，输出便携 EXE；不生成控制台安装器。
- 正式 Codex Relay 的 `tauri.conf.json`、`mainBinaryName=CodexRelay` 和 NSIS bundle 不引用控制台 binary、资源或 capability。
- 实际交付脚本把 EXE 复制到被 Git 忽略的 `dist/release-console/`；二进制不提交 Git。

## 4. 前端架构与组件边界

全部 Vue 文件使用 Composition API、`<script setup lang="ts">`、typed props/emits。只有控制台自己的 `src/services/tauri.ts` 导入 Tauri IPC；不引入 Pinia。

### `App.vue`

只负责应用壳、Element Plus 配置、发布会话 composable 注入和下列组件编排，不承载发布业务逻辑。

### `useReleaseSession`

发布 UI 的唯一状态源。持有只读 `inspection`、`plan`、`session`、`steps`、`selectedStep`、`busy`、`error` 和明确动作：`inspect`、`preparePlan`、`start`、`resume`、`cancel`、`publish`、`exportSummary`。通过请求序列和 session ID 丢弃过期响应/事件。

### 展示组件

- `RepositorySetupPanel`：仓库路径、预检结果、版本选择；只通过 emits 请求检查或计划。
- `ReleasePlanPanel`：计划文件、提交范围和完整发布说明编辑器；使用 typed `v-model`，显示校验错误。
- `ReleaseTimeline`：显示阶段、状态、耗时和当前步骤；不触发后端。
- `ReleaseStepDetails`：显示当前步骤的脱敏日志、退出码、证据和失败建议。
- `DraftAuditPanel`：显示 Release/资产/manifest 审计矩阵；只有全部通过才发出公开请求。
- `PublishConfirmDialog`：明确显示版本、候选 SHA、Release ID 和不可逆后果；默认聚焦取消。
- `ReleaseResultPanel`：分别显示公开结果、Latest 复核、清理状态和未执行项，并允许导出摘要。

布局采用左侧纵向步骤、右侧上下文详情；在窄窗口改为单列。所有状态同时使用文字/图标，不仅依赖颜色；日志区域可键盘滚动，错误与禁用原因可见。

## 5. Tauri IPC 契约

公开 command 保持少而稳定：

```text
inspect_release_repository(repositoryPath)
prepare_release_plan(repositoryPath, targetVersion)
start_release(confirmedPlan, onEvent: Channel<ReleaseEvent>)
get_release_session(repositoryPath)
resume_release(sessionId, onEvent: Channel<ReleaseEvent>)
cancel_release(sessionId)
publish_release(sessionId, expectedDraftIdentity, onEvent: Channel<ReleaseEvent>)
export_release_summary(sessionId, destinationPath)
```

- command 只解析参数、调用一次应用服务并映射 `CommandResult<T>`。
- 长流程由后台 orchestrator 执行；`start`/`resume` 返回 session snapshot，并通过 Tauri typed `Channel` 推送结构化事件。
- 事件是带 tag 的 camelCase DTO：`sessionUpdated`、`stepStarted`、`stepLog`、`stepCompleted`、`stepFailed`、`draftReady`、`releasePublished`。
- 前端不能传入可执行文件、任意 argv、API endpoint、仓库 owner/name、workflow 文件名或 Release PATCH body。

## 6. Rust 分层

### Domain/model

- `ReleaseSession`、`ReleasePhase`、`ReleaseStep`、`StepStatus`、`ReleasePlan`、`DraftIdentity`、`ArtifactEvidence`、`ReleaseEvent`。
- 稳定错误码按 `RELEASE_*`、`GIT_*`、`GITHUB_*`、`PROCESS_*`、`STATE_*` 分类，公开消息为简体中文。

### Services

- `RepositoryInspectionService`：组合 Git、GitHub 和工具探测，只读生成预检。
- `ReleaseNotesService`：提交分类、模板生成、说明校验与秘密模式扫描。
- `ReleaseCandidateTransaction`：计划文件锁、指纹、精确备份、内存生成、原子替换、解析、写后验证和精确回滚。
- `LocalVerificationService`：专项检查、完整检查、普通构建、产物与差异证据。
- `GitReleaseService`：精确暂存、提交、推送、远端 SHA 复核。
- `GithubReleaseService`：触发/轮询 Run、获取 Draft、下载资产、公开和查询清理 Run。
- `DraftAuditService`：Release、资产、`latest.json`、签名关联和 SHA-256 审计。
- `ReleaseOrchestrator`：唯一状态机，调用上述服务并持久化每个安全检查点。

### Infrastructure

- `SafeProcessRunner`：从现有 Windows Job Object 进程边界提取为通用实现；固定 executable/argv、环境白名单、实时输出、总量上限、超时、取消和进程树终止。现有 Codex 进程适配继续通过同一底层，避免第二套进程终止实现。
- `GitBackend`：只调用固定 `git.exe` 子命令。
- `GhBackend`：只调用固定 `gh.exe` 子命令，JSON 请求走 stdin；不读取认证文件或 Token 环境变量。
- `ReleaseStateStore`：在 worktree 的绝对 Git dir 下创建 `codex-relay-release-console/`，原子保存版本化 session 与有界脱敏日志。
- `ArtifactWorkspace`：使用 `tempfile` 创建下载目录，验证路径位于系统临时根，完成后显式清理。
- 复用 `codex-relay-core` 的原子写、SHA-256 指纹和日志脱敏基础设施；扩展脱敏覆盖 `GH_TOKEN`、`GITHUB_TOKEN`、`TAURI_SIGNING_PRIVATE_KEY` 及密码赋值。

## 7. 发布状态机

```text
idle
  -> inspected
  -> planned
  -> applyingCandidate
  -> localChecks
  -> localBuild
  -> sourceAudit
  -> committed
  -> pushed
  -> workflowQueued
  -> workflowRunning
  -> auditingDraft
  -> awaitingPublishApproval
  -> publishing
  -> verifyingPublishedRelease
  -> monitoringCleanup
  -> completed | completedWithWarnings
```

任一步骤都可进入 `failed` 或 `cancelled`，但恢复策略取决于远端边界：

- `committed` 前：失败/取消先精确清理 Git 暂存区，再触发候选事务回滚并验证原字节；若任一步不完整，保留恢复标记并返回 `RELEASE_ROLLBACK_INCOMPLETE`。
- `committed` 后、`pushed` 前：保留精确本地提交并从推送重试；首版不自动撤销或重写该提交，放弃候选需由维护者明确人工处置。
- `pushed` 后：不得重写远端或伪造回滚；恢复从 workflow dispatch、Run 或 Draft 身份继续。
- Draft 审计失败：保留 Draft，不自动删除；修复必须产生新候选或由用户在 GitHub 明确处理。
- Release 已公开而清理失败：状态为 `completedWithWarnings`，展示 Release 已公开和清理失败两个独立事实。

每次恢复都通过外部事实重建：Git HEAD、远端 main SHA、workflow run head SHA、Draft ID/tag/target、Latest 和 cleanup run；状态文件只提供候选 ID，不能作为完成证据。

## 8. 候选文件事务

权威计划文件固定为：

```text
package.json
package-lock.json
src-tauri/Cargo.toml
src-tauri/crates/codex-relay-core/Cargo.toml
src-tauri/Cargo.lock
.github/release-notes.md
```

- JSON 使用保留未知键的 `serde_json::Value`，只更新根包版本与 lock 根包版本，使用两空格和末尾换行。
- 两个 Cargo manifest 和 `Cargo.lock` 使用 `toml_edit` 定位明确 package 名并局部更新版本；未知内容和注释保留。
- `.github/release-notes.md` 保存维护者最终确认的完整正文，成为源码审计和 workflow 的唯一发布说明来源。
- 事务开始时记录全部原字节、存在状态和 SHA-256；所有新字节在写入前完成解析和跨文件版本验证。
- 每个目标使用同目录临时文件、flush、替换、重读和语义验证；任何失败恢复所有目标并比较原始字节。
- 事务期间再次检查工作树只有计划文件变化；外部编辑返回 `RELEASE_SOURCE_CONFLICT`。

## 9. 发布说明生成

- 基线由公开 Latest tag 确定，提交范围为 `<latest-tag>..HEAD`。
- 识别 Conventional Commit 的 `feat`、`fix`、`perf`、`refactor`、`revert`；去除 type/scope 前缀并保持提交主题原语言。
- `chore`、`test`、`ci`、纯任务归档和发布证据提交默认不进入“更新内容”；若没有用户可见提交，保留可编辑占位警告但禁止开始发布，要求维护者填写真实内容。
- 草稿结构固定为“更新内容 / 更新方式 / 注意事项”。上一版本、目标版本、签名校验、未知发布者和数据保留文本由模板生成。
- 用户可以编辑全文，但开始前校验标题、版本、非占位内容和必需安全说明；高置信秘密模式命中立即阻止。

## 10. Git 与远端竞态

- 预检运行 fetch，要求 HEAD 与 `origin/main` 相等、工作区/索引为空、remote URL 规范化后精确为目标仓库。
- 本地分支可以名为 `master`，但必须明确推送 `HEAD:refs/heads/main`。
- 暂存只使用固定文件列表；提交前比较 `git diff --cached --name-only` 与计划集合完全相等。
- 推送前再次读取 `origin/main`；使用普通非强制 push。推送后用 `ls-remote` 验证远端 main 等于候选 SHA。
- 若另一提交抢先进入 main，push 非快进或 workflow SHA 校验失败，控制台安全停止，不自动 rebase、merge 或 force push。

## 11. GitHub Actions 契约迁移

- 新增 `.github/release-notes.md`，移除 `release.yml` 中每个版本硬编码的正文。
- `workflow_dispatch` 增加必填 `expected_version` 和 `expected_sha`。
- checkout 后新增“验证发布请求”步骤：
  - `${{ github.sha }}` 必须等于 `expected_sha`；
  - `package.json.version` 必须等于 `expected_version`；
  - npm/Cargo/lock 版本一致；
  - 发布说明文件存在、非空、无占位并包含目标版本；
  - 使用安全 delimiter 把正文写入 step output。
- `tauri-action.releaseBody` 改为验证步骤的 output；Draft、两个 Secrets、updater JSON 和固定 Action SHA 保持不变。
- 结构测试改为长期动态契约，不再为每个版本硬编码用户功能文案。
- 控制台使用 `gh workflow run release.yml --ref main --json`，stdin 只包含期望版本和 SHA。Run URL 若 CLI 返回则直接记录，否则按 workflow/head SHA/创建时间查询唯一 Run。

## 12. Draft 与公开审计

目标资产必须且只能是：

```text
Codex.Relay_<version>_x64-setup.exe
Codex.Relay_<version>_x64-setup.exe.sig
latest.json
```

审计顺序：

1. 精确 tag `v<version>` 只对应一个 Draft Release。
2. Release `target_commitish` 和 tag ref 都解析为候选 SHA。
3. Draft/Prerelease、标题、正文和资产集合符合计划。
4. 使用 `gh api` 的 `Accept: application/octet-stream` 语义把三个 Draft 资产写入临时文件。
5. 记录资产 API ID、名称、长度和 SHA-256，并比较 API size 与实际大小。
6. 解析 `latest.json`，验证 version/notes、`windows-x86_64` 与 `windows-x86_64-nsis`、URL 都指向本次安装器 asset API ID。
7. 两个平台内联签名相同，并与独立 `.sig` 文件去除末尾换行后的内容一致。

公开按钮点击后完整重做步骤 1–3 和 manifest/签名身份检查，再按 Release ID PATCH `draft=false`。公开后通过公共 Latest/tag/manifest 再验证一次，不复用 Draft 下载结果作为公开证据。

## 13. 进程、环境与日志安全

- 允许的可执行文件只有解析后的 `git.exe`、`npm.cmd`、`cargo.exe` 和 `gh.exe`；可执行路径在预检后固定到会话。
- 不调用 `cmd /c`、`powershell -Command` 或拼接命令字符串。
- 子进程环境先清空，再加入 Windows/工具链运行所需白名单；明确排除 `OPENAI_API_KEY`、`CODEX_HOME`、Relay 路径覆盖、`GH_TOKEN`、`GITHUB_TOKEN`、`TAURI_SIGNING_PRIVATE_KEY` 和密码。
- `gh` 只使用其现有安全凭据存储；如果仅通过 Token 环境变量认证，预检失败并提示运行 `gh auth login`。
- 命令日志只显示规范化 executable 名、脱敏参数、输出片段、退出码和耗时；每步日志有大小上限，超限截断并明确标记。
- 发布说明和 Git 提交主题先扫描再显示；日志红线命中在持久化前脱敏。

## 14. 测试与验证策略

### Rust

- 纯单元测试：SemVer、提交分类、说明模板、状态转换、错误映射、GitHub JSON、manifest 和签名关联。
- 临时文件测试：候选事务、原子状态、损坏恢复、指纹冲突、未知 JSON/TOML 保留、回滚失败。
- 临时 Git 集成：真实仓库与 bare remote，覆盖精确暂存、提交、push、非快进和计划外文件。
- mock 命令/GitHub：Run、Draft、资产、公开、Latest 和 cleanup，不触及真实仓库远端。
- Windows 进程测试：实时输出、超时、取消、后代终止、环境白名单和输出上限。

### Vue

- mock typed Tauri service/Channel，覆盖预检、说明编辑、阶段事件、失败、恢复、公开二次确认和完成/警告分离。
- 测试按钮禁用原因、焦点恢复、Escape、默认安全动作、日志文本展示、900×620 和窄窗口类。
- `App.vue` 保持组合面，组件 props/emits 和 composable 状态只读。

### 结构与集成

- 结构测试证明正式 bundle 不包含控制台，workflow 使用说明文件、expected version/SHA、Draft 和固定 Secrets/Action SHA。
- 依赖图继续证明 `codex-relay-core` 不依赖 Tauri；控制台 Tauri crate 是独立 workspace member。
- 完成前运行专项测试、控制台 typecheck/test、workspace fmt/Clippy/tests、`npm run check`、正式 Codex Relay 普通 build 和控制台 `--no-bundle` 实际 build。
- 枚举两个 EXE/NSIS 的实际路径、大小、时间和 SHA-256；只声明本轮实际构建的产物，不声明安装、签名或升级。

## 15. 兼容性、迁移与回滚

- 现有发布行为保持 Draft-first、固定 updater endpoint/公钥、GitHub Secrets 名称和清理策略。
- 工作流迁移后，人工 GitHub 页面仍可通过输入 expected version/SHA 触发；发布控制台不是唯一恢复入口。
- `.github/release-notes.md` 在历史 Git 中保存每次最终说明，但当前文件只表示下一次/最近一次候选；Release 历史仍由 GitHub 保留策略决定。
- 如果控制台实现需要回滚，可删除独立 workspace/package/scripts，并把 `release.yml` 恢复为内联 `releaseBody`；正式 Codex Relay 产品配置与用户数据不需要迁移。
- 错误 Draft 不由首版自动删除；公开 Release 不允许原地替换资产，修复继续使用更高 SemVer。
