# v0.5.4 发布证据

## 预检与回退基线

- `main`、`origin/main` 与远端 `refs/heads/main` 初始均为 `08dd3ad5d85c2cc8de8396a48383743e3638fbf8`。仅有主分支和一个工作树；目标版本尚无 Release/Tag，无活动发布 Run。
- 公开 Latest `v0.5.3` 为 Release `420770763`，tag 绑定 `d381c9a1eb0057ce1495da9cf54fcb38cc9c43fb`。本轮已匿名下载和核对旧固定 Latest、带 tag 的清单与三项资产，和上次记录的实际 SHA-256 一致；未运行安装器。
- 发布工作流启用，两个 updater Actions Secrets 的名称存在，未读取值。初始 GitHub Actions、Git Operations 和 API Requests 状态均为 `operational`。

## 本地候选

- 更新六个版本与说明文件；应用 Tauri 配置、更新公钥和安装器模板不改。发布结构、请求和历史清理专项 19 项通过；候选内容的独立发布请求校验退出 0。
- 首次 `npm run check`（Rust 1.99.1）前端 332 项、发布控制台 107 项通过；严格 Clippy 以 `chunks_exact_to_as_chunks` 拦下 Windows 环境变量解码，整体退出 101，未启动构建。core 专项 Clippy 在修复前复现退出 101，改用固定两字节切片并显式检查余数后退出 0，未屏蔽 lint。
- Rust 1.100.0 下直接对比修复前后两段解码函数：覆盖全部 65,536 个 UTF-16 单元的正常对齐与奇数长度输入，以及缺终止符、内嵌 NUL、合法代理对等，总共 131,077 组结果一致。
- GitHub Actions 使用 `stable`；根据官方工具链清单安装 Rust 1.100.0，未切换全局默认。安装器的自动 rustup 自更新报告 `cargo-miri.exe` 已存在；随后的 `rustc +1.100.0` 和 Clippy 版本检查退出成功，独立工具链可用。
- 所有本地验证使用系统临时根的成对 Relay 路径，进程中移除 updater 签名环境变量。未读取真实 Codex/Relay 数据或签名 Secret。
- 最终 `npm run check` 在 UTC `05:01:54` 开始、`05:20:05` 退出 0。Trellis 8 项、主前端 59 文件/332 项、发布控制台 17 文件/107 项、Rust 462 项通过、0 失败、1 项既有 ignored；类型检查、格式、严格 Clippy、依赖图与路径安全门禁通过。Rust 测试编译 9 分 20 秒；随机隔离注册表子键中的原生提交、回滚与竞争写测试也通过。
- 既有规范已要求匹配 CI stable 工具链、严格 Clippy 和安全路径；本次等价语句重写未引入新产品契约，不修改长期规范。

### 普通 Windows 构建

`npm run build` 在 UTC `05:20:05` 开始、`05:31:02` 退出 0，Rust 优化构建 9 分 56 秒。无 updater 签名变量，未生成本版 `.sig` 或 `latest.json`。两项产物的 FileVersion 均为 `0.5.4`，Authenticode 均为 `NotSigned`。Vite/Rollup 报告既有依赖注释与大包非阻塞警告。

