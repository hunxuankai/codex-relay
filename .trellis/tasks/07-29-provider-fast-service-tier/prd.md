# 为 Provider 增加 Fast 配置

## 目标

让 Codex Relay 为每个 Provider 独立保存 Fast 偏好，并在应用该 Provider 时把
偏好安全投影到 Codex 官方全局配置。用户切换 Provider 后无需再手工维护
`service_tier`，同时不会把 Relay 私有元数据写入官方 Provider 块。

## 外部契约与已确认约束

- Codex 的 `service_tier` 是 `config.toml` 顶层全局字符串，不是
  `[model_providers.<id>]` 字段。
- 官方明确支持 `service_tier = "fast"`，并说明它映射到请求的 `priority`；官方
  没有定义 `off`、`standard`、`default` 或 `auto` 作为关闭值。关闭 Fast 必须
  删除顶层 `service_tier`。
- `[features].fast_mode` 是全局能力门禁，不是 Provider 偏好。当前 Codex CLI
  0.144.4 默认开启该 stable 功能，但 Relay 在应用 Fast Provider 时仍单向确保
  `fast_mode = true`；任何关闭 Fast 的操作都不得自动写入 `false`。
- 当前官方 Codex 0.144.4 模型目录中，Relay 内置的 `gpt-5.6-sol`、
  `gpt-5.6-terra`、`gpt-5.6-luna`、`gpt-5.5` 和 `gpt-5.4` 支持 Fast，
  `gpt-5.4-mini` 不支持。模型目录随 Relay 版本发布，不在运行时调用 Codex 更新。
- Fast 可能提高 ChatGPT credits 或 API Priority processing 费用；界面只说明费用
  可能增加，不固化会随产品变化的价格数字。
- 官方资料和当前客户端目录证据见 [research.md](./research.md)。

## 需求

### R1 Provider 私有偏好

- 每个 Provider 保存一个布尔 `fastEnabled`，新建和既有 Provider 默认关闭。
- 私有偏好写入 `provider-preferences.json` v3，不保存任意 `serviceTier` 字符串。
- v1/v2 兼容读取为 v3 且 `fastEnabled = false`；只读加载不写盘，下一次成功用户
  事务再升级。
- 不从当前全局 `service_tier` 反向导入或猜测某个 Provider 的 Fast 偏好。

### R2 模型能力与校验

- Rust 模型目录是 Fast 支持能力的唯一真相，并通过 typed DTO 下发前端。
- Fast 只在当前偏好模型支持时可编辑。详情页和编辑页不得硬编码模型名单。
- 不支持 Fast 的模型下，开关必须关闭、禁用，并显示“当前模型不支持 Fast”或
  等价的明确原因。
- 绕过前端请求为不支持的模型开启 Fast 时，后端返回
  `MODEL_FAST_UNSUPPORTED`，不写任何受管文件。
- v3 文件包含“不支持 Fast 的模型 + `fastEnabled = true`”时按损坏偏好失败关闭，
  不在读取阶段静默修正。

### R3 详情页行为

- Provider 详情的现有模型偏好区域显示 Fast 开关和费用影响提示。
- 修改当前 Provider 的 Fast 后，私有偏好与当前 `config.toml` 在同一事务中更新。
- 修改非当前 Provider 的 Fast 后，只保存私有偏好，并提示将在应用时生效。
- Fast 已开启时在详情页切换到不支持的模型，模型切换继续成功，Fast 在同一事务
  中自动关闭；当前 Provider 同时删除顶层 `service_tier`，非当前 Provider 只更新
  私有偏好。

### R4 编辑页行为

- 新建和编辑页显示同一 Fast 开关；新建默认关闭。
- 编辑草稿的实际偏好模型不支持 Fast 时，草稿自动关闭 Fast 并禁用开关。
- 编辑当前 Provider 时，Fast 变化沿用现有“保存后立即同步当前 Codex 配置”选项：
  勾选才同步顶层配置，未勾选只保存私有偏好。
- 新建并选择“保存后立即启用”时，Fast 与 Provider、模型、推理强度和认证一起
  原子应用。

### R5 官方配置投影

- 应用 Fast Provider 时写入 `service_tier = "fast"`，并在同一事务中确保
  `[features].fast_mode = true`。
- 应用非 Fast Provider，或在需要同步当前 Provider 的操作中手动/自动关闭 Fast
  时，只删除顶层 `service_tier`；`features.fast_mode` 的既有值保持不变。修改非
  当前 Provider 的关闭偏好时，当前 `config.toml` 保持不变。
