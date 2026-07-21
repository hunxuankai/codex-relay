# Tauri 应用内更新契约

## 1. 范围与触发条件

修改手动检查更新、`tauri-plugin-updater`、GitHub Releases、`latest.json`、updater artifacts、更新公钥或 GitHub Actions 签名环境变量时，必须遵循本规范。该集成跨越 Vue、Tauri 配置、Rust 插件、NSIS 和 GitHub Actions，因此不能只靠单层测试验收。

## 2. 签名与入口

- 基础配置：`src-tauri/tauri.conf.json` 的 `plugins.updater.endpoints: string[]` 与 `plugins.updater.pubkey: string`。
- 发布覆盖：`src-tauri/tauri.updater.conf.json` 的 `bundle.createUpdaterArtifacts: true`。
- 普通构建：`npm run build`，不得引用发布覆盖或要求更新私钥。
- 发布构建：`npm run build:release` 或 GitHub Actions 中等价的 `tauri build --config src-tauri/tauri.updater.conf.json`。
- GitHub Secrets：`TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`；`GITHUB_TOKEN` 由 Actions 自动提供。
- 客户端入口：用户在设置页显式调用 `UpdateClient.checkForUpdate()`；启动、挂载和后台定时器不得调用远端检查。
- Sandbox 主机入口：`scripts/windows-sandbox/prepare-update-test.ps1 -InstallerPath <path> -ExpectedSha256 <64-hex> -ExpectedTargetVersion <semver> [-StageRoot <temp-path>] [-PrepareOnly]`。
- Sandbox guest 入口：`guest-bootstrap.ps1` 生成升级前报告并启动基线安装器，`guest-start-app.ps1` 使用安全覆盖重新启动应用，`guest-verify.ps1 -ExpectedVersion <semver>` 生成升级后报告。

## 3. 契约

- 唯一更新清单地址为 `https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json`，必须使用 HTTPS 且不能来自用户输入。
- 公钥是可公开提交的 minisign 信任根；私钥和密码只存在于 GitHub Actions Secrets 与开发者控制的离线备份中。
- 发布工作流只能由 `workflow_dispatch` 触发，先运行 `npm run check`，再生成 Windows x64 NSIS、`.sig` 和 `latest.json`。
- Release 必须先为 Draft；版本、说明、资产 URL 和签名核对完成后才可发布。
- `tauri-action` 的 `releaseBody` 会同时写入 Release 描述和 `latest.json.notes`，因此工作流中必须直接提供可公开的最终说明，不能使用“稍后补充”占位文案。事后只编辑 Draft 描述不会重写已上传清单。
- `tauri-action` 生成的 `latest.json` 可以把平台 URL 写成 GitHub REST asset API（`https://api.github.com/repos/<owner>/<repo>/releases/assets/<id>`）。当前锁定的 `tauri-plugin-updater 2.10.1` 在下载包时会在缺少 `Accept` 的情况下自动加入 `Accept: application/octet-stream`，因此不应仅凭浏览器或普通 GET 返回资产元数据就判定清单失效。
- 第一个带 updater 的版本需要手动安装；只有更高 SemVer 版本才能验证应用内升级。
- Tauri 更新签名与 Windows Authenticode 相互独立。没有 Authenticode 证据时，安装器仍按“未知发布者”报告。
- Sandbox staging 必须位于系统临时目录的真子路径，目标及其现有父路径不得包含 junction/symlink 等 `ReparsePoint`，且既有 staging 必须为空；安装器源路径不得位于真实 `.codex` 或 `%LOCALAPPDATA%\CodexRelay`。输入映射为只读，结果映射为可写，剪贴板和打印机重定向关闭。
- 真实 guest 只接受 `WDAGUtilityAccount` 且脚本根为 `C:\CodexRelaySandbox`；dry-run 只接受系统临时目录。`CODEX_RELAY_CODEX_HOME` 与 `CODEX_RELAY_APP_DATA_DIR` 必须成对指向 guest 的 `dev-data` 子目录。
- NSIS 注册表中的 `InstallLocation` 可能是 `"C:\Program Files\Codex Relay"` 形式。`guest-start-app.ps1` 与 `guest-verify.ps1` 必须共同使用 `Get-CanonicalInstallLocation`：先剥离一对完整外层双引号，再执行路径规范化；空值或仍含双引号的值返回 `SANDBOX_INSTALL_LOCATION_INVALID`。
- `before.json` 只记录三个受保护 fixture 的相对路径、长度和 SHA-256；`after.json` 记录版本、安装目录、可执行文件存在性和逐文件比较结果。报告不得包含 fixture 内容或 `test-key-*` 文本，每个白名单路径必须恰好出现一次。

