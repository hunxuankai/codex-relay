# 实施计划

## 顺序与 TDD 行为切片

1. **开发 watcher 冲突门禁** `[completed 2026-07-23]`
   - 先添加合成命令行测试，覆盖普通 `tauri dev` 失败、`--no-watch` 通过、无进程通过。
   - 实现 `scripts/check-rust-dev-environment.ps1`，只读进程命令行，不终止任何用户进程。
   - 在 `package.json` 为三个快速 Rust 测试入口添加 npm pre-script，并显式固定 target-dir。

2. **安全无 watcher 开发入口** `[completed 2026-07-23]`
   - 先为 `prepare-dev-data.ps1` 增加不启动真实应用的参数观察测试。
   - 增加 `-NoRustWatch`，保持假配置、假密钥和成对覆盖逻辑唯一，不复制第二套准备脚本。
   - 添加 `dev:safe:no-watch` package script，验证最终 Tauri 参数包含 `--no-watch`。

3. **TLS 依赖图收敛** `[completed 2026-07-23]`
   - 先新增构建图失败检查，证明当前 `aws-lc-sys` 存在且检查为红。
   - 修改 `Cargo.toml` 的 reqwest/rustls features，更新锁文件。
   - 运行构建图检查，确认 `aws-lc-sys` 消失、ring 明确启用；不得手工只删锁文件条目。

4. **专项行为与兼容性验证** `[completed 2026-07-23]`
   - 运行 Provider HTTP、gateway、availability 服务专项测试，确认客户端构造、代理、JSON、SSE
     和错误分类不变。
   - 运行 `path_safety` 与 `provider_workflow`，证明新入口/依赖不触及真实路径或业务流程。
   - 运行 `cargo check`/Clippy 时保留真实失败；当前用户 `tauri dev` 进程不由本任务终止。

5. **文档、规范与完整门禁**
   - 更新 README 开发/测试章节，以及 testing 规范中的快速循环与完整门禁边界。
   - 将构建图检查纳入 `check:rust`，避免以后重新引入重复 TLS provider。
   - 运行 `npm run check`、前端构建和 Git/密钥/路径审计。

6. **前后基准与 core crate 决策**
   - 使用固定默认 target 测缓存命中；在安全临时 target 测冷构建并清理。
   - 记录命令、耗时、并发进程和产物规模；与本任务 PRD 的基线比较。
   - 若典型 Provider 行为切片仍稳定超过 20 秒，只记录并建议后续 core crate 任务，不在本任务
     临时实施架构拆分。

## 风险文件与回滚点

- `scripts/prepare-dev-data.ps1`：必须继续先设置安全双覆盖；任何无法证明隔离的实现立即回滚。
- `scripts/check-rust-dev-environment.ps1`：只能读进程，不能调用 `Stop-Process`/`taskkill`。
- `package.json`：快速入口不能替代或削弱现有完整 `check:rust`。
- `src-tauri/Cargo.toml` / `Cargo.lock`：TLS provider 变化必须通过依赖图、Provider HTTP 测试和完整
  构建门禁；失败时只回滚该切片。
- `.trellis/spec/testing/verification.md` / README：不得建议绕过真实路径保护或删除缓存作为日常优化。

## `task.py start` 前检查

- [ ] PRD 已完成收敛，无开放产品问题。
- [ ] design/implement 明确不终止用户进程、不触碰真实 Codex/Relay 路径。
- [ ] 用户已明确要求开始优化。
- [ ] inline 模式实施，不创建子 Agent 或重复工作流。
- [ ] 实施前加载 `trellis-before-dev` 与相关 backend/security/testing 规范。

## 验证命令

```powershell
npm run test:rust-dev-guard
npm run check:rust:deps
npm run test:rust:lib -- provider_http
npm run test:rust:lib -- codex_gateway
npm run test:rust:lib -- provider_availability_service
npm run test:rust:path-safety
npm run test:rust:provider-workflow
npm run check
npm run build:frontend
git diff --check
```

## 当前进度

前 4 个行为切片及文档/规范更新已完成。完整门禁曾两次暴露真实问题，目前正在完成
Codex gateway 间歇测试的稳定性回归；稳定后再进入最终完整检查与冷构建基准。

## 已完成

- 新增 `scripts/check-rust-dev-environment.ps1`，只读 Tauri CLI 进程命令行；普通 watcher 返回 2，
  `--no-watch` 和无进程返回 0，不终止或修改任何进程。
- 新增 `scripts/tests/rust-dev-scripts.tests.ps1`，以合成命令行覆盖三种门禁行为。
- 新增 `test:rust:lib`、`test:rust:path-safety`、`test:rust:provider-workflow`，统一通过 npm
  pre-script 运行门禁，并显式复用 `src-tauri/target`。
