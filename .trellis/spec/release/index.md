# 发布规范导航

## 开发前检查

- 使用或修改独立发布控制台：先读取 [publishing.md](publishing.md)，确认控制台只服务当前仓库、
  不进入正式安装包，且 Sandbox/安装/UAC/应用内升级仍是独立证据。
- 准备或执行 Windows 更新发布：读取 [publishing.md](publishing.md)。
- 修改 Tauri bundle、安装范围、安装目录、升级或卸载：读取 [tauri-nsis.md](tauri-nsis.md)。
- 修改应用内更新、GitHub Releases、updater artifacts 或更新密钥：读取 [updater.md](updater.md)。
- 发布、证书、SmartScreen 或产物声明：读取 [signing.md](signing.md)。
- 任何发布任务同时读取 `../testing/verification.md` 和 `../security/data-retention.md`。

## 质量检查

- per-machine、管理员权限和注册表范围是否一致？
- 固定 D 盘判断、Program Files 回退、升级目录恢复和目录页顺序是否保持？
- 卸载是否继续保留 Codex 配置和应用数据？
- 普通构建是否仍不依赖更新私钥，发布构建是否只生成 Draft updater 资产？
- endpoint、公钥、Secrets 名称和 `latest.json` 目标是否与结构测试一致？
- package、Cargo、锁文件、发布说明和结构测试中的版本是否一致？
- Draft 的目标提交、最终说明、NSIS、`.sig` 和 `latest.json` 是否逐项核对后才公开？
- Draft 审计是否用 `target_commitish` 绑定候选且不提前要求 tag ref，并只规范化说明的行尾与末尾空白？
- 控制台是否阻止同仓库重复 session/后台管线，并在 commit 前失败时先清索引再验证六文件回滚？
- GitHub Run 发现与监控预算是否覆盖已观测的一小时以上 Windows 冷构建？
- 正式 Release 公开后，历史 Release、资产和对应 tag 是否由清理工作流删除，且当前
  `releases/latest` 对象被排除？
- 构建、安装与签名是否按实际证据分别报告？

## 文件

- [Windows 更新发布操作指南](publishing.md)
- [Tauri 与 NSIS](tauri-nsis.md)
- [Tauri 应用内更新](updater.md)
- [代码签名](signing.md)
