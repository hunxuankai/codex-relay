# 实施与验证

## 当前阶段

Phase 3.4：核心修复、专项、完整检查、控制台构建与副本哈希核验已通过，待精确提交。工作区初始干净，HEAD 为 d6c07f5，上游 origin/main。

## 步骤

- [x] 核验 GitHub 真实 Run 与分支筛选差异，读取 cleanup 服务、CLI adapter 和测试。
- [x] 写入并自审 PRD、设计与执行计划。
- [x] RED：原生 gh cleanup invocation 必须接受发布标签并按标签查询。
- [x] GREEN：传递 PublishedReleaseEvidence、验证 tag、过滤 headBranch；运行相关 Rust 测试。
- [x] 补充非法标签、相邻其他标签 Run、编排上下文与真实成功/失败边界验证。
- [x] 完成 Trellis check、完整 npm check、控制台构建与安全审计。
- [ ] 更新发布规范、验证记录并精确提交。

最终归档、会话日志和普通上游推送按 3.5 执行，完成后核对 HEAD 与 origin/main 一致。

## 验证命令

- `cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test github_release cleanup`
- `cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test github_release --test release_orchestrator`
- `npm run check`
- `npm run build:release-console`；当前未检测到运行中的控制台，可更新默认交付位置。若打包时出现占用，再用同一脚本显式输出到 `artifacts/release-console/cleanup-run-fix`，不终止用户进程。
- `git diff --check`、秘密扫描、交付文件 SHA-256 比对和 Git 上游核验。

## 初始证据与限制

- `gh run list ... --branch v0.5.2 --event release --created >=2026-09-10T11:04:53Z`：退出 0，命中 Run 34469403040（completed/success，headBranch=v0.5.2）。先前相同条件使用 main 返回空列表。
- 不重复执行真实清理，不公开或删除任何 Release，不修改既有完成会话；历史黄色结果是当时保存的未确认记录。
- 主应用安装、UAC、升级、签名与 Sandbox 不属于本次修复；不会把构建或自动化测试记为这些行为成功。

## 回归证据

- RED：`--test github_release cleanup_invocation_filters_the_published_tag_instead_of_main` 退出 101，1 项失败；有效标签被旧原生命令构造器报为 `unsupported ref`。
- GREEN：`--test github_release cleanup` 退出 0，3 项通过。
- 扩展验证：`--test github_release --test release_orchestrator` 退出 0，23 项 GitHub + 17 项编排测试通过；包含非法标签、相邻其他标签/旧 Run 排除、发布身份传递和成功/失败区分。
- `cargo fmt --all` 退出 0。所有测试命令均设置成对安全临时 Relay 覆盖；watcher 门禁退出 0。
- 主会话审查已确认只读 tag 查询与 release workflow 的 main 派发规则分离，前端 IPC 与持久化 schema 无变化。

## 完整检查

- `npm run check` 退出 0：Trellis 8 项，主前端 60 文件/356 项，控制台前端 17 文件/107 项通过，两套类型检查通过。
- Rust 依赖图、workspace fmt 与 Clippy（all-targets/all-features，warnings 视为错误）通过；Rust 484 项通过、0 失败、1 项既有慢速递归探针 ignored。
- 本轮 Rust dev 检查 14.58 秒，test 编译/链接 4 分 09 秒；未调整生产或测试超时来绕过门禁。
- 11 个变更/任务文件的高置信度秘密扫描无命中，Git 跟踪的 auth.json/providers.json 为零；`git diff --check` 退出 0。
- 安装、UAC、升级、Sandbox、签名和重新发布未执行；本轮只读查询不重复触发清理，也不修改远端 Release/tag。

## 控制台交付

- `npm run build:release-console` 退出 0，包含 Tauri no-bundle 构建及默认 postbuild 打包。
- 前端构建 13.52 秒，Rust release 编译/链接 2 分 37 秒；保留两条现有 VueUse/Rollup 注释警告。
- 标准交付文件 `artifacts/release-console/CodexRelayReleaseConsole.exe` 已更新，大小 12,969,984 字节，最后写入时间 `2026-09-10T20:10:14.1373324+08:00`。
- SHA-256：`8D0970A08148BFB4AD4057EC181257800B361307C31AAEAF4A50842495C1C3F2`；源文件与交付副本哈希一致，git check-ignore 确认不入 Git。
- 后续使用标准路径的新 EXE。既有完成会话保留当时未确认的历史警告；已公开的 v0.5.2 及其成功清理无需重跑。
