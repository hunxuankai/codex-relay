# 产品契约

## 适用范围

Codex Relay 是面向 Windows 10/11、当前登录用户和个人可信计算机场景的 Tauri 2 桌面工具，用于管理 Codex Provider 配置、API Key、事务备份、自检、托盘和开机启动。

## 首版能力

- 读取、创建、编辑、删除和切换 `config.toml` 中的 Provider。
- 仅支持 `responses` Wire API；Provider ID 创建后不可修改。
- 每个 Provider 可保存多个命名 Base URL 和多个命名 API Key，并在详情页独立选择；两组列表不配对。
- 在 `providers.json` v2 保存命名密钥集合和密钥预选，在 `auth.json` 保存当前生效密钥。
- 在 `provider-preferences.json` v3 保存 Provider 列表顺序、命名 Base URL 集合、模型集合、当前偏好、逐模型推理强度和默认关闭的 `fastEnabled`；v1/v2 只读迁移时 Fast 默认关闭，只在下一次成功用户事务写出 v3。
- Provider 详情和编辑页提供模型目录驱动的 Fast 布尔开关。当前目录支持 GPT-5.6 Sol/Terra/Luna、GPT-5.5 和 GPT-5.4，不支持 GPT-5.4 Mini；不支持时保持关闭并显示原因，模型回退时在同一事务自动关闭。
- 左侧 Provider 列表支持拖动排序和聚焦手柄后的上下方向键排序；放开后立即展示，并通过受保护事务跨刷新和重启保留，不改变当前 Provider 或 Codex 官方配置。
- 使用统一事务提供备份、冲突检测、原子替换、写后验证和失败回滚。
- 支持关键/扩展自检、文件监控、系统托盘、单实例、当前用户开机启动和 Windows 通知。
- 支持用户在设置页显式检查公开 GitHub Releases 更新，并在 Tauri 签名校验后启动 NSIS 更新。
- 支持为 Codex Relay 自身网络请求配置无认证 HTTP/HTTPS 代理，并由用户显式测试或检测固定本机代理端口；Provider 详情提供用户显式触发的 API 可用性测试和 Codex 兼容性测试。验证区域默认“不使用代理”，用户仅在设置页“网络代理”已启用时才能取消该选项；两类测试随后共同使用已保存的 Relay 代理。普通 Codex CLI 请求不受影响。
- Provider 测试结果只保存在当前前端会话中，不写入 Provider DTO、应用数据、日志或通知；配置指纹变化后旧结果失效。点击 API 测试后详情弹窗立即打开并在请求/响应区域显示独立 loading，结果可携带同次请求生成的有界 trace 原位更新；弹窗支持 Escape、遮罩和关闭按钮关闭及结果入口再次打开。Codex 结果仍只包含安全摘要；trace 不含 Header、API Key 或代理地址。
- 首次没有 Provider 时显示引导，不自动写入虚假 Provider。

## 明确非目标

- 启动、自检、Provider 列表刷新和文件监控不调用模型接口；只有用户在 Provider 详情显式启动 API 可用性测试或 Codex 兼容性测试时，才向目标 Provider 发送一次模型请求。API 测试是无工具、非流式、最多 16 个输出 token 的最小 Responses 请求；Codex 测试是一次正常 Codex 回合，可能消耗更多 token 和等待更久。除固定更新源检查外，启动阶段不发起其他网络请求。
- 不提供 Credential Manager、Keyring、DPAPI、Stronghold 或其他密钥加密。
- 启动时自动检查一次更新，应用进程运行期间每小时检查一次；自动检查失败静默处理，发现更新后在页头提醒并跳转设置页。仍不提供强制更新、自动下载、自动安装、云同步、团队权限、远程管理、多用户隔离或 Provider ID 修改。
- 不把 Codex CLI 缺失视为阻塞 Provider 管理的错误。
- 不提供通用 `service_tier` 下拉框，不发明 `off` / `standard` / `default` / `auto` 关闭值，也不在运行时调用 `codex debug models` 或真实网络探测 Fast 能力。

## 数据与卸载契约

- `config.toml` 是 Codex 官方 Provider/顶层选择、实际 Base URL 和当前 Fast 投影真相；`provider-preferences.json` 是 Relay Provider 显示顺序、命名 URL、模型与 Fast 偏好真相；`providers.json` 是命名密钥与密钥预选存储；`auth.json` 是当前生效认证。
- 应用 Fast 时写顶层 `service_tier = "fast"` 并单向确保 `[features].fast_mode = true`；关闭时只删除 `service_tier`。当前 Provider 修改立即投影，非当前 Provider 只保存偏好。
- 普通 Provider 列表只暴露命名 URL、密钥名称/状态和配置完整性，不得返回密钥值。完整密钥只在用户显式打开管理器后进入短生命周期前端状态。
- 卸载器只移除程序和快捷方式，不删除 `.codex`、Codex Relay 应用数据、密钥、日志或备份。
- 新鲜 NSIS 安装可以选择目录；已登记安装的升级固定原目录，不提供并存安装或自动跨盘迁移。需要更换位置时先卸载旧版，再重新安装。
- 怀疑泄漏时，删除或替换本地命名密钥不等于远端吊销；用户必须在 Provider 平台轮换凭据。

## 发布契约

最终交付的主程序名为 `CodexRelay.exe`，Windows bundle 为 per-machine NSIS。构建成功不等于安装、升级、卸载或签名成功；每项声明都必须有对应本轮证据。
