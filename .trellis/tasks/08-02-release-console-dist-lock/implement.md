# 发布控制台交付目录隔离实施计划

## 当前进度

- [x] 切片 1 RED：结构测试 6 项中 2 项因默认目录和文档仍为旧路径而按预期失败。
- [x] 切片 1 GREEN：脚本、忽略规则、README、发布规范完成最小迁移；专项结构测试 6/6 通过。
- [x] 切片 2：新交付控制台运行期间，主前端与普通 NSIS 构建均退出 0，控制台保持存活。
- [x] 完整质量门禁、规范复核和秘密/忽略规则审计。
- [ ] 精确提交、任务归档、会话日志与普通推送。

## 缺陷复盘与防复发

- **根因类别**：B（跨层契约）+ D（测试覆盖缺口）+ E（隐式假设）。`dist` 的所有者是可整体清理
  输出的主 Vite 构建，却同时被当作运行中维护工具的稳定交付目录；既有结构测试只验证“独立工具不进
  主 bundle”，没有验证输出根所有权和 Windows 文件锁组合行为。
- **此前为何未解决**：前一任务已观察到运行中的 canonical EXE 无法被包装脚本覆盖，但使用另一个
  验证目录完成了当轮构建，没有把该证据追溯到 `ordinary-build → vite build → emptyDir(dist)` 的
  同一结构冲突，因此症状在下一次真实发布中再次出现。
- **架构预防**：把维护产物移到独立 `artifacts/` 根，使 Vite 清理操作在结构上无法触及运行中的控制台。
- **测试预防**：结构测试断言默认交付路径不属于 `dist`，并保留运行新交付 EXE 时执行主前端与普通
  构建的 Windows 真实回归。
- **知识沉淀**：`.trellis/spec/release/publishing.md` 新增交付目录所有权七段契约；跨层思考指南新增
  构建输出目录所有权检查。仓库不存在 `src/templates/markdown/spec/`，无对应模板可同步。

## 本轮验证证据

- RED：`npx vitest run src/release-console-structure.test.ts` 退出 1；6 项中 2 项按预期因旧脚本路径和
  旧文档路径失败。
- GREEN：同一专项测试退出 0，6/6 通过。首次 `npm run check:frontend` 暴露当前 TypeScript lib
  不支持 `String.replaceAll`；改为兼容的全局正则后，专项 6/6 和主 `npm run typecheck` 均退出 0。
- `npm run check:frontend`：退出 0；主前端 59 个文件/325 项、发布控制台 16 个文件/76 项通过。
- `npm run build:release-console`：退出 0；新交付 EXE 为 12,665,856 字节，源/目标 SHA-256 均为
  `9C9106E9BFA132978FCB062F84FCBC632E79C9656E1D56FEC4D555B18A878DA2`，且 `artifacts/` 被 Git 忽略。
- Windows 真实回归：从新路径隐藏启动本轮独立控制台进程；`npm run build:frontend` 和
  `npm run build` 均退出 0，构建结束时控制台仍存活，随后只关闭该进程并验证退出。普通构建生成：
  - `CodexRelay.exe` 19,434,496 字节，SHA-256
    `382594B255C68518FD22D9C08B131C31943A1CE30245273C54BABB5E6E00945B`；
  - `Codex Relay_0.4.0_x64-setup.exe` 4,684,543 字节，SHA-256
    `C39354E04943DD1999DABC81BD7251A72567C802EFB74D684E9E2E19638A4E4F`。
- 上述只证明测试、便携控制台构建、运行中目录隔离和普通主程序/NSIS 构建；未执行签名、安装、升级、
  UAC、公开 Release 或远端清理。
- 最终 `npm run check` 使用成对安全 Relay 临时路径退出 0：Trellis 8 项通过；主前端 59 个文件/
  325 项、发布控制台 16 个文件/76 项通过；Rust 依赖图、fmt、Clippy 和整个 workspace 测试通过，
  包括 core 248 项、path safety 3 项、provider workflow 1 项及发布控制台全部集成套件。