- 扩展 `prepare-dev-data.ps1` 的唯一安全数据准备路径，新增 `-NoRustWatch` 与不启动应用的
  `-DryRun`；默认入口仍调用 `npm.cmd run dev`，无 watcher 入口调用
  `npm.cmd run dev -- --no-watch`。
- 新增 `npm run dev:safe:no-watch`，没有增加绕过双路径覆盖的普通 no-watch 入口。
- 新增依赖图检查及合成测试；`check:rust` 会先拒绝 `aws-lc-sys` 或缺少显式 `ring` provider。
- `reqwest` 关闭默认 feature，保留 charset、HTTP/2、JSON、system-proxy 和 platform verifier；
  显式启用 Rustls `ring`，锁文件移除 `aws-lc`、CMake 及其专属依赖。
- 新增共享 `ensure_ring_crypto_provider`，在应用启动、Provider HTTP 客户端和 Codex gateway
  客户端构造前幂等安装 provider，避免 `rustls-no-provider` 运行时 panic。

## 验证证据

- 红测：门禁脚本缺失时测试失败，watcher 场景期望退出 2、实际无法执行脚本。
- 第 2 切片红测：旧脚本未声明 dry-run 参数并继续启动开发进程；测试进程树已按根 PID 清理，
  未终止其他用户进程。
- `npm run test:rust-dev-guard`：6 tests passed，覆盖门禁三场景、安全入口默认/no-watch 参数和
  package script 固定 target。
- 依赖图红测：真实 `cargo tree` 检出 `aws-lc-sys`；修改 feature 后
  `npm run check:rust:deps` 通过。
- TLS 首次绿测前，Provider HTTP 6 项中 5 项因“未安装 crypto provider”panic；添加共享安装
  边界后 `npm run test:rust:lib -- provider_http` 为 6/6 通过，实际根 crate 编译 21.78 秒。
- `npm run test:rust:lib -- codex_gateway`：3/3 通过，缓存命中编译 0.93 秒。
- `npm run test:rust:lib -- provider_availability_service`：8/8 通过，缓存命中编译 0.96 秒。
- `npm run test:rust:path-safety`：3/3 通过；TLS 图变化后的首次 integration target 编译
  1 分 33 秒，测试 0.71 秒。
- `npm run test:rust:provider-workflow`：1/1 通过；复用普通 lib 后编译 8.09 秒，测试 0.51 秒。
- `npm run test:rust:lib`：170/170 通过；格式化引起的根测试目标重建 13.72 秒，测试 4.32 秒，
  整个 npm 入口 20.70 秒。
- 第一次 `npm run check` 暴露 `src/release-config.test.ts` 仍断言旧的单字符串 npm 调用；已改为
  验证参数数组入口，专项 12/12 通过。
- 第二次 `npm run check` 在 170 个 Rust lib 测试中间歇失败：
  `monitored_gateway_turns_upstream_tool_calls_into_safe_failure` 预期
  `CODEX_TOOL_CALL_BLOCKED`，偶发得到进程失败或预检不支持分类。不能用随后单次通过掩盖该失败。
- 压力诊断先后捕获 `ConnectionReset` 与 Windows 10053 `ConnectionAborted`；既出现于手写
  `TcpStream`，也出现于改用 `reqwest` 后的 `.send()`，证明根因是本机回环连接被间歇中止，
  不是 JSONL、工具集合或手写响应解析本身。
- 旧 preflight 在读取发生 `Io` 后仍回写 HTTP 400。假 Codex 会把 400 当成完整响应而不重试，
  服务器又不提交 `Io` 结果，最终固定等待 15 秒并错误显示 `CODEX_PREFLIGHT_FAILED`。
- preflight 现只对已完整解析但验证失败的请求返回 400；传输 `Io` 直接断开并继续监听。已验证
  请求的响应收尾错误不覆盖验证报告，最终仍需 Codex 进程输出同时通过，未放宽安全门禁。
- 协议处理拆为通用 `Read + Write` 并以内存流单测；真实 TCP 由两项 Provider Codex 服务测试
  保留。假 Codex 测试后端改用 `reqwest`，只对预检传输失败做最多 3 次短退避重试；真实
  Provider gateway/API 请求仍不重试。
- 最终结构新增两项确定性回归，分别证明传输 `Io` 不伪装成 400、完整非法请求仍 fail-closed；
  正常 gateway 与工具阻断真实 TCP 流程各 500/500 压力通过。
- 最终 Rust lib 测试二进制为 172 项；默认并发模式直接循环 20 轮全部通过，每轮约
  4.14–10.37 秒，未再出现错误分类漂移或 15 秒预检超时。
- 最终 `npm run check` 退出 0，总耗时 352 秒：Trellis 8/8、前端 139/139、Rust lib
  172/172、`path_safety` 3/3、`provider_workflow` 1/1。依赖图仍为 ring-only；Clippy
  检查约 15.22 秒，随后完整 `cargo test` 为生成全部测试目标重新编译/链接约 3 分钟。
