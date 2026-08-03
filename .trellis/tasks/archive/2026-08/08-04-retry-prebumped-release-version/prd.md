# 修复已预升版本发布重试

## 目标

当目标版本已经由一次未完成的发布候选提交写入本地及远端默认分支、但 GitHub Latest
仍停留在更低版本时，发布控制台应允许维护者继续以原目标版本创建新的发布计划，完成
Draft 构建与后续发布；不得要求维护者虚增补丁版本。

## 已确认事实

- GitHub 当前 Latest 为 `v0.4.0`，而仓库 `package.json`、Cargo manifests 和锁文件已由
  提交 `5d5f33d` 升至 `0.5.0`。
- 上一次 `v0.5.0` GitHub Release Run 在完整检查阶段失败，没有生成 Draft Release；之后的
  修复提交位于已预升版本的候选提交之后。
- 控制台当前把 `package.json.version` 作为 `previous_version`，并用
  `target_version <= previous_version` 触发 `RELEASE_VERSION_NOT_HIGHER`；预检返回的
  `latestReleaseTag` 仅展示，没有参与发布计划。
- 发布候选事务固定管理六个文件，必须继续保留指纹、备份、原子写入、写后验证和可验证回滚。
- 当前持久化失败会话没有候选 SHA 或 Draft，不能作为可恢复的远端发布会话继续。

## 需求

- 版本递增门禁必须以当前正式公开 Release 为基准，而不是假设仓库内包版本始终等于公开版本。
- 当仓库版本等于目标版本且目标版本严格高于公开版本时，发布计划必须保留公开版本作为发布说明
  的起点，并允许版本文件保持目标版本。
- 没有正式公开 Release 时必须保持既有首次发布兼容行为：以仓库版本作为递增基准，不猜测一个
  不存在的公开版本。
- 当目标版本不高于公开版本时仍须拒绝，稳定错误码保持 `RELEASE_VERSION_NOT_HIGHER`。
- 仓库版本低于目标版本的常规首次发布路径必须保持现有行为。
- 仓库版本高于目标版本或 Latest tag 不是 `v<SemVer>` 时，必须返回稳定、可理解的失败，不写候选文件。
- 六文件计划只允许提交实际发生变化的计划文件；若六文件均已是目标内容，必须重新核对本地 HEAD
  与远端 main 后复用当前 HEAD，不制造空提交，也不扩大可暂存文件集合。
- 复用或创建候选提交后继续使用现有精确 SHA Push、候选事务 finalize、会话检查点和失败回滚契约；
  进度日志必须准确区分“创建新候选提交”和“复用已同步 HEAD”。
- 前后端必须展示同一权威版本事实，避免把“线上 Latest”显示为 `v0.4.0`、却按本地 `0.5.0`
  拒绝的矛盾状态。
- 不触发真实 GitHub workflow、Draft、Tag、Release、安装、签名或应用内升级；测试仅使用临时仓库。

## 验收标准

- [x] AC1：公开 Latest=`v0.4.0`、仓库版本=`0.5.0`、目标=`0.5.0` 时，可生成计划，计划的
  `previousVersion` 为 `0.4.0`、`targetVersion` 为 `0.5.0`。
- [x] AC2：上述计划可在未修改源文件的临时仓库中应用；六文件事务、指纹与回滚契约保持有效。
- [x] AC3：公开 Latest=`v0.4.0`、仓库版本=`0.4.0`、目标=`0.5.0` 的常规路径继续通过。
- [x] AC4：目标等于或低于公开 Latest 时返回 `RELEASE_VERSION_NOT_HIGHER`。
- [x] AC5：仓库版本高于目标时返回 `RELEASE_REPOSITORY_VERSION_AHEAD`；Latest tag 无法安全解析时
  返回 `RELEASE_LATEST_VERSION_INVALID`，两者都不写候选文件。
- [x] AC6：只有发布说明等部分计划文件变化时，只提交真实变化的计划文件；存在非计划改动、暂存项、
  未跟踪文件、HEAD 或远端漂移时继续拒绝。
- [x] AC7：六文件均无变化时不创建空提交，复用已同步 HEAD，并继续以该精确 SHA 执行现有 Push 验证。
- [x] AC8：没有正式 Release 时继续以仓库版本生成首次发布计划；界面 Latest 与计划的
  `previousVersion` 不再矛盾。
- [x] AC9：专项测试、受影响层检查和完整质量门禁在成对安全 Relay 临时覆盖下通过，且真实
  Codex/Relay 数据路径保持未访问。

## 范围外事项

- 自动删除或改写既有 Git 提交、Tag、Draft 或 Release。
- 自动重跑当前已失败且没有候选 SHA 的终态会话。
- 改变 GitHub Actions、签名、安装、升级、卸载或数据保留行为。
