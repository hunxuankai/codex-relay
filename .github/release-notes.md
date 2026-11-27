## 更新内容

- 新增：在 Providers 的「CLI Profiles」中查看已有的 Codex CLI Profile、复制启动命令，并为可管理的专用变量设置或更换 Key。
- 新增：可一键复用同一供应商已保存的 Key，仅写入该 Profile 指定的 Windows 用户环境变量，不更改配置文件或默认供应商选中的 Key，也不会自动跟随以后换 Key。
- 调整：移除「仅应用连接」入口；修复切换供应商后详情没有同步刷新的问题。

## 更新方式

已安装 `v0.5.3` 的用户可在设置页点击“检查更新”，再选择“下载并安装”；也可从 GitHub Releases 手动下载安装器，更新到 `v0.5.4`。应用内下载会经过 Tauri updater 签名校验，安装阶段应用会退出，并可能请求 Windows 管理员权限。

## 注意事项

本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。

CLI Profiles 功能要求 Codex CLI 0.134.0 或更高版本。更换环境变量 Key 后，请从已刷新的终端启动 Codex；必要时完全关闭终端宿主后重新打开。Windows 用户环境变量不是加密密钥库。
