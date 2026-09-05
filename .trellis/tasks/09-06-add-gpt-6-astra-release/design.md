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
