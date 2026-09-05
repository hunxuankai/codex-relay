# 编辑页新增 gpt-6-astra 并发布更新

## 目标

让用户在 Provider 编辑页的“可用模型”中选择 `gpt-6-astra`，保存并正常使用其模型偏好，随后发布可由现有客户端发现的正式更新。

## 已确认事实

- 用户明确要求“给编辑页的可用模型加一个 gpt-6-astra，然后发布更新”，已授权实施和发布。
- 编辑页消费后端内置模型目录；权威数据位于 `src-tauri/crates/codex-relay-core/src/services/provider_preference_service.rs`。
- 开始时本地版本及公开 Latest 均为 `0.5.0`，工作分支 `master` 跟踪 `origin/main`，工作区干净。已确认目标补丁版本为 `0.5.1`，无同名 Release/Tag。
- 当前运行时提供的 `gpt-6-astra` 能力为 `low`（默认）、`medium`、`high`、`xhigh`、`max`、`ultra`，支持 priority 服务层。

## 需求

- R1：内置目录包含且只包含一个 `gpt-6-astra`；编辑页可选择、保存，详情页可使用目录允许的推理强度和 Fast 偏好。
- R2：既有模型、已保存偏好、Provider 选择及事务安全行为保持兼容，不自动修改任何用户配置。
- R3：版本、锁文件和简体中文发布说明一致；完成本轮完整检查、普通 Windows 构建和 GitHub Draft 资产审核后公开更新。
- R4：按已配置上游普通推送；记录候选提交、Actions、发布地址、资产哈希和远端一致性。不得推送 Tag 或 force push。
- R5：仅使用临时测试路径及成对 Relay 覆盖；不读取、写入或删除真实 Codex/Relay 数据，不访问签名私钥或输出秘密。
- R6：修复本轮 CI 暴露的 Rust 1.98 Clippy 阻断：主应用及发布控制台的 Tauri async command 使用框架支持的传输错误类型，业务错误继续通过既有 `CommandResult<T>` 返回，前端 JSON/Promise 行为保持一致。

## 验收标准

- [x] 目录与偏好公开接口接受 `gpt-6-astra`，默认推理强度为 `low`，允许 `ultra`，Fast 可用。
- [x] 既有模型选择及编辑测试通过，本轮 `npm run check` 通过。
- [x] 使用与失败 CI 相同的 Rust 1.98 验证最终候选，Clippy 严格门禁及完整检查通过，不通过抑制 lint 或降低工具链绕过失败。
- [x] 不带更新私钥的本地 `npm run build` 通过，记录实际 EXE/NSIS 大小、时间与 SHA-256。
- [ ] Draft 指向已验证候选，版本、说明、Windows 安装器、`.sig`、`latest.json` 相互一致；更新签名核验通过后公开。
- [ ] Latest 公开指向新版本，下载清单及资产核验通过；记录历史 Release 清理结果。
- [ ] 相关提交、Trellis 归档与会话日志完成并推送，远程跟踪分支与本地 HEAD 一致。

## 范围外

- 不更换发布信任根、不引入模型在线探测、不修改安装和卸载行为。
- 本机真实配置不用于测试；未实际执行的安装、应用内升级、UAC、重启和卸载不宣称成功。
