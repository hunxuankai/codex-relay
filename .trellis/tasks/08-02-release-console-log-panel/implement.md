# 发布控制台固定日志区域实施计划

## 0. 当前状态与启动门禁

- [x] 用户审查并批准 `prd.md`、`design.md`、`implement.md`。
- [x] 运行 `python ./.trellis/scripts/task.py start .trellis/tasks/08-02-release-console-log-panel`，
  确认任务进入 `in_progress`。
- [x] 加载 `trellis-before-dev`，按索引复核 release、frontend、backend、testing、security 和共享思考规范。
- [x] 全程使用 inline TDD；每个行为切片必须先运行红测并记录预期失败，再写最小生产实现。
- [x] 不派发写入型子代理，不建立额外计划目录、worktree 或重复任务生命周期。

## 0.1 当前进度（2026-08-03）

### 已完成

- Store/DTO 切片完成：轮换、恢复、分页、损坏前缀修复、三类上限、截断状态跨重启恢复和读取错误分类。
- recorder 完成连续 sequence、基础脱敏、单条 UTF-8 截断、一次易失 warning、持久化后实时事件镜像。
- process sink 完成增量 UTF-8、CRLF、ANSI、仓库根/代理/环境值/token 安全处理、64 KiB policy 分块、
  安全观察尾部、双流来源和幂等 `finish()`。
- `ProcessLocalVerificationBackend` 已支持可选 recorder，每个命令创建独立 sink，并在 runner 所有结果出口
  先刷新尾部；生产 application 注入留在切片 4。
- 结构化 progress 已覆盖 candidate、四个本地门禁、sourceAudit、commitPush、remoteRun、draftAudit、
  publishApproval、onlineVerification 和 cleanup；发布 Run/cleanup Run 共用状态投影与 5 分钟心跳 tracker。
- Application/IPC 切片完成：start 原子轮换日志，resume/publish 恢复 sequence，同一 recorder 注入本地、
  orchestrator 与 GitHub 管线；`GetSession` 返回 snapshot，`GetLogs` 使用当前 session context 分页。
- 日志初始化、打开、追加和事件 channel 失败均不改变发布结果；持久化不可用时切到易失模式并只显示
  一次明确 warning，损坏有效前缀可恢复且 warning 随页面返回。
- TypeScript typed client 与 `useReleaseSession` 完成 snapshot/page 契约、2,000 条最新页、100,000 条
  实时事件有界验证、sequence 去重、history/unread、旧 channel/分页失效和独立日志错误状态。
- `ReleaseLogPanel` 已完成固定工具带、最新跟随、历史阅读、分页、复制、warning/error/failure、键盘滚动
  和卸载后的异步 DOM 防护；`ReleaseStepDetails` 只保留会话事实。
- `App.vue` 已改为标题、独立滚动发布工作区、底部日志区三行 `100dvh` 布局，窄窗口把上方内容合并为
  单列滚动；latest-page 请求期间到达的实时 sequence 会与响应合并，不被较旧页面覆盖。
- Trellis check 人工跨层审查已修复四类遗漏：分页读取不再借用 `open()` 改写不可信尾部；start 日志
  初始化失败时先发送初始 session 再发送易失 warning；application 失败边界用同一 recorder 持久化权威
  code；本地工具绝对路径与跨块长 Bearer/已知敏感值不会进入公开日志。
- AC5 实时截断投影已补齐：store 压缩时把按同一 2,000 条分页规则构造的权威最新页返回 recorder，
  对应 `stepLog` 只在压缩事件上携带该有界页；最新视图立即替换已淘汰记录，历史阅读保持当前条目但
  同步总量、字节、截断 warning 与未读状态，不再要求手动刷新或重启后才看到截断证据。
- 长期规范已更新：发布规范记录固定日志区、JSONL/分页/三类上限、压缩实时页与非致命 I/O；后端
  日志规范记录只替换敏感值、保留诊断上下文，以及长 Bearer/已知敏感值跨流块的权威安全切点。

### 关键决策

- 原始文本先按完整逻辑行整体 sanitizer，再把安全文本拆为公开 entry；超长无换行流保留 256 字节安全
  尾部，且切点不得穿过敏感匹配或终端控制序列。
- 日志持久化失败切换为易失模式并只发一次 warning，不改变本地门禁退出码、取消或进程错误分类。
- 日志分页扫描放入 `spawn_blocking`；前端不能提交 repository/log path 或 limit，`GetLogs` 只接受当前
  `SessionContext` 的 session ID 与可选 `beforeSequence`。
- Windows 原子写入会先 rename 现有目标；故障测试必须用禁止 delete sharing 的文件句柄稳定触发 rename
  失败，空目录不能作为可靠写入失败夹具。
- latest-page 响应只可更新服务端权威前缀；请求期间已经进入当前 session 的更大实时 sequence 必须按
  sequence 合并，并让 total/truncated/warning 元数据保持单调，避免刷新造成日志倒退。
