# 技术设计

## 模型目录与兼容性

在 `provider_preference_service.rs` 的唯一 `MODEL_CATALOG` 首项加入 `gpt-6-astra`，能力为 `low`、`medium`、`high`、`xhigh`、`max`、`ultra`，默认 `low`，`supports_fast = true`。能力来源为本会话运行时提供的模型元数据；不读取用户 Codex 配置，不增加联网探测。

`ProviderService::list_providers` 已将目录映射为 `ModelCatalogItem`；编辑页的 `ElOption` 和详情偏好直接消费该数据，无需新增前端目录或组件分支。保存继续经过既有 Provider/Transaction 服务。保留其余模型、旧偏好格式和选择行为。

## 验证边界

复用目录契约测试更新数量与能力；通过 `provider_workflow` 的真实服务公开接口覆盖编辑当前 Provider 到新模型、默认强度、Fast、详情强度切换及 TOML 未知内容保留。使用 `tempfile` 和 `AppPaths::for_test`，不 mock 被测业务。既有组件测试验证通用编辑交互。

## 发布与恢复

先核验公开 Latest，再同步 npm、两个应用/core Cargo manifest、锁文件与 `.github/release-notes.md`。沿用完整检查、无更新私钥普通构建、精确提交及普通上游推送、GitHub 手动 Draft 构建、资产审计与公开流程。运行时生成发布 tag，不由本地推送 tag。

Draft 审计绑定候选 SHA，校验版本、最终说明、安装器与两种 Windows 平台清单、文件大小和 SHA-256、独立签名关联；优先用已有工具验证 updater 签名。发布后核验 Latest 下载、标签目标和清理工作流。公开后发现问题只发布更高 SemVer 修复，不替换已公开资产。

本轮不改安装/卸载/数据保留代码；未执行的 Sandbox/VM、真实安装、UAC、重启或应用内升级保留为未验证。

## 发布阻断修复：Tauri 传输错误类型

首次 CI Run 使用 Rust/Clippy 1.98.0，本地原工具链为 1.97.1；新版 lint 对 29 个既有 `Result<CommandResult<T>, ()>` 报 `clippy::result_unit_err`。Tauri 2.11.5/tauri-macros 2.6.3 对带 `State<'_, _>` 的 async command 仍要求外层 Result，故不能删除该层。

五个 command 文件统一改为 `Result<CommandResult<T>, tauri::ipc::InvokeError>`。这是 Tauri 支持的传输错误类型，不新增依赖或错误封装；所有函数体继续返回 `Ok(CommandResult<T>)`，不把业务错误移到外层 Err。锁定 serde 1.0.229 没有 Infallible 的 Serialize 实现，Tauri 也没有对应转换，不采用该替代。

红色证据来自本轮实际 CI。安装独立 1.98.0 工具链而不改变系统默认，使用该版本 Clippy、完整检查和普通构建核验最终候选；新候选仍发布尚未占用的 `0.5.1`。同步后端签名规范并记录工具链差异，保留原失败 Run。

## 发布阻断修复：Windows 进程测试夹具

第二次 CI 已通过 Clippy，但 core 的后代取消与大 stdout 文件测试超时。9 个进程测试原本就有串行锁和 30 秒预算；取消测试的首个状态写入依赖 `Set-Content`，大文件依赖 `New-Object`，异常诊断又依赖 `Out-String`。纯 Console/字符串 stdin 夹具在同次 CI 约 0.2 秒通过，带 cmdlet 的夹具耗时显著较高；尚无 CI 进程级 trace 证明内部等待点。

先在大 stdout 夹具禁用 PowerShell 模块自动加载，确认现有 `New-Object` 依赖确定性失败；随后把测试夹具统一改为 PowerShell 5.1 支持的纯 .NET 构造、文件和线程 API，并持续禁用自动加载，使未来误用模块 cmdlet 快速失败。保留现有进程树/流式断言及 release-console 的 20 秒脚本自截止，不修改生产实现或扩大超时。
