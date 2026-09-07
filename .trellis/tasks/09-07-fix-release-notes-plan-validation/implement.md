# 实施与验证记录

## 当前阶段

三个行为切片、完整检查与便携 EXE 构建已完成，发布规范和产物证据已补齐，进入提交与收尾。

## 有序实施

- [x] 追踪生成说明、输入状态、Rust 校验和候选计划边界，加载发布、安全、测试与 Vue 规范。
- [x] 切片 1：恢复 `0.5.1` 已完成会话后生成 `0.5.2`，先写用户边界失败测试，再阻断历史说明回填。
- [x] 切片 2：更改目标、仓库或 Latest 后旧计划和说明失效；保留同一计划的人工编辑。
- [x] 切片 3：过期的预检、生成计划和会话恢复响应不得恢复失效上下文。
- [x] 运行专项、类型检查与完整项目检查；复核 Rust 安全校验仍生效。
- [x] 构建发布控制台，核对实际产物路径、长度、时间和 SHA-256。
- [x] Trellis check、规范沉淀与提交前安全检查。

代码与任务材料在阶段 3.4 精确暂存提交；阶段 3.5 归档、记录会话日志并普通 push 到当前 `master`
已配置的 `origin/main`。最终提交和远端 SHA 一致性证据由会话日志、Git 历史及交付回复记录。

## 验证命令

所有自动测试和构建使用成对安全临时 `CODEX_RELAY_CODEX_HOME` / `CODEX_RELAY_APP_DATA_DIR`。不读取真实用户数据，不改变 Cargo target。

- `npm run test --workspace @codex-relay/release-console -- src/App.test.ts`
- `npm run test --workspace @codex-relay/release-console -- src/App.test.ts src/composables/useReleaseSession.test.ts`
- `npm run typecheck:release-console`
- `npm run check:rust-dev-env`
- `cargo test --manifest-path src-tauri/Cargo.toml -p codex-relay-release-console --test release_notes --test release_candidate`
- `npm run check`
- `npm run build:release-console`
- `git diff --check`、精确暂存差异与安全扫描、普通 push 后远程跟踪 SHA 与本地 HEAD 比较。

## 证据

- 首轮 `App.test.ts -t 'generates fresh notes'` 退出 1，实际 `prepare_release_plan` 目标为 `0.5.2`，`notes` 却是旧 `v0.5.0 → v0.5.1` 正文。
- 移除 `loadSession()` 中历史 `manifestNotes` 回填后，`App.test.ts` 18/18 通过，退出 0。
- 上下文变更专项首次 3 项全部按预期失败：目标和 Latest 变化后旧计划仍存在，仓库变化后旧说明仍存在。加入上下文监听及显式计划失效后，`App.test.ts` 21/21 通过，退出 0。
- 跨轮继续时，测试包装字符串因会话内缓存不存在形成 `undefinednpm.cmd`，未启动测试；恢复并验证安全双路径后重跑成功，该次命令不计为测试通过。
- 晚响应专项首次 3 项全部按预期失败；加入请求序列失效与返回值门禁后全部通过。
- `App.test.ts` 最终 22/22 通过；同时执行类型检查时，合并专项的既有十万条日志测试用时约 7.1 秒，超过原有 5 秒预算，该轮为 37 通过、1 超时（退出 1）。未改实现或超时，单独重跑 `useReleaseSession.test.ts` 后 16/16 通过（退出 0），日志测试约 1.1 秒。后续重型验证串行执行。
- `npm run typecheck:release-console` 与 `npm run check:rust-dev-env` 均退出 0。
- Rust 专项 `release_notes` 4 项、`release_candidate` 14 项全部通过，退出 0；首次编译/链接耗时 4 分 16 秒，测试本身分别约 0.04 秒和 0.82 秒。
- 首轮 `npm run check` 在根前端套件结束时退出 1：345 项通过，既有 `release-request.test.ts` 的 PowerShell 子进程用例用时约 5.4 秒，超过外层 5 秒预算。单独运行该文件后 1/1 通过（约 2.46 秒），秘密拒绝断言没有变化。
- 随后完整 `npm run check` 退出 0：Trellis 8 项；根前端 60 文件/346 项；控制台专项 17 文件/97 项；Rust 465 项通过、1 项按既有配置忽略（通过生产进程再次运行完整检查的递归重型用例）。依赖图、fmt、Clippy 和两套类型检查均通过。根套件中的同一 PowerShell 测试约 4.37 秒通过；未修改测试或生产超时。
- `git diff --check` 退出 0。新增差异与任务材料高置信度密钥扫描为 0。初次宽泛数据目录审计命中既有 `dev-data/.gitkeep`，随后“文件必须为零字节”的过严断言失败；实际读取确认仅含换行。实际开发数据、构建目录和二进制均被 Git 忽略，没有纳入提交。

## 构建产物

`npm run build:release-console` 退出 0，未向构建进程提供 updater 私钥。Vite 耗时约 15.72 秒，Rust
Release 编译/链接约 3 分 18 秒。Vite 报告两个第三方 `@vueuse/core` PURE 注释位置警告，已由 Rollup
移除注释继续构建；没有压制警告。

源文件与默认便携副本经本轮 `Get-Item` / `Get-FileHash` 核对一致：

- 源：`src-tauri/target/release/CodexRelayReleaseConsole.exe`
- 交付：`artifacts/release-console/CodexRelayReleaseConsole.exe`
- 大小：12,979,712 字节
- 最后写入：`2026-09-08T02:41:57.8512826+08:00`
- SHA-256：`2F10DFA7159CC2F3D76BECB070FB707C5E184C874BD2F07B465748CB14650B6D`

本轮未启动真实发布管线或公开 `0.5.2`，未执行人工点击、安装、UAC、升级、卸载或 Authenticode 签名，
这些行为不由测试和便携构建证据替代。

## 规范检查

- 使用 Vue Composition API、现有 typed IPC 和 readonly 状态；计划失效没有更改历史会话或 Rust 校验契约。
- 发布规范新增“发布说明草稿与计划上下文契约”，记录历史证据与新草稿的分离、三类上下文失效和晚响应返回值门禁。
- 改动只涉及维护者发布控制台，不改变主软件能力、数据保留或安装行为，“关于”页面无需调整。

## 范围与风险

不改主软件 `0.5.1` 版本文件，不触发公开发布。计划失效不能改变历史会话或运行中的发布事实；测试 mock IPC，仅 Rust 临时目录测试访问 fixture 文件。