- `load_page` 是只读可信前缀解析器，不能为恢复继续追加而原子重写文件；只有 resume/publish 的 `open()`
  拥有修复后缀并恢复 sequence 的写职责，避免实时 append 与分页修复竞争。
- 流式 sanitizer 的安全切点由 core `safe_log` 的同一组权威正则决定；敏感值跨切点时允许延迟该段输出，
  内存仍受既有 1 MiB 进程输出上限约束，不能为了固定 64 KiB 刷新而泄漏 continuation。
- 普通 `stepLog` 继续只携带单条 entry；只有发生持久化压缩时才附带最多 2,000 条的权威最新页，避免
  每条日志重复传页，同时让前端无需猜测 marker 复用 sequence 后哪些旧记录已被淘汰。

### 验证证据

- `release_log` integration：15/15 通过。
- `infrastructure::release_log::tests`：7/7 通过。
- `local_verification` integration：7 通过、0 失败、1 个既有完整项目探针保持 ignored。
- core `codex_process` 专项：9/9 通过。
- 切片 3 格式化后：release-console lib 25/25、release_orchestrator 14/14、github_release 17/17 通过。
- 首次加入 direct `regex` 依赖后一次全目标重链在 304 秒超时；产物时间持续更新且无遗留进程，限定
  `--lib`/目标缓存后的重跑通过，未把该次超时计为成功。
- 切片 4 格式化后：release-console lib 32/32、commands 4/4、release_state 9/9、release_phase 4/4 通过。
- 一次初始化失败回归首次 19/20：空目录被 `atomic_write` 成功 rename，未触发目标分支；改用禁止
  delete sharing 的锁定文件后专项与完整 lib 通过，未把错误夹具的首次运行计为成功。
- 切片 5：`tauri.test.ts` 6/6、`useReleaseSession.test.ts` 10/10 通过；其中 100,000 条实时日志用例
  保持一页 2,000 条且约 1.2 秒完成；发布控制台 `vue-tsc` 退出 0。
- 切片 6：latest-page/实时竞态回归 1/1 通过；typed client、composable、日志面板、步骤详情和 App
  五文件专项 39/39 通过。
- 最新 `vue-tsc` 首次因 `App.test.ts` 对强类型 `getComponent()` 结果调用不存在的 `exists()` 而退出 2；
  改用同文件既有 `findComponent().exists()` 模式后退出 0，随后 `App.test.ts` 17/17 通过。
- 发布控制台完整 Vitest：17 个测试文件、87/87 通过；本轮发布控制台 typecheck 退出 0。
- 浏览器 900x620：页面 `scrollWidth/clientWidth=900/900`、0 个越界元素，日志面板
  `top=434/bottom=620`，时间线与工作区分别可滚动；三个可见日志按钮高度均为 32px。
- 浏览器 600x760：页面 `600/600`、0 个越界元素，上方单列容器 `scrollHeight=1786`；滚动到
  `scrollTop=540` 后页面 `scrollY=0`、日志面板仍为 `top=532/bottom=760`。临时系统深色主题下日志正文
  对比度 15.36:1；恢复浅色与默认视口后浏览器 console warning/error 为 0。
- 发布控制台 Rust 包全套：147 项通过、0 失败、1 个既有完整项目探针按设计 ignored；首次全目标编译
  与链接 3 分 27 秒，总命令 4 分 41 秒，未把编译等待时间误报为测试执行时间。
- 成对安全 Relay 覆盖下完整 `npm run check` 退出 0、耗时 8 分 29 秒：路径预检为 true，两个受保护
  路径匹配均为 false；Trellis 8/8、主前端 60 文件 336/336、发布控制台 17 文件 87/87、Rust workspace
  399 项通过并保留 1 个既有 ignored，依赖图、fmt、Clippy 与三 crate doc-tests 同时通过。
- 审查 RED/GREEN：`load_page` 不可信尾部由 `InvalidLog` 转为 1/1 通过，application 分页字节不变由
  失败转为 1/1 通过，初始化 warning 顺序由失败转为 1/1 通过，权威失败 code 与工具路径回归由失败
  转为 1/1 通过，core safe-log 2/2、长 Bearer 与长已知值专项各 1/1 通过。
- AC5 实时截断 RED/GREEN：Rust 首次因 `StepLog` 没有 `page` 字段编译失败，Vue 首次仍显示已淘汰
  记录且 `truncated=false`；实现后 `release_log` integration 18/18、`useReleaseSession` 12/12、
  发布控制台 `vue-tsc` 退出 0。
- 修正后的提交前扫描只输出信号类别与文件名：高置信度 token、Authorization 值、Bearer 值、带认证
  代理、用户绝对路径和调试语句六类均无命中；关键词命中人工归属于脱敏实现、明确假 fixture 或任务
  契约。`git diff --check` 退出 0，仓库内无 `session.log.jsonl`，Git 未跟踪 `auth.json/providers.json`。
