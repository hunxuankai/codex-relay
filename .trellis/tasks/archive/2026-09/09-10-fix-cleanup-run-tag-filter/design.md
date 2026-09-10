# cleanup Run 标签筛选设计

## 边界与数据流

GitHub 的 release 事件 Run 归属于发布标签。控制台此前沿用 main 分支筛选，服务又只持有发布时间，导致合法 Run 永远无法命中。

使用现有 `PublishedReleaseEvidence` 贯穿 `ReleaseOrchestrator → ReleaseRemoteBackend → GithubReleaseService` 的 cleanup 入口；它已经包含核验后的 tag 与 publishedAt，避免引入另一份未绑定的标签输入。

`GhOperation::CleanupRuns` 使用 `GhRequest.tag_name`，`git_ref=None`。SystemGhBackend 验证标签为 `v<SemVer>`，构造固定 `gh run list --workflow cleanup-old-releases.yml --branch <tag> --event release --created >=<publishedAt>`；正式发布 workflow 的 main 边界保持原契约。

列表 JSON 增加 `headBranch`，RawCleanupRun 在 Rust 边界解析一次；服务只选择 headBranch 等于 published.tagName、创建时间不早于发布时刻的唯一 Run。后续监控继续核对发现的 Run ID/URL。

## 兼容性与安全

- 无前端 IPC、session schema 或用户设置变更。
- 保持 existing cleanup 成功/失败投影和公开 Release 证据；不自动改写既有 completedWithWarnings 记录。
- 不修改 cleanup 删除脚本，不增加 workflow_dispatch、Release PATCH、DELETE 或 tag 推送。
- 原生调用仍直接 argv，标签不能是任意分支或命令参数；不记录原始 gh JSON、环境或凭据。
- 保持共享 Cargo target；测试全部用 mock/临时目录，成对 Relay 覆盖用于所有验证命令。

## 验证

- RED：现有 GhRequest 已能携带 tagName；生产 invocation 对有效发布标签仍报 unsupported ref，证明旧行为。
- GREEN：服务、adapter 与原生 argv 完整传递发布标签。测试返回目标标签和其他标签的相邻 Run，证明仅选择当前版本。
- 保留 cleanup success/failure、首次完成进度日志、固定 dispatch main 与 asset 下载调用测试。
- 完整 npm check、控制台 build、复制副本哈希及只读真实 gh 标签筛选证据。
