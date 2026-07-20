# README 与安全开发启动修正实施计划

> **面向代理执行者：** 必须使用 `superpowers:subagent-driven-development`（推荐）或 `superpowers:executing-plans` 逐项执行本计划。所有步骤使用复选框（`- [ ]`）跟踪。

**目标：** 修复 Windows PowerShell 下 `dev:safe` 把 `npm` 错误解析为 `pm` 的问题，并使 README 的环境要求、开发启动、构建、安装范围和常见错误说明与当前仓库实现一致。

**架构：** 外部入口继续保持 `npm run dev:safe`，仅将 PowerShell 脚本内部递归调用改为明确的 `npm.cmd run dev`。使用现有 `src/release-config.test.ts` 增加回归断言，先验证旧脚本失败，再实施最小修复；README 按已经完成的逐行审计结果统一更新。

**技术栈：** TypeScript、Vitest、PowerShell、Markdown、Tauri 2、Git

---

### 任务 1：为 PowerShell npm 调用增加失败回归测试

**文件：**
- 修改：`src/release-config.test.ts`
- 检查：`scripts/prepare-dev-data.ps1`

- [ ] **步骤 1：添加脚本调用约束测试**

在 `src/release-config.test.ts` 的开发 API Key 测试之后新增：

```typescript
  it('uses the Windows npm command shim when safe development launches Tauri', () => {
    expect(prepareDevData).toContain('& npm.cmd run dev')
    expect(prepareDevData).not.toMatch(/&\s+npm\s+run\s+dev/)
  })
```

- [ ] **步骤 2：运行测试并确认按预期失败**

运行：

```powershell
npm run test -- --run src/release-config.test.ts
```

预期：新增测试失败，失败原因是当前脚本包含 `& npm run dev`，但不包含 `& npm.cmd run dev`。

### 任务 2：实施最小脚本修复并验证绿色状态

**文件：**
- 修改：`scripts/prepare-dev-data.ps1:72`
- 测试：`src/release-config.test.ts`

- [ ] **步骤 1：修改脚本内部调用**

将：

```powershell
  & npm run dev
```

替换为：

```powershell
  & npm.cmd run dev
```

外部入口仍为 `npm run dev:safe`，脚本内部不得调用 `npm.cmd run dev:safe`，以免递归进入自身。

- [ ] **步骤 2：重新运行专项测试**

运行：

```powershell
npm run test -- --run src/release-config.test.ts
```

预期：`src/release-config.test.ts` 全部通过。

### 任务 3：修正 README 的环境与安全开发说明

**文件：**
- 修改：`README.md:26-94`

- [ ] **步骤 1：补充重要目录**

在目录结构中加入：

```text
src-tauri/installer/         自定义 NSIS 安装模板
AGENTS.md                    仓库级安全、测试与文档规则
```

- [ ] **步骤 2：修正 Node.js 最低版本**

将环境要求中的 `Node.js 20+` 改为：

```text
Node.js 20.19+ 或 22.12+ 与 npm
```

该要求与当前 Vite 7.3.6 和 `@vitejs/plugin-vue` 的 `engines.node` 一致。

- [ ] **步骤 3：准确说明 `dev:safe` 调用链和测试密钥**

保留用户命令：

```powershell
npm run dev:safe
```

在说明中明确：外部使用 npm 脚本入口；`prepare-dev-data.ps1` 内部使用 `npm.cmd run dev` 避免 Windows PowerShell 将命令错误解析为 `pm`；生成的假密钥是 `test-key-provider-a-not-real` 和 `test-key-b-not-real`。

- [ ] **步骤 4：补充普通 `dev` 的安全警告**

明确说明 `npm run dev` 不会自动设置隔离目录，只有当前终端同时设置以下变量时才能安全用于本地隔离开发：

```powershell
$env:CODEX_RELAY_CODEX_HOME = "$PWD\dev-data\codex"
$env:CODEX_RELAY_APP_DATA_DIR = "$PWD\dev-data\app-data"
npm run dev
```

