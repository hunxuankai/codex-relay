# 实施与验证记录

## 当前阶段

Phase 3.5：全部行为切片、`npm run check`、控制台构建与交付文件哈希核验均已通过；工作提交为 `14858c7effc47a9784c095bfde450d82fe59a570`，进入归档、会话日志及最终推送。用户“继续”授权继续处理此故障，未授权公开或删除 Release。

## 有序计划

- [x] 读取项目规范与现有工作区状态，创建独立 Trellis 任务。
- [x] 核验 Run/Draft 与本地失败快照；定位错误丢失、单次故障中断和终态恢复缺口。
- [x] 持久化并自审 PRD、设计与执行计划；核心实施和检查保持 inline。
- [x] 切片 1 RED：通过监控公开入口复现一次 `GH_COMMAND_FAILED` 后无法继续成功；记录真实失败。
- [x] 切片 1 GREEN：保留类型化错误、增加有界查询重试及完整身份门禁；运行监控专项。
- [x] 切片 2 RED/GREEN：验证具体 remoteRun 错误的日志/事件/session 传播，避免泛化丢失。
- [x] 切片 3 RED/GREEN：验证新/旧失败会话受限恢复、陈旧状态保护、同一 Run 复核和零重复派发。
- [x] 切片 4 RED/GREEN：恢复按钮覆盖实际历史失败快照、代理/忙状态和非恢复失败。
- [x] Trellis check：完整 npm 检查、风险专项、diff/秘密/路径检查和必要控制台构建。
- [x] 更新发布规范与验证记录，精确提交本次改动。

收尾出口按固定顺序执行：归档 → 开发日志 → 对当前分支已配置上游 `origin/main` 普通 push → 核验本地/远端 HEAD 一致。归档材料记录至工作提交；最终推送结果与哈希以收尾答复和 Git 跟踪状态为准。

## 验证命令

- `cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --lib <目标测试>`
- `cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test github_release --test release_orchestrator --test release_state`
- `npx vitest run tools/release-console/src/components/release/ReleaseRecoveryPanel.test.ts tools/release-console/src/App.test.ts`
- `npm run typecheck:release-console`
- `npm run check`
- `npm run build:app --workspace @codex-relay/release-console`，再运行 `scripts/package-release-console.ps1 -DestinationDirectory artifacts/release-console/run-monitor-fix`。现有默认交付 EXE 正在运行，采用同一构建与打包脚本的显式输出路径。
- `git diff --check`、精确暂存差异检查与本轮秘密扫描。

测试只使用临时目录、mock 与成对安全 Relay 覆盖；保持共享 Cargo 缓存，不并行执行相互竞争的 Cargo 构建，不启动真实主应用。

## 初始证据

- 工作分支 `master`，上游 `origin/main`，初始 HEAD `fe39d72a2f88898ce1db7e476cacac1c90f34857`，工作区原本干净。
- `gh run view 34454972967 --repo hunxuankai/codex-relay --json ...`：退出 0；Run `completed/success`，更新时间 `2026-09-10T08:42:22Z`。
- `gh release view v0.5.2 --repo hunxuankai/codex-relay --json ...`：退出 0；Draft=true、prerelease=false、targetCommitish 为原候选，NSIS、`.sig`、`latest.json` 三项资产已上传。仅核验元数据，未下载审计或公开。
- 本地 Git 元数据的安全投影：session `release-20260910080431-0000000000000002`，phase=`failed`，failure=`workflowRunning/releasePipeline/RELEASE_REMOTE_FAILED`，Run ID 与候选 SHA 均保留。
- 用户现有控制台进程仍运行；未终止、未改动其 session。
- 历史实际查询错误类别已丢失，不能认定为某个具体网络错误。

## 切片证据