| 产物 | 字节 | 最后写入 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19454464 | 2026-11-27T05:31:02.8892989Z | `acd4208c41e66c36a3a403238cf8bfdb11729078e05b2e5928a918ac865b3247` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.4_x64-setup.exe` | 4714482 | 2026-11-27T05:31:02.7586486Z | `9478a94d674f8c0477b253ee3dc557b21518c263d4cf4bbd1ae387372b17f9b5` |

## 候选与 GitHub Actions

- 候选为 `094d96ff6c7b569d442f85da84d7fa5be6821d31`（`chore(release): 准备 v0.5.4 发布`）。最终 SHA 上的发布请求校验、暂存范围和高置信度凭据扫描通过。
- 普通推送 `HEAD:refs/heads/main` 返回 `08dd3ad..094d96f`。推送后的第一次 `git ls-remote` 因瞬时 TLS 连接错误退出；仅重试只读查询，后续 Git 远端、GitHub ref API、本地 `HEAD` 及 `origin/main` 均核对一致。没有重复推送或推送 tag。
- 仅触发一次 `release.yml`，输入 `expected_version=0.5.4`、`expected_sha=094d96ff6c7b569d442f85da84d7fa5be6821d31`。确认唯一 Run [45946562364](https://github.com/hunxuankai/codex-relay/actions/runs/45946562364)，事件 `workflow_dispatch`、分支 `main`、候选一致、attempt `1`，创建 UTC `05:33:59`。
- 触发响应直接返回该 Run URL。辅助列表定位首次将已解析的 UTC 时间重新按本地时间转换，误判未发现；用原 Run ID 查验，再按保留 ISO 字符串的 UTC 查询确认候选仅有这一个 Run，未重复触发。Job UTC `05:34:03` 开始；完整检查 `05:34:36`–`05:41:45` 成功，随后进入 Draft 构建。
- 临时签名探针的首次编译因 `sha2` 输出不实现 `LowerHex` 失败；仅修正临时探针的逐字节十六进制格式化。重编译成功后，控制台生产审计路径核对已公开 `v0.5.3` 基线成功，更新签名通过、翻转一个字节的负向对照被拒绝。没有修改产品发布服务。
- Job `140332492425` 自 UTC `05:34:03` 至 `05:46:55` 执行 12 分 52 秒，创建到 Job 启动 4 秒。构建 Draft 步骤 UTC `05:41:45`–`05:46:42` 成功（4 分 57 秒）；Run 最终 `completed/success`。

## Draft 审计

- Draft Release `460212909`，标题 `Codex Relay v0.5.4`，`target_commitish` 为候选 SHA，`draft=true` 且 `prerelease=false`。当时远端仍只有 `v0.5.3` tag；没有提前创建或推送 tag。
- 生产 `DraftAuditService` 下载并校验三项实际资产的大小、SHA-256 与 GitHub digest。`latest.json.version` 是 `0.5.4`，说明和候选相同（只规范化行尾及末尾空白）；`windows-x86_64` 和 `windows-x86_64-nsis` 均指向此 NSIS，独立签名与内联签名一致。
- 使用仓库公开更新公钥验证实际安装器 minisign 签名通过；翻转一个字节后负向对照被拒绝。安装器 FileVersion `0.5.4`，Authenticode `NotSigned`。Draft 审计 UTC `05:47:47` 完成且退出 0。

| Draft 资产 | GitHub ID | 字节 | SHA-256 |
| --- | ---: | ---: | --- |
| `Codex.Relay_0.5.4_x64-setup.exe` | 799160259 | 4715763 | `c1919ea259c621df8f941122b28be039231d7e74e513063b8b7e68d84703749d` |
| `Codex.Relay_0.5.4_x64-setup.exe.sig` | 799160320 | 424 | `c21320d9da0893d5837bf142127cd9f7554ae18b189e052d191cd667a998d205` |
| `latest.json` | 799160350 | 2532 | `6f007a0aade20605b1820f0124b3221609d8fb0c4a10e1bdc1ab21fdb1ff890e` |

## 正式公开与线上复核

- 生产 `GithubReleaseService::publish_release` 在 PATCH 前完整重审并比对同一 Draft，按 Release ID 单次公开。正式 Release `460212909`（`v0.5.4`）公开时间为 `2026-11-27T05:48:34Z`，候选 SHA 不变。
- 生产 `verify_published_release` 核对 Latest、tag 引用、说明和三项资产与 Draft 证据一致，实际公开安装器的 minisign 签名及篡改负向对照再次通过，退出 0。
- 第一次无认证安装器下载遇到 schannel TLS 握手失败；只重试读取，未重新发布。UTC `05:50:34` 无认证核验退出 0：公开 Latest、按 tag 查询和 tag ref 身份一致；三项资产以 `application/octet-stream` 下载的实际大小和哈希均与 Draft 一致。
- 固定 `releases/latest/download/latest.json`、按 tag 下载的清单和已审计清单字节相同；两项 Windows 平台 URL 与内联/独立签名均一致。手动安装器下载同样匹配，正式 Release 页面返回 HTTP 200。

## 历史清理

- 正式公开事件触发唯一清理 Run [45947994962](https://github.com/hunxuankai/codex-relay/actions/runs/45947994962)，事件 `release`、标签 `v0.5.4`、SHA 与候选相同。创建 UTC `05:48:36`，Job `05:48:38`–`05:48:44`，`completed/success`。
- 清理前是正式 `v0.5.3`（ID `420770763`、资产 `665987590`/`665987635`/`665987685`）和 Draft `v0.5.4`，远端 tag 只有 `v0.5.3`。UTC `05:49:27` 复核仅正式 `v0.5.4`、其三项已审计资产及 tag `v0.5.4` 保留；旧 Release 与三个资产 API 均返回 HTTP 404。没有人工删除远端对象或单独触发清理。

## 限制与收尾

- 未运行安装器或启动 Sandbox/VM 应用进行真实安装、应用内升级、UAC、自动重启、卸载和用户数据保留测试；构建、下载与 updater 签名证据不等同于这些行为。更新签名不代表 Windows Authenticode。
- 任务证据提交后按 Trellis 流程归档并记录会话日志，再普通推送至已配置的 `origin/main`。发布 tag 保持指向候选提交，最终工作树、上游与远端一致性由归档后的命令核对并在交付中报告。
