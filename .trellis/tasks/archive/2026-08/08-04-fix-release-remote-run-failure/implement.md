# 修复发布远程 Run 失败实施计划

## 当前进度

- [x] 读取 Run `30833079285` 的 Job 与失败日志，确认唯一失败断言和未生成 Draft 的边界。
- [x] 在本机记录原测试普通临时路径下 6/6 通过的环境相关基线。
- [x] 在安全临时目录用 8.3 别名复现输入短路径、证据长路径的差异。
- [x] 红灯：让专项测试显式使用 8.3 目标路径，确认原始字符串断言因预期原因失败。
- [x] 绿灯：把路径断言改为两端真实路径身份比较，保持字节、大小和 SHA-256 断言。
- [x] 质量复核：短路径 helper 在调用 PowerShell 前拒绝空值或相对路径，避免目标退化到仓库目录。
- [x] 运行专项测试、前端检查和成对安全 Relay 覆盖下的完整 `npm run check`。
- [x] 把 8.3 别名与真实文件身份比较补入发布控制台交付目录规范，并完成 Trellis 全范围检查。
- [ ] 精确提交本任务改动，归档任务、记录会话日志并普通 push 到已配置上游。

## 验证证据

- 红灯：`npx vitest run src/release-console-structure.test.ts --maxWorkers=1` 退出 1；唯一失败为
  8.3 短路径与 PowerShell 长路径的原始字符串断言，其余 5 项通过。
- 绿灯：相同专项命令退出 0，1 个测试文件、6 项测试通过；fixture 明确使用 8.3 目标路径。
- 受影响层：成对安全 Relay 覆盖下 `npm run check:frontend` 退出 0；主前端 60 个文件、338 项，
  发布控制台 17 个文件、89 项，两个 TypeScript 检查均通过。
- 最终门禁：最新代码和规范在新的成对安全 Relay 覆盖下运行 `npm run check`，退出 0；Trellis
  8 项、主前端 60/338、发布控制台 17/89，以及 Rust fmt、Clippy、249 项主库测试和各集成套件
  通过；1 个完整项目后端探针按既有设计 ignored。
- 安全审计：`git diff --check` 退出 0；本次文件的高置信度密钥扫描无命中；没有新增认证文件；
  `codex-relay-final-check-*` 临时根已清理。没有运行远端发布、Draft 构建、签名、安装或升级。

## 验证命令

```powershell
npx vitest run src/release-console-structure.test.ts --maxWorkers=1
npm run check:frontend
$safeRoot = Join-Path $env:TEMP ('codex-relay-check-' + [guid]::NewGuid().ToString('N'))
$env:CODEX_RELAY_CODEX_HOME = Join-Path $safeRoot 'codex-home'
$env:CODEX_RELAY_APP_DATA_DIR = Join-Path $safeRoot 'app-data'
npm run check
git diff --check
```

完整检查前创建两个安全覆盖目录，检查后只清理本轮生成的 `$safeRoot`。不运行正式发布 workflow，
不把本地测试通过写成 Draft、签名、安装或升级成功。

## 风险与回滚点

- 风险文件：`src/release-console-structure.test.ts`。
- 路径 fixture 依赖 Windows `cmd.exe` 和 8.3 别名；该测试原本已依赖 `pwsh.exe`，运行边界仍为
  Windows。若目标卷禁用 8.3 名称，测试仍验证真实路径身份，但本轮 GitHub runner 红灯证据保留。
- 生产脚本与发布 workflow 不修改。绿灯失败时先回到根因分析，不叠加远端监控或构建行为改动。
