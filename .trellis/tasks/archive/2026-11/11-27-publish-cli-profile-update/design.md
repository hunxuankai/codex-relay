# 发布设计

沿用现有 Windows x64 NSIS 与 Tauri updater 渠道，按仓库惯例从 `0.5.3` 增至 `0.5.4`。发布控制台的现有便携程序早于最近的发布；本轮用操作指南的命令行流程在当前会话可轮询执行，不引入新发布后端。签名仍仅由 GitHub Actions 读取已有 Secrets 完成。

## 候选到线上

1. 同步根 npm 元数据、Cargo 根包和共享 core 的版本与锁文件，并从 `.github/release-notes.md` 提供最终发布说明。Tauri 的应用版本继续读取 `package.json`。
2. 先在成对临时 Relay 覆盖下验证候选，使用不含 updater 签名变量的进程执行普通构建。随后精确提交推送；工作流通过 `expected_version`、`expected_sha` 绑定 `main` 候选。
3. CI 只创建 Draft。临时审计探针复用发布控制台的 `SystemGhBackend`、`DraftAuditService` 和 `GithubReleaseService`，结合 updater 的 minisign 库独立验签，不提交探针代码。修改线上状态之前重审同一 Draft 的 `target_commitish`、说明、三项资产、尺寸、摘要、manifest 平台指向与签名关联。公开后再从匿名端点复核实际下载，并跟踪由公开事件触发的 cleanup Run。

## 兼容与失败边界

对 CLI Profile 的修改由用户主动操作；复用只写目标 Profile 的 Windows 用户环境变量，不改 TOML 或默认供应商 Key，不自动同步后续改动。普通升级沿用现有安装器与数据保留策略。

任何本地失败阻止候选推送；CI 或 Draft 审计失败保留旧 Latest，不重触发不明 Run、不静默覆盖 Draft。公开后不替换同版本资产；发现问题按更高版本修复。GitHub 既有清理工作流只能保留当前 Latest，不手工删除旧对象。任务证据独立于候选版本提交，并可在原 CI Run 上恢复。
