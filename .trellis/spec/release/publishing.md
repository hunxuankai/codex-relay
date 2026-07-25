# Windows 更新发布操作指南

本文面向 Codex Relay 维护者，说明如何把已经完成并验证的改动发布为 Windows NSIS 安装包和 Tauri 应用内更新。发布工作流只允许手动触发，并且必须先生成 Draft Release，核对完成后再人工公开。

## 1. 发布边界

- 默认发布源是 GitHub 默认分支 `main`，工作流为 `.github/workflows/release.yml`。
- 工作流触发键固定为 `workflow_dispatch:`，并通过 `releaseDraft: true` 保证只先生成 Draft；普通提交不得自动公开 Release。
- 应用版本使用 SemVer，发布版本必须严格高于当前公开版本。
- 应用版本来源是 `package.json`；package lock、Cargo 元数据和发布说明必须同步。
- 普通本地构建不需要更新私钥；带 `.sig` 和 `latest.json` 的 updater 资产只由 GitHub Actions 发布构建生成。
- 发布工作流使用 GitHub 自动提供的 `GITHUB_TOKEN`，并从 Actions Secrets 读取 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。不得读取、复制或输出 Secret 值。
- Tauri updater 签名用于验证更新资产；Windows Authenticode 用于验证 Windows 发布者身份，两者相互独立。当前项目未启用 Authenticode，因此安装器可能显示“未知发布者”。
- 发布、安装、升级、重启和数据保留是不同证据。未执行的步骤必须明确记录为未验证。

开始前同时阅读 [Tauri 应用内更新](updater.md)、[Tauri 与 NSIS](tauri-nsis.md)、[代码签名](signing.md)、[验证与完成证据](../testing/verification.md) 和 [数据保留与清理](../security/data-retention.md)。

## 2. 发布前确认

1. 确认本次产品改动、测试和文档已经完成，不把无关改动混入发布提交。
2. 确认当前公开版本和目标版本。以下命令只读取公开 Release，不需要 Token：

   ```powershell
   $headers = @{ "User-Agent" = "Codex-Relay-Release-Check" }
   $latest = Invoke-RestMethod `
     -Headers $headers `
     -Uri "https://api.github.com/repos/hunxuankai/codex-relay/releases/latest"
   $latest | Select-Object tag_name, name, published_at, draft, prerelease
   ```

3. 选择严格更高的 SemVer，例如从 `1.2.3` 提升到 `1.2.4`。不要复用已存在的 Tag 或 Release 版本。
4. 确认 Git 工作区状态和远端分支：

   ```powershell
   git status --short --branch
   git remote -v
   git branch -vv
   ```

5. 在 GitHub 仓库的 `Settings → Secrets and variables → Actions` 中确认以下 Secret 名称存在：

   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

   只确认名称和更新时间，不查看或输出值。私钥丢失、公钥需要更换或 Secret 状态不确定时停止发布，先处理信任根恢复方案。

## 3. 同步版本与发布说明

### 3.1 更新 npm 版本

在 PowerShell 中输入不带 `v` 的新版本号：

```powershell
$newVersion = Read-Host "输入新版本号，例如 0.1.3"
npm version $newVersion --no-git-tag-version
```

该命令应更新 `package.json`、`package-lock.json` 和 lock 根包版本，但不会创建 Git Tag。

### 3.2 更新 Cargo 版本

把 `src-tauri/Cargo.toml` 中 `[package]` 的 `version` 改为同一个版本。随后运行 Cargo 或项目检查，让 `src-tauri/Cargo.lock` 中 `codex-relay` 根包版本同步。

核对四个版本来源：

```powershell
Select-String -LiteralPath package.json,package-lock.json -Pattern '"version"'
Select-String -LiteralPath src-tauri/Cargo.toml -Pattern '^version\s*='
rg -n -A 2 '^name = "codex-relay"$' src-tauri/Cargo.lock
```

### 3.3 更新发布结构测试

检查 `src/release-config.test.ts`。如果其中仍有上一版本专用的测试名称、固定版本断言或“上一版本 → 新版本”说明断言，必须同步到本次候选；更通用的长期契约可以保留为版本一致性断言。

### 3.4 写入最终发布说明

在 `.github/workflows/release.yml` 的 `releaseBody` 中直接写入可以公开的最终简体中文说明，至少包含：

- 本版本的用户可见变化；
- 从哪个已公开版本更新；
- 安装或升级注意事项；
- 当前没有 Windows Authenticode、可能显示“未知发布者”；
- 升级不会主动删除 Codex 配置、Codex Relay 应用数据、日志或备份。

`releaseBody` 会同时进入 Release 页面和 `latest.json.notes`。Draft 生成后只编辑 GitHub 页面说明不会重写已经上传的 `latest.json`；发现说明错误时必须修正工作流并重新生成 Draft。

最后搜索待发布版本和上一版本，人工确认没有遗漏需要同步的位置：

```powershell
rg -n -S $newVersion package.json package-lock.json src-tauri .github README.md .trellis/spec/release
```