- `git diff --check` 退出 0；高置信度秘密形态命中 0；`artifacts/` 由 `.gitignore` 命中，Git 未跟踪
  交付 EXE。换行符提示为现有 Windows CRLF 转换提示，不是空白错误。

## 1. 实施顺序

### 1.1 RED：锁定新路径契约

- 修改 `src/release-console-structure.test.ts`：
  - 期望 README 与发布规范使用 `artifacts/release-console/CodexRelayReleaseConsole.exe`；
  - 断言 `.gitignore` 包含 `artifacts/`；
  - 断言包装脚本默认目录包含 `artifacts\release-console` 且不再包含默认
    `dist\release-console`；
  - 显式断言默认交付路径不在根 `dist` 树内。
- 运行：

  ```powershell
  npx vitest run src/release-console-structure.test.ts
  ```

- 预期：因生产脚本、忽略规则和文档仍使用旧路径而失败；保存失败断言作为 RED 证据。

### 1.2 GREEN：最小路径迁移

- `scripts/package-release-console.ps1`：默认目录改为 `artifacts\release-console`。
- `.gitignore`：新增 `artifacts/`。
- `README.md` 与 `.trellis/spec/release/publishing.md`：当前构建/运行入口改为新路径。
- 不修改 Vite、Tauri、门禁命令、session、候选事务或 GitHub 编排。
- 重跑专项测试，预期通过。

### 1.3 重构与一致性检查

- 搜索非归档当前资料中的旧路径，确认只允许历史任务证据保留：

  ```powershell
  rg -n "dist[\\\\/]release-console" README.md package.json scripts src tools .trellis/spec
  ```

- 检查测试命名和断言是否描述“目录所有权分离”，避免只断言字符串替换。

## 2. 分层验证

### 2.1 专项与前端

```powershell
npx vitest run src/release-console-structure.test.ts
npm run check:frontend
```

记录退出码和实际测试数量，不把专项通过描述成完整项目通过。

### 2.2 控制台产物

```powershell
npm run build:release-console
```

- 枚举 `artifacts/release-console/CodexRelayReleaseConsole.exe` 的路径、大小、最后写入时间和 SHA-256。
- 核对与 `src-tauri/target/release/CodexRelayReleaseConsole.exe` 的大小和 SHA-256 一致。

### 2.3 Windows 文件锁真实回归

- 创建系统临时的 `CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR`，确认均不指向受保护真实目录。
- 从新 `artifacts` 路径隐藏启动控制台并记录本轮 PID；不结束任何既有 Codex Relay 或其他控制台进程。
- 在该 PID 存活时依次运行：

  ```powershell
  npm run build:frontend
  npm run build
  ```

- 两条命令均须退出 0；第二条必须枚举实际主程序和 NSIS 产物。检查控制台 PID 在构建后仍存活，再只关闭
  该 PID并验证退出；安全临时目录须在路径校验后清理。

### 2.4 完整门禁

```powershell
npm run check
git diff --check
git status --short --ignored
```

- 检查没有二进制、临时路径、真实 Codex/Relay 数据或密钥进入 Git。
- 复核 `artifacts/` 被忽略，主应用正式 bundle 不包含控制台名称或路径。

## 3. 风险文件与回滚点

- `scripts/package-release-console.ps1`：默认交付路径的唯一生产入口；显式覆盖不可回归。
- `src/release-console-structure.test.ts`：公开路径与隔离契约。
- `.gitignore`、`README.md`、`.trellis/spec/release/publishing.md`：必须与脚本同步。
- 若真实构建失败，保留首次错误和阶段；不得通过关闭门禁、放宽 Vite 清理或杀死未知进程来制造成功。

## 4. 完成条件

- PRD 的 AC1–AC6 全部由本轮测试、构建和观察证据覆盖。
- 质量检查通过后精确暂存本任务文件，使用简体中文 Conventional Commit 提交。
- 完成规范更新、任务归档和会话日志提交后，普通 push 当前跟踪分支，并验证远端跟踪分支与本地
  `HEAD` 一致。