- 规范更新后的首次最终 `npm run check` 在成对安全临时覆盖下退出 101：路径预检均为 false，Trellis
  8/8、主前端 60 文件 337/337、发布控制台 17 文件 88/88 与 Rust 依赖图通过；Rust Clippy 因
  `finish_with_error_details` 有 8 个参数触发 `too_many_arguments`，尚未进入 workspace tests。
- 将相关 `step_id/code/message` 聚合为借用型 `ReleaseFailureDetails`，未添加 lint allow；release-console
  全目标 Clippy 退出 0。一次未限定 `--lib` 的过滤测试在 184 秒外层预算超时且无测试结果，确认无遗留
  cargo/rustc 进程后用 `--lib release_application` 精确重跑，20/20 通过。
- 同一对已验证安全 Relay 临时路径下最终 `npm run check` 退出 0、耗时 10 分 44.6 秒：路径匹配均为
  false；Trellis 8/8、主前端 60 文件 337/337、发布控制台 17 文件 88/88，Rust 依赖图、fmt、全
  workspace Clippy/tests 与三 crate doc-tests 均通过，保留 1 个既有完整项目探针 ignored。
- `npm run build:release-console` 退出 0、耗时 3 分 9.9 秒；源与交付 EXE 均为 12,939,264 字节，
  最后写入时间 `2026-08-03T22:30:30.7087942+08:00`，SHA-256 均为
  `3A5E22147E15F25F7ED95345AF6067FDA9210FF271633D9CE1082ACF8412E1EC`；后续竞态修复使该产物成为
  历史证据，必须重新构建后才能作为最终交付哈希。
- 最终跨层审查发现压缩会合法降低计数/字节，而压缩前发出的分页响应仍按单调元数据合并。新增 RED
  稳定复现迟到响应把 marker 与 8,000 字节/80 条替换回旧记录及 10,000/100；接受权威压缩页时使
  在途日志请求 generation 失效后转绿。发布控制台 typecheck 退出 0、完整 17 文件 89/89 通过。
- 竞态修复后的最终 `npm run check` 在同一对安全临时路径下退出 0、耗时 4 分 51.8 秒：路径匹配为
  false/false；Trellis 8/8、主前端 60 文件 338/338、发布控制台 17 文件 89/89，Rust 依赖图、fmt、
  workspace Clippy/tests 与三 crate doc-tests 全部通过，保留 1 个既有 ignored 探针。
- 当前源码的最终 `npm run build:release-console` 退出 0、耗时 2 分 48.1 秒；源/交付 EXE 均为
  12,939,264 字节，时间 `2026-08-03T22:54:26.9135207+08:00`，SHA-256 均为
  `4E4364E82C570BC5D77DA33D11D8B3AD69A7244226CF590F8EE59E087396C549`。

### 下一步

- 完成最终 Trellis 审查、精确暂存、提交、归档、会话日志与普通 push。

### 尚未解决的问题

- 提交、归档和 push 尚未执行。

## 1. 文件地图

### 1.1 新建文件

| 文件 | 单一职责 |
|---|---|
| `tools/release-console/src-tauri/src/services/release_log.rs` | policy、JSONL store、recorder 与 progress sink |
| `tools/release-console/src-tauri/src/infrastructure/release_log.rs` | 原始输出 sanitizer、增量 decoder 与 process sink |
| `tools/release-console/src-tauri/tests/release_log.rs` | 真实临时 Git dir 下的轮换、追加、分页、上限、损坏与会话隔离集成测试 |
| `tools/release-console/src/components/release/ReleaseLogPanel.vue` | 一页日志展示、跟随、分页、复制和可访问性 |
| `tools/release-console/src/components/release/ReleaseLogPanel.test.ts` | 日志面板公开 props/emits 与用户行为测试 |

### 1.2 修改文件

