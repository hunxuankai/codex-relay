# 发布控制台仓库状态与版本展示实施计划

> 执行方式：Codex inline；Trellis `tdd` 工作流负责状态与门禁，不派发写入型子代理。

## 目标

通过测试驱动实现仓库路径记忆、线上 Latest 展示、发布说明机制提示和范围横幅移除，并完成发布控制台跨层回归。

## 文件图

- 新建 `tools/release-console/src/composables/useRepositoryPreference.ts`：版本化仓库偏好。
- 新建 `tools/release-console/src/composables/useRepositoryPreference.test.ts`：偏好行为测试。
- 修改 `tools/release-console/src/App.vue`、`src/App.test.ts`：接线、保存成功路径、删除横幅。
- 修改 `tools/release-console/src/components/release/RepositorySetupPanel.vue` 及测试：Latest 展示。
- 修改 `tools/release-console/src/components/release/ReleasePlanPanel.vue` 及测试：非 AI 说明。
- 修改 `tools/release-console/src/types/release.ts`：typed preflight DTO。
- 修改 `tools/release-console/src-tauri/src/models.rs`：Rust DTO。
- 修改 `tools/release-console/src-tauri/src/services/release_application.rs`：规范化路径与 Latest 投影。
- 修改受影响的 Rust/TypeScript fixture：补齐新增字段。
- 完成后按 `trellis-update-spec` 判断并更新 `.trellis/spec/release/publishing.md`。

## 行为切片 1：版本化仓库偏好与横幅移除

公开接口：`useRepositoryPreference(storage?)`、发布控制台根界面。

- [x] RED：新增 `useRepositoryPreference.test.ts`，断言有效 v1 值可恢复、损坏/未知版本/空路径回退为空、`remember` 保存 trim 后路径、`getItem`/`setItem` 抛错不阻断。
- [x] RED：更新 `App.test.ts`，断言已保存路径出现在仓库输入框，且页面不包含“首版只完成可视化一键发布与在线复核”。
- [x] 运行：

  ```powershell
  npx vitest run src/composables/useRepositoryPreference.test.ts src/App.test.ts
  ```

  工作目录：`tools/release-console`。预期因 composable/接线缺失或旧横幅仍存在而失败。

- [x] GREEN：实现版本化 JSON 读取与显式 `remember`；`App.vue` 改用该 ref，删除横幅和 `.scope-banner` 专用样式。
- [x] GREEN：重跑同一命令，2 个文件共 10 项测试通过。

证据：首次运行因 `useRepositoryPreference` 模块不存在而 2 个 suite 按预期失败；实现后相同命令退出 0，偏好专项 7 项、App 3 项通过。

## 行为切片 2：预检返回规范化路径和线上 Latest

公开接口：Rust/TypeScript `ReleasePreflightResult` DTO、`inspect_release_repository`。

