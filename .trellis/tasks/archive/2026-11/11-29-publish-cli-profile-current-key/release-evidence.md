# v0.5.6 发布证据

时间均为 UTC；本地执行日期为上海时间 2026-11-29。

## 预检与质量门禁

- 基线 `main`、`origin/main` 和远端 `refs/heads/main` 为 `49cfb37cb8148ac7e49ebb5e34002a52147c0d47`，包含功能 `dca1f0a`。该功能按 Windows 用户变量值在同一供应商中匹配已保存 Key 名称，不显示原文；不把它当成既有终端的进程环境。
- 初始唯一正式 Latest 为 `v0.5.5`（Release `461511927`，tag 指向 `fa26ef77101b9ffcf23ffc72b6cd735fc9795c58`），三个资产 ID 为 `804334092`、`804334143`、`804334174`。没有 `v0.5.6` Release、tag 或活动发布 Run。发布 workflow `317154946` 已启用，只核对了两个 updater Actions Secret 名称，未读取值。
- 在旧版本运行发布请求校验因 `packageJson=0.5.5` 退出 1，七处版本与最终中文发布说明同步后校验通过。发布请求、结构、配置和历史清理 Vitest 专项 4 个文件、25 项通过，5.2 秒，退出 0。
- `npm run check` 使用 Node.js `22.20.0`、Rust `1.97.1`，`17:16:11`–`17:24:36`（505.1 秒），退出 0。Trellis 8 项、前端 59 个文件/333 项、发布控制台独立套件 17 个文件/107 项、Rust 466 项通过，0 失败；Rust 另有 1 项既有 ignored。类型检查、Rust 依赖图、格式和严格 Clippy 通过。
- 测试及普通构建使用成对的临时 Relay 覆盖，不使用真实用户配置或应用数据；普通构建在同一进程隔离环境中移除两个 updater 签名变量，不读取其值。

### 普通 Windows 构建

`npm run build` 在 `17:24:36`–`17:27:26` 执行 169.9 秒，退出 0，未生成新版 updater `.sig` 或 `latest.json`。Vite 有既有第三方注释和包体积警告。构建后的首次文件采集把主程序名称写成 `codex-relay.exe` 而失败；随后按构建日志的实际名称独立校验两个新生成文件，均为 FileVersion/ProductVersion `0.5.6`、Authenticode `NotSigned`。