| 文件 | 变化 |
|---|---|
| `tools/release-console/src-tauri/src/models.rs` | 增加 log entry/page/snapshot DTO，调整 `StepLog` payload |
| `tools/release-console/src-tauri/src/services/mod.rs` | 注册 `release_log` 模块 |
| `tools/release-console/src-tauri/src/infrastructure/mod.rs` | 注册 `release_log` 模块 |
| `tools/release-console/src-tauri/src/services/local_verification.rs` | 把固定命令 ID 与 progress/output 边界贯通 |
| `tools/release-console/src-tauri/src/infrastructure/local_verification.rs` | 为每个 npm/Cargo 命令创建并完成 process log sink |
| `tools/release-console/src-tauri/src/services/release_orchestrator.rs` | 注入 progress sink，记录本地/Git/GitHub 结构化阶段日志与远端状态变化 |
| `tools/release-console/src-tauri/src/services/github_release.rs` | 为 cleanup Run 轮询记录状态投影变化与 5 分钟心跳 |
| `tools/release-console/src-tauri/src/services/release_application.rs` | 初始化 recorder，维持失败顺序，加载 snapshot/page，日志 I/O 非致命警告 |
| `tools/release-console/src-tauri/src/app_state.rs` | 增加分页 request/response 变体 |
| `tools/release-console/src-tauri/src/commands.rs` | 增加 `get_release_logs` 并调整 `get_release_session` 返回类型 |
| `tools/release-console/src-tauri/src/lib.rs` | 注册新 Tauri command |
| `tools/release-console/src-tauri/tests/local_verification.rs` | 流式输出、完成前事件、失败 flush 与安全诊断测试 |
| `tools/release-console/src-tauri/tests/release_orchestrator.rs` | 全阶段 progress、失败顺序和发布语义回归 |
| `tools/release-console/src-tauri/tests/github_release.rs` | cleanup Run tracker wiring 与既有清理语义回归 |
| `tools/release-console/src-tauri/tests/release_state.rs` | 证明 session schema v1 与独立日志兼容 |
| `tools/release-console/src-tauri/tests/commands.rs` | snapshot/page command 与完整 DTO 透传 |
| `tools/release-console/src/types/release.ts` | Rust 同构 TypeScript DTO |
| `tools/release-console/src/services/tauri.ts` | typed snapshot/page command 和实时 log event |
| `tools/release-console/src/services/tauri.test.ts` | command 名、camelCase 参数、返回解包和 Channel 测试 |
| `tools/release-console/src/composables/useReleaseSession.ts` | sequence reducer、最新/历史页、游标、未读和旧 channel 失效 |
| `tools/release-console/src/composables/useReleaseSession.test.ts` | 实时/恢复去重、分页、切仓库和一页上限测试 |
| `tools/release-console/src/components/release/ReleaseStepDetails.vue` | 移除日志展示，只保留会话事实 |
| `tools/release-console/src/components/release/ReleaseStepDetails.test.ts` | 更新单一职责断言 |
| `tools/release-console/src/App.vue` | 三行固定布局并组合 `ReleaseLogPanel` |
| `tools/release-console/src/App.test.ts` | 底部日志组合、动作转发和布局契约 |
| `.trellis/spec/release/publishing.md` | 安全诊断日志、JSONL、分页、上限、恢复与验证契约 |
| `.trellis/spec/backend/error-and-logging.md` | 发布日志保留诊断上下文且不泄漏秘密的通用边界 |

现有实现若证明某个测试更适合模块单元测试，可以不创建重复集成用例，但不得省略该公开行为或把
store/process/component 本身替换为 mock。

## 2. 行为切片 1：日志 DTO 与持久化 Store

### 2.1 RED

- [x] 在 `tests/release_log.rs` 先写以下失败测试：
  - 新会话原子替换旧 session ID 的日志；
  - 打开同一 session 恢复最后 sequence、字节数、记录数和截断状态，继续 append 不覆盖或重复编号；
  - 打开带不完整尾行或中间损坏的文件时原子保留有效前缀并返回 recovery warning，随后追加仍可读取；
  - append 后最新页按 sequence 升序返回，`nextBeforeSequence` 可读取更早页；
  - 缺失文件返回空页，末行不完整返回有效前缀和 warning；
  - 中间损坏、schema 不支持、session 不匹配和 sequence 非递增时停止信任后续行；
  - 使用小 `ReleaseLogPolicy` 分别触发 bytes、entries、entry bytes 上限，保留错误/阶段/最新输出并
    写入截断记录；压缩标记复用最大被淘汰 sequence，后续 recorder sequence 仍严格递增；
  - 直接交给 store 的 `Authorization: Bearer test-key-store-not-real` 在序列化前经基础纵深脱敏；
    `ReleaseLogEntry` 的自定义 Debug 不输出 `message` 正文。