- 切片 1 RED：`cargo test ... --lib run_monitor_recovers_after_one_failed_query_without_dispatching`，退出 101；0 通过、1 失败，失败值为旧逻辑的 `GITHUB_BACKEND_FAILED`。
- 切片 1 GREEN：同命令退出 0，1 通过；复查 `--lib run_monitor` 退出 0，6 通过，覆盖成功后重置、连续四次失败、取消/安全错误、身份/响应/结论异常与预算边界。
- 切片 2 RED：`--test release_orchestrator remote_monitor_failure_preserves_specific_safe_code_and_run_checkpoint` 退出 101，期望 `GITHUB_PROCESS_TIMEOUT` 实际仍为 `RELEASE_REMOTE_FAILED`；修复后同命令退出 0，1 通过。
- 切片 2 补充：`--lib --test release_orchestrator monitor` 中 lib 8 项全部通过，包含错误日志 → 权威失败 session → stepFailed 顺序。该命令整体退出 101，因为同时加入的切片 3 回归如期失败。
- 切片 3 RED：`legacy_failed_monitor_session_resumes_the_same_run_without_dispatching` 在旧实现返回 `RemoteStateInvalid`；加入受限恢复后同命令退出 0，1 通过。
- 边界复核 RED：`--lib --test release_state monitor` 中 8 通过、4 失败，确定性复现未知 conclusion 被误分类、失败终态未记录、最后一次尝试误报将重试、旧请求误改新会话。修正后 `--lib --test release_state --test release_orchestrator monitor` 退出 0，12 个 lib + 2 个编排 + 3 个状态测试全部通过。
- 切片 4 RED：`ReleaseRecoveryPanel.test.ts -t 'legacy failed session'` 退出 1，按钮仍为“查看上次结果”；实现后退出 0，目标 1 项通过。随后恢复面板 + App 共 40 项通过，控制台 typecheck 退出 0。
- UI 空值复核 RED：空 `cleanupWarning` 被误认为无更晚证据，目标测试退出 1；改为严格 `null` 判断，最终完整前端两套测试均通过。
- 所有本轮测试命令成对设置安全临时 `CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR`；watcher 门禁退出 0。

## 完整检查

- `npm run check`：退出 0；Trellis 8 项、主 Vitest 60 文件/356 项、控制台 Vitest 17 文件/107 项通过；两套 vue-tsc 通过。
- Rust 依赖图、workspace fmt、workspace Clippy（all-targets/all-features，warnings 视为错误）通过；workspace Rust 481 项通过、0 失败、1 项既有完整项目慢速递归探针 ignored。该 ignored 项不等于本轮主 `npm run check` 未执行。
- Rust 编译/链接 3 分 06 秒，Git 临时仓库集成测试 71.41 秒；路径安全 3 项通过，Provider 工作流 3 项通过。
- `git diff --check` 退出 0；对 15 个改动/任务文件的高置信度密钥扫描无命中，Git 跟踪的 `auth.json`/`providers.json` 为零。
- 主会话完成源代码与跨层审查：只读轮询、错误分类、session 原子恢复、请求所有权、UI 入口与代理门禁一致；未引入原始 gh 输出或真实配置访问。主应用“关于”页面契约不受独立发布控制台修复影响。

## 控制台交付证据

- `npm run build:app --workspace @codex-relay/release-console`：退出 0；前端构建 11.18 秒，Rust release 编译/链接 2 分 32 秒。
- `pwsh -NoProfile -File scripts/package-release-console.ps1 -DestinationDirectory artifacts/release-console/run-monitor-fix`：退出 0。
- 实际交付文件：`artifacts/release-console/run-monitor-fix/CodexRelayReleaseConsole.exe`，12,969,984 字节，最后写入时间 `2026-09-10T18:33:30.6682795+08:00`。
- SHA-256：`CFA7F93A350D7CB38E684D21D868471307DB6A55E08F5088CAAABF23D32171BA`。源文件与交付副本哈希相等，`git check-ignore` 确认交付 EXE 不入 Git。
- 构建保留两条现有 VueUse/Rollup 注释警告；无构建失败。不把本次便携控制台构建当作主应用 NSIS、安装、签名或发布成功。
- 最后只读复核仍为 `v0.5.2` Draft，targetCommitish=`fe39d72a2f88898ce1db7e476cacac1c90f34857`；未公开、替换或删除。
- 默认交付路径的旧 EXE 仍由用户运行，因此使用独立子目录。退出旧控制台后打开修复版并点击“继续监控”，将通过原有审计重新核对同一 Run/Draft；本轮未操作真实 UI 恢复或公开。

## 审查与限制

- 自审：只读 Run 查询重试不包含 dispatch/publish；终态恢复走专用门禁，不全局放开状态转换；UI 不获得可信写权限。
- 只读辅助复核发现：真实失败结论应先留下完整结构化终态投影；未知或空白 conclusion 不应作为已确认远端失败；最后一次尝试不可声称还会重试。将用专项回归修正。
- 主应用、安装/UAC/升级/卸载、签名与 Release 公开不属于本次验证；不得将测试和构建等同这些行为。
