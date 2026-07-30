# `service_tier` 调查记录

调查日期：2026-07-29。

## 官方配置参考

来源：<https://developers.openai.com/codex/config-reference/#configtoml>
（当前会重定向到 ChatGPT Learn 的 Configuration Reference）。

官方条目把顶层 `service_tier` 标为 `string`，说明它是新 turn 的首选 service
tier；可使用 `fast` 或当前模型公布的其他 tier，并明确说明 `fast` 映射到请求值
`priority`。

这意味着 `service_tier` 不是文档中固定列举的封闭枚举。当前文档唯一直接推荐给
用户写入 `config.toml` 的 Fast 值是：

```toml
service_tier = "fast"
```

文档没有把 `off`、`standard`、`default` 或 `auto` 定义为关闭 Fast 的 Codex
配置值。Relay 的关闭语义应删除顶层 `service_tier`，让 Codex 回到未指定 tier 的
默认行为。

## 官方 Speed 文档

来源：<https://learn.chatgpt.com/docs/agent-configuration/speed>

当前页面说明：

- Fast 将受支持模型的速度提高约 1.5 倍，并以更高 credits/API 费率换取速度。
- 当前列出的支持模型系列是 GPT-5.6、GPT-5.5 和 GPT-5.4。
- CLI 可以使用 `/fast on`、`/fast off` 和 `/fast status`。
- 持久化 Fast 默认值需要同时配置顶层 `service_tier = "fast"` 和
  `[features].fast_mode = true`。
- ChatGPT 登录使用 credits 规则；API Key 使用 API token 定价，Priority
  processing 有独立费率。

## 当前 Codex 模型目录快照

在安全临时 `CODEX_HOME` 下查询本机官方 `codex-cli 0.144.4`：

```powershell
codex debug models --bundled
```

结构化解析结果：

- `additional_speed_tiers` 的唯一值是 `fast`。
- `service_tiers[].id` 的唯一值是 `priority`，名称为 `Fast`。
- 当前带该能力的条目为 `gpt-5.6-sol`、`gpt-5.6-terra`、
  `gpt-5.6-luna`、`gpt-5.5` 和 `gpt-5.4`。

该目录证明当前客户端模型元数据中的 Fast/priority 映射，但模型目录可随 Codex
更新变化，Relay 不应据此把所有未来 tier 固化成静态枚举。

另外，在没有用户配置的安全临时 `CODEX_HOME` 下运行：

```powershell
codex features list
```

Codex CLI 0.144.4 报告 `fast_mode` 为 `stable` 且 `true`。因此两项职责不同：

- `service_tier = "fast"` 选择 Fast service tier。
- `[features].fast_mode = true` 打开 Fast 功能门禁；当前版本默认已经打开，显式
  写入只是在配置中重复默认值。

官方 Speed 页面给出的双字段写法能显式保证门禁开启，但不能据此把
`features.fast_mode` 误当成每个 Provider 的 Fast 选择。

## 对 Relay 方案的含义

- 产品界面保持 Provider 级布尔 `Fast`，开启时写官方用户值 `fast`。
- 不做通用 service tier 下拉框；官方配置当前没有稳定、封闭的候选集合。
- 关闭时删除 `service_tier`，不发明非 Fast 值。
- `features.fast_mode` 是全局能力门禁，不应被误当作 Provider 选择本身。当前默认
  为真时显式写入属于重复默认值，但 Relay 为保证“应用 Fast Provider”这一用户
  动作完整生效，最终采用单向管理：Fast 开启时确保门禁为 true，任何关闭 Fast
  的操作都不自动写 false。
- Fast 是否可用同时取决于当前模型和实际 Provider/API 能力，不能仅凭布尔值承诺
  远端一定接受 Priority processing。
