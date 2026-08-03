# 诊断记录

## 失败现场

- 会话：`release-20260802084619-0000000000000002`
- 失败步骤：`release-console-rust-tests`
- 固定命令：`cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console`
- 控制台消息：Cargo 退出码 `101`；候选六文件已回滚，未提交或推送。

## 本轮证据

1. 当前控制台进程来自 `artifacts/release-console/CodexRelayReleaseConsole.exe`，排除旧的
   `dist` 输出树自锁。
2. 原失败时间窗内的 Cargo fingerprint 显示候选测试目标在 16:47:27–16:50:01
   全部完成编译；没有同时间窗的 rustc `output-*` 错误文件。因此 101 发生在测试执行阶段，
   不是 manifest、lock、编译或链接错误。
3. 回滚后的当前仓库复跑固定命令：110 项通过、1 项按设计忽略、退出码 0。
4. 隔离 worktree 精确恢复六文件 `0.5.0` 候选后：`cargo test --no-run` 退出 0；
   完整固定命令同样 110 项通过、1 项忽略、退出码 0。
5. 使用临时诊断启动器复用项目 `SafeProcessRunner`，让候选 Cargo 位于与生产相同的外层
   Windows Job Object 和过滤环境中执行；安全用户/Relay 路径全部指向临时目录。结果仍为
   110 项通过、1 项忽略、`HARNESS_EXIT_CODE=0`。
6. Windows Application 事件日志在失败时间窗没有对应 Cargo/Rust 测试进程崩溃记录。
7. 清除本轮隔离 worktree 产生的共享 target 文件后，在原仓库路径、原失败缓存和完整
   `0.5.0` 六文件候选下重放固定命令：110 项通过、1 项忽略、退出码 0；随后六文件
   SHA-256 与重放前逐项一致。
8. 恢复 `0.4.0` 后再次运行固定命令：110 项通过、1 项忽略、退出码 0。

诊断过程中曾因让临时 worktree 共用主仓库 `CARGO_TARGET_DIR`，人为制造出一次
“同名 `codex-relay-core` 来自两个源路径”的编译错误。该错误有当前时间的 rustc
`output-*` 证据，而原失败时间窗没有同类文件，因此明确排除为原始原因。本轮按时间窗只清理
了自己产生的 3892 个 target 文件（约 7.59 GiB），再完成同路径候选与恢复版本验证。

## 结论

- 已排除：候选版本内容必然失败、同路径版本切换必然失败、Cargo 版本解析失败、
  rustc/linker 固定错误、旧 `dist` 文件锁、外层 Windows Job Object 的确定性冲突。
- 剩余分类：当时测试执行阶段的一次非确定性 Windows/Git/PowerShell 子进程或资源竞争失败。
  `git_release` 套件会并行启动大量真实 Git 子进程，并使用 30 秒进程超时，是较高概率来源；
  但没有原始 stdout/stderr，不能把该概率判断写成已证实的具体失败用例。
- 无法唯一还原的原因：本地门禁调用 `SafeProcessRunner` 时未提供事件 sink，且
  `session.json` 只持久化 `phase/stepId/code`，stdout、stderr 和失败测试名称没有保存。

## 当前安全状态

- 主工作区在诊断前为干净状态；正式候选已由控制台回滚。
- 没有候选提交、Push 或 GitHub Run。
- 本轮只新增 Trellis 诊断材料；隔离 worktree、临时启动器和安全临时目录在交付前清理。
