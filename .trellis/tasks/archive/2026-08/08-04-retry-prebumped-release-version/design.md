# 修复已预升版本发布重试设计

## 根因与设计目标

预检已经从 GitHub 获得正式 Latest tag，但计划生成把 `package.json.version` 同时当作仓库版本和
公开版本。一次候选提交已推送、远端 Run 后续失败时，这两个事实会分离：仓库已经是目标版本，公开
Release 仍是上一版本。修复需要显式区分“公开版本基准”“仓库当前版本”“目标版本”，并让后续
候选提交逻辑支持版本文件已经处于目标状态。

## 方案比较

1. 采用：公开基准与仓库版本分离，并让候选提交支持部分变化或零变化。该方案保留已推送历史，
   继续使用固定六文件事务和精确 SHA Push，覆盖常规发布与失败后修复重试。
2. 不采用：把仓库版本回退到上一公开版本后重新生成候选。该方案会制造反向版本提交或要求改写
   已推送历史，破坏发布证据。
3. 不采用：要求填写 `0.5.1`。线上从未公开 `0.5.0`，虚增版本会把工具状态缺陷转嫁给发布编号。

## 数据流与边界

### 公开版本基准

`SystemReleaseApplication::prepare_plan` 继续复用同一次权威预检结果。若
`latestReleaseTag=Some("v<SemVer>")`，Rust 去除固定小写 `v` 并解析、规范化为公开版本；tag 缺少
固定前缀或 SemVer 无效时返回 `RELEASE_LATEST_VERSION_INVALID`。若没有正式 Release，则回退到
`package.json.version`，保持首次发布兼容行为。Vue 不解析或重建该事实。

生成提交说明使用 `v<公开版本>..HEAD`，发布说明的更新起点和 `ReleasePlanSummary.previousVersion`
均使用公开版本。远端 main SHA 继续与计划一起保存在内存，作为开始发布后的 Git 竞态门禁。

### 候选事务

`ReleaseCandidateTransaction` 增加接受显式公开版本的计划入口，既有入口保留并委托给新入口，以免
破坏普通调用方。新入口仍一次读取固定六文件并记录全部指纹、原字节和目标字节：

- `公开版本 < 目标版本` 由 `ReleaseNotesService` 校验；
- `仓库版本 <= 目标版本` 是候选文件方向约束；仓库版本更高时返回
  `RELEASE_REPOSITORY_VERSION_AHEAD`；
- 仓库版本等于目标时，版本文件的 `before` 与 `after` 可以相同；事务仍保存六文件恢复状态，
  保持原子写、写后验证和回滚契约。

### 提交与 Push

`GitReleaseService::commit_candidate` 先验证六文件当前字节全部等于计划目标，再根据
`before != after` 计算真实变化集合。它重新读取本地 HEAD 和远端 main，二者都必须等于计划时记录的
同步 SHA；工作区变化必须精确等于真实变化集合，且暂存区与未跟踪集合为空。

- 有真实变化：只 `git add -- <真实变化文件>`，复核暂存/未暂存集合后创建现有中文候选提交。
- 无真实变化：不调用 `git add` 或 `git commit`，直接把已复核的当前 HEAD 作为候选 SHA。

两条路径随后共用现有 `Committed` 检查点、固定 main RefSpec Push、远端 SHA 验证和候选事务
finalize。Orchestrator 根据计划是否有真实变化记录“已创建提交”或“复用已同步 HEAD”，不伪造行为。

## 错误与兼容性

- 保留 `RELEASE_VERSION_NOT_HIGHER`，其含义恢复为目标不高于当前公开版本。
- 新增 `RELEASE_LATEST_VERSION_INVALID` 和 `RELEASE_REPOSITORY_VERSION_AHEAD`。
- 不改变 command 参数、Tauri/TypeScript DTO、session schema 或 GitHub workflow 输入。
- 常规 `0.4.0 -> 0.5.0` 六文件提交保持原行为；首次发布无 Latest 时保持原基准。
- HEAD、远端、非计划文件、暂存区、未跟踪文件或计划目标字节漂移继续在任何 Git 写入前失败。

## 测试与安全

- 候选事务集成测试覆盖公开 `0.4.0`、仓库/目标 `0.5.0` 的幂等计划、应用和高于目标拒绝。
- application 纯函数测试覆盖 Latest tag 规范化、无 Release 回退和非法 tag。
- 临时 Git 仓库测试覆盖部分变化只提交真实文件、零变化复用新修复 HEAD，以及 HEAD 漂移拒绝。
- Orchestrator 测试覆盖两类安全进度消息；既有回滚、Committed 恢复和精确 Push 测试继续运行。
- 不调用真实 GitHub workflow，不读取或写入真实 `.codex` 与 `%LOCALAPPDATA%\CodexRelay`。

## 回滚

代码回滚只需恢复计划基准和 Git 提交逻辑；不涉及数据迁移。发布候选运行时仍由已有六文件 marker、
backup 和逐字节验证负责回滚，失败时继续保留真实 `RELEASE_ROLLBACK_INCOMPLETE` 证据。
