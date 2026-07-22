# 产品契约

## 适用范围

Codex Relay 是面向 Windows 10/11、当前登录用户和个人可信计算机场景的 Tauri 2 桌面工具，用于管理 Codex Provider 配置、API Key、事务备份、自检、托盘和开机启动。

## 首版能力

- 读取、创建、编辑、删除和切换 `config.toml` 中的 Provider。
- 仅支持 `responses` Wire API；Provider ID 创建后不可修改。
- 在 `providers.json` 保存各 Provider 密钥，在 `auth.json` 保存当前生效密钥。
- 在 `provider-preferences.json` 保存 Relay 的模型集合、当前偏好和逐模型推理强度；Codex 顶层 `model`、`model_reasoning_effort` 才是当前生效配置。
- 使用统一事务提供备份、冲突检测、原子替换、写后验证和失败回滚。
- 支持关键/扩展自检、文件监控、系统托盘、单实例、当前用户开机启动和 Windows 通知。
- 支持用户在设置页显式检查公开 GitHub Releases 更新，并在 Tauri 签名校验后启动 NSIS 更新。
- 支持为 Codex Relay 自身网络请求配置无认证 HTTP/HTTPS 代理，并由用户显式测试或检测固定本机代理端口；Provider 详情提供用户显式触发的 API 可用性测试和 Codex 兼容性测试。API 测试使用 Relay 代理，Codex 兼容性测试沿用受限的 Codex 网络环境，不改变普通 Codex CLI 请求。
- Provider 测试结果只保存在当前前端会话中，不写入 Provider DTO、应用数据、日志或通知；配置指纹变化后旧结果失效。
- 首次没有 Provider 时显示引导，不自动写入虚假 Provider。

## 明确非目标

- 启动、自检、Provider 列表刷新和文件监控不调用模型接口；只有用户在 Provider 详情显式启动 API 可用性测试或 Codex 兼容性测试时，才向目标 Provider 发送一次模型请求。API 测试是无工具、非流式、最多 16 个输出 token 的最小 Responses 请求；Codex 测试是一次正常 Codex 回合，可能消耗更多 token 和等待更久。除固定更新源检查外，启动阶段不发起其他网络请求。
- 不提供 Credential Manager、Keyring、DPAPI、Stronghold 或其他密钥加密。
- 启动时自动检查一次更新，应用进程运行期间每小时检查一次；自动检查失败静默处理，发现更新后在页头提醒并跳转设置页。仍不提供强制更新、自动下载、自动安装、云同步、团队权限、远程管理、多用户隔离或 Provider ID 修改。
- 不把 Codex CLI 缺失视为阻塞 Provider 管理的错误。

## 数据与卸载契约

- `config.toml` 是 Codex 官方 Provider/顶层选择配置真相；`provider-preferences.json` 是 Relay 模型偏好真相；`providers.json` 是每个 Provider 密钥存储；`auth.json` 是当前生效认证。
- 普通 Provider 列表只暴露 `apiKeyConfigured`，不得返回密钥。
- 卸载器只移除程序和快捷方式，不删除 `.codex`、Codex Relay 应用数据、密钥、日志或备份。
- 怀疑泄漏时，界面清空本地密钥不等于远端吊销；用户必须在 Provider 平台轮换凭据。

## 发布契约

最终交付的主程序名为 `CodexRelay.exe`，Windows bundle 为 per-machine NSIS。构建成功不等于安装、升级、卸载或签名成功；每项声明都必须有对应本轮证据。
