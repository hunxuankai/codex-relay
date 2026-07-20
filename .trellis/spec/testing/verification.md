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
