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
- 修复提交 `86c0d92` 的新 Run `30873762688` 已证明 core 249 项全部通过、原流式测试通过；
  随后唯一失败转移到 `tools/release-console/src-tauri/tests/local_verification.rs:357`，同样是
  延迟输出测试的 10 秒固定完成等待。
- 修复提交 `ebf0d69` 的 Run `30880121507` 已完整成功，Step #7 与 Step #8 均为 success，并生成
  Draft Release `364638762`；目标提交、说明和三个 updater 资产的实际大小与 SHA-256 已核对。
- GitHub 在 Draft 阶段尚未创建 `refs/tags/v0.5.0`，真实 tag API 返回 404；现有控制台却在 Draft
  审计中无条件查询 tag，因此即使 workflow 成功仍会返回 `GITHUB_BACKEND_FAILED`。
- Windows Actions/GitHub 把 LF 且带末尾换行的候选说明保存为 CRLF 且无末尾换行；正文语义不变，
  但现有原始字符串比较会返回 `GITHUB_DRAFT_AUDIT_FAILED`。
- 上一轮 Run `30833079285` 的 Windows 8.3 路径缺陷已经由 `cfe932c` 修复；该用例在本轮
  已通过，本次是独立的后续失败。

## 需求

- core 与 release-console 的流式输出测试必须用可观察条件协调“首段输出已收到”和“允许子进程
  结束”，不能依赖固定 sleep 恰好给测试线程留下调度窗口。
- 真实 PowerShell 子进程及首个输出事件等待必须使用既有 30 秒测试级冷 Runner 预算；不得
  修改生产调用方传入的超时、取消、输出上限或 Job Object 行为。
- release-console 集成测试的子脚本必须有短于测试预算的自截止，避免外层断言失败时遗留生产
  `LOCAL_COMMAND_TIMEOUT`（2 小时）命令。
- 测试必须继续证明首段 stdout 在进程完成前可见、最终 stdout 为完整的 `firstsecond`、
  stderr 为空且退出码为 0。
- Draft 审计必须用已严格比对的 `target_commitish` 绑定候选 SHA，不得在正式公开前要求 Git tag ref；
  公开后的在线复核仍必须验证 tag ref 类型和 SHA。
- Release 正文与 `latest.json.notes` 只允许 CRLF/LF 和 workflow 末尾 `TrimEnd()` 差异；任何正文、
  段落、版本或内部空白漂移仍必须拒绝。
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
- [x] AC8：release-console `local_verification` 流式日志测试在条件协调和自截止下专项连续 3 次通过，
  并继续断言首段日志先于完成、完整尾部和退出码。
- [ ] AC4：本次相关改动精确提交并普通 push 后，远端跟踪分支与本地 `HEAD` 一致。
- [ ] AC5：最终候选 SHA 的 GitHub 发布 Run 完整成功，Step #7 与 Step #8 均为 success。
- [ ] AC6：最终生成的 `v0.5.0` Release 保持 Draft，目标提交和三类 updater 资产审计通过；未执行公开、
  安装、升级或卸载时不声称这些行为成功。
- [x] AC7：差异与秘密扫描确认没有真实密钥、认证文件或真实 Codex/Relay 用户数据进入改动和证据。
- [x] AC9：Draft tag 404 与 GitHub 行尾规范化均有先红后绿回归；修复后的生产 `SystemGhBackend`
  已对真实 Draft 完成 Release ID、候选 SHA、三个资产和 manifest/签名关联审计。

## 范围外事项

- 自动公开 `v0.5.0`、清理历史 Release 或推送 Tag；为最终唯一候选 Run 重建本轮未公开 Draft 时，
  只允许在精确核对 Release ID、tag、候选 SHA 和 `draft=true` 后处理该 Draft，不触及其他 Release。
- 修改发布控制台远端监控、GitHub workflow、Tauri 签名、安装、升级、卸载或数据保留行为。
- 用重跑相同失败提交、增加 workflow retry 或放宽产品错误断言掩盖测试缺陷。