- `service_tier` 和 `features.fast_mode` 不得写入 Provider 块。
- 使用 `toml_edit` 局部修改，保留注释、未知字段、其他 Provider、其他 feature、
  profiles、MCP 和 sandbox 配置。

### R6 事务与用户反馈

- 创建、编辑、切换、详情模型切换和 Fast 修改继续经过共享
  `TransactionService` 的锁、指纹、备份、临时解析、原子替换、写后验证和可验证
  回滚。
- 操作成功消息必须区分“当前配置已写入”和“仅保存偏好”。自动关闭 Fast 时说明
  原因；失败消息使用稳定安全错误，不泄漏路径、配置全文或密钥。
- 任何失败都不得留下 `provider-preferences.json` 与 `config.toml` 相互矛盾的
  半完成状态。

## 可观察行为切片与公开边界

1. `ProviderPreferenceService` 读取 v1/v2 后返回 v3 内存模型且 Fast 关闭；序列化
   v3 能稳定往返，非法 Fast/模型组合失败关闭。
2. `config_service` 对 Fast 开启写顶层 `service_tier` 并单向开启
   `features.fast_mode`；对 Fast 关闭只删除 `service_tier`，其他 TOML 字节语义保留。
3. `ProviderService::create_provider` 创建 Fast 关闭/开启 Provider；立即启用时四个
   受管文件保持一致。
4. `ProviderService::update_provider_fast` 对当前 Provider 同时写 preferences/config，
   对非当前 Provider 只写 preferences；不支持模型返回稳定错误且零写入。
5. `ProviderService::update_provider_preference` 从支持模型切到不支持模型时自动关闭
   Fast，并按当前/非当前语义更新文件。
6. `ProviderService::switch_provider` 在 Fast Provider 与非 Fast Provider 间切换时，
   顶层 Provider、模型、推理强度、认证和 Fast 投影全部与目标偏好一致。
7. typed Tauri service 与 `useProviders` 透传 Fast DTO/动作，mutation 后重新加载后端
   权威状态，晚响应或失败不保留失真 UI。
8. `ProviderPreferenceControls` 和 `ProviderEditor` 展示正确开关、费用/不支持提示、
   禁用态、自动关闭行为和同步选择。

Rust 文件行为测试使用 `tempfile` / `AppPaths::for_test` 和真实事务服务，不 mock
事务或文件替换。Vue 测试只 mock typed Tauri/composable 边界。不调用真实 Provider、
Codex 会话或收费 API，不访问真实 `%USERPROFILE%\.codex` 或
`%LOCALAPPDATA%\CodexRelay`。

## 验收标准

- [x] AC1：不同 Provider 的 `fastEnabled` 可独立保存；新建、v1 和 v2 数据均默认关闭。
- [x] AC2：Fast 支持能力来自后端模型目录；支持模型可操作且显示费用提示，不支持
      模型关闭、禁用并显示原因。
- [x] AC3：直接为不支持模型开启 Fast 返回 `MODEL_FAST_UNSUPPORTED`，四个受管文件
      保持不变。
- [x] AC4：详情页修改当前/非当前 Provider 分别执行立即同步/延后生效语义。
- [x] AC5：Fast 开启后切换到不支持模型会原子关闭；合法模型切换不被阻止。
- [x] AC6：编辑当前 Provider 只有勾选现有同步选项才修改当前顶层配置。
- [x] AC7：应用 Fast Provider 后 `service_tier = "fast"` 且
      `features.fast_mode = true`；应用非 Fast Provider 后只移除 `service_tier`。
- [x] AC8：TOML 注释、未知字段、其他 Provider 和其他 `[features]` 项保持不变。
- [x] AC9：外部修改冲突、临时解析/写入/验证失败和回滚失败继续遵守现有事务契约。
- [x] AC10：README、关于页和项目规范准确说明 Fast 的私有偏好、全局投影、模型限制
      和费用影响。
- [x] AC11：专项测试、前后端完整检查、路径安全、任务校验和差异审计均有本轮真实
      证据；未执行的人工桌面观察必须明确报告。

## 范围外事项

- 通用 service tier 下拉框或允许用户输入任意 tier。
- 运行时调用 `codex debug models`、在线更新模型目录或远端探测 Fast 支持能力。
- 为每个模型分别记忆 Fast；Fast 仍是 Provider 级偏好。
- 自动把 `[features].fast_mode` 写成 `false`。
- 反向导入现有全局 `service_tier`、修改 Codex 本体或改变官方配置格式。