## 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| 未设置签名 Secrets 的普通构建 | `npm run build` 仍成功，不生成 updater 签名资产 |
| 未设置或错误的签名 Secrets 的发布构建 | 构建失败，不创建可发布成功状态 |
| endpoint 断网、非 2xx、无效 JSON 或无目标平台 | 检查进入安全错误状态，本地 Provider 功能继续可用 |
| updater 资产签名无效 | 不启动 NSIS，不报告更新成功 |
| Release 仍为 Draft | `releases/latest` 客户端不可消费该版本 |
| `latest.json.notes` 仍为占位文案 | 不得发布；修正工作流并重新生成 Draft 资产 |
| 平台 URL 是 GitHub REST asset API | 用当前锁定 updater 的二进制请求语义验证；请求必须携带 `Accept: application/octet-stream`，下载字节数和 SHA-256 必须与 Release 资产一致 |
| UAC 取消或安装器失败 | 不得报告升级成功；允许用户重新打开旧版本 |
| 私钥丢失或公钥更换 | 通过手动安装更高版本恢复，不原地替换已发布资产 |
| staging 不在临时根、非空或经过 reparse point | 分别返回 `SANDBOX_STAGE_ROOT_MUST_BE_TEMPORARY`、`SANDBOX_STAGE_ROOT_NOT_EMPTY` 或 `SANDBOX_STAGE_ROOT_REPARSE_POINT`，不读取安装器、不创建目录 |
| guest 身份/根目录不符或两项覆盖越界 | 返回 `SANDBOX_GUEST_REQUIRED` 或 `SANDBOX_BEFORE_REPORT_PATH_UNSAFE`，不启动安装器或应用 |
| NSIS 安装目录带一对外层双引号 | 去除外层引号后得到规范绝对路径，start 与 verify 使用同一结果 |
| 安装目录为空或剥离外层引号后仍含双引号 | 返回 `SANDBOX_INSTALL_LOCATION_INVALID`，不启动应用且不生成成功核验 |
| 安装器 SHA-256 不符 | 返回 `SANDBOX_INSTALLER_HASH_MISMATCH`，不生成 `.wsb` |
| 安装器源位于真实 Codex/Relay 数据目录 | 在解析或读取文件前返回 `SANDBOX_INSTALLER_PATH_UNSAFE`，不创建 staging |
| 升级前报告缺项、重复项或未知项 | 返回 `SANDBOX_BEFORE_REPORT_FILES_INVALID`，不生成成功报告 |
| 版本不符、可执行文件缺失或 fixture 哈希变化 | 写出 `after.json` 后返回 `SANDBOX_UPDATE_VERIFICATION_FAILED`，不得声明升级成功 |

## 5. 良好、基线与错误用例

- 良好：用户点击检查，发现更高版本，下载资产通过内置公钥校验，NSIS 沿用已登记安装目录完成升级。
- 基线：没有用户点击时不访问网络；普通本地构建在没有任何签名环境变量时生成常规 NSIS。
- 基线：GitHub API asset URL 的普通 GET 返回 JSON 元数据，但 updater 通过 `Accept: application/octet-stream` 获取实际安装器，下载哈希与 Release 资产一致。
- 错误：在基础配置开启 `createUpdaterArtifacts`，导致每次本地构建都要求私钥。
- 错误：只用浏览器打开或普通 GET 检查 GitHub API asset URL，看到 JSON 后直接替换已发布资产或判断 Release 损坏。
- 错误：把私钥内容、公钥对应密码或 GitHub Token 写入仓库、任务材料、日志或命令行参数。
- 错误：发布后替换同一版本的二进制或 `.sig`；修复必须使用新的更高 SemVer。
- 良好：先核对公开安装器 SHA-256，再由主机准备器生成临时 `.wsb`；guest 只写测试数据到显式覆盖，并把无密钥哈希报告写回专用结果映射。
- 基线：HKLM 中的 `InstallLocation` 为带外层双引号的绝对路径；安全启动和升级后核验剥离同一对引号后定位已安装 EXE。
- 错误：把带引号注册表字符串直接传给 `Path.GetFullPath`；它可能被视为含非法字符或相对路径，导致已安装 EXE 被误报为不存在。
- 错误：把仓库、真实 `.codex`、真实 `%LOCALAPPDATA%\CodexRelay` 或经过 junction 的目录直接映射为可写 Sandbox 目录。