- [x] 在 `models.rs` 单元测试增加 log entry/page/snapshot camelCase 序列化形状。
- [x] 运行：

  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test release_log
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console models
  ```

- [x] 预期：因 `ReleaseLogStore`、policy 和 DTO 尚不存在而编译失败；确认失败来自目标行为缺失。

### 2.2 GREEN

- [x] 在 `models.rs` 增加 `ReleaseLogSource`、`ReleaseLogLevel`、`ReleaseLogEntry`、
  `ReleaseLogPage`、`ReleaseSessionSnapshot`，把事件改为
  `ReleaseEvent::StepLog { entry: ReleaseLogEntry }`，并为 entry 实现不包含 `message` 正文的自定义
  `Debug`。
- [x] 在 `release_log.rs` 实现版本 1 JSONL envelope、`ReleaseLogPolicy::default()`、
  `ReleaseLogStore::{initialize, open, append, load_page}`；`open` 必须从有效前缀恢复计数和最后 sequence。
- [x] recorder 是生产写入唯一入口并在构造 entry 前执行基础 `safe_log::redact`；store 在序列化前重复
  基础脱敏作为纵深防御，不在 store 复制路径/ANSI/代理专用 sanitizer。
- [x] 单条序列化结果超过 1 MiB 时 recorder 在 UTF-8 边界截断并写入 warning；压缩标记复用最大被
  淘汰 sequence，store 不分配后续实时 sequence，且任何记录都不能突破硬上限。
- [x] 默认 policy 精确为 50 MiB、100,000、1 MiB、64 KiB、2,000；测试使用同一生产类型的小值。
- [x] rotation/compaction 使用 `atomic_write`；普通 append 写入单行并 flush，所有尺寸按序列化字节计算。
- [x] 重跑专项测试，预期全部通过；随后运行 `cargo fmt` 覆盖相关 Rust 文件。

### 2.3 重构检查

- [x] Store 不依赖 Tauri、UI event 或 orchestrator；文件路径只能由 git dir 构造。
- [x] 生产代码没有只供测试调用的方法；policy 注入是正常依赖，不绕过真实 store 行为。
- [x] `session.json` schema version、字段和原子保存测试保持不变。

## 3. 行为切片 2：本地实时输出与安全处理

### 3.1 RED

- [x] 在 `infrastructure/release_log.rs` 模块测试先定义并验证希望得到的 sanitizer/process sink 行为：
  - UTF-8 字符跨 chunk 不产生重复替换符；
  - stdout/stderr 保持来源；CRLF 规范化且 ANSI 被移除；
  - repository root 替换为 `<repo>`，文件名、行列、测试名和错误正文保留；
  - `test-key-stream-not-real`、Bearer、GitHub token、Authorization、query key、代理 URL 和敏感环境值
    不出现在 entry、JSON 或 Debug；
  - 64 KiB 分块和超长无换行输入产生连续记录，单条不越过 1 MiB；
  - `finish()` 刷新两个流尾部且重复调用不重复日志。
- [x] 在 `tests/local_verification.rs` 增加真实临时 PowerShell 慢进程：先 flush 第一段输出，再等待后输出；
  测试在 process future 完成前收到第一条安全日志。
- [x] 增加非零退出、超时/取消或 backend error 中至少一个失败路径，断言输出尾部先于失败证据。
- [x] 运行：

  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console infrastructure::release_log
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test local_verification
  ```

- [x] 预期：现有 backend 仍向 runner 传 `None`，测试因没有实时日志而失败。

### 3.2 GREEN

- [x] 在 `infrastructure/release_log.rs` 实现 `ReleaseLogSanitizer`、`ReleaseProcessLogSink`，在
  `services/release_log.rs` 实现 `ReleaseLogRecorder`；recorder 先尝试 store，再 best-effort 发送
  `StepLog`，持久化失败只发送一次易失 warning。
- [x] sanitizer 复用 core `safe_log::redact`，不复制第二套现有 JSON/Bearer/GitHub token 正则。
- [x] `ProcessLocalVerificationBackend` 接收 recorder/progress 依赖，为每个 command 创建 sink，并在
  runner 的 Ok/Err/Cancelled 所有出口调用 `finish()`。
- [x] 重跑 RED 命令，预期通过；再运行 core `codex_process` 相关测试，确认 Job/超时/输出上限不回归。

### 3.3 重构检查

- [x] `on_output` 不写日志原始 bytes 到 Debug/tracing，不记录 argv 或 environment。
- [x] recorder 的锁只保护 sequence、store 和一次 warning 状态，不跨 await 持锁。
- [x] 日志 I/O 错误不能替换 `LocalVerificationFailure`、退出码或取消分类。

## 4. 行为切片 3：全流程结构化进度

### 4.1 RED

- [x] 在 `tests/release_orchestrator.rs` 使用真实 recorder/store 和既有 fake backend 增加测试：
  - 成功的 `run_to_pushed` 产生 candidate、四个本地命令、sourceAudit、commitPush 的开始/完成日志；
  - 本地命令失败先记录输出/错误，再执行现有回滚和权威 failure，不记录后续步骤；
  - commit/push 失败保留现有 committed/rollback 语义，日志只含安全 SHA 摘要和稳定错误；
  - Draft、publish、online verification、cleanup 的成功/失败和 warning 都有对应 step ID。
- [x] 在 `release_orchestrator.rs` 模块测试中用首个响应即完成的 fake `GhBackend` 驱动生产
  `GithubRemoteBackend`，验证发布 Run 的安全投影确实交给 tracker 并记录，测试不等待真实轮询。
- [x] 在 `tests/github_release.rs` 使用首个响应即完成的 fake `GhBackend` 验证 cleanup Run 的发现/最终
  安全投影确实交给 tracker，且清理失败仍只是既有 warning 语义。
- [x] 在 `services/release_log.rs` 为共享 `ReleaseRunProgressTracker` 写模块测试，直接传入可控时间验证
  status/job/step 变化、未变化的 5 秒轮询静默和 5 分钟心跳；不得让测试真实等待。
