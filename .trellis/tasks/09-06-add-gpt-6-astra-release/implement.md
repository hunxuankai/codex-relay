# 实施与验证记录

## 阶段与计划

- [x] 加载 Trellis、后端、前端、测试、安全及发布规范；确认用户实施与发布授权。
- [x] 完成 PRD、设计和计划审查；目录单一来源、真实路径禁用、无 tag push。
- [x] 行为切片：先运行当前 Provider 编辑为 `gpt-6-astra` 的失败回归，再更新内置目录并通过专项测试。
- [x] 同步 README、补丁版本和最终简体中文发布说明；核对公开 Latest。
- [x] `trellis-check` inline 审查、`npm run check`、移除签名环境变量后的 `npm run build`、实际产物枚举。
- [x] 规范更新判断、精确暂存提交、按 `master` 已配置上游普通推送候选并核验 SHA。
- [x] CI 阻断修复：29 个 Tauri command 外层错误改为 `InvokeError`，使用 Rust 1.98.0 重新检查、构建并提交新候选。
- [x] 第二次 CI 阻断修复：PowerShell 夹具禁用模块自动加载并使用纯 .NET API，专项连续三次、完整检查和最终构建通过后发布新候选。
- [x] 每个候选只触发一次 GitHub 发布 Run；修复两轮真实失败后，第三轮检查和构建成功，核验 Draft 与签名后公开。
- [x] 核验 Latest、公开资产、tag、历史清理；更新验收证据。
- 交付收尾由 `trellis-finish-work` 在本记录提交后执行：归档、会话日志、最终普通 push，并在答复中报告远端与 HEAD 的核对结果。

## 验证命令

- `npm run test:rust:provider-workflow`
- `npm run test:rust:lib -- provider_preference_service`
- `npx vitest run src/components/ProviderEditor.test.ts src/components/ProviderPreferenceControls.test.ts src/release-config.test.ts`
- `npm run check`
- 移除当前命令进程的两个 `TAURI_SIGNING_*` 环境变量后运行 `npm run build`。
- `git diff --check`、精确暂存差异和秘密/路径审计、发布 API 与资产哈希核验。

所有本地测试/构建命令使用临时目录下成对 Relay 覆盖，复用 `src-tauri/target`。

## 当前检查点

2026-09-06：开发与发布验收完成。`v0.5.1` 已于北京时间 11:11:39 公开；候选 `9f0b005a1063a37292a81460fcb418a75f4ef588`，Release ID `383434699`，发布 Run `34005156714` 与清理 Run `34008378763` 均成功。

