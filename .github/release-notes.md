## 更新内容

- CLI Profiles 卡片现在按对应 Windows 用户环境变量的实际值，显示同一供应商中匹配到的已保存 Key 名称。未匹配时会提示，始终不显示密钥原文。
- 在 Relay 中更换 Key 后，卡片会自动更新；如果从 Windows 外部修改了环境变量，可点击「刷新」。这里显示的是 Windows 用户变量，不代表已经打开的终端正在使用的值。

## 更新方式

已安装 `v0.5.5` 的用户可在设置页点击“检查更新”，再选择“下载并安装”；也可从 GitHub Releases 手动下载安装器，更新到 `v0.5.6`。应用内下载会经过 Tauri updater 签名校验，安装阶段应用会退出，并可能请求 Windows 管理员权限。

## 注意事项

本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。
