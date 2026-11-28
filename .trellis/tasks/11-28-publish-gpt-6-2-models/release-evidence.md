# v0.5.5 发布证据

## 预检

- 初始 `main`、`origin/main` 和远端 `refs/heads/main` 均为 `5047610091b9f6321d4b9ac44cd1088f1fad5e2c`，仅一个分支和工作树。远端默认分支是 `main`，目标版本暂无 Release 或 tag。
- 初始正式 Latest `v0.5.4`（Release `460212909`），tag 指向 `094d96ff6c7b569d442f85da84d7fa5be6821d31`。其三个资产为安装器 `799160259`、独立签名 `799160320`、更新清单 `799160350`，作为发布后保留规则的复核基线。
- 发布工作流已启用，预检确认两个 updater Actions Secret 名称存在，未访问值。初始 GitHub Actions、API Requests 和 Git Operations 均为 `operational`。

## 模型与本地门禁

- 从 OpenAI Codex 提交 `6c0cee502d215e5df2dd9a303cf2e252229c9e12` 中的 `codex-rs/models-manager/models.json` 核对两个新模型的强度、默认值和 Fast 能力。保持旧型号和持久化兼容逻辑不变。
- `npm run test:rust:provider-workflow -- editing_provider_to_gpt_6_2` 在目录实现前退出 101，两个用例分别因对应模型缺失失败。实现后 `npm run test:rust:provider-workflow -- editing_provider_to_gpt_6` 通过四项（退出 0）。验证六档推理强度、Fast、TOML 投影、原认证字节保留及非法 `none` 不改四个受管文件。
- 发布请求预检先发现 npm lock 顶层版本漏同步，修正后本地缺少 `GITHUB_OUTPUT`；传入独立临时输出路径后退出 0。发布结构、请求和历史清理专项 19 项通过。测试和普通构建使用成对的临时 Relay 覆盖，不接触真实用户数据或更新私钥。
- `npm run check`（Rust 1.100.0）从 UTC `11:04:59` 到 `11:15:08`，退出 0。Trellis 8 项、主前端 59 文件/332 项、发布控制台单独套件 17 文件/107 项、Rust 464 项通过，0 失败、1 项既有 ignored；格式、类型检查、严格 Clippy、Rust 依赖图和路径安全检查通过。

### 普通 Windows 构建

`npm run build`（同一隔离环境、移除两个 updater 签名变量）从 UTC `11:15:08` 到 `11:18:24`，退出 0。两项产物的 FileVersion 为 `0.5.5`，Authenticode 为 `NotSigned`，无本版 `.sig` 或 `latest.json`。Vite/Rollup 有依赖注释和包体积既有警告，不阻止构建。

| 产物 | 字节 | 最后写入 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19455488 | 2026-11-28T11:18:24.7072504Z | `1b6ea1489d332a583e424ed03d10dda333076041858676a6f2325164a1bc1480` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.5_x64-setup.exe` | 4714517 | 2026-11-28T11:18:24.2154358Z | `227bcaa23ae27bffa52f270892fb26932d8c1f3fe57ab8f5a3c8d3534b489b5d` |
