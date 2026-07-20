# 发布规范导航

## 开发前检查

- 修改 Tauri bundle、安装范围、安装目录、升级或卸载：读取 [tauri-nsis.md](tauri-nsis.md)。
- 发布、证书、SmartScreen 或产物声明：读取 [signing.md](signing.md)。
- 任何发布任务同时读取 `../testing/verification.md` 和 `../security/data-retention.md`。

## 质量检查

- per-machine、管理员权限和注册表范围是否一致？
- 固定 D 盘判断、Program Files 回退、升级目录恢复和目录页顺序是否保持？
- 卸载是否继续保留 Codex 配置和应用数据？
- 构建、安装与签名是否按实际证据分别报告？

## 文件

- [Tauri 与 NSIS](tauri-nsis.md)
- [代码签名](signing.md)
