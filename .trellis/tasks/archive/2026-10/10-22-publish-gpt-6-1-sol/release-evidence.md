# v0.5.3 发布证据

## 初始与候选

- 初始 main：`80b1730069fe408201c50518481d9ee125c6b301`，与 origin/main 一致。
- 上一公开版本：v0.5.2，Release ID `386122082`，目标 `fe39d72a2f88898ce1db7e476cacac1c90f34857`，公开 UTC `2026-09-10T11:04:53Z`。
- 目标：v0.5.3；预检确认无目标 Release/Tag，无活动发布 Run，Actions operational，updater Secret 名称完整。

## 本地验证

Rust 1.99.1 / Clippy 0.1.99 / Node v22.20.0。发布结构专项 19 项和最终 `npm run check`、普通 `npm run build` 均退出 0。
完整检查：Trellis 8 项，根 Vitest 356 项，发布控制台 Vitest 107 项，Rust 485 项通过、1 项既有 ignored。
检查/构建使用成对临时 Relay 路径，不接触真实 Codex/Relay 数据；构建进程没有 updater 签名变量。

| 本地产物 | 字节 | 最后写入 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19446272 | 2026-10-22T12:20:39.7281823Z | `f29868a88ba9663de83cb94097eb13492cd2beb22426bec659f1ca18e34f70bc` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.3_x64-setup.exe` | 4704881 | 2026-10-22T12:20:39.6696330Z | `7999ca6b1286ddde373b329ddffb475cad7de67365ff7bfdd1ea9e1732d9c5f6` |

两个产物 FileVersion 均为 0.5.3、Authenticode 均为 `NotSigned`。普通构建没有生成本版 `.sig` 或 `latest.json`。

## 候选与 CI（恢复时实时核对）

- 候选：`d381c9a1eb0057ce1495da9cf54fcb38cc9c43fb`（`chore(release): 准备 v0.5.3 发布`）。恢复时本地 HEAD、origin/main 和远端 main 查询一致。
- 唯一发布 Run：[39785972510](https://github.com/hunxuankai/codex-relay/actions/runs/39785972510)，事件 `workflow_dispatch`，分支 `main`，候选 SHA 精确匹配。
- 创建 UTC `2026-10-22T13:20:31Z`，Job 开始 `13:20:35Z`、结束 `13:38:58Z`；创建至 Job 开始 4 秒，Job 执行 18 分 23 秒。
- “运行完整检查”从 `13:21:24Z` 至 `13:31:49Z`，10 分 25 秒，结论 success；“构建 Draft Release”从 `13:31:49Z` 至 `13:38:11Z`，6 分 22 秒，结论 success。
- Draft Release ID `420770763`，`v0.5.3`，目标候选一致，`draft=true`、`prerelease=false`。恢复轮次没有重新触发 Run。

## Draft 审计与更新签名

恢复轮次执行已有临时探针的 `audit`，直接调用生产 `SystemGhBackend` / `DraftAuditService`，退出 0。Release ID `420770763`、tag `v0.5.3`、候选 SHA、标题、Draft/非预发布状态及最终说明均匹配。

| 资产（GitHub ID） | 字节 | SHA-256 |
| --- | ---: | --- |
| `Codex.Relay_0.5.3_x64-setup.exe`（665987590） | 4706225 | `604b6a32b2dd72da955ee0dc649387c1c626d5b3467bf1913de340c2246c86bd` |
| `Codex.Relay_0.5.3_x64-setup.exe.sig`（665987635） | 424 | `ee49bea9d222c0138dae868e6d20e93d1f3e91733bf99d44fd376d2c2f78f832` |
| `latest.json`（665987685） | 2310 | `521262e2e74f019e7d1f9064e7af855e39ea846235c3e511940da1293de296f0` |

- 实际下载大小、SHA-256 与 GitHub digest 一致，资产数量严格为三个。
- 清单版本 `0.5.3`，说明与候选正文一致（仅规范化行尾和末尾空白）；两个 Windows x64 平台指向同一安装器资产，清单内联签名与独立 `.sig` 一致。
- 使用仓库公开更新公钥和 `minisign-verify` 独立验证真实安装器，通过；将安装器第一个字节翻转后验证失败，负向对照通过。
- 只使用系统临时目录和成对 Relay 覆盖，子进程中移除两个 updater 签名环境变量；没有读取私钥，也未运行安装器。
- 下载的 CI 安装器 FileVersion 为 `0.5.3`，Authenticode 为 `NotSigned`。恢复时重算本地两项普通构建产物的哈希，均与前述本地构建记录一致；本地与 CI 构建产物分别记录，不要求二者哈希相同。

## 恢复轮次的诊断与审查

- 恢复轮次重新执行 `npm run check`，UTC `14:12:47` 至 `14:27:18`，退出 0。Trellis 8 项、根前端 60 文件/356 项、发布控制台前端 17 文件/107 项、Rust 485 项通过，0 失败、1 项既有 ignored。类型检查、fmt、严格 Clippy、依赖图及路径安全均通过；Rust 测试编译 7 分 17 秒。
- 该检查使用 Rust 1.99.1，成对 Relay 覆盖位于既有安全临时根的 `resume-check-codex` / `resume-check-app-data`，进程中移除两个 updater 签名环境变量。完整输出和退出码保留在临时 `resume-check.log` / `resume-check.exit`。
- 首次独立重验发布请求时漏传 `GitHubOutput`，脚本按预期在缺少输出文件时退出 1；核对脚本第 121 行后，以系统临时输出文件补齐参数，使用实际 HEAD 重新验证并退出 0。此为调用环境问题，没有修改验证脚本或产品代码。
- 候选差异只包含六个版本/说明文件及本任务资料，依赖版本没有漂移；本轮产品源码和发布工作流没有变化。
- 全部 Git 跟踪文件的高置信度秘密前缀扫描无命中，也未跟踪 `auth.json` 或 `providers.json`。
- 当前本地/远端只有 `main` / `origin/main`，仅一个工作树，没有需要合并或清理的开发分支。
- 长期规范已覆盖同一 Run 恢复、Draft/公开审计、更新签名和按标签识别 cleanup；本次没有新的代码契约需要更新规范。

## 公开发布与在线复核

- 生产 `GithubReleaseService::publish_release` 在 PATCH 前完整重审同一 Draft，按 Release ID 公开，探针退出 0。
- 公开 Release ID `420770763`、tag `v0.5.3`，公开 UTC `2026-10-22T14:29:33Z`（北京时间 22:29:33）。
- Release 页面：<https://github.com/hunxuankai/codex-relay/releases/tag/v0.5.3>。
- 公开后生产 `verify_published_release` 审计退出 0：Latest 身份、正式 tag 的 commit SHA、说明及三项资产证据与 Draft 完全一致；实际公开安装器的 minisign 验证及篡改负向对照再次通过。
- UTC `14:33:31` 完成无认证 HTTP 核验，退出 0：公开 Latest 与 tag 指向同一候选，资产 API 以 `Accept: application/octet-stream` 返回的三个文件大小/哈希均与 Draft 相同。
- 固定 `releases/latest/download/latest.json` 与 `releases/download/v0.5.3/latest.json` 字节相同；清单版本、说明、两个 Windows x64 平台 URL、内联/独立签名均一致。手动安装器下载链接也返回相同安装器字节。

## 历史清理监控

- 公开事件触发唯一清理 Run：[39796115034](https://github.com/hunxuankai/codex-relay/actions/runs/39796115034)，事件 `release`，分支 `v0.5.3`，候选 SHA 相同，创建 UTC `14:29:37Z`。
- 首次生产清理监控退出 1，仅暴露 `BackendFailed`，未取得底层错误码，保留原始 `cleanup.log` / `cleanup.exit`。随后只读查询同一 Run 成功，返回 queued；没有重新触发发布或清理工作流。
- GitHub 状态接口当时显示 Actions 为 `degraded_performance`（状态更新 UTC `14:18:15Z`），Git Operations / API Requests 为 operational；这项外部状态与查询错误分别记录，不据此推定底层错误原因。
- 官方事故 [Incident with Actions Results](https://stspg.io/44w94j036yc8) 的 UTC `14:35:29` 更新说明工作流与步骤结果回传存在延迟；后续同一 Run 由 queued 变为 in_progress。当前 Release/Tag 列表先于 Run 终态显示清理后的结果，仍按独立证据核对。
- 恢复生产只读监控后退出 0，仍为同一 Run `39796115034`，最终 `completed/success`；Job UTC `14:29:40` 至 `14:29:46`，执行 6 秒；实际删除步骤从 `14:29:41` 至 `14:29:45`，执行 4 秒。结果回传等待与实际执行时间分别记录。
- 清理前快照：正式 `v0.5.2`（Release ID `386122082`）及 Draft `v0.5.3`（`420770763`），各有安装器、`.sig`、`latest.json` 三项资产；当时远端 tag 为 `v0.5.2`。
- 最终列表复核：仅正式 `v0.5.3`（`420770763`）及其三个已审计资产，远端 tag 仅 `v0.5.3`。旧 `v0.5.2` Release、其资产和 tag 已不在列表中，当前 Latest 保留。
- 本轮仅公开既有 Draft，并观察公开事件触发的清理；没有重新 dispatch、重新 publish、手动删除远端对象或本地推送 tag。

## Git 收尾

本轮产品代码未变更，工作提交仅包含本任务 PRD、实施检查点及发布证据。按既定顺序提交工作材料、归档任务、记录会话日志，最后普通推送已配置的 `origin/main`。最终提交哈希、推送输出和 HEAD/上游/远端一致性由 Phase 3.5 的实际命令核对并在交付响应中报告。

## 限制

未执行真实安装、应用内升级、UAC、自动重启、卸载或真实用户数据保留测试。构建/托管发布/更新签名证据不能替代这些行为。
