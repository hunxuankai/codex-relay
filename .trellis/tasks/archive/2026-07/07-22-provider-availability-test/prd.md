# Provider 可用性与 Codex 兼容性测试

## 目标

在 Provider 详情中提供两种由用户显式触发、结果彼此独立的测试：默认使用 Relay 直接发送
最小无工具 Responses 请求，确认 Provider 的 API 配置可调用；高级入口使用真实 `codex exec`
验证 Codex CLI 端到端兼容性。两种测试均不得修改用户现有 `config.toml`、`auth.json`、当前
Provider 或模型偏好。

## 已确认事实与决策

- 默认操作是“测试 API 可用性”；“Codex 兼容性测试”位于独立高级入口，默认测试不会隐式
  启动 Codex CLI。
- API 测试验证 Base URL、Bearer 认证、Provider 当前偏好模型、Responses JSON 格式和一次
  最小生成；它不宣称 Codex CLI 能读取或使用这套配置。
- Codex 兼容性测试验证 CLI 版本、临时 Provider 配置、回环假密钥注入、请求构造、流式
  Responses 处理和退出结果；真实密钥由 Rust 受监控转发层短暂注入上游 Header，成功路径不得
  出现任何工具调用。
- 两种测试各向目标 Provider 发送一次模型请求。API 测试正文和输出上限都很小；Codex 兼容性
  测试包含 Codex 自身上下文，token 消耗可能接近一次正常 Codex 回合。界面必须在按钮附近
  准确说明费用与延迟差异，且只有用户显式点击后才联网。
- 测试使用 Provider 已保存的当前偏好模型，不增加临时模型选择器；未配置密钥、配置无效或
  没有模型偏好时不发起请求。
- 测试结果只保存在本次前端会话内，不写入 Provider DTO、配置文件、应用数据、日志、通知或
  备份。Provider 文件指纹变化后，旧结果失效。
- API 测试使用当前 Relay 网络代理设置；不读取代理环境变量作为隐式回退，不跟随 HTTP
  重定向，也不重试，以免重复计费或把 Authorization 发送到意外目标。
- Codex 兼容性测试不套用 Relay 更新代理。它只继承 Codex 正常联网所需的 HTTP(S) 代理、
  `NO_PROXY` 和 CA 环境变量白名单，不继承其他任意用户环境变量。
- Codex 官方手册确认：`CODEX_HOME` 设置整套状态根目录，Provider `env_key` 指定密钥环境
  变量，`-c/--config` 可做单次配置覆盖。当前 `codex-cli 0.144.4` 的帮助与本地 Mock 实验另行
  确认 `--ignore-user-config`、`--ignore-rules`、`--ephemeral`、`--strict-config` 和 `--json`。
- 当前安全支持版本初始只允许精确的 `codex-cli 0.144.4`。其他版本在向真实 Provider 发请求
  前返回“当前 Codex 版本不支持安全兼容性测试”，后续版本须重新通过同等实验门禁后加入
  允许列表。
- 兼容性测试必须使用独立临时 `CODEX_HOME`、`CODEX_SQLITE_HOME` 和工作目录；子进程只通过
  每次运行生成的 Provider 专属环境变量收到回环假密钥。真实密钥只存在于 Rust 上游 Header，
  不写临时 `config.toml` 或 `auth.json`，不进入 argv、stdout/stderr、JSONL、日志或普通前端状态。
- 在真实请求前，兼容性测试先用假密钥和回环 Mock 运行同一套严格配置，确认当前进程的初始
  工具集合精确等于允许集合 `update_plan`、`view_image`。不匹配时不得联系真实 Provider。
- 严格配置使用纯文本 model catalog、关闭 request-user-input、shell、apps、browser/computer、
  image generation、plugins、tool suggest、multi-agent、Hook、web search 和 MCP。纯文本 catalog
  使 `view_image` 在读取文件前失败；`update_plan` 只可改变临时进程状态。
- Provider 返回任意工具调用都判定测试失败。未知工具必须由 Codex 拒绝；已暴露的两个工具也
  不计为成功。不得把 Hook 当作安全边界，因为 Hook 超时、非零退出或 malformed 输出会放行，
  hosted tools 也不经过本地 Hook。
- 若 Windows 系统 managed requirements 存在、无法证明 managed hooks 未被强制启用，兼容性
  测试在启动 Codex 前 fail closed。临时 Home 不含登录认证，因此不加载用户认证或云身份配置。
