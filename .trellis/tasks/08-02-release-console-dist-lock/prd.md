# 修复发布控制台与主构建输出目录冲突

## 目标

- 消除发布控制台运行期间触发主应用普通构建时，Vite 因无法清理正在运行的控制台 EXE 而退出 1 的稳定故障。
- 保持发布控制台为仓库内独立、便携、Git 忽略且不进入 Codex Relay 正式安装包的维护者工具。

## 已确认事实

- `scripts/package-release-console.ps1:15` 默认把控制台复制到
  `dist/release-console/CodexRelayReleaseConsole.exe`。
- `src-tauri/tauri.conf.json:10-11` 在普通 Tauri 构建前执行 `npm run build:frontend`，并把
  根目录 `dist` 作为主应用前端产物目录；根 `vite.config.ts` 没有覆盖默认 `outDir/emptyOutDir`。
- Windows 会锁定正在运行的 EXE。对 `dist/release-console` 中的文件持有独占句柄后执行
  `npm run build:frontend`，稳定返回退出码 1，并报告
  `[vite:prepare-out-dir] EBUSY: resource busy or locked, unlink ...`；释放句柄后同一命令退出 0。
- 归档任务 `08-02-release-gate-failure-progress` 已记录：运行中的 canonical
  `dist/release-console` EXE 无法由包装脚本覆盖，只能改用另一个输出目录验证。
- 本地门禁在该失败后已完成六个候选文件回滚，未创建候选提交、未推送，也未触发远端 Run。

## 需求

### R1：输出目录隔离

- 发布控制台默认交付目录不得位于主应用 Vite 会清理的 `dist` 目录树内。
- 主应用前端构建不得因为发布控制台正在运行而失败。

### R2：交付契约同步

- `npm run build:release-console` 仍先构建独立控制台，再由现有包装脚本复制便携 EXE 并输出实际路径、大小、时间和 SHA-256。
- 包装脚本的显式 `-SourcePath` 与 `-DestinationDirectory` 覆盖继续有效。
- 新默认交付目录必须被 Git 忽略，且不得进入主应用 Tauri bundle、NSIS 或 updater 资产。

### R3：文档与测试

- README、发布规范和结构测试使用同一个新默认路径。
- 回归测试必须证明默认控制台交付目录与主应用 Vite `dist` 输出树分离，并保留包装脚本复制与哈希证据。

### R4：真实验证边界

- 验证运行中的交付控制台不会阻止 `npm run build:frontend` 和普通 `npm run build`。
- 构建、测试、便携控制台运行、签名、安装和发布证据继续分别报告；本任务不声称未执行行为成功。

## 方案选择

### 方案 A：把控制台交付目录移到 `artifacts/release-console`（推荐）

- 优点：只改变维护者工具的复制路径、忽略规则、文档和结构测试；主应用 Vite/Tauri 产物契约不变，风险最低。
- 代价：维护者需要改用新路径启动控制台，已有文档和测试必须同步。

**决定：采用方案 A。** 用户已确认把默认交付路径固定为
`artifacts/release-console/CodexRelayReleaseConsole.exe`。

### 方案 B：把主应用前端产物改到 `dist/app`

- 优点：继续保留 `dist/release-console` 路径，两个产物仍在同一父目录下。
- 代价：改变主应用 `vite build` 与 Tauri `frontendDist` 的生产构建路径，影响面和实际构建验证成本更高。

### 方案 C：关闭 Vite 的 `emptyOutDir`

- 优点：改动最少，运行中的控制台不会被删除。
- 代价：主前端产物可能残留旧哈希文件，容易把过期资源带入 Tauri 构建，不采用。

## 验收标准

- [x] AC1：默认包装输出位于 `artifacts/release-console/CodexRelayReleaseConsole.exe`，且该目录被 Git 忽略。
- [x] AC2：结构测试先因旧默认路径失败，再在实现后通过，并验证默认目录不属于根 `dist` 树。
- [x] AC3：包装脚本临时目录测试继续验证逐字节复制、实际路径、大小和 SHA-256。
- [x] AC4：README 与 `.trellis/spec/release/publishing.md` 全部切换到新路径，不保留当前有效操作中的旧路径。
- [x] AC5：从新路径运行控制台时，`npm run build:frontend` 退出 0；普通 `npm run build` 完成实际构建，控制台进程不被强制结束。
- [x] AC6：发布控制台专项测试、前端相关检查和完整 `npm run check` 通过；Git 工作区不包含二进制、临时目录或秘密。

## 范围外

- 不改变本地门禁的四条命令、顺序、超时、回滚或错误码。
- 不改变主应用 Vite `dist`、Tauri bundle、NSIS/updater、GitHub Actions 或正式发布流程。
- 不把控制台二进制提交 Git，也不为旧 `dist/release-console` 路径增加自动迁移或删除逻辑。