历史任务和历史示例可以保留旧版本；当前配置、测试和发布说明不得漂移。

## 4. 本地验证候选

### 4.1 运行完整检查

```powershell
npm run check
```

任何测试、格式、Clippy 或编译失败都必须先修复。成功重试不能抹掉首次失败和根因记录。

### 4.2 验证普通构建不依赖更新私钥

只从当前 PowerShell 进程移除签名环境变量，然后运行普通构建：

```powershell
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
npm run build
```

不要把私钥值放进命令行、脚本、日志、Trellis 任务或普通环境文件。`npm run build` 成功只证明普通 Release 主程序和 NSIS 安装器已生成，不证明 updater 签名、安装或升级成功。

### 4.3 枚举实际产物

```powershell
$artifacts = @(
  Get-Item -LiteralPath "src-tauri/target/release/CodexRelay.exe"
  Get-ChildItem -LiteralPath "src-tauri/target/release/bundle/nsis" -Filter "*.exe"
)

$artifacts | Select-Object FullName, Length, LastWriteTime
$artifacts | Get-FileHash -Algorithm SHA256
```

必须记录实际路径、大小、时间和 SHA-256，不能根据约定猜测文件名。普通构建不应生成本次发布的 `.sig` 或 `latest.json`。

### 4.4 提交前检查

```powershell
git diff --check
git status --short --ignored
git diff -- package.json package-lock.json src-tauri/Cargo.toml `
  src-tauri/Cargo.lock src/release-config.test.ts .github/workflows/release.yml
```

人工复核：

- 没有真实 API Key、Authorization Header、Token、认证文件或密钥内容；
- 没有真实 `%USERPROFILE%\.codex` 或 `%LOCALAPPDATA%\CodexRelay` 数据；
- 没有把构建目录、临时下载或 Sandbox staging 纳入 Git；
- 发布说明与实际改动一致。

## 5. 提交并推送候选

按本次实际改动精确暂存，不使用会把无关文件一并加入的宽泛命令。提交信息默认使用简体中文，例如：

```powershell
git add <本次发布文件>
git commit -m "chore(release): 准备 v<新版本> 发布"
git push origin HEAD:main
```

推送后确认远端 `main` 精确包含候选提交。不要手动创建 Tag；`tauri-action` 会依据应用版本和 `tagName: v__VERSION__` 创建 Release Tag。

## 6. 触发发布工作流

1. 打开 GitHub 仓库的 `Actions` 页面。
2. 选择“发布 Windows 更新”。
3. 点击 `Run workflow`。
4. 选择 `main`，再次确认后启动。
5. 等待所有步骤完成：检出源码、配置 Node.js、配置 Rust、`npm ci`、`npm run check` 和“构建 Draft Release”。

工作流失败时不得创建或描述为成功发布。先记录失败步骤、日志边界和对应提交，修复后使用新提交重新运行。

### 6.1 运行时长与外部状态门禁

发布工作流同时包含外部排队、Windows 冷环境准备、完整检查和 Draft 构建；这些阶段的证据必须分开
记录，不能把排队时间误写成构建耗时，也不能把测试失败归因于 GitHub 服务故障。

触发前和每次重试前：

- 只保留一个活动候选 Run；先检查 GitHub Actions 状态页和仓库 Run 队列。若状态页显示
  `major_outage` 或 Run 长时间 `queued`，记录为外部阻塞，不重复触发相同提交。
- 记录 `createdAt`、`startedAt`、`completedAt`、候选提交和失败步骤；分别计算排队时长与 Job 执行时长。
- 将失败归类为：外部服务/排队、工作流或工具诊断、CI 环境时序、产品/测试契约。只有修复并完成本地
  针对性回归和完整检查后，才用新候选提交重试；不得用相同提交盲目重跑来掩盖失败。
- 迭代期间先运行专项测试和本地安全检查，最后才运行完整 `npm run check`、普通构建和唯一的发布 Run。
  完整检查、Draft 构建、Draft 审计、公开复核和 Sandbox 升级是不同时间段，发布记录应分别列出。

历史 Windows 冷 runner 的完整检查和 Draft 构建耗时只能作为排程参考，不能改写成产品性能承诺；测试预算
必须留在测试层，不能为了缩短发布时间放宽生产超时或错误断言。

## 7. 核对 Draft Release

Action 成功后，Release 必须仍为 Draft。发布前逐项核对：

- Tag 和标题是目标版本，例如 `v0.1.3` 和 `Codex Relay v0.1.3`；
- 目标提交与触发工作流的候选提交一致；
- `draft=true`、`prerelease=false`；
- Release 页面说明是最终文案；
- 资产包含且只包含预期的 NSIS 安装器、对应 `.sig` 和 `latest.json`；
- `latest.json.version` 等于目标版本；
- `latest.json.notes` 与 Release 说明一致；
- 平台包含 `windows-x86_64` 和 `windows-x86_64-nsis`，并指向本次 NSIS 资产；
- 清单内联签名与独立 `.sig` 内容一致；
- 安装器、`.sig` 和 `latest.json` 的实际大小与 SHA-256 已记录；
- 没有私钥、密码、Token、认证 Header 或用户数据进入说明和资产。

如果平台 URL 是 GitHub REST asset API，按 updater 的下载语义核对实际字节，而不是使用普通 GET：

```powershell
curl.exe -L -H "Accept: application/octet-stream" `
  "https://api.github.com/repos/hunxuankai/codex-relay/releases/assets/<asset-id>" `
  -o "$env:TEMP\codex-relay-updater.exe"
