# 设计

## 模型目录

- 扩充 `provider_preference_service` 中唯一的 Rust 模型目录。Provider 状态 DTO 已把目录传给创建/编辑弹窗和偏好控件，无需增加前端常量或修改界面、DTO、事务和迁移流程。
- 依据 OpenAI Codex 仓库提交 `6c0cee502d215e5df2dd9a303cf2e252229c9e12` 的 `codex-rs/models-manager/models.json`：两款模型均默认 `low`，六个强度为 `low / medium / high / xhigh / max / ultra`，`additional_speed_tiers` 含 `fast`。这只确定内置能力，不保证第三方网关有访问权限。
- 两款新模型与 `gpt-6.1-sol` 共用已核对的六档推理强度；旧 `gpt-6-astra` 的五档与 v4 读取兼容性不变。新增目录项不写入用户文件，显式修改仍经过现有 Provider 事务。
- 回归测试走公开 Provider 状态与保存边界，在独立 `AppPaths::for_test` 目录验证配置和认证字节，不调用 Codex 或模型供应商。

## 发布

- 当前 Latest 是 `v0.5.4`，以 `v0.5.5` 作为此次兼容更新。同步 npm/Cargo 版本及简体中文说明，不改 workflow、安装器模板或签名信任根。
- 必要本地门禁成功后，在配置的 `main` 上提交和普通推送候选；唯一一次 `workflow_dispatch` 用版本与 SHA 绑定 Draft。公开前核对 Release ID、版本、候选提交、说明、NSIS、签名和清单；任一不符不公开。
- 正式公开会触发项目既有 Latest 保留工作流；先记录旧版本身份再核验清理结果。不得额外手动删除或推送 tag。发布后保留审计证据，归档并验证最终同步。
