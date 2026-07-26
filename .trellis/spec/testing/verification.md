# 验证与完成证据

## 证据原则

不得声称测试、构建、回滚、签名、安装、升级、卸载或人工行为成功，除非本轮有对应命令输出或人工观察。超时、文件锁、首次失败和未执行项目必须如实保留；成功重试不能抹掉根因和限制。

## 标准检查

```powershell
npm run typecheck
npm run test
npm run check:frontend
npm run check:rust
npm run check
```

风险对应：纯前端先跑专项 Vitest + typecheck；Rust 格式/逻辑跑 fmt、Clippy、Rust tests；跨层或完成前跑 `npm run check`。改变构建/NSIS 配置时还需实际运行相应 build。

## 按风险分层验证

完整测试不是每个局部改动的默认起点，但“改动很小”也不能替代与风险相称的证据。先按
变更边界分类，再选择最低充分检查：

| 变更类型 | 最低验证 | 何时扩大 |
|---|---|---|
| 单层、局部、低风险（例如单个展示组件或纯函数），不涉及共享状态、跨层 DTO、并发/取消、安全、配置、构建或发布 | 直接相关的专项测试；受影响层的类型/静态检查；`git diff --check` | 专项失败后修复、发现共享依赖或行为影响邻近模块时，升级到受影响层完整套件。 |
| 跨层数据流、共享 composable/基础设施、异步并发/取消、秘密/路径安全、配置迁移，或测试/构建脚本改动 | 相关层完整套件，并运行 `npm run check` 或等价的后端/前端全套检查；按触发条件补安全专项、构建或人工观察 | 任何发布、安装、卸载、签名或数据保留边界变化，都必须执行对应实际流程。 |
| 提交、任务归档、发布前 | 项目规定的完整检查（当前为 `npm run check`；涉及产物时另跑实际 build） | 不得以旧报告、缓存结果或专项通过替代本轮证据。 |

### 可执行选择

局部前端改动可以先运行：

```powershell
npx vitest run src/components/Target.test.ts
npm run typecheck
git diff --check
```

如果改动触及共享异步状态、跨层契约或安全边界，至少升级为：

```powershell
npm run check:frontend
npm run check
```

命令中的路径和范围必须按本轮实际改动替换；不能为了省时漏掉直接相关测试。报告必须分别
列出已运行、失败、跳过和未适用的项目，并保留退出码或测试数量。

### 反模式：把专项通过当成全量通过

```text
# 错误：只跑一个组件测试，就声称整个项目检查通过。
npx vitest run src/components/Target.test.ts

# 正确：先如实报告专项证据；只有运行 npm run check 后才能声称完整检查通过。
```

这条分层规则既减少低风险局部修改的等待，也防止共享状态、跨层和发布风险被“改动很小”
掩盖；任何未执行的完整项目必须在交付说明中明确保留。

## 构建证据

构建报告必须枚举实际文件路径、大小、最后写入时间和 SHA-256；不得只按约定猜测文件名。Debug、Release 主程序和 NSIS 安装器是不同产物：

- Debug：`src-tauri/target/debug/`
- Release：`src-tauri/target/release/CodexRelay.exe`
- NSIS：`src-tauri/target/release/bundle/nsis/`

如果目标 exe 被运行进程锁定导致 Windows 错误 5，该次构建不计为成功；确认进程退出和文件可独占打开后才能重试。

## 安全审计

- 检查 `git status --short --ignored` 与 `git ls-files`，确认开发数据、target、认证文件和密钥存储未跟踪。
- 扫描高置信度密钥前缀并人工复核 `OPENAI_API_KEY`、Authorization、Bearer 命中。
- 路径检查必须证明测试未操作真实 `.codex` 和 Codex Relay 应用数据。
- `git diff --check` 与暂存差异检查在提交前通过。

## 报告矩阵

| 声明 | 最低证据 |
|---|---|
| 测试通过 | 本轮命令、退出码、测试数量 |
| Release/NSIS 已生成 | 构建退出 0 + 实际产物枚举 |
| 托盘/窗口行为正常 | Windows 人工或自动化观察 |
| 已安装/升级/卸载 | 隔离用户或虚拟机的真实操作 |
| 已签名 | 签名工具和时间戳验证输出 |

旧验证报告的时间戳、哈希和某次成功结果不是未来完成声明的证据，只保留在 Git 历史。