- 超时、用户取消、输出超限、解析失败或安全门禁失败必须终止整个 Windows 进程树；只调用
  `kill_on_drop` 不足够。清理前验证目录仍位于系统临时目录，清理失败必须如实报告。
- API 请求超时为 30 秒；Codex 版本探测为 3 秒、回环预检为 15 秒、真实兼容性请求为 60 秒、
  进程终止与清理最多再等待 5 秒。界面允许取消当前测试。
- 开发和自动化测试只使用回环 Mock、注入的假 HTTP/CLI 边界、临时目录和
  `test-key-*-not-real`。不得运行会读取真实 `%USERPROFILE%\.codex` 的已安装 Codex，也不得
  使用用户此前粘贴的真实形态密钥；该密钥应在 Provider 侧撤销并轮换。

## 功能需求

### Provider 测试入口

- Provider 详情新增独立可用性面板，默认按钮为“测试 API 可用性”，高级按钮明确命名为
  “运行 Codex 兼容性测试”。列表卡片不增加第二套结果展示，避免同一状态出现多个来源。
- 高级测试开始前显示确认对话框，说明会启动本机 Codex、发送一次正常 Codex 回合、可能产生
  高于 API 测试的 token 消耗，但不会修改当前 Codex 配置或认证。
- 同一时刻只运行一个 Provider 测试。运行期间测试按钮切换为可取消状态，Provider 编辑、
  删除、切换和偏好修改入口禁用，选择其他 Provider 仍可查看。
- API 与 Codex 结果分开保存和显示；重新运行某一类测试只替换该类结果。

### API 可用性测试

- 后端通过现有 Provider 读取边界解析目标 Provider、当前偏好模型和已保存密钥；密钥不得通过
  Tauri command 输入或返回前端。
- 请求固定为无工具、非流式、最多 16 个输出 token 的最小 Responses 调用；不接受用户自定义
  prompt，不展示或记录 Provider 返回的原始文本和错误正文。
- 成功必须同时满足：HTTP 2xx、响应体在大小上限内、可解析为 JSON，且包含可识别的 Responses
  完成结果。仅 TCP 连接成功或仅认证成功不算通过。
- 响应体读取设置 256 KiB 上限；超限立即停止并返回协议错误。

### Codex 兼容性测试

- 启动前解析 `codex --version`，检查精确允许列表和所有本地安全前置条件；任一项不满足时不得
  访问真实 Provider。
- 每次运行创建唯一临时根目录，包含 Home、SQLite 状态、空工作目录和纯文本 model catalog；
  不从真实 Home 复制任何文件。
- 回环预检使用假密钥，捕获 Codex 首个 Responses 请求并核对 Provider、模型、Authorization
  形态、工具集合和请求大小；预检成功后才使用真实目标运行第二个 `codex exec`。
- 第二个 `codex exec` 仍连接本机受监控转发层；该层复核请求并用 Rust 内部真实密钥转发上游，
  在 function/custom tool call SSE 到达 Codex 前阻断。真实运行同时解析有大小上限的 JSONL 与
  stderr，不把原始内容写入日志。只有看到正常完成事件、
  退出码为 0、没有工具调用、没有安全警告且清理成功时才判定通过。
- 版本不支持、managed config 存在、严格配置不被接受、catalog schema 不兼容、工具集合漂移、
  未知 JSONL 契约、任一工具调用、子进程终止失败或临时目录清理失败都必须 fail closed。

### 结果与错误分类

- 公开结果包含：Provider ID、测试类型、状态、稳定 code、安全中文 message、模型、耗时、测试
  时间，以及可选 HTTP 状态或 Codex 版本；不包含密钥、请求/响应正文、命令行、环境变量、
  临时路径或堆栈。
- 状态固定为 `passed`、`failed`、`unsupported`、`cancelled`。预期的远端失败通过结果 DTO
  返回；Tauri `CommandError` 只表示无法建立安全测试上下文等命令级失败。
- API 分类至少覆盖：配置/密钥/模型前置失败、DNS/连接/TLS、超时、401/403、404、429、5xx、
  其他 HTTP、响应过大和 Responses 格式无效。
- Codex 分类至少覆盖：CLI 缺失、版本不支持、安全门禁失败、回环预检失败、认证/模型/Provider
  远端失败、JSONL/退出异常、工具调用、超时、取消、进程树终止失败和清理失败。