- 红色：`npm run test:rust:provider-workflow -- editing_provider_to_gpt_6_astra_saves_and_applies_model_preferences` 退出 1；唯一失败为编辑目录缺少 `gpt-6-astra`，符合预期，编译约 63 秒。
- 绿色：`npm run test:rust:provider-workflow` 退出 0，2 项通过；新模型目录、编辑保存、默认 low、Fast、ultra 投影与未知 TOML/认证保留通过，编译约 30 秒、测试约 1 秒。
- GitHub 只读核验：Latest `v0.5.0`，Release ID `364697150`；两项 updater Secret 名称存在，未读取值。辅助研究确认 `v0.5.1` 未占用且无运行中的发布流程。
- npm 和两个 Cargo manifest/lock 已同步 `0.5.1`；前端结构测试使用动态版本，无需改写固定断言。
- 前端专项：3 个文件、27 项通过（编辑页、详情偏好、发布配置），退出 0。
- 中断记录：首次 `test:rust:lib -- provider_preference_service` 执行因会话中断失去终态输出，恢复后未声称通过；后续完整检查已重新覆盖整个 core 测试模块。
- 完整检查：`npm run check` 退出 0；Trellis 8 项，根 Vitest 60 文件/338 项，发布控制台 Vitest 17 文件/89 项，Rust 463 项通过、1 项既有完整检查嵌套探针按配置 ignored；两个前端 typecheck、Rust fmt/Clippy 和依赖图检查通过。`path_safety` 3 项、`provider_workflow` 2 项通过。
- 安全审查：改动只含固定模型元数据、假 key 测试、README 与版本/发布说明；Git 跟踪文件无真实认证文件、开发数据或构建产物。使用系统临时目录成对覆盖，未访问真实 Codex/Relay 数据。
- 初始模型目录切片的规范更新判断：无需新增长期契约，README 同步实际能力。后续 CI 修复产生的传输类型与夹具规则已另行沉淀到相关后端/测试规范。
- 普通构建：进程内移除两个 updater 签名变量后 `npm run build` 退出 0；Rust Release 编译 7 分 46 秒，实际生成主程序与 NSIS，版本均为 `0.5.1`。Vite 保留第三方 PURE 注解和约 504 kB chunk 提示，不影响成功状态；未生成本次 `.sig`，Authenticode 实测 `NotSigned`。
- 构建产物与后续线上证据见 `release-evidence.md`。`git fetch --no-tags origin main` 后本地与上游差异为 `0/0`。
- 候选提交 `6270662b8f0bb8000e3ea5bde9e7b8f811317bf1` 已普通推送至 `origin/main`；本地 HEAD、远程跟踪分支及 `git ls-remote` 返回同一 SHA。
- 真实发布请求验证通过；唯一 dispatch Run `33991815065`（创建 UTC `2026-09-05T21:01:37Z`），已验证 Run 的 headSha 等于候选。
- 临时操作目录为系统 TEMP 下 `CodexRelay-gpt6-20260906-cb7ca7d8`，保存 check/build 日志、退出码、local-artifacts.json、minisign 0.12 与独立 release-probe 项目；不在真实 Codex/Relay 数据目录。
- `SystemGhBackend` 审计探针改为系统临时目录下独立 Cargo 项目，复用 `src-tauri/target`。此前仓库临时 example 的复制/清理复合命令被自动策略以 `blocked by policy` 拒绝，未启动；未尝试该仓库写入/删除，替代方式不修改仓库源码。
- 当前无安装、应用内升级、UAC、重启或卸载证据。

## 首次 CI 故障

首次 CI 的前端 338 项和控制台前端 89 项全部通过；Rust Clippy 1.98.0 拒绝主程序 18 个及控制台 11 个 `Result<_, ()>`，后续 Rust tests/Draft 构建未执行。本地此前使用 1.97.1，因此旧检查通过不覆盖新工具链 lint。

## 下一步

提交完整发布证据，执行 Trellis 归档与会话日志，再普通推送当前分支已配置的 `origin/main`；最终核对 HEAD、远程跟踪分支及实际远端 SHA。所有安装、UAC、重启、卸载和真实应用内升级继续按未验证报告。

临时审计探针已在独立 temp Cargo 项目编译通过；首次 sha2 0.11 的摘要 LowerHex 格式化不兼容已改为逐字节两位十六进制，未改动产品代码。

## 关键决策与验证证据

- 已核验锁定 Tauri 宏、IPC 的 `Into<InvokeError>` 约束和现有 JSON 映射；采用框架 `InvokeError`，不添加 lint allow，不去掉外层 Result，不引入自定义错误或升级依赖。
- 五个文件共 29 个返回类型已修改；逐文件归一化新增类型/import 后与候选原文件完全相等，证明所有函数体未变。
- `rustc 1.98.0`、`clippy 0.1.98` 下 workspace fmt 和完整严格 Clippy 退出 0，冷检查约 3 分 32 秒。
- 设置仅作用于命令进程的 `RUSTUP_TOOLCHAIN=1.98.0` 后，完整 `npm run check` 再次退出 0：Trellis 8 项、根前端 338 项、控制台前端 89 项、Rust 463 项通过及 1 项既有 ignored；Rust 测试冷编译约 8 分 08 秒。
- 相同 1.98.0 工具链的无 updater 私钥普通构建退出 0，Rust Release 冷编译约 9 分 27 秒；主程序 19445760 字节，NSIS 4702342 字节，版本均 `0.5.1`，哈希见发布证据。
- 修复提交 `031dbec4b4fbe794fb51d7766eda1c4d7912b747`，普通 push 成功，HEAD/远程跟踪/ls-remote 三者一致。第二次请求校验通过后只 dispatch 一次，Run `33997571201`。
- `backend/service-boundaries.md` 新增可执行传输结果契约，`backend/provider-availability-testing.md` 两处签名已同步。

