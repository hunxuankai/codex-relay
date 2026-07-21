# 总体架构

## 分层

```text
Vue 视图与组件
  ↓ typed services / composables
Tauri command adapters
  ↓ AppState 共享服务与事务互斥
Rust domain services
  ↓ path / atomic file / fingerprint / logging infrastructure
Windows 当前用户文件与系统集成
```

前端不直接访问文件系统。`src/services/tauri.ts` 是唯一 `invoke` 边界，负责命令名、camelCase DTO、`CommandResult<T>` 解包和安全错误。composable 暴露只读状态与显式动作，并使用请求序列防止旧响应覆盖新事件。

## 数据所有权

| 文件 | 权威内容 | 禁止行为 |
|---|---|---|
| `config.toml` | Provider 名称、URL、Wire API、模型、未知字段与当前 Provider | 不能整文件反序列化后重建 |
| `providers.json` | Provider ID → API Key | 不能当作 Provider 定义唯一来源；损坏时不能覆盖 |
| `auth.json` | 当前生效 API Key | 不能通过普通列表、日志或事件返回 |
| `settings.json` | 窗口、托盘、引导和自启偏好 | 自启显示必须同时查询 Windows 实际状态 |

## 启动顺序

1. 注册 Single Instance，第二实例只唤醒已有窗口。
2. 尽早创建托盘占位菜单。
3. 解析路径，初始化脱敏日志、设置、Provider 服务和自启后端。
4. 安装 `AppState`、文件监控与日志守卫。
5. 从磁盘刷新托盘，恢复仍与显示器相交的窗口边界。
6. 根据 `--autostart` 与设置决定显示窗口或仅托盘。
7. 同步运行关键自检，后台运行扩展自检并发事件。

启动时不访问模型或更新网络；Codex 探测只运行有超时的本地 `codex --version`。更新网络请求只能由设置页的用户显式动作触发。

## 手动更新数据流

```text
SettingsView → UpdatePanel → useUpdater → typed updater service
→ tauri-plugin-updater → 固定 GitHub Releases latest.json
→ Tauri 公钥校验 → per-machine NSIS 被动更新
```

`src/services/tauri.ts` 负责把官方 updater 句柄规范化为应用 DTO；组件不得解析远端下载地址或持有插件对象。基础 Tauri 配置拥有固定 endpoint 与公开公钥，发布覆盖只负责开启 updater artifacts，任何签名私钥都不进入应用配置或前端状态。

## 写入数据流

```text
Vue typed DTO + 文件指纹
→ command 单次委托
→ ProviderService 业务验证
→ TransactionService 全局写锁与最新快照
→ 指纹检查与统一备份
→ toml_edit / JSON 服务在内存生成
→ 同目录临时文件、解析、替换、写后验证
→ 托盘、事件、自检和安全通知刷新
```

Provider 主界面与托盘必须调用同一个 `ProviderService::switch_provider`。当前 Provider 不得删除；目标无默认模型时保留现有顶层 `model`。

## 事件边界

- 监控 `config.toml`、`auth.json`、`providers.json`，对突发变化防抖。
- 应用事务通过写入守卫和最终指纹只抑制自身事件；外部修改必须刷新状态并触发扩展自检。
- 事件 payload 只包含 DTO、指纹、状态或安全消息，不含文件全文和密钥。
- `settings-changed` 只表示设置/自启变化；`app-notification` 只表示显式操作结果。