## 可观察行为切片

1. 通过 API 测试 command 指定一个合法 Provider 后，真实 HTTP 客户端向回环 Provider 发送一次
   无工具最小请求，Bearer Header 使用后端短暂读取的假密钥；返回 2xx Responses JSON 时得到
   `passed`，公开 DTO、Debug、日志和前端状态均不含密钥。Mock 仅替代外部 Provider，不 mock
   Provider 配置/偏好/密钥读取。
2. 回环 Provider 分别返回 401、429、5xx、超大正文、非 JSON 或挂起时，API command 返回稳定
   分类且不重试；取消后请求停止。测试使用真实序列化、Header、响应上限和超时逻辑。
3. Codex 兼容性 command 在 CLI 缺失、版本不在允许列表、系统 managed requirements 存在或
   回环预检工具集合漂移时返回 `unsupported`，并证明真实 Provider 没有收到请求。CLI 进程边界
   使用可注入假实现；安全判定逻辑不 mock。
4. 假 Codex 可执行文件按当前 JSONL 契约完成回环预检和真实运行时，command 返回 `passed`，
   argv/JSONL/stdout/stderr/临时文件均不含假密钥，真实配置目录哨兵前后完全一致。Mock 只替代
   Codex 二进制与 Provider 网络，不跳过命令构造、环境白名单、解析和清理。
5. 假 Codex 返回工具调用、未知 JSONL、超量输出、挂起或派生子进程时，测试返回对应失败；
   取消/超时终止整个进程树并验证临时根目录删除。进程树测试不得只断言 `kill_on_drop`。
6. 前端从 Provider 详情分别启动 API 和高级 Codex 测试，显示独立 loading/result，确认高级
   操作并可取消；Provider 指纹变化后旧结果消失。组件只 mock typed service/composable，不访问
   文件系统或网络。
7. 应用启动、自检、Provider 列表刷新和文件监控不会自动触发任一测试；只有用户点击才联网或
   启动 Codex。

## 验收标准

- [ ] Provider 详情提供默认 API 测试和独立高级 Codex 兼容性测试，语义、费用提示和结果明确
  分开。
- [ ] API 测试完成一次无工具最小生成，覆盖认证、模型、Responses 格式、超时、响应上限和
  稳定错误分类；不跟随重定向、不重试。
- [ ] Codex 测试使用精确版本允许列表、回环工具面预检、严格临时配置、纯文本 catalog、空
  MCP、禁用 hosted/plugin/tool 能力和任意工具调用失败策略。
- [ ] 两种测试都不修改真实 `config.toml`、`auth.json`、Provider 选择或偏好；自动化路径哨兵
  证明默认目录递归快照不变。
- [ ] API Key 不进入 argv、临时文件、普通 DTO、Debug、日志、通知、快照、测试输出或 Trellis
  材料；真实 Provider 响应正文不展示、不记录。
- [ ] 超时和取消能终止完整 Windows 进程树；清理前校验临时根目录，清理失败不声称成功。
- [ ] CLI 缺失、版本漂移、managed requirements、工具集合漂移或未知协议均在真实请求前或
  最早安全边界 fail closed，不回退到 Hook-only 或默认模型元数据。
- [ ] Vue 使用独立可用性 composable 和聚焦组件，公开只读状态与显式动作；Provider 文件变化
  会使旧测试结果失效。
- [ ] 产品契约、README、关于页及其测试更新为“仅用户显式测试时访问模型网络”，准确区分
  API 最小请求与 Codex 正常回合的费用预期，启动阶段仍不自动访问 Provider。
- [ ] `npm run check` 通过；若未用位于受保护目录之外的安全 Codex 测试发行版执行人工端到端
  验证，交付时明确说明该限制。

## 范围外

- 启动时、定时或后台持续健康检查，以及自动重试或自动切换 Provider。
- 持久化历史测试结果、延迟排行榜、批量测试全部 Provider 或在托盘中增加测试入口。
- 用户自定义测试 prompt、输出内容展示、token/费用精确统计或性能基准。
- 修改真实 Codex 配置/认证后再回滚的测试方案。
- 支持不在安全允许列表中的 Codex 版本，或在门禁失败时降级为 Hook-only、默认 catalog、宽松
  工具集合或只杀主进程。