- [x] 运行：

  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console run_progress_tracker
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console github_remote_backend
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test release_orchestrator
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test github_release
  ```

- [x] 预期：现有 orchestrator/cleanup 轮询没有 progress sink，新增日志断言失败，但既有发布状态与
  cleanup 结果断言仍通过。

### 4.2 GREEN

- [x] 定义 `ReleaseProgressSink` started/log/completed/failed 行为和 no-op 实现；
  `ReleaseOrchestrator::new()` 使用 no-op，生产 `with_progress` 注入 recorder。
- [x] 在 `services/release_log.rs` 实现 `ReleaseRunProgressTracker`；输入安全投影和单调时间，只返回
  change/heartbeat/silent 决策，不拥有轮询、睡眠或发布状态。
- [x] 在本地、candidate、source audit、commit/push、remote、Draft、publish、verify、cleanup 的真实边界
  记录结构化消息，不从本地化 stderr 推导机器状态。
- [x] `GithubRemoteBackend::wait_for_run` 比较发布 Run 的上次安全状态投影；
  `GithubReleaseService::monitor_cleanup` 对 cleanup Run 使用同一投影/clock 规则。两处只记录变化或
  5 分钟心跳，不暴露 gh 原始 JSON。
- [x] 重跑专项并确保既有 `release_phase`、`git_release`、`github_release` 测试保持绿色。

### 4.3 重构检查

- [x] step ID 复用现有时间线/失败 ID；显示标签仍由 Vue 决定，Rust 不保存第二套本地化映射。
- [x] Git/gh 原始 stdout/stderr、request args、代理 URL 和机器 JSON 没有进入 recorder。
- [x] progress sink 失败不改变 orchestrator 返回值或 state store transition。

## 5. 行为切片 4：Application、command 与分页 IPC

### 5.1 RED

- [x] 在 `release_application.rs` 单元测试和 `tests/commands.rs` 增加：
  - start 初始化/轮换 log store，并在 channel 断开后继续持久化；
  - resume/publish 打开同一 log store，恢复最后 sequence 后继续追加且不覆盖已有日志；
  - `GetSession` 返回 session + 最新 2,000 条 + 总数/字节/截断/warning；
  - `GetLogs` 只允许当前 `SessionContext` 的 session ID，并按 before sequence 返回更早页；
  - 没有日志文件的旧 session 返回空页；损坏日志 warning 不阻止 session 恢复；
  - 日志 initialize/append 失败只产生 warning，不把成功发布改成 failed；
  - 失败顺序为输出尾部、持久化 error、权威 SessionUpdated、StepFailed；
  - Tauri command 只执行一次 application request，参数字段为 `sessionId/beforeSequence`。
- [x] 运行：

  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test commands
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console release_application
  ```

- [x] 预期：snapshot/page request 尚不存在或返回类型不匹配而失败。

### 5.2 GREEN

- [x] 增加 `ApplicationRequest::GetLogs` 和对应 response；把 `GetSession` response 改为 optional snapshot。
- [x] `SystemReleaseApplication` 保存 session context 后构造 store/recorder：start 调用 `initialize`，
  resume/publish/load 调用 `open` 并恢复最后 sequence；所有管线注入同一 recorder，分页读取使用
  blocking 文件边界。
- [x] `commands.rs` 增加 `get_release_logs`，`lib.rs` 注册命令，保持 `CommandResult<T>` 错误契约。
- [x] 失败日志统一走 recorder，application 只在权威 `SessionUpdated` 后发送 `StepFailed`。
- [x] 重跑专项、`release_state` 和 `release_phase` 测试。

### 5.3 重构检查

- [x] command 仍只做参数转换、单次 application 调用和 response 映射，不直接打开日志文件。
- [x] 任意 repository path、日志路径、limit 或排序参数不能从前端进入新 command。
- [x] 当前会话失败日志与 `ReleaseSession.failure` 一致，但日志不能替代权威 failure。

## 6. 行为切片 5：typed client 与 Vue 状态

### 6.1 RED

- [x] 在 `services/tauri.test.ts` 添加完整 snapshot/page fixture，验证：
  - `get_release_session` 解包 snapshot；
  - `get_release_logs` 使用精确 camelCase 参数；
  - `{ kind: 'stepLog', entry }` channel 透传完整 entry，不在 service 二次解析。
- [x] 在 `useReleaseSession.test.ts` 添加：
  - load 恢复最新页；实时 entry 按 `sessionId+sequence` 去重并追加；
  - 最新页超过 2,000 时只裁剪显示数组，total metadata 继续增长；
  - 10 万个 `stepLog` 只经过有界日志 reducer，通用 `events` 数组不接收这些记录；
  - load earlier 显示历史页且实时事件不覆盖当前阅读页；返回最新恢复跟随；
  - 新 start 轮换页面，切仓库/load 使旧 channel 和旧分页响应失效；
  - warning、truncated、hasEarlier、unread 和失败状态保留。
- [x] 运行：

  ```powershell
  npm run test --workspace @codex-relay/release-console -- src/services/tauri.test.ts src/composables/useReleaseSession.test.ts
  ```

