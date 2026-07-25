# GitHub Releases 仅保留最新版本下载

## 目标

让 GitHub Releases 页面和公开下载入口只展示、提供当前最新的正式版本，减少
历史安装包及 updater 清单造成的维护负担，同时不破坏应用使用固定
`releases/latest/download/latest.json` 检查更新的契约。

## 已确认事实

- 当前发布工作流为 `.github/workflows/release.yml`，仅由 `workflow_dispatch` 触发，
  通过 `tauri-apps/tauri-action` 先创建 Draft Release；现有工作流没有历史 Release
  清理步骤或独立清理工作流。
- 公开仓库当前有 `v0.1.0`、`v0.1.1`、`v0.1.2`、`v0.2.0`、`v0.2.1` 五个正式
  Release；每个 Release 都包含 NSIS 安装器、`.sig` 和 `latest.json` 三项资产，
  `v0.2.1` 是当前 Latest。
- 客户端唯一更新清单地址是
  `https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json`，
  因此发布新版本后必须始终保留一个可公开消费的 Latest Release。
- 任务范围是 GitHub Releases 的历史成品保留策略，不涉及删除用户本地配置、应用
  数据、日志或备份。

## 需求

- 新版本正式发布后，自动清理除当前 Latest 之外的历史正式 Release 及其打包资产，
  并删除这些 Release 对应的 Git tags；不在 Draft 审核完成前删除当前公开版本。
- 清理逻辑必须只操作本仓库的 Release 资源，保留当前刚发布的 Release；失败时在
  Actions 中明确失败，不得静默宣称清理成功。
- 发布工作流和客户端 updater 的现有手动触发、Draft、签名资产及 `latest.json`
  契约保持不变。
- 为清理策略增加结构回归测试，并更新维护者文档/发布规范，说明历史下载链接的
  失效边界和当前 Latest 的保留规则。

## 验收标准

- [x] 新增的自动化清理流程只在 Release 已正式发布后运行，并能识别当前 Release，
      不会在 Draft 阶段删除旧 Latest。
- [x] 清理流程对历史 Release 的删除范围和错误处理有可审计实现，且不会把令牌、
      签名私钥或用户数据写入日志、任务材料或测试输出。
- [x] 历史 Release、其资产和对应 Git tags 均被纳入清理范围；当前正式发布的
      Release 与 tag 始终被排除。
- [x] 结构测试覆盖触发时机、写权限、当前 Release 排除条件和清理命令/接口。
- [x] `releases/latest` 与 `latest.json` 的现有更新入口仍被保留；文档明确说明
      历史版本下载链接是否继续可用。
- [x] 运行相关专项测试、`git diff --check`，并按发布规范记录未执行的真实 GitHub
      Actions/远端清理证据，不夸大本地测试结果。

## 已解决的范围决定

- 历史 Release 删除后同时删除对应 Git tags。历史版本的 Release 页面、打包资产、
  tag 下载链接和源码快照均不作为长期下载入口；唯一保留的公开版本是当前 Latest。

## 说明

- `prd.md` 只记录需求、约束和验收标准。
- 轻量任务可以只保留 PRD。
- 复杂任务在运行 `task.py start` 前还需添加记录技术设计的 `design.md`，以及记录实施计划的 `implement.md`。
