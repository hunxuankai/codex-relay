# v0.5.1 发布证据

## 候选与授权

- 用户要求增加 `gpt-6-astra` 并发布更新；目标 `v0.5.1`，上一公开版本 `v0.5.0`（Release ID `364697150`）。
- 当前本地分支 `master`，已配置上游 `origin/main`；候选发布需要先推送源码供 GitHub 构建，发布证据、归档和日志提交随后统一最终推送。
- 两项 updater 签名 Secret 名称存在；未读取或输出秘密。Windows Authenticode 未启用。

## 首个候选的本地验证（Rust 1.97.1）

- `npm run check`：退出 0；Trellis 8 项、根 Vitest 338 项、发布控制台 Vitest 89 项、Rust 463 项通过。
- Rust 1 项既有嵌套完整检查探针保持 ignored；已实际运行顶层完整检查，不把 ignored 计为通过。
- `path_safety` 3 项与 `provider_workflow` 2 项通过；所有本地执行使用系统临时目录下成对 Relay 路径覆盖。
- 无更新私钥普通 `npm run build`：退出 0；主程序和 NSIS 均为 `0.5.1`，未生成本次 updater `.sig`。

| 产物（仓库相对路径） | 字节数 | 最后修改 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19433984 | `2026-09-05T20:26:01.4680522Z` | `1ea25bad6d2225a114d35cbfae4d7a103350df29d60ad035114d230c12600a89` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.1_x64-setup.exe` | 4693216 | `2026-09-05T20:26:01.4049608Z` | `c942f046e20bb6eaf73f19335f2467b04edd3d15a4bd1dea46be55492785f3ec` |

两个产物的 `Get-AuthenticodeSignature` 状态均为 `NotSigned`；不能把后续 updater 签名描述为 Windows 发布者签名。

## 修复后候选的本地验证（Rust 1.98.0）

- 独立工具链：`rustc 1.98.0 (88d9e12ae 2026-08-18)`、`clippy 0.1.98`；通过命令进程的 `RUSTUP_TOOLCHAIN` 使用，没有修改项目依赖或降低 CI 门禁。
- Tauri 29 处外层错误改为 `InvokeError`，函数体归一化比较完全不变；业务 JSON/Promise 契约保持一致。
- 严格 workspace Clippy 退出 0；随后 `npm run check` 退出 0，Trellis 8 项、根前端 338 项、控制台前端 89 项、Rust 463 项通过及 1 项既有 ignored。
- 移除 updater 签名环境变量后 `npm run build` 退出 0；Release 冷编译约 9 分 27 秒。

| 产物（仓库相对路径） | 字节数 | 最后修改 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19445760 | `2026-09-05T22:40:22.0079569Z` | `49943aed9125865933bac0534894f8124bca90683d2d787a4c87a94756097f44` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.1_x64-setup.exe` | 4702342 | `2026-09-05T22:40:21.9406110Z` | `a84783b65f4a5907168819f3fd5e31037ea427cfebe1814f31cf654998bfaa79` |

两个产物版本均为 `0.5.1`，Authenticode 均为 `NotSigned`。

## 夹具修复后最终本地验证（Rust 1.98.0）

- core 9 项进程测试连续三轮全部通过；release-console 流式测试连续三轮通过，`local_verification` 全套 7 项通过、1 项既有 ignored。
- 最终 `npm run check` 和移除 updater 签名变量后的 `npm run build` 均退出 0。完整检查仍为 Trellis 8 项、根前端 338 项、控制台前端 89 项、Rust 463 项通过及 1 项既有 ignored。
- 普通 Release 重建约 2 分 48 秒；测试夹具修改之外，生产进程 runner 与 `031dbec` 中的实现逐字节一致。

| 产物（仓库相对路径） | 字节数 | 最后修改 UTC | SHA-256 |
| --- | ---: | --- | --- |
| `src-tauri/target/release/CodexRelay.exe` | 19445760 | `2026-09-06T01:32:43.7285139Z` | `204c9c32b3820480309e997fae9c608fda96cfd6e6f122f6eed43d62cd49a1a4` |
| `src-tauri/target/release/bundle/nsis/Codex Relay_0.5.1_x64-setup.exe` | 4701384 | `2026-09-06T01:32:43.6685572Z` | `7cb9ccff8897e82581c2f4635029284fc6f37eda10830ab23ba7922ef75d6512` |

两个产物版本均为 `0.5.1`。

## 发布与审计进度

- 候选提交：`6270662b8f0bb8000e3ea5bde9e7b8f811317bf1`。
- 推送：`git push origin HEAD:refs/heads/main` 成功；HEAD、`@{upstream}`、`git ls-remote origin refs/heads/main` 一致。
- 真实 `validate-release-request.ps1` 校验通过，版本 `0.5.1`，SHA 与候选一致，GitHubOutput 写入安全临时目录。
- 首次发布 Run：[33991815065](https://github.com/hunxuankai/codex-relay/actions/runs/33991815065)，创建 UTC `2026-09-05T21:01:37Z`，结束 UTC `2026-09-05T21:07:28Z`；headSha 与候选一致，结论 failure。前端全部通过；Rust/Clippy 1.98.0 对 29 个既有 Tauri async command 外层 `Result<_, ()>` 报 `result_unit_err`，Draft 构建跳过。
- Draft 审计、公开及 Latest/历史清理核验尚未执行。
- 独立验签工具：官方 minisign `0.12` win64 归档，252505 字节，实际 SHA-256 `37b600344e20c19314b2e82813db2bfdcc408b77b876f7727889dbd46d539479`；本机版本命令输出 `minisign 0.12`。后续只使用公开公钥验证资产，不接触私钥。
- 修复后候选：`031dbec4b4fbe794fb51d7766eda1c4d7912b747`，普通 push 至 `origin/main` 成功，HEAD/上游/远端 SHA 一致。
- 第二次发布 Run：[33997571201](https://github.com/hunxuankai/codex-relay/actions/runs/33997571201)，创建 UTC `2026-09-05T23:02:26Z`，结论 failure。Clippy 修复通过，core 247 项通过、2 项 Windows 进程夹具超时，Draft 构建跳过。触发前已确认无活动发布 Run、无 `v0.5.1` Release。

## 限制与失败记录

- 回归测试在添加模型前按预期失败；实现后通过。
- 一次 Rust 专项在会话中断后失去终态输出，未报告成功；后续完整检查重新覆盖。
- Vite 输出第三方 PURE 注解和 chunk 大小提示，构建退出 0；没有放宽检查或修改依赖。
- 未执行真实安装、应用内升级、UAC、重启、卸载或 Sandbox/VM 人工观察。
