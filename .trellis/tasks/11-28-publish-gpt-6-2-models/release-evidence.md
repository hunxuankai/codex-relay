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

## 候选与 Actions

- 候选 `fa26ef77101b9ffcf23ffc72b6cd735fc9795c58`。暂存范围、差异空白检查、凭据模式扫描和最终 SHA 的发布请求校验均通过。普通推送 `HEAD:refs/heads/main` 返回 `5047610..fa26ef7`；本地 HEAD、跟踪分支、远端 Git ref 与 GitHub API 查验一致。
- UTC `11:20:29` 仅触发一次 `release.yml`，输入 `expected_version=0.5.5` 和候选完整 SHA，得到 Run [46139422066](https://github.com/hunxuankai/codex-relay/actions/runs/46139422066)。没有推送 tag。
- 临时审计探针直接链接旧缓存产物时被 Rust 版本兼容检查拒绝；随后用 Rust 1.100.0 和本地依赖构建成功，未改发布控制台代码。生产公开审计路径验证既有 `v0.5.4`，实际安装器的 updater 签名通过，翻转一字节的负向对照被拒绝。
- 匿名读取旧版固定 updater 端点与 asset API 成功，实际 `latest.json` 和 `.sig` 的 SHA-256 与基线一致。
- Run `46139422066` 是 `main` 上绑定候选 SHA 的唯一 `workflow_dispatch`、attempt 1。监视在约 13 分钟时遇到 EOF 退出 1；恢复时仅查询原 Run，确认 `completed/success`，未重新触发。
- Run UTC `11:20:32` 创建，Job `140961832344` 于 `11:20:35` 开始、`11:34:33` 结束：排队 3 秒，Job 13 分 58 秒；完整检查 `11:21:35`–`11:28:53`，7 分 18 秒；Draft 构建 `11:28:53`–`11:34:16`，5 分 23 秒，均成功。

## Draft 审计

- Draft Release `461511927`，标题 `Codex Relay v0.5.5`，`target_commitish` 为候选 SHA，`draft=true`、`prerelease=false`；远端仍只有 `v0.5.4` tag。
- 初次生产审计报告 `AssetDownloadFailed`，未公开。分别对同三项资产做只读下载，大小/hash 均匹配；随后只重试同一 Draft 审计，UTC `11:37:33` 退出 0。未据此推定 GitHub 故障原因，也未产生新候选或资产。
- 生产 `DraftAuditService` 验证最终说明、三项资产大小和 SHA-256、manifest 版本 `0.5.5` 与说明、两个 Windows 平台的安装器 URL、内联签名及独立签名一致。使用仓库公开 updater 公钥验证实际 NSIS 的 minisign 签名通过，翻转一个字节后被拒绝。安装器 FileVersion `0.5.5`，Authenticode `NotSigned`。

| Draft 资产 | GitHub ID | 字节 | SHA-256 |
| --- | ---: | ---: | --- |
| `Codex.Relay_0.5.5_x64-setup.exe` | 804334092 | 4719064 | `56359085d57dd4ed0bbffb7ad3e8e5ec450b60da4378f8c05358fce306b83824` |
| `Codex.Relay_0.5.5_x64-setup.exe.sig` | 804334143 | 424 | `f65b669b2c985f33030c1f1b483ce235047e8374a81cef66aa1857cb7b53c381` |
| `latest.json` | 804334174 | 2151 | `08f22b940de5d8a7da8d3365986e5b01baf1ad35183f3c56a154514089834009` |

## 正式公开与复核

- 使用生产 `GithubReleaseService::publish_release`，在更新前完整重审并比对同一 Draft，只按 Release ID 单次公开。正式 Release `461511927`（`v0.5.5`）公开时间 `2026-11-28T11:39:01Z`，tag 指向 `fa26ef77101b9ffcf23ffc72b6cd735fc9795c58`。
- `verify_published_release` 验证在线 Latest、tag、说明与三项资产和 Draft 一致，实际公开安装器的 minisign 签名及篡改负向对照再次通过；UTC `11:39:34` 退出 0。
- UTC `11:40:53` 独立匿名检查退出 0。Latest 和按 tag 查询命中同一公开 Release，三项资产下载的大小及 SHA-256 与 Draft 一致。固定 latest 及按 tag 下载的两份清单字节一致，两项 Windows 平台的 URL 与签名正确；手动安装器下载一致、Release 页面 HTTP 200。公开 NSIS FileVersion `0.5.5`，Authenticode `NotSigned`。

## 历史清理

- 唯一公开事件触发清理 Run [46140874028](https://github.com/hunxuankai/codex-relay/actions/runs/46140874028)，`release` 事件、`v0.5.5` 标签，候选 SHA 一致。UTC `11:39:02` 创建，Job `140966291212` 在 `11:39:04`–`11:39:09` 成功；清理步骤用时 3 秒。
- UTC `11:40:14` 仅正式 `v0.5.5`、其三个已核验资产及 tag `v0.5.5` 保留；旧 Release `460212909` 和以前三个资产 API 均返回 404。未手动删除远端对象或另行触发清理。

## 限制与收尾

- 未执行实机或 Sandbox 安装、应用内升级、UAC、自动重启、卸载、真实用户数据保留或供应商调用验证。已验证的构建、托管和 Tauri 更新签名不代表上述行为或 Authenticode 已通过。
- 临时审计目录及独立探针是本轮生成物；自动清理命令被工具策略拦截，残留路径只在当前会话提醒，不纳入仓库。
- 发布证据提交后归档任务、记录日志，再普通推送到 `origin/main`；发布 tag 保持指向候选提交，最终 HEAD 与远端以归档后 Git 核验为准。
