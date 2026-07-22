# Provider 可用性测试研究证据

本文件只记录使用本地 Mock、临时目录和 `test-key-*-not-real` 得到的脱敏结果，不记录
Authorization、API Key、完整认证文件或真实 Provider 响应。

## CLI 与配置隔离

- 本机版本：`codex-cli 0.144.4`。
- `CODEX_HOME` 可切换 Codex 的整套状态根目录；`codex exec --ephemeral` 不保存会话，
  但仍可能创建 SQLite、系统 Skill 副本和插件同步临时目录，因此清理边界必须是整个临时
  Home，而不是只删 `config.toml`/`auth.json`。
- `-c model_providers.<id>.*` 和 Provider 专属 `env_key` 成功让 Mock 收到
  `POST /v1/responses`；密钥不在 argv、请求 JSON、stdout 或 stderr 中，只存在于受控子进程
  的 Authorization Header。
- 实验结束时临时 Home 中没有 `config.toml`、`auth.json` 或 `sessions` 目录；递归清理成功。
- `--ignore-user-config` 跳过 `config.toml`，但不会阻止同一临时 Home 的 `hooks.json` 加载。

## 工具枚举与 Hook

| 配置 | 初始 Responses 工具名 |
| --- | --- |
| 默认 | `shell_command`, `update_plan`, `request_user_input`, `view_image`, `multi_agent_v1`, `web_search` |
| 关闭 shell/apps/browser/computer/image/plugins/tool-suggest/multi-agent，`web_search="disabled"` | `update_plan`, `request_user_input`, `view_image` |

- `PreToolUse` deny Hook 可阻断上述三个残余本地函数工具，并在下一次请求中产生
  `function_call_output` 拒绝结果。
- Hook 拒绝后 Codex 仍可能退出码为 0；应同时检查 JSONL `item.completed`、工具输出和
  stderr。
- Hook 超时、非零退出或 malformed 输出会放行 `update_plan`，所以不能把 Hook 当成唯一
  安全边界。
- 官方手册标注 hosted tools（如 WebSearch）不走本地工具 Hook；必须配置关闭并检查初始
  工具白名单。
- `tools.experimental_request_user_input.enabled=false` 可移除 `request_user_input`。
- 关闭普通工具、插件、hosted tools、Hook 和 MCP，并加载纯文本 model catalog 后，初始工具面
  收敛为 `update_plan` 与 `view_image`。

## 文件读取风险与防御

- 无 Hook 时，模型返回 `view_image` 的绝对路径会让 Codex 读取工作目录外的图片，并把
  base64 `input_image` 回传给 Provider；当前 Windows `read-only` 参数不足以单独证明路径
  隔离。
- 临时 `model_catalog_json` 中将目标模型的 `input_modalities` 限为 `['text']` 后，
  `view_image` 在读取前返回“不支持图像输入”，同一实验没有 base64 内容。
- 该 catalog schema/字段可能随 Codex 版本变化；若生成、解析或加载失败，必须停止兼容性
  测试，不得回退到默认模型元数据。
- 在严格配置下，Mock 强制返回未暴露的 `request_user_input`、`shell_command`、
  `apply_patch`、`web_search`、`multi_agent_v1` 时，Codex 均返回 `unsupported call`，没有执行
  文件或外部副作用；`update_plan` 只更新临时状态。
- 成功判定仍要求没有任何工具调用；上述结果只证明恶意/异常 Provider 响应不会越过当前工具
  边界，不代表这些调用可以算测试成功。

## 进程与清理

- 未关闭插件能力时，Codex 可派生 `git fetch` 插件同步子进程。
- 本地 Mock 长连接实验中，按 PID 递归终止 Windows 进程树后，临时 Home 和工作目录均可
  清理；实现应使用等价的 Job Object/进程树终止，并在清理前验证路径位于系统临时目录。

## 门禁结论

API 直连测试可以在无工具请求、短时密钥环境和临时目录下实施。原始 Hook-only
`codex exec` 方案不安全，但以下组合已通过当前 `0.144.4` 本地 Mock 门禁：

- `--strict-config`、`--ignore-user-config`、`--ignore-rules`、`--ephemeral`；
- 临时 `CODEX_HOME`/工作目录与最小环境变量；正式实现还应显式把 `CODEX_SQLITE_HOME`
  指向该临时 Home；
- Provider 密钥只通过专属环境变量传入；
- 纯文本 model catalog；
- 禁用 request-user-input、shell、apps、browser/computer、image generation、plugins、
  tool-suggest、multi-agent、Hook、web search 与 MCP；
- 初始工具精确白名单只允许 `update_plan`、`view_image`；
- 任一工具调用均判失败，未知工具/配置/schema/版本立即 fail closed；
- Windows 进程树终止与校验后的整个临时目录清理。

正式实现若无法验证其中任一条件，必须报告“当前 Codex 版本不支持安全兼容性测试”，不得降级到
Hook-only 或默认模型元数据。

## 不采用的预检命令

- `codex doctor --json` 不适合作为产品安全预检。即使使用临时 `CODEX_HOME`，它仍会执行外部
  网络可达性检查并检查 Codex 安装来源，超出了“只验证本次临时 Provider 配置”的最小访问
  边界。
- 正式实现使用精确版本允许列表、系统 managed requirements 存在性检查和本地回环首请求工具
  枚举；自动化测试使用假 Codex executable，不运行用户安装的 Codex。