- [x] 预期：TypeScript DTO、client 方法和 composable 状态尚不存在，测试按目标缺失失败。

### 6.2 GREEN

- [x] 同步 TypeScript `ReleaseLogEntry`、`ReleaseLogPage`、`ReleaseSessionSnapshot` 和
  `{ kind: 'stepLog'; entry: ReleaseLogEntry }` 事件形状。
- [x] typed client 增加 `getReleaseLogs`，调整 `getReleaseSession`；只有 tauri service 导入 `invoke/Channel`。
- [x] `useReleaseSession` 增加 readonly `logPage`、`logViewMode: 'latest' | 'history'`、
  `unreadLogCount`、`logRequestPending`、`logError`；独立日志请求序列和现有 channel generation 同时
  防止旧响应覆盖。动作名统一为 `loadEarlierLogs`、`refreshLogPage`、`returnToLatestLogs`。
- [x] event reducer 收到 `stepLog` 时只更新有界日志状态并立即返回，不再把同一 entry 追加到通用
  `events` 数组；started/completed/failed/session 等低频事件继续维持既有时间线行为。
- [x] 重跑专项与 `npm run typecheck --workspace @codex-relay/release-console`。

### 6.3 重构检查

- [x] 组件不能直接修改 log refs；派生范围/按钮状态使用 computed，不用 watcher 复制真相。
- [x] 不把完整 50 MiB 日志放入 localStorage、全局 store 或 `ReleaseSession`。
- [x] 同一个 sequence 的恢复和实时事件只保留一份。

## 7. 行为切片 6：固定日志面板与响应式布局

### 7.1 RED

- [x] 新建 `ReleaseLogPanel.test.ts`，挂载真实组件并验证：
  - 空态和一页 entry 的时间、step ID、source、换行文本；
  - `hasEarlier`、历史页、最新页、warning、truncated、失败和未读状态的可见文本；
  - “更早”“更新”“返回最新”分别发出 `load-earlier`、`refresh-log-page`、`return-to-latest` typed event，
    busy 时禁用且原因可见；
  - 向上滚动暂停跟随，新 entry 到达不抢夺历史阅读，返回最新滚到底部；
  - 复制当前安全页，clipboard 失败显示安全错误而不影响会话；
  - `aria-label=发布诊断日志`、`tabindex=0`，无 `v-html`。
- [x] 更新 `App.test.ts` 和 `ReleaseStepDetails.test.ts`：日志 panel 位于主 layout 之后，动作转发到
  composable，完整传入 `logPage/logViewMode/unreadLogCount/logRequestPending/logError/failure`，
  StepDetails 不再渲染日志。
- [x] 运行：

  ```powershell
  npm run test --workspace @codex-relay/release-console -- src/components/release/ReleaseLogPanel.test.ts src/components/release/ReleaseStepDetails.test.ts src/App.test.ts
  ```

- [x] 预期：组件不存在、App 仍把日志放在 workspace 内，测试按预期失败。

### 7.2 GREEN

- [x] 使用 `<script setup lang=\"ts\">` 创建 `ReleaseLogPanel.vue`；props 只读，分页 emits 显式类型化，
  props 精确对应 composable 的五项日志状态与 `ReleaseFailureEvidence | null`，DOM ref 使用 Vue 3.5
  `useTemplateRef`。
- [x] 日志用文本插值和 `white-space: pre-wrap`；跟随滚动在 `nextTick` 后执行，并在卸载/换页时不访问
  已销毁 DOM。
- [x] `App.vue` 改为三行 100dvh 网格；桌面保留左右独立滚动，窄窗口上方单列共同滚动；日志高度
  `clamp(180px, 30vh, 280px)`。
- [x] `ReleaseStepDetails.vue` 删除 log view、过滤 computed 和对应 CSS，只保留会话事实。
- [x] 重跑专项、发布控制台全部 Vitest 和 typecheck。

### 7.3 重构检查

- [x] 日志区为全宽工具带，不是嵌套卡片或浮层；动态文本不改变固定布局轨道。
- [x] 所有按钮文本在 600px 宽度下换行/收纳正常，点击目标符合 32/36px 项目基线。
- [x] 状态和失败不能只靠颜色，浅色/深色变量均有足够对比度。

## 8. 规范、集成与安全回归

- [x] 更新 `.trellis/spec/release/publishing.md`：替换“任何 stdout/stderr 都不得进入事件”的绝对表述，
  明确原始输出禁止、安全诊断允许、混合日志来源、JSONL schema、50 MiB/100,000/1 MiB、分页、
  截断、损坏与日志 I/O 非致命边界。
- [x] 更新 `.trellis/spec/backend/error-and-logging.md`：只替换敏感值、保留非敏感诊断上下文；禁止
  argv/environment/认证内容和第二套脱敏逻辑。
- [x] 运行所有直接相关 Rust 测试：

  ```powershell
  cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console
  ```

