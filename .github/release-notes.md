## 更新内容

- 新增“仅应用连接”：可以保持顶层 `model_provider` 身份，只把另一 Provider 已选中的 Base URL 与 API Key 应用为当前连接，既有会话可继续沿原 Provider 身份使用。
- Provider 卡片会明确显示“仅应用连接”“已应用”“更新连接”或“恢复自身连接”；确认框只展示 Provider、地址名称和密钥名称，不显示 API Key 值。
- 首次覆盖会固定当前身份原有的地址与密钥条目作为恢复点；恢复自身连接或普通切换 Provider 时，会在同一受管事务中复原旧目标，避免留下半完成连接状态。

## 更新方式

已安装 `v0.3.0` 的用户可在设置页点击“检查更新”，再选择“下载并安装”；也可从 GitHub Releases 手动下载安装器，从 `v0.3.0` 更新到 `v0.4.0`。下载会经过 Tauri updater 签名校验，安装阶段应用会退出，并可能请求 Windows 管理员权限。

## 注意事项

连接覆盖关系保存在 `provider-preferences.json` v4 中，只记录稳定条目 ID；如果已经应用连接，降级前应先“恢复自身连接”并保留当前备份。本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。