- 优化后默认 target 缓存命中 `cargo test --lib --no-run` 为 4.12 秒；仅更新时间戳触发根
  crate 重建为 26.81 秒，未删除依赖缓存或修改源码内容。
- 独立冷 target 位于已验证的 `%TEMP%` 子目录，`cargo test --lib --no-run` 退出 0，耗时
  422.67 秒（约 7 分 3 秒）、3944 个文件、2.68 GiB；finally 再次验证路径后删除，删除后
  不再存在。相对基线超过 7 分 43 秒，至少减少约 40 秒（约 8.7%）。
- `npm run build:frontend` 退出 0（1717 modules，22.43 秒）；脚本合成测试 6/6 与 3/3
  通过；`git diff --check` 通过。高置信度 `sk-` 扫描为 0，Git 未跟踪认证文件、开发数据
  或 target/dist 产物（仅允许 `dev-data/.gitkeep`）。

## 关键决策

- 保留稳定错误分类与工具预检；仅测试假 Codex 的预检传输允许有界重试，产品真实 Provider
  请求和 gateway 不重试。
- 稳定性证明同时覆盖目标测试高次数循环和完整 Rust 测试二进制多轮循环，不能只跑一次过滤测试。
- 当前任务仍不拆分 crate；完成默认 target 与安全临时 target 基准后再根据 20 秒门禁做后续决策。
- 当前根 crate 重建 26.81 秒，仍超过 20 秒门禁；决定后续单独规划 `codex-relay-core` 拆分，
  本任务不扩大到跨 crate 迁移。

## 调试复盘：preflight 回环错误分类漂移

### 1. 根因类别

- **D：测试覆盖缺口 + E：隐式假设 + B：跨层契约**。
- 测试夹具隐式假设 Windows loopback 连接总会以正常 EOF 结束，并把传输 `Io` 与完整 HTTP
  请求验证失败合并处理。读取被本机中止后服务器仍返回 400，假 Codex 因此不触发网络重试，
  而服务器结果通道又没有提交验证结果，最终形成固定 15 秒超时和错误分类漂移。

### 2. 前几轮修复为何失败

1. 只增加响应 `flush/shutdown`：改善了关闭时序，但没有区分传输错误与协议拒绝。
2. 假客户端只按 `Content-Length` 读取：没有覆盖正常 gateway 的 close-delimited SSE，导致客户端
   过早关闭。
3. 客户端主动半关闭写端：在 Windows 并发回环下反而出现 10053 `ConnectionAborted`。
4. 增加服务器 ready 门禁：压力测试第 115 次仍失败，证伪“线程尚未进入 accept”假设。
5. 用 `reqwest` 替代手写客户端：仍捕获 `.send()` 传输中止，证明问题不在手写解析器。
6. 仅增加重试并让服务器继续监听：`Io` 仍被写成 400，客户端认为请求完整结束，服务器最终超时。

### 3. 预防机制

| 优先级 | 机制 | 具体行动 | 状态 |
|---|---|---|---|
| P0 | 架构 | 协议处理拆为通用 `Read + Write`，TCP 只负责超时与连接收尾 | DONE |
| P0 | 错误契约 | 传输 `Io` 不返回 400；完整验证失败才返回 400 | DONE |
| P0 | 双证据 | preflight 报告与 Codex JSONL 输出必须同时通过 | DONE |
| P0 | 回归测试 | 内存流确定性覆盖 `Io`/非法请求，真实 TCP 两场景各 500 次压力 | DONE |
| P1 | 规范 | 更新 Provider 可用性测试规范中的传输/协议边界 | DONE |

### 4. 系统性扩展

- 其他手写 loopback fixture 也应完整消费请求/响应，避免把 EOF、RST 与 HTTP 状态混为一谈。
- 高次数稳定性验证应先编译一次、直接循环测试二进制；不要每轮调用 Cargo 造成重复链接。
- 单体 lib 测试链接仍是主要增量成本；后续 `codex-relay-core` 任务应把不依赖 Tauri 的模块与
  测试迁出根 crate，以缩小典型 Provider 行为切片。

### 5. 知识沉淀

- [x] `.trellis/spec/backend/provider-availability-testing.md` 已更新。
- [x] 新增确定性回归和真实 TCP 压力证据。
- [x] 已记录后续 `codex-relay-core` 独立规划决定。
- [x] 本仓库不存在 `src/templates/markdown/spec/`，无需模板同步。

## 下一步

1. 审阅完整差异并将值得长期保留的回环测试边界更新到规范。
2. 执行提交前验证、提交与归档。

## 尚未解决的问题

- 当前任务无未解决实现问题；`codex-relay-core` 属于已决定的后续独立规划范围。