- [x] 运行发布控制台前端完整检查：

  ```powershell
  npm run typecheck --workspace @codex-relay/release-console
  npm run test --workspace @codex-relay/release-console
  ```

- [x] 使用 `rg -l` 扫描本次差异和 fixture 的高置信度密钥形态，只报告文件名并人工复核；确认没有
  真实值、完整认证文件、代理地址或用户路径进入 Git/测试输出。
- [x] 检查 `git diff --check`、`git status --short --ignored`，确认 `session.log.jsonl` 测试文件只在
  系统临时目录，未写入当前仓库 `.git`。

## 9. 完整验证与人工观察

### 9.1 成对安全 Relay 覆盖

- [x] 在系统临时目录创建本轮唯一 `codex-home` 与 `app-data`，验证解析后的路径不等于也不位于真实
  `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay`，再设置：

  ```powershell
  $env:CODEX_RELAY_CODEX_HOME = '<本轮已验证的临时 codex-home>'
  $env:CODEX_RELAY_APP_DATA_DIR = '<本轮已验证的临时 app-data>'
  npm run check
  ```

- [x] 记录真实退出码、测试文件/用例数量、首次失败和重试；不得用专项通过代替完整检查。

### 9.2 发布控制台构建

- [x] 运行：

  ```powershell
  npm run build:release-console
  ```

- [x] 枚举 `artifacts/release-console/CodexRelayReleaseConsole.exe` 及源 EXE 的实际路径、大小、最后写入
  时间和 SHA-256；只在命令退出 0 且文件可读取时报告构建成功。

### 9.3 浏览器布局验证

- [x] 启动 release-console 前端开发服务器到空闲 localhost 端口；清除该 origin 的 repository 偏好，
  避免触发真实仓库或 Tauri command。
- [x] 使用浏览器分别在 900x620 与 600x760 截图，验证：
  - 日志工具带位于视口底部且非空白；
  - 上方区域可滚动，日志区不遮挡操作；
  - 页面 `scrollWidth <= clientWidth`，按钮和文本不越界；
  - 浅色和系统深色主题均无不可读文字。
- [x] 动态日志、分页、失败和 2,000 条渲染上限以自动化组件/集成测试为证据，不用静态截图替代。

## 10. 验收映射

| 验收标准 | 实施/验证位置 |
|---|---|
| AC1 固定响应式布局 | 切片 6、9.3 |
| AC2 本地实时 + 全阶段结构化日志 | 切片 2、3 |
| AC3 失败顺序与发布语义 | 切片 2、3、4，既有 orchestrator/state 回归 |
| AC4 跨重启分页与隔离 | 切片 1、4、5 |
| AC5 三类上限与有界 IPC/DOM | 切片 1、5、6 |
| AC6 安全且保留上下文 | 切片 2、8，秘密/路径扫描 |
| AC7 交互与可访问性 | 切片 6、9.3 |
| AC8 全量回归与构建 | 8、9.1、9.2 |

## 11. 风险文件与回滚点

- `services/release_log.rs`：容量、损坏、sequence 与 recorder 基础脱敏边界集中点；任何越界都必须先
  停在该切片修复。
- `infrastructure/release_log.rs`：原始 bytes、路径、ANSI、代理和增量解码边界；不得绕过 recorder
  直接发送事件或写文件。
- `release_application.rs` / `release_orchestrator.rs`：不得让日志错误改变发布状态或重复管线。
- `github_release.rs`：cleanup Run 的轮询日志不得改变清理 warning 语义，也不得输出 gh 原始 JSON。
- `local_verification.rs`：不得阻塞 pipe 读取、丢失进程退出码或改变 Job Object 取消边界。
- `useReleaseSession.ts`：不得让旧 channel/page 覆盖新仓库，或把 50 MiB 全量复制到前端。
- `App.vue`：900x620 和窄窗口必须同时验证，不能用日志浮层掩盖网格问题。

若某切片 GREEN 无法保持既有发布状态测试通过，回滚该切片的生产改动并保留红测与真实失败说明，
返回 `design.md` 修订边界。不得删除 `session.json`、候选事务、远端对象或用户数据来恢复测试。

## 12. 完成与交付

- [x] `trellis-check` 通过规范符合性、专项、全量、跨层和秘密/路径检查。
- [x] 按本轮实际学习更新长期规范，避免把任务临时细节写成通用规则。
- [ ] 精确暂存本任务代码、测试、规范和任务材料，排除
  `.trellis/tasks/08-02-release-console-rust-gate-101/` 等无关改动。
- [ ] 使用简体中文 Conventional Commit 提交实现；归档当前任务并记录会话日志后提交收尾材料。
- [ ] 将当前分支普通非强制 push 到已配置远程跟踪分支；验证远端 tracking ref 与本地 `HEAD` 一致，
  报告提交哈希、远端分支、测试/构建证据和所有未执行项。
- [x] 不推送 Tag，不执行发布、签名、安装、升级、卸载或远端清理。
