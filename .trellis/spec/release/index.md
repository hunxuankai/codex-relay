# 发布规范导航

## 开发前检查

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
- 构建、安装与签名是否按实际证据分别报告？

## 文件

- [Tauri 与 NSIS](tauri-nsis.md)
- [Tauri 应用内更新](updater.md)
- [代码签名](signing.md)
