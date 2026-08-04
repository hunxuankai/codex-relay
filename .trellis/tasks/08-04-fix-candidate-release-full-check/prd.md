# 修复候选发布完整检查失败

## 目标

修复 GitHub 发布 Run `30869666756` 在“运行完整检查”阶段暴露的 Windows 冷 Runner
进程测试时序缺陷，使候选发布在保持生产进程超时、安全终止和输出流契约不变的前提下，
稳定通过完整门禁并生成可审计的 `v0.5.0` Draft Release。

## 已确认事实

- Run `30869666756` 的 `release` Job 在 Step #7“运行完整检查”失败，Step #8“构建
  Draft Release”因此跳过；本轮没有生成 Draft。
- 唯一失败用例是
  `infrastructure::codex_process::tests::generic_runner_streams_output_before_process_completion`；
  core 其余 248 项测试通过。
- 失败用例已经收到首个 stdout 事件，随后 `SafeProcessRunner` 在硬编码 5 秒预算到期时返回
  `Timeout`；远端时间戳显示该用例耗时约 5.03 秒。
- 同一文件的既有 Windows 冷 Runner 进程树测试统一使用 30 秒的
  `PROCESS_TREE_TEST_TIMEOUT`，测试规范明确该预算只用于 CI 容差，不改变产品超时。
- 本机专项测试 1/1、完整 core 测试 249/249 通过，说明缺陷依赖冷 Runner 调度，不能以本机
  缓存命中替代远端失败证据。
- 上一轮 Run `30833079285` 的 Windows 8.3 路径缺陷已经由 `cfe932c` 修复；该用例在本轮
  已通过，本次是独立的后续失败。

## 需求

- 流式输出测试必须用可观察条件协调“首段输出已收到”和“允许子进程结束”，不能依赖固定
  500 毫秒 sleep 恰好给测试线程留下调度窗口。
- 真实 PowerShell 子进程及首个输出事件等待必须使用既有 30 秒测试级冷 Runner 预算；不得
  修改生产调用方传入的超时、取消、输出上限或 Job Object 行为。
- 测试必须继续证明首段 stdout 在进程完成前可见、最终 stdout 为完整的 `firstsecond`、
  stderr 为空且退出码为 0。
- 所有测试文件和协调标记只使用系统临时目录，不读取、写入或删除真实 `.codex` 与
  `%LOCALAPPDATA%\CodexRelay`，不记录认证信息或密钥。
- 修复后必须先通过专项重复运行和成对安全 Relay 覆盖下的本地完整 `npm run check`，再提交、
  普通 push，并使用新候选 SHA 触发唯一的 `v0.5.0` 发布 Run。
- 远端 Run 必须完整成功，且生成的 Draft Release 版本、目标 SHA、说明、NSIS、`.sig`、
  `latest.json`、大小、SHA-256 与签名关联通过审计；不得自动公开 Release。

## 验收标准

- [x] AC1：流式输出测试通过条件协调证明输出发生在进程完成前，不再依赖固定短 sleep。
- [x] AC2：`codex_process` 专项测试连续至少 3 次通过，生产进程超时和终止实现没有变化。
- [x] AC3：成对安全 Relay 临时覆盖下 `npm run check` 退出 0，Rust core 249 项及其余项目门禁通过。
- [ ] AC4：本次相关改动精确提交并普通 push 后，远端跟踪分支与本地 `HEAD` 一致。
- [ ] AC5：新候选 SHA 的 GitHub 发布 Run 完整成功，Step #7 与 Step #8 均为 success。
- [ ] AC6：生成的 `v0.5.0` Release 保持 Draft，目标提交和三类 updater 资产审计通过；未执行公开、
  安装、升级或卸载时不声称这些行为成功。
- [x] AC7：差异与秘密扫描确认没有真实密钥、认证文件或真实 Codex/Relay 用户数据进入改动和证据。

## 范围外事项

- 自动公开 `v0.5.0`、清理历史 Release 或推送 Tag。
- 修改发布控制台远端监控、GitHub workflow、Tauri 签名、安装、升级、卸载或数据保留行为。
- 用重跑相同失败提交、增加 workflow retry 或放宽产品错误断言掩盖测试缺陷。
