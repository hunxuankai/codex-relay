# 修复候选发布完整检查失败设计

## 根因与边界

流式输出测试启动真实 Windows PowerShell，并把 `SafeProcessRunner` 的调用预算硬编码为 5 秒，
首事件等待硬编码为 2 秒。Run `30869666756` 已证明首段输出正常到达，但冷 Runner 在执行
500 毫秒 sleep 后没有在剩余预算内完成，测试于约 5.03 秒返回 `Timeout`。这是测试夹具把本机
调度速度当成 CI 不变量，不是生产流式读取、发布控制台监控或 GitHub workflow 故障。

生产 `SafeProcessRunner` 必须继续尊重每个调用方传入的显式超时。本次只调整
`#[cfg(test)]` 模块中的 PowerShell 夹具与测试预算。

## 测试协调

测试在 `tempfile` 目录创建 PowerShell 脚本、释放标记路径和可选诊断路径：

1. 子进程写出并 flush `first`。
2. 子进程按短间隔轮询释放标记，等待测试明确允许结束；轮询受测试级 deadline 约束。
3. 测试通过事件通道收到 `first`，确认 stream 类型、字节内容以及 runner 尚未结束。
4. 测试写入释放标记，子进程写出 `second` 并正常退出。
5. 测试断言最终输出、空 stderr 与退出码。

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

## 发布验证与回滚

本地门禁通过后创建新修复提交并普通 push 到已配置的 `origin/main`，再以 `expected_version=0.5.0`
和精确 `expected_sha` 触发唯一发布 Run。Run 成功后只审计 Draft，不公开。

若专项或完整门禁失败，停止在本地并回到根因分析；若远端仍在同一测试失败，保留 Run 证据，
不重复盲跑。代码回滚只涉及测试模块与测试规范，不需要产品数据迁移或清理远端资产。
