# 产品契约

## 适用范围

Codex Relay 是面向 Windows 10/11、当前登录用户和个人可信计算机场景的 Tauri 2 桌面工具，用于管理 Codex Provider 配置、API Key、事务备份、自检、托盘和开机启动。

## 首版能力

- 读取、创建、编辑、删除和切换 `config.toml` 中的 Provider。
- 仅支持 `responses` Wire API；Provider ID 创建后不可修改。
- 在 `providers.json` 保存各 Provider 密钥，在 `auth.json` 保存当前生效密钥。
- 使用统一事务提供备份、冲突检测、原子替换、写后验证和失败回滚。
- 支持关键/扩展自检、文件监控、系统托盘、单实例、当前用户开机启动和 Windows 通知。
- 首次没有 Provider 时显示引导，不自动写入虚假 Provider。

## 明确非目标

- 不调用模型接口验证 Base URL 或 API Key；启动阶段不发起网络请求。
- 不提供 Credential Manager、Keyring、DPAPI、Stronghold 或其他密钥加密。
- 不提供自动更新、云同步、团队权限、远程管理、多用户隔离或 Provider ID 修改。
- 不把 Codex CLI 缺失视为阻塞 Provider 管理的错误。

## 数据与卸载契约

- `config.toml` 是 Provider 非秘密配置真相；`providers.json` 是每个 Provider 密钥存储；`auth.json` 是当前生效认证。
- 普通 Provider 列表只暴露 `apiKeyConfigured`，不得返回密钥。
- 卸载器只移除程序和快捷方式，不删除 `.codex`、Codex Relay 应用数据、密钥、日志或备份。
- 怀疑泄漏时，界面清空本地密钥不等于远端吊销；用户必须在 Provider 平台轮换凭据。

## 发布契约

最终交付的主程序名为 `CodexRelay.exe`，Windows bundle 为 per-machine NSIS。构建成功不等于安装、升级、卸载或签名成功；每项声明都必须有对应本轮证据。
