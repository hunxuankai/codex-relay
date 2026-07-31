# TDD 与路径隔离

## 行为切片

新功能和缺陷修复按一个公开行为切片循环：

1. 写一个描述预期行为的测试。
2. 运行它，确认因目标行为缺失而失败。
3. 写最小实现使测试通过。
4. 运行专项测试；保持绿色后才重构。
5. 运行与风险相称的完整检查，并把命令证据写入任务检查点。

文档和纯工作流迁移没有运行时代码行为时，以结构验证、引用验证和失败预演代替虚构的单元测试。

## Rust 隔离

- 文件单元/集成测试使用 `tempfile` 或 `AppPaths::for_test`。
- 测试模式使用环境覆盖时，`CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR` 必须成对设置。
- 缺少覆盖时必须立即失败，不能回退生产路径。
- `path_safety` 在安全临时目录构造默认路径哨兵，运行 Provider/备份流程后比较递归快照完全不变。

## 前端隔离

- Vitest/Vue Test Utils mock typed Tauri service 或 composable，不启动真实文件写入。
- 测试数据只使用 `test-key-*-not-real`。
- 组件测试关注用户可见行为、DTO 和错误消息，不锁死私有方法或内部调用次数。

## Vitest 与 Windows Sandbox 并发契约

### 1. 范围与触发条件

`src/sandbox-update.test.ts` 会在 Vitest worker 中同步启动多个 Windows PowerShell 子进程。
新增或调整 Sandbox、PowerShell、jsdom 重型测试，或者修改 `vite.config.ts` 的测试并发时，
必须遵守本节。该约束只控制测试基础设施资源争用，不改变产品运行时超时。

### 2. 签名

权威配置位于 `vite.config.ts`：

```ts
test: {
  maxWorkers: 4,
  environment: 'jsdom',
}
```

Sandbox 用例继续使用各自明确的 Vitest 预算（当前为 `20_000` 毫秒）；不得用全局超时掩盖
单个脚本或断言回归。

### 3. 契约

- Windows 全套前端测试最多并行 4 个 Vitest worker，避免 jsdom worker 与同步
  `powershell.exe` 子进程在常见 8 逻辑核主机上过度争用。
- `npm test`、`npm run check:frontend`、`npm run check` 和 GitHub 发布工作流必须消费同一
  `vite.config.ts` 配置，不在 CI 另行恢复默认 worker 数。
- 调试 Sandbox 超时时，先单独运行失败用例，再以 `--maxWorkers=4` 运行完整前端套件；
  单独正常而默认并发失败表示测试资源争用，不等于产品脚本逻辑失败。
- 不得通过放宽生产超时、安全错误断言或路径防护来缩短测试；只有真实脚本工作量增长并有
  测量证据时，才评估单测试预算。

### 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| `vite.config.ts` 缺少 `maxWorkers: 4` | 发布结构测试失败，不进入完整发布检查 |
| Sandbox 用例单独通过、默认高并发下接近/超过 20 秒 | 先按 worker 争用调查，不直接提高测试或生产超时 |
| `--maxWorkers=4` 全套通过 | 固化并发上限，并重新运行 `npm run check` |
| 限制 worker 后单独用例仍失败 | 按脚本/断言缺陷继续系统化调试，不能归因于资源争用 |
| 测试触及真实 Codex/Relay 路径 | 立即失败；并发调整不能覆盖路径安全红线 |

### 5. 良好、基线与错误用例

- 良好：4 worker 下 40 个前端文件全部通过，Sandbox 负向用例在 20 秒预算内返回预期
  `SANDBOX_*` 错误。
- 基线：单独运行 Sandbox 用例明显快于全套，但两者都保持相同安全断言和临时路径。
- 错误：保留默认 8 worker，看到 `onTaskUpdate` 或 20 秒测试超时后只把用例预算改成
  60 秒。
- 错误：为了减少 PowerShell 启动次数而跳过 reparse point、重复报告项或真实路径拒绝测试。

### 6. 必需测试

- `src/release-config.test.ts` 结构断言 `vite.config.ts` 的 `test.maxWorkers` 固定为 4。
- 单独运行新增或失败的 Sandbox 用例，确认它因真实脚本结果通过/失败，而非 worker RPC。
- 完成前使用成对 Relay 临时覆盖运行 `npm run check`，确认前端无测试超时或未处理
  `onTaskUpdate` 错误，Rust 与路径安全门禁继续执行。

### 7. 错误与正确做法

错误：只提高超时，继续让重型 PowerShell 测试与所有 jsdom 文件按默认并发竞争。

```ts
test: {
  testTimeout: 60_000,
}
```

正确：限制 worker 数，保留每个 Sandbox 用例自己的严格预算和安全断言。

```ts
test: {
  maxWorkers: 4,
  environment: 'jsdom',
}
```

## 重点覆盖

- TOML 注释、未知字段、其他 Provider 与顶层功能保留。
- 损坏 JSON 保留原件、外部修改冲突和缺少密钥。
- 临时文件解析失败、写入失败、写后验证失败和回滚失败。
- 并发切换、备份恢复、20 份保留限制和原始字节恢复。
- 日志、错误、事件和 Debug 输出不泄漏密钥。
