# 修复候选发布完整检查失败设计

## 根因与边界

流式输出测试启动真实 Windows PowerShell，并把 `SafeProcessRunner` 的调用预算硬编码为 5 秒，
首事件等待硬编码为 2 秒。Run `30869666756` 已证明首段输出正常到达，但冷 Runner 在执行
500 毫秒 sleep 后没有在剩余预算内完成，测试于约 5.03 秒返回 `Timeout`。这是测试夹具把本机
调度速度当成 CI 不变量，不是生产流式读取、发布控制台监控或 GitHub workflow 故障。

生产 `SafeProcessRunner` 必须继续尊重每个调用方传入的显式超时。本次只调整
`#[cfg(test)]` 模块中的 PowerShell 夹具与测试预算。

## Draft 审计后续根因与边界

Run `30880121507` 首次成功生成真实 Draft 后，发布控制台的两个平台假设才进入可观察路径：

1. GitHub Draft 的 `tag_name` 是预定名称，正式公开前 `refs/tags/v0.5.0` 不存在，tag API 返回 404；
   现有夹具却在 Draft 场景伪造了成功的 `GetTag` 响应。
2. Windows Actions 的多行 output 经 GitHub 后把 LF/末尾换行规范化为 CRLF/无末尾换行；Release 正文
   与 `latest.json.notes` 相互一致，但与工作区原始字节不同。现有测试让三者共享同一个 `String`，
   没有覆盖传输边界。

Draft 身份已经由唯一 Draft、精确 `target_commitish == candidate_sha`、标题、说明和资产共同绑定，
因此 Draft 阶段使用该字段作为 `target_commit_sha`；只有公开后的 `audit_published` 查询真实 tag ref。
说明比较只规范化 CRLF/孤立 CR 为 LF 并对两侧 `trim_end`，内部字符与空白继续严格相等。

修复提交通过本地全量门禁并普通 push 后，构建更新后的便携控制台。为验证同一个按钮端到端路径，
现有未公开 Draft 只能在精确身份守卫下删除并由唯一新 Run 重建；不得公开、修改其他 Release 或创建 tag。

## 测试协调

core 与 release-console 测试都在 `tempfile` 目录创建 PowerShell 脚本、释放标记路径和可选诊断路径：

1. 子进程写出并 flush `first`。
2. 子进程按短间隔轮询释放标记，等待测试明确允许结束；轮询受测试级 deadline 约束。
3. 测试通过事件通道收到 `first`，确认 stream 类型、字节内容以及 runner 尚未结束。
4. 测试写入释放标记，子进程写出 `second` 并正常退出。
5. 测试断言最终输出、空 stderr 与退出码。

release-console 的 `ProcessLocalVerificationBackend` 生产命令预算仍为 2 小时；集成测试脚本额外
设置 20 秒的 PowerShell 自截止，外层事件/完成等待使用 30 秒测试预算。这样测试失败时子进程
会先自行退出，不会把测试错误变成遗留的长命令。

runner 和首事件等待共用 `PROCESS_TREE_TEST_TIMEOUT`。该 30 秒值只给 Windows 冷启动、管道读取
和调度留下有界容差，不成为产品 SLA；条件满足时测试会立即继续，不固定等待 30 秒。

## 方案权衡

- 采用：临时文件条件协调加共享测试预算。它直接控制“进程尚未完成”的条件，消除 500 毫秒
  时间猜测，并保持真实 PowerShell、pipe、Job Object 和事件通道覆盖。
- 不采用：只把 5 秒改成 30 秒而保留固定 sleep。它能修复本次超时，却仍可能因测试线程在
  sleep 窗口内未获调度而在 `run.is_finished()` 处偶发失败。
- 不采用：mock `SafeProcessRunner` 或事件 sink。mock 无法验证真实 Windows pipe 在进程完成前
  推送字节的行为。
- 不采用：修改生产默认超时或 workflow retry。两者都会把测试时序缺陷扩散到产品或发布层。
- 不采用：仅把 release-console 外层 `timeout` 从 10 秒扩大而保留 750 毫秒 sleep；这仍会把冷
  Runner 调度延迟误判为行为失败，并可能在生产命令 2 小时预算内留下后台进程。

## 发布验证与回滚

本地门禁通过后创建新修复提交并普通 push 到已配置的 `origin/main`，再以 `expected_version=0.5.0`
和精确 `expected_sha` 触发唯一发布 Run。Run 成功后只审计 Draft，不公开。

若专项或完整门禁失败，停止在本地并回到根因分析；若远端仍在同一测试失败，保留 Run 证据，
不重复盲跑。代码回滚只涉及测试模块与测试规范，不需要产品数据迁移或清理远端资产。

重建 Draft 前必须再次验证 Release ID、`tag_name=v0.5.0`、旧候选 SHA 和 `draft=true`，只删除该
未公开对象；身份任一漂移立即停止。新 Run 失败时保留实际状态，不重复触发或删除其他 Release。