| 产物 | 字节 | 最后写入 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19530752 | 2026-11-28T17:27:26.2253553Z | `02a7ffc99d059111514c651cef3df7c64d071cc75c427a869b35f1514585062f` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.6_x64-setup.exe` | 4718108 | 2026-11-28T17:27:26.0897891Z | `13b54e05ad990d49f19a3c6b2e3499bfab279b2cc9cab9decafee870734c4735` |

## 候选与 Actions

- 候选 `4abc9ca6631e7f04e58e3946c47f63b4d1bacc31` 精确提交六个发布文件，暂存范围、差异空白检查、凭据模式扫描和提交后基于完整 SHA 的发布请求校验均通过。普通推送 `49cfb37..4abc9ca` 到 `origin/main`；本地 HEAD、跟踪分支、远端 Git ref 和 GitHub API 均回读为同一候选。
- 触发前复查 Actions、API Requests 和 Git Operations 为 `operational`，唯有基线 Latest 与旧 tag，不存在冲突 Draft 或活动 Run。`17:29:10` 仅调度一次，向 `release.yml` 传入 `expected_version=0.5.6` 和候选完整 SHA，得到 Run [46170716918](https://github.com/hunxuankai/codex-relay/actions/runs/46170716918)。Run 由 `workflow_dispatch` 于 `17:29:13` 创建，`main`、同 SHA、attempt 1；未自行创建或推送 tag。
- 临时审计探针直接链接时先遇到跨 Rust 工具链缓存、SHA-256 十六进制格式化及缺失 Windows 导入库路径；改用当前工具链匹配产物并补齐链接路径后编译通过，没有改变工作区代码或 updater 依赖。
- 等待 CI 期间，`17:31:05` 对旧版固定 updater 端点、两个 Windows 平台、Release 页和三项公开资产执行同一套匿名检查，下载结果与旧版已审计摘要相符。`17:41:35` 读取 Job 接口一次返回 EOF，监控只恢复读取原 Run，没有重新调度。
- Run `46170716918` 最终 `completed/success`，Job `141058310167` 在 `17:29:17`–`17:46:45` 成功，排队 4 秒，Job 用时 17 分 28 秒。全量检查 `17:30:14`–`17:40:02`（9 分 48 秒），Draft 构建 `17:40:02`–`17:46:33`（6 分 31 秒），均成功。

## Draft 审计

- Draft `461742287` 标题 `Codex Relay v0.5.6`，`target_commitish` 是候选完整 SHA，`draft=true`、`prerelease=false`。此时远端保持 `v0.5.5` 正式 Latest，尚无 `v0.5.6` tag。
- 使用生产 `SystemGhBackend` 和 `DraftAuditService::audit` 验证实际 Draft，`17:47:29`–`17:47:47`，18.3 秒，退出 0。核对最终中文发布说明和三项资产大小、SHA-256；`latest.json` 版本及说明匹配，`windows-x86_64` 和 `windows-x86_64-nsis` 的 URL 都指向安装器资产 `805441439`，两份内联签名与独立 `.sig` 内容相同。
- 使用仓库现有公开 updater 公钥和锁定的 `minisign-verify 0.2.5` 校验实际 NSIS 下载字节成功；翻转一个字节后返回 `InvalidSignature`。安装器 FileVersion/ProductVersion `0.5.6`，Windows Authenticode 状态 `NotSigned`。

| Draft 资产 | GitHub ID | 字节 | SHA-256 |
| --- | ---: | ---: | --- |
| `Codex.Relay_0.5.6_x64-setup.exe` | 805441439 | 4721073 | `a22cccac0f47b3384215b42dbbff297896c7a797183e2aefde6f2116abda8c6c` |
| `Codex.Relay_0.5.6_x64-setup.exe.sig` | 805441481 | 424 | `a19678a0f4d37889070921e0cec9b07654ddb894b1a786a71d19e37c931fe128` |
| `latest.json` | 805441503 | 2224 | `9ad4c569f7b1bca1f81a139960c22d6469d7095d99580194710ed26dfa0b6b25` |

## 正式公开与复核

- 生产 `GithubReleaseService::publish_release` 对同一 Draft 完整再审，按 Release ID `461742287` 单次公开，`17:48:14`–`17:48:27`，12.3 秒，退出 0。正式 Latest 发布于 `2026-11-28T17:48:27Z`；tag `v0.5.6` 指向候选 `4abc9ca6631e7f04e58e3946c47f63b4d1bacc31`。
- 公开后，`verify_published_release` 确认 Latest、tag、正文、下载字节和三项资产证据与同一 Draft 一致，`17:49:23` 退出 0。
- `17:49:26` 独立匿名检查退出 0：GitHub Release 页 HTTP 200，Latest API 与 tag Release API 返回同一 ID、状态与摘要；匿名 Asset API 三项下载均匹配已验签的 Draft 证据。固定 `releases/latest/download/latest.json` 与版本 URL 的清单摘要相同；两个 Windows 平台和签名未漂移。网页手动下载安装器的摘要也一致、FileVersion `0.5.6`、Authenticode `NotSigned`。

## 历史清理

- 历史清理 Run [46172438628](https://github.com/hunxuankai/codex-relay/actions/runs/46172438628) 是发布后的 `release` 事件，标签 `v0.5.6`、候选 SHA 一致，`17:48:28` 创建，Job 在 `17:48:31`–`17:48:37` 成功，清理步骤 3 秒。
- `17:49:17` 验证只保留正式 Release `461742287`、上述三项资产和 `v0.5.6` tag；旧 Release `461511927`、旧资产 `804334092`、`804334143`、`804334174` 及旧 tag `v0.5.5` 均返回 HTTP 404。没有手工删除远端对象或另行触发清理。

## 边界与收尾

- 未执行真实安装、Sandbox 或应用内升级、UAC、重启、卸载、实际用户数据保留或真实供应商调用。已验证 Tauri updater 资产签名，不代表存在 Windows Authenticode 发布者签名。
- 没有修改发布工作流、公钥、安装器配置或现有持久化契约，未发现需要改写的长期规范。发布 tag 固定在候选提交；发布记录与归档、会话日志按 Trellis 流程普通推送，最终 `main`/`origin/main`/GitHub ref 一致性以推送回读为准。
