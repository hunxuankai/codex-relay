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

## 3. 契约

- 唯一更新清单地址为 `https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json`，必须使用 HTTPS 且不能来自用户输入。
- 公钥是可公开提交的 minisign 信任根；私钥和密码只存在于 GitHub Actions Secrets 与开发者控制的离线备份中。
- 发布工作流只能由 `workflow_dispatch` 触发，先运行 `npm run check`，再生成 Windows x64 NSIS、`.sig` 和 `latest.json`。
- Release 必须先为 Draft；版本、说明、资产 URL 和签名核对完成后才可发布。
- `tauri-action` 的 `releaseBody` 会同时写入 Release 描述和 `latest.json.notes`，因此工作流中必须直接提供可公开的最终说明，不能使用“稍后补充”占位文案。事后只编辑 Draft 描述不会重写已上传清单。
- 第一个带 updater 的版本需要手动安装；只有更高 SemVer 版本才能验证应用内升级。
- Tauri 更新签名与 Windows Authenticode 相互独立。没有 Authenticode 证据时，安装器仍按“未知发布者”报告。

## 4. 验证与错误矩阵

| 条件 | 必需结果 |
|---|---|
| 未设置签名 Secrets 的普通构建 | `npm run build` 仍成功，不生成 updater 签名资产 |
| 未设置或错误的签名 Secrets 的发布构建 | 构建失败，不创建可发布成功状态 |
| endpoint 断网、非 2xx、无效 JSON 或无目标平台 | 检查进入安全错误状态，本地 Provider 功能继续可用 |
| updater 资产签名无效 | 不启动 NSIS，不报告更新成功 |
| Release 仍为 Draft | `releases/latest` 客户端不可消费该版本 |
| `latest.json.notes` 仍为占位文案 | 不得发布；修正工作流并重新生成 Draft 资产 |
| UAC 取消或安装器失败 | 不得报告升级成功；允许用户重新打开旧版本 |
| 私钥丢失或公钥更换 | 通过手动安装更高版本恢复，不原地替换已发布资产 |

## 5. 良好、基线与错误用例

- 良好：用户点击检查，发现更高版本，下载资产通过内置公钥校验，NSIS 沿用已登记安装目录完成升级。
- 基线：没有用户点击时不访问网络；普通本地构建在没有任何签名环境变量时生成常规 NSIS。
- 错误：在基础配置开启 `createUpdaterArtifacts`，导致每次本地构建都要求私钥。
- 错误：把私钥内容、公钥对应密码或 GitHub Token 写入仓库、任务材料、日志或命令行参数。
- 错误：发布后替换同一版本的二进制或 `.sig`；修复必须使用新的更高 SemVer。

## 6. 必需测试

- `src/release-config.test.ts` 断言固定 HTTPS endpoint、公开公钥、`updater:default` 权限、基础/发布配置分离、Secret 名称、手动触发、Draft 和 NSIS 优先。
- 工作流结构测试断言 `releaseBody` 使用多行最终说明，并明确拒绝占位文案，防止占位内容进入 `latest.json.notes`。
- composable 测试断言创建时不检查远端、显式检查、单飞、旧响应防护、确认、进度和安全失败状态。
- 服务测试只 mock 官方 updater 与本地版本 API，断言 DTO 规范化、资源释放和错误脱敏。
- 在显式移除两个签名环境变量后运行 `npm run build`，枚举 EXE/NSIS 的实际路径、大小和 SHA-256。
- 在真实 GitHub Actions 中核对 Draft Release 的 NSIS updater 资产、`.sig` 与 `latest.json`。
- 在 Sandbox/VM 中用首个手动安装版本和更高版本验证 UAC、安装目录、重启与数据保留；未执行场景必须明确记录为未验证。

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
