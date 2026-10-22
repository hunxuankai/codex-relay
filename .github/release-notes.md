## 更新内容

- 新增：Provider 的可用模型加入 `gpt-6.1-sol`，支持保存为偏好模型并应用到 Codex 配置。
- 新增：`gpt-6.1-sol` 支持 Fast，推理强度默认为 `low`，可选 `low`、`medium`、`high`、`xhigh`、`max` 和 `ultra`。
- 修复：发布控制台遇到暂时性 Run 查询失败时有界重试，并可恢复同一发布 Run，保留具体失败信息。
- 修复：发布控制台按已公开版本的标签定位历史清理 Run，正确识别清理结果。

## 更新方式

已安装 `v0.5.2` 的用户可在设置页点击“检查更新”，再选择“下载并安装”；也可从 GitHub Releases 手动下载安装器，从 `v0.5.2` 更新到 `v0.5.3`。下载会经过 Tauri updater 签名校验，安装阶段应用会退出，并可能请求 Windows 管理员权限。

## 注意事项

本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。