## 6. 必需测试

- `src/release-config.test.ts` 断言固定 HTTPS endpoint、公开公钥、`updater:default` 权限、基础/发布配置分离、Secret 名称、手动触发、Draft 和 NSIS 优先。
- 工作流结构测试断言 `releaseBody` 使用多行最终说明，并明确拒绝占位文案，防止占位内容进入 `latest.json.notes`。
- composable 测试断言创建时不检查远端、显式检查、单飞、旧响应防护、确认、进度和安全失败状态。
- 服务测试只 mock 官方 updater 与本地版本 API，断言 DTO 规范化、资源释放和错误脱敏。
- 在显式移除两个签名环境变量后运行 `npm run build`，枚举 EXE/NSIS 的实际路径、大小和 SHA-256。
- 在真实 GitHub Actions 中核对 Draft Release 的 NSIS updater 资产、`.sig` 与 `latest.json`。
- 若 `latest.json` 使用 GitHub REST asset API URL，确认锁定的 updater 版本会补充 `Accept: application/octet-stream`，并用等价公开请求下载实际资产、核对大小和 SHA-256；依赖升级后必须重新验证这一行为。
- 在 Sandbox/VM 中用首个手动安装版本和更高版本验证 UAC、安装目录、重启与数据保留；未执行场景必须明确记录为未验证。
- `src/sandbox-update.test.ts` 必须执行主机准备、guest bootstrap/start/verify dry-run，并断言映射权限、成对覆盖、报告脱敏、受保护路径唯一性、reparse point 拒绝和真实路径源文件拒绝。
- start/verify dry-run 必须把安装目录包装为一对外层双引号，并断言输出及 `after.json.installLocation` 等于无引号的规范绝对路径；防止测试只覆盖手写的理想注册表值。
- 真实 Sandbox 启动后，必须观察 `before.json` 已写回再安装基线；升级后必须运行桌面核验入口并读取 `after.json`，不能用进程存在或窗口出现代替数据保留证据。

## 7. 错误与正确做法

错误：让所有构建都依赖签名私钥。

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  }
}
```

正确：基础配置只保存公开信任根，发布覆盖单独开启签名资产。

```json
// src-tauri/tauri.conf.json
{
  "plugins": {
    "updater": {
      "endpoints": ["https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json"],
      "pubkey": "<公开 minisign 公钥>"
    }
  }
}

// src-tauri/tauri.updater.conf.json
{
  "bundle": {
    "createUpdaterArtifacts": true
  }
}
```

错误：把 GitHub API asset URL 的默认 JSON 响应当作 updater 实际下载结果。

```powershell
curl.exe -L "https://api.github.com/repos/<owner>/<repo>/releases/assets/<id>"
```

正确：按 updater 的下载请求语义验证公开资产，并将结果与 Release API 公布的大小和 SHA-256 对照。

```powershell
curl.exe -L -H "Accept: application/octet-stream" `
  "https://api.github.com/repos/<owner>/<repo>/releases/assets/<id>" `
  -o updater.exe
Get-FileHash -Algorithm SHA256 updater.exe
```

错误：把可写 staging 放到未经约束的路径，或直接双击未经哈希核对的安装器。

```powershell
Start-Process .\downloaded-installer.exe
```

正确：显式提供公开资产哈希和目标版本；默认 staging 由脚本在系统临时目录创建并检查 reparse point。

```powershell
.\scripts\windows-sandbox\prepare-update-test.ps1 `
  -InstallerPath $installer `
  -ExpectedSha256 $publishedSha256 `
  -ExpectedTargetVersion '0.1.1'
```

错误：直接规范化可能带引号的 NSIS 注册表值。

```powershell
$installLocation = Get-CanonicalPath -Path $installation.InstallLocation
```

正确：通过共享 helper 统一 start/verify 的引号和路径规则。

```powershell
$installLocation = Get-CanonicalInstallLocation -Path $installation.InstallLocation
```
