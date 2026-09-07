# 修正 GPT-6 Astra 推理强度并兼容旧偏好

## 目标

根据用户确认，将 `gpt-6-astra` 的可选推理强度限定为 `low / medium / high / xhigh / max`，并保持旧版已保存偏好的可用性。

## 已确认事实

- 用户要求继续完成本项修正，已授权实施及验证后的提交、普通推送。
- 后端目录、现有流程测试及 README 错误包含 `ultra`，目录是界面选项与保存校验的共同来源（`provider_preference_service.rs:44`、`provider_workflow.rs:67`、`README.md:294`）。
- v0.5.1 能把 Astra 的 `ultra` 保存到当前 v4 私有偏好。读取时严格校验目录，直接删除目录项会使这类文件无法读取（`provider_preference_service.rs:388`、`:452`、`:597`）。

## 需求

- R1：Astra 只提供上述五档；默认值继续为 `low`，Fast 能力继续有效。
- R2：新提交的 Astra `ultra`、`none` 等非法强度必须被拒绝，不改变受管文件。
- R3：兼容旧版已保存的 v4 Astra `ultra`，按最高有效档位 `max` 读取；读取本身不得写盘，随后显式用户事务可保存规范化结果。
- R4：其他模型和非法值继续严格校验；保留有效配置、未知 TOML、认证和备份恢复语义。
- R5：测试只使用临时路径与明确假密钥；完整检查通过后提交、归档、记录会话并推送已配置上游。

## 验收标准

- [x] 通过 `ProviderService::list_providers` 取得的 Astra 目录恰好为五档；逐档保存及投影成功。
- [x] 通过 `update_provider_preference` 提交 `ultra` 或 `none` 返回 `INVALID_MODEL_REASONING_EFFORT`，四个受管文件字节不变。
- [x] 含旧 v4 Astra `ultra` 的偏好可以读取为 `max`，读取前后文件与备份数量不变。
- [x] 显式切换后偏好和当前 TOML 均保存 `max`；原始备份可以恢复，读取仍保持兼容。
- [x] 序列化新非法偏好及其他模型的 `ultra` 仍失败，不扩大兼容范围。
- [x] Provider 专项、路径安全、`npm run check` 和 `git diff --check` 有本轮通过证据。

## 范围外

不修改其他模型能力，不变更文件版本，不构建或发布新版安装包，不操作真实用户 Codex/Relay 配置。
