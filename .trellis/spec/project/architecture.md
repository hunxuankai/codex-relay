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
| `config.toml` | Provider 名称、实际 URL、Wire API、Codex 顶层当前 Provider/模型/推理强度/Fast 投影、未知字段 | 不能整文件反序列化后重建；不得写入 Relay 私有数组 |
| `providers.json` v2 | Provider ID → 命名 API Key 列表与 `selectedApiKeyId` | 不能当作 Provider 定义唯一来源；损坏时不能覆盖；普通 DTO 不返回值 |
| `provider-preferences.json` v4 | Provider 显示顺序、命名 Base URL 列表、可用模型、当前偏好、逐模型推理强度、`fastEnabled` 和可选 `connectionOverride` 稳定 ID 关系 | 不保存第二份 URL 游标或 URL/Key 值副本；不能把私有元数据写入 Codex Provider 块 |
| `auth.json` | 当前生效 API Key | 不能通过普通列表、日志或事件返回 |
| `settings.json` | 窗口、托盘、引导、自启和应用内网络代理偏好 | 自启显示必须同时查询 Windows 实际状态；代理只允许无认证 HTTP(S) URL |

## 启动顺序

1. 注册 Single Instance，第二实例只唤醒已有窗口。
2. 尽早创建托盘占位菜单。
3. 解析路径，初始化脱敏日志、设置、Provider 服务和自启后端。
4. 安装 `AppState`、文件监控与日志守卫。
5. 从磁盘刷新托盘，恢复仍与显示器相交的窗口边界。
6. 根据 `--autostart` 与设置决定显示窗口或仅托盘。
7. 同步运行关键自检，后台运行扩展自检并发事件。

Rust/Tauri 启动阶段不访问模型网络；Codex 探测只运行有超时的本地 `codex --version`。前端设置首次加载结束后，`App.vue` 持有的唯一 updater 控制器会静默访问固定更新源一次，并在应用进程存活期间每小时检查一次；自动检查失败不提示，也不会自动下载或安装。

## 更新数据流

```text
App.vue → 唯一 useUpdater → typed updater service
   ├→ UpdateAvailableBanner
   └→ SettingsView → UpdatePanel
→ tauri-plugin-updater → 固定 GitHub Releases latest.json
→ Tauri 公钥校验 → per-machine NSIS 被动更新
```

每次自动或手动检查开始时，应用级控制器都从已保存设置读取当前有效代理；Update 会话沿用检查时的代理下载。页头提醒和设置页共享同一 release/session，进入设置页后不得为已发现的版本再次访问更新源。代理测试与固定本机候选检测复用同一 updater 检查边界，不新增可绕过 endpoint、公钥或签名校验的 HTTP 客户端。

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

Provider 主界面与托盘必须调用同一个 `ProviderService::switch_provider`。当前 Provider 不得删除；切换时使用目标 Provider 的预选 URL、密钥、模型和推理强度。详情页 URL/密钥选择分别调用独立 command，不得隐式切换全局当前 Provider。

Provider 列表排序使用独立 `reorder_providers` typed command，提交完整 ID 排列与文件指纹；
`ProviderService` 只通过 `TransactionService` 更新 `provider-preferences.json.providerOrder`。
列表投影先采用已保存顺序，再按 `config.toml` 原顺序追加未记录或外部新增的 Provider；不得为
排序重排 `config.toml` 表、改变活动 Provider 或复制第二份 Provider DTO 真相。

Provider Fast 使用独立 `update_provider_fast` typed command。`ProviderService` 拥有模型能力校验和
当前/非当前语义：当前 Provider 同事务写偏好与 `config.toml`，非当前 Provider 只写偏好；切换
Provider 时再把目标布尔偏好投影为顶层 `service_tier`。前端只消费 `ProviderProfile.fastEnabled`
和 `ModelCatalogItem.supportsFast`，不得复制支持模型 ID 集合。

保持身份的连接覆盖使用独立 `apply_provider_connection` / `restore_provider_connection` typed command。
应用输入只含来源 Provider ID 与四文件指纹，恢复输入只含四文件指纹；目标、条目 ID、URL 和 Key
均由 `ProviderService` 从事务前最新快照解析。应用或更新只改目标 Provider 块的 `base_url`、当前
`auth.json` 和 v4 关系，不改顶层身份、模型、推理强度或 Fast。普通切换与创建后立即启用必须在
同一事务先恢复旧目标块、清除关系，再应用新 Provider，不能留下失去恢复点的覆盖。

列表读取、自检和可用性目标解析共用后端连接关系投影：有效关系把当前身份标记为 `routed`，
失效关系标记为 `stale` 并拒绝新的覆盖或联网测试。读取边界不得迁移、修复或清除关系；显式恢复
在顶层身份仍是目标时同时恢复 URL/认证，否则只恢复旧目标 URL 并保留新当前身份的认证。

## 事件边界

- 监控 `config.toml`、`auth.json`、`providers.json`、`provider-preferences.json`，对突发变化防抖。
- 应用事务通过写入守卫和最终指纹只抑制自身事件；外部修改必须刷新状态并触发扩展自检。
- 事件 payload 只包含 DTO、指纹、状态或安全消息，不含文件全文和密钥。
- `settings-changed` 只表示设置/自启变化；`app-notification` 只表示显式操作结果。
