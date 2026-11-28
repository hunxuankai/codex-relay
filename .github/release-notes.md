## 更新内容

- 新增：Provider 的「可用模型」中可以选择 `gpt-6.2-astra` 和 `gpt-6.2-sol`。两款模型均支持 Fast，推理强度默认为 `low`，可选 `low`、`medium`、`high`、`xhigh`、`max` 和 `ultra`。
- 更新不会自动更换已有的模型或默认设置。实际能否调用新模型取决于供应商和账户权限。

## 更新方式

已安装 `v0.5.4` 的用户可在设置页点击“检查更新”，再选择“下载并安装”；也可从 GitHub Releases 手动下载安装器，更新到 `v0.5.5`。应用内下载会经过 Tauri updater 签名校验，安装阶段应用会退出，并可能请求 Windows 管理员权限。

## 注意事项

本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。