- [ ] **步骤 5：说明 `-PrepareOnly` 的进程边界**

在 `-PrepareOnly` 命令后明确说明：它只创建或刷新安全数据；因为命令启动了子 PowerShell，脚本内设置的环境变量不会回传当前终端。若随后手动运行 `npm run dev`，必须在当前终端重新设置两个 Relay 覆盖变量。

### 任务 4：修正 README 的构建、安装与功能措辞

**文件：**
- 修改：`README.md:48-50`
- 修改：`README.md:96-107`
- 修改：`README.md:199-227`

- [ ] **步骤 1：修正 Program Files 回退说明**

将“系统 64 位 Program Files 目录”改为“与构建目标架构匹配的系统 Program Files 目录”，并说明当前交付的 x64 安装包通常回退到 `C:\Program Files\Codex Relay`。

- [ ] **步骤 2：区分 Release 和 NSIS 构建命令**

说明：

- `npm run build:debug` 只生成 Debug 主程序，不打包；
- `npm run build:release` 与 `npm run build` 等价，按当前 `targets: ["nsis"]` 同时生成 Release 主程序和 NSIS；
- `npm run bundle:nsis` 是显式只请求 NSIS bundle 的替代入口，不需要在 `build:release` 后重复运行。

- [ ] **步骤 3：明确主题行为**

将“明暗主题”改为“跟随系统的明暗主题”，避免暗示存在手动主题切换设置。

- [ ] **步骤 4：澄清当前用户与全机安装的边界**

将当前限制中的“仅面向 Windows 10/11 当前用户”改为：程序仅支持 Windows 10/11，安装器为 per-machine，但 Provider、Codex 配置、应用数据和开机启动均按当前登录用户管理。

- [ ] **步骤 5：补充 Cargo/PATH 常见错误**

在常见错误中加入：`cargo metadata ... program not found` 表示启动 VS Code 或终端的父进程 PATH 中没有 Cargo；先运行 `cargo --version`，完全重启启动器/终端，或从能识别 Cargo 的终端使用 `code .` 打开项目。

### 任务 5：完整验证并提交

**文件：**
- 修改：`README.md`
- 修改：`scripts/prepare-dev-data.ps1`
- 修改：`src/release-config.test.ts`
- 新增：`docs/superpowers/plans/2026-07-20-readme-development-accuracy.md`

- [ ] **步骤 1：运行专项测试**

运行：

```powershell
npm run test -- --run src/release-config.test.ts
```

预期：全部通过，测试总数比修改前增加 1。

- [ ] **步骤 2：静态核对关键文档内容**

运行：

```powershell
rg -n 'Node.js 20\.19\+|npm run dev:safe|npm\.cmd run dev|PrepareOnly|build:release|bundle:nsis|当前登录用户|cargo metadata' README.md
rg -n '& npm\.cmd run dev' scripts/prepare-dev-data.ps1
```

预期：README 包含所有修正说明，脚本只使用 `npm.cmd run dev` 启动 Tauri。

- [ ] **步骤 3：运行完整检查**

运行：

```powershell
npm run check
```

预期：TypeScript、前端测试、Rust 格式检查、Clippy 和 Rust 测试全部通过。

- [ ] **步骤 4：检查差异范围**

运行：

```powershell
git diff --check
git status --short
git diff --stat
```

预期：只修改 README、开发数据脚本、发布配置测试和本实施计划；不得修改应用运行代码、Tauri 配置、依赖或包锁文件。

- [ ] **步骤 5：提交**

运行：

```powershell
git add README.md scripts/prepare-dev-data.ps1 src/release-config.test.ts docs/superpowers/plans/2026-07-20-readme-development-accuracy.md
git diff --cached --check
git commit -m "fix: correct safe development instructions"
```

预期：提交成功，工作树干净。