- [x] RED：在 `release_application.rs` 单元测试中断言 `[draft, prerelease, published]` 返回 published tag，空列表/仅非正式 Release 返回 `None`。
- [x] RED：更新 command/DTO fixture 的预期形态，要求 `repositoryPath` 为规范化路径、`latestReleaseTag` 为 `Some("v0.4.0")`。
- [x] 运行：

  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console latest_published_release_tag
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test commands
  ```

  工作目录：仓库根。预期因 helper/DTO 字段缺失而编译失败或断言失败。

- [x] GREEN：在 Rust DTO 增加 `repository_path` 与 `latest_release_tag`；从已解析 Release 列表提取 Latest；new release 预检返回规范化路径，recovery 返回相同路径和 `None`。
- [x] GREEN：同步 TypeScript DTO 与所有 fixture；Rust helper 1 项、command 1 项、Git preflight 2 项及控制台 typecheck 均通过。

证据：首次 Rust 运行因 `latest_published_release_tag` 不存在而出现 3 个 `E0425`，符合预期 RED；实现后 helper 1/1、commands 1/1、Git preflight 2/2 通过，`vue-tsc` 退出 0。

## 行为切片 3：成功预检后记住规范化路径

公开接口：用户点击“检查仓库”后的输入值和下次启动行为。

- [x] RED：App 交互测试 mock typed Tauri invoke，输入原路径后点击检查；后端返回不同的规范化 `repositoryPath`，断言输入框和 v1 偏好都更新为后端值；预检失败时不得覆盖旧偏好。
- [x] 运行：

  ```powershell
  npx vitest run src/App.test.ts
  ```

  预期因 App 尚未保存成功结果而失败。

- [x] GREEN：`inspectRepository` 取得非空结果后更新 ref 并调用 `remember(result.repositoryPath)`；失败保持现有输入和旧偏好。
- [x] GREEN：重跑 App 与偏好专项测试，2 个文件共 12 项通过。

证据：首次 App 专项 5 项中 1 项按预期失败，输入仍为 `D:\\typed\\repository` 而非后端返回的规范化路径；最小接线后 App 5/5、偏好 7/7 通过，失败预检保持旧偏好的负向用例同时为绿。

## 行为切片 4：Latest 与生成机制的用户可见展示

公开接口：`RepositorySetupPanel`、`ReleasePlanPanel`。

- [x] RED：RepositorySetupPanel 测试断言 `v0.4.0` 被显示，`latestReleaseTag: null` 时显示“尚无正式版本”。
- [x] RED：ReleasePlanPanel 测试断言文案包含“根据 Git 提交与固定模板生成”和“不调用 Codex”。
- [x] 运行：

  ```powershell
  npx vitest run src/components/release/RepositorySetupPanel.test.ts src/components/release/ReleasePlanPanel.test.ts
  ```

  预期因用户可见字段/文案缺失而失败。

- [x] GREEN：预检摘要新增“线上 Latest”；发布说明帮助文案改为批准文本；窄布局继续使用现有两列降级规则。
- [x] GREEN：重跑组件专项测试，2 个文件共 4 项通过。

证据：首次组件专项 4 项中 3 项因 Latest/生成机制文案缺失按预期失败；实现后 RepositorySetupPanel 3/3、ReleasePlanPanel 1/1 通过。

## 行为切片 5：跨层回归、规范与提交

- [x] 运行控制台前端完整测试和类型检查：

  ```powershell
  npm run test:release-console
  npm run typecheck:release-console
  ```

- [x] 运行控制台 Rust 全包、格式和 Clippy：

  ```powershell
  cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
  cargo clippy --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --all-targets -- -D warnings
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console
  ```

- [x] 使用成对安全 Relay 覆盖运行项目完整检查，不读取真实用户目录：

  ```powershell
  $tempRoot = Join-Path $env:TEMP ("codex-relay-check-" + [guid]::NewGuid().ToString("N"))
  $env:CODEX_RELAY_CODEX_HOME = Join-Path $tempRoot "codex-home"
  $env:CODEX_RELAY_APP_DATA_DIR = Join-Path $tempRoot "relay-data"
  npm run check
  Remove-Item -LiteralPath $tempRoot -Recurse -Force
  ```

- [x] 加载 `trellis-update-spec`，更新发布控制台操作说明：仓库成功预检后会记住、预检展示 Latest、说明不调用 Codex；保留真实安装/升级证据边界，不能因横幅删除而修改契约。
- [x] 运行 `git diff --check`、秘密/路径审计和精确暂存检查。
- [x] 按 AGENTS.md 授权直接提交本任务相关改动，提交信息使用简体中文，并记录提交哈希与本轮真实验证证据：`6405eee`。

质量证据：

- 控制台前端完整测试退出 0：11 个文件、26 项测试；`vue-tsc` 退出 0。
- 首次 `cargo fmt --check` 发现 `git_release.rs` 一处断言换行，运行标准 rustfmt 后复查退出 0；控制台 Clippy 退出 0。
- 控制台 Rust 全包退出 0：81 项测试，无失败。
- 成对临时覆盖下 `npm run check` 退出 0，用时 372 秒；Trellis 8 项、根 Vitest 54 文件/274 项、控制台 Vitest 11 文件/26 项、Rust workspace 379 项均无失败，临时检查目录无残留。
- 本地 Vite 只读视觉检查：900×620 为双栏（`248px 581px`），760×560 为单栏（`721px`）；两者横向溢出均为 false，范围横幅不可见，“不调用 Codex”文案可见，浏览器错误日志为空。检查后已关闭标签、重置视口并停止本地监听进程。
- 未执行真实 GitHub workflow、Draft/Release 写操作、Sandbox、安装、UAC、应用内升级、签名或发布控制台 EXE 构建；本任务不声称这些行为成功。

## 自审结果

- PRD 的 AC1–AC7 均映射到上述行为切片。
- 没有 `TBD`、`TODO` 或未定义接口。
- Rust `repositoryPath/latestReleaseTag` 与 TypeScript camelCase 字段一致。
- 偏好存储、GitHub 查询、会话状态和发布说明生成各自保持单一职责。
- 不触发真实 GitHub workflow、Release 写入、Sandbox、安装、UAC 或应用内升级。