## 缺陷复盘

- 根因类别 D/E：现有 Tauri 返回类型依赖旧 Clippy 未对 async 函数报告单位错误；本地与 CI 使用不同 Rust stable 版本，旧本地检查未覆盖新 lint。
- 修复范围同时覆盖主程序和发布控制台，避免只修首个报告 package 后再次失败。
- 预防：记录并比对工具链版本；保留框架支持的传输类型和安全 DTO；使用实际 CI 工具链验证，不靠压制警告或降级规避。
- 历史失败 Run 和旧工具链构建证据保留，不覆盖成新候选成功记录。

## 第二次 CI 与夹具修复证据

- Run `33997571201`：Clippy 已通过，core 247 项通过、2 项失败。后代取消测试 30 秒内没有状态/PID 文件，大 stdout 文件测试返回 Timeout；其余带 cmdlet 夹具在同次 CI 也耗时约 23–28 秒，纯 Console/stdin 夹具约 0.2 秒通过。
- 本地可复现依赖：在大 stdout 夹具关闭模块自动加载后，原 `New-Object` 版本退出 1（0.55 秒）；独立 PowerShell 复现明确返回 `CommandNotFoundException`。改用 `[byte[]]::new` 后同一测试退出 0（0.42 秒）。
- core 9 项进程测试均改用直接 .NET API，父/后代/流式/输入输出夹具持续禁用模块自动加载；release-console 流式夹具同步替换，并保留 20 秒自截止。
- 首轮 core 9 项全部通过（8.47 秒）。生产 process runner 在 `cfg(test)` 前的内容与提交 `031dbec` 逐字节相同，30 秒预算和所有断言保留。
- core 9 项连续三轮通过（8.47 / 8.31 / 8.46 秒）；release-console 流式持久化专项连续三轮通过，完整 `local_verification` 7 项通过、1 项既有 ignored。当前使用 Rust 1.98.0 运行最终 `check-final.log`，成功后自动执行无私钥的 `build-final.log`。
- 最终 `check-final.log` 和 `build-final.log` 均退出 0；Trellis 8 项、根前端 338 项、控制台前端 89 项、Rust 463 项通过与 1 项既有 ignored；普通 Release 编译约 2 分 48 秒，产物版本与哈希已重新枚举。
- 夹具修复提交 `9f0b005a1063a37292a81460fcb418a75f4ef588` 普通 push 成功；HEAD/上游/ls-remote 一致。第三次 Run `34005156714` 创建 UTC `2026-09-06T01:55:57Z`，headSha 已核对一致。

## 发布与在线验收

- 真实 `SystemGhBackend` 完成 Draft 全量审计，独立 minisign 验证安装器和可信注释签名通过，发布前再次审计同一 Release ID 后公开。
- Release：[v0.5.1](https://github.com/hunxuankai/codex-relay/releases/tag/v0.5.1)，公开 UTC `2026-09-06T03:11:39Z`。公开后生产完整审计最终退出 0，三项资产与 Draft 证据完全一致。
- 公开 Latest、tag ref、无需认证的 Latest/Tag 清单、安装器及 `.sig` 下载均已核验；实际大小/哈希见 `release-evidence.md`。
- 清理 Run `34008378763` success；分页 Release/tag 列表只剩 `v0.5.1`，旧 `v0.5.0` 发布资产已按仓库策略移除。
- 在线复核前两次 CLI 调用/资产下载失败已保留；直接公开下载和最终完整审计通过，没有为重试修改代理、Token、正式配置或已公开资产。
- 未获取 CI 进程级 trace，不把模块内部等待机制写成已完全证实；去除该依赖后的第三次 CI 已通过全部检查。