Get-FileHash -Algorithm SHA256 "$env:TEMP\codex-relay-updater.exe"
Remove-Item -LiteralPath "$env:TEMP\codex-relay-updater.exe"
```

Draft 说明、版本、签名或资产有任何问题时不要公开。删除错误 Draft，修正源码或工作流，提交推送后重新运行。

## 8. 人工公开 Release

只有 Draft 核对全部通过后，才在 GitHub Release 页面点击 `Publish release`。公开后立即确认：

```powershell
$headers = @{ "User-Agent" = "Codex-Relay-Release-Check" }
$release = Invoke-RestMethod `
  -Headers $headers `
  -Uri "https://api.github.com/repos/hunxuankai/codex-relay/releases/latest"
$manifest = Invoke-RestMethod `
  -Headers $headers `
  -Uri "https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json"

$release | Select-Object tag_name, target_commitish, draft, prerelease, published_at
$manifest | Select-Object version, pub_date
```

预期结果：

- `releases/latest` 返回新 Tag；
- `draft=false`、`prerelease=false`；
- `latest.json` 返回新版本；
- tag 下载和 `releases/latest/download/latest.json` 的清单内容一致；
- 公开资产的大小、SHA-256、URL 和签名与 Draft 核对结果没有漂移。

## 9. 验证应用内升级

公开 Release 只能证明托管状态，不能替代真实升级验证。使用 Windows Sandbox 或隔离 VM，从已知良好的上一公开版本执行：

1. 使用安全临时 staging 和成对的 `CODEX_RELAY_CODEX_HOME`、`CODEX_RELAY_APP_DATA_DIR` 覆盖准备测试数据。
2. 核对基线安装器版本、大小和 SHA-256 后安装上一版本。
3. 确认上一版本能进入设置页。
4. 点击“检查更新”，确认发现目标版本。
5. 点击“下载并安装”，观察下载、Tauri 签名接受、NSIS 启动、可能出现的 UAC 和应用退出。
6. 升级后运行 `guest-verify.ps1 -ExpectedVersion <新版本>` 或对应桌面核验入口。
7. 读取 `after.json`，确认：
   - 实际版本等于目标版本；
   - `CodexRelay.exe` 存在；
   - 升级沿用登记安装目录；
   - 白名单 fixture 的长度和 SHA-256 前后相同；
   - 报告不含 fixture 内容、API Key 或认证 Header。
8. 单独记录应用是否自动重启、是否出现 UAC，以及取消、断网、错误签名和下载失败路径是否执行。

禁止使用真实 `.codex`、真实 `%LOCALAPPDATA%\CodexRelay`、仓库目录或经过 junction/symlink 的可写路径作为 Sandbox 数据目录。人工覆盖安装只能记录为“恢复已知良好基线”，不能算作 updater 成功。

## 10. 失败、修复与回滚

- 本地检查失败：停止发布，修复并重新运行完整检查。
- GitHub Actions 失败：不得发布；记录失败步骤，使用新提交重试。
- Draft 错误：删除 Draft 和错误资产，修正后重新生成。
- UAC 取消、安装器失败或升级后无法启动：不得报告升级成功；允许用户重新打开旧版本或人工安装已知良好版本。
- 已公开版本有缺陷：不得原地替换安装器、`.sig` 或 `latest.json`，发布严格更高的 SemVer 修复。
- 更新私钥丢失或公钥必须更换：现有客户端不能信任新密钥，必须设计手动安装更高版本的恢复路径。
- 没有 Authenticode 证据：继续按“未知发布者”报告，不能把 Tauri updater 签名描述成 Windows 发布者签名。

## 11. 发布记录最低内容

每次发布至少保留以下非秘密证据：

- 版本、候选提交和工作流 Run URL；
- 本地 `npm run check` 与普通 `npm run build` 的退出状态；
- Release/NSIS 实际路径、大小、时间和 SHA-256；
- Draft Release ID、Tag、目标提交和三个资产的大小、SHA-256；
- `latest.json` 的版本、说明、平台 URL 和签名关联核对；
- 公开时间与公开端点复核结果；
- Sandbox/VM 中实际执行和未执行的安装、升级、UAC、重启及数据保留场景；
- 所有真实失败、限制和未完成项。

这些证据可以记录在对应 Trellis 发布任务中，但不得包含 Secret 值、完整认证文件、API Key、Authorization Header 或用户数据内容。
