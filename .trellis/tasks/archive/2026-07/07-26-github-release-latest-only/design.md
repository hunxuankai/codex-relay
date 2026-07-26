# GitHub Releases 仅保留最新版本下载设计

## 架构与边界

新增独立的 `.github/workflows/cleanup-old-releases.yml`，不把清理步骤插入现有
`release.yml` 的 Draft 构建阶段。该工作流：

1. 在 Release `published` 事件后触发；另提供无参数的 `workflow_dispatch` 作为
   失败后的安全重试入口。
2. 使用 GitHub Actions 自动提供的 `github.token`，只申请
   `contents: write`，在 Ubuntu runner 上调用预装的 `gh` 与 `jq`。
3. 以 `releases/latest` 返回的正式 Release 作为唯一保留对象。Release 事件中的
   ID/tag 必须与该端点一致；不一致时停止，不进行任何删除，避免 API 最终一致性或
   并发发布导致误删。
4. 分页读取本仓库全部 Release，排除保留对象后，逐项删除 Release 及其对应 tag。
   删除操作使用 GitHub API 的存在性检查，允许清理在部分完成后安全重试；任一实际
   删除失败都以非零状态结束。

现有 `.github/workflows/release.yml` 仍负责检查、签名构建和 Draft Release；只有
维护者人工核对并公开 Release 后，新的清理工作流才会运行。因此 Draft 阶段仍可
回滚，不会暂时删除当前公开 Latest。

## 数据流与不变量

```text
Release published / 手动重试
        |
        v
查询 releases/latest ---- 一致性校验 ---- 分页读取全部 Release
        |                                      |
        +---- 保留当前 ID/tag                  +---- 历史 ID/tag
                                                   |
                                    删除 Release + 删除 tag（逐项、可重试）
```

必须保持以下不变量：

- 清理前已确认保留 ID、tag 均非空，且对应 Release 不是 Draft/Prerelease。
- 删除候选只能来自 Release API 返回的 `(id, tag_name)`，不按通配符删除任意 tag。
- 当前保留 ID 或 tag 出现在候选列表时跳过；API 一致性校验失败时整体停止。
- Token 只通过 `GH_TOKEN` 环境变量传给 `gh`，不写入命令参数、日志、任务材料或
  测试 fixture。
- 删除 Release 会移除其资产；随后删除对应 tag，使历史 Release、打包成品、tag
  下载链接和源码快照都不再作为公开入口。

## 兼容性与迁移

- 客户端继续使用固定的
  `https://github.com/hunxuankai/codex-relay/releases/latest/download/latest.json`，
  不需要修改 updater 配置或签名公钥。
- 已安装的旧客户端只要仍能访问 `releases/latest`，仍可升级到当前 Latest；旧版
  安装器和旧 `latest.json` 链接在清理后不可再下载，这是本次明确接受的范围。
- 清理不触碰用户本地 Codex 配置、`%LOCALAPPDATA%\CodexRelay`、日志、备份或任何
  应用数据。
- 现有发布工作流的 `workflow_dispatch`、Draft、`releaseBody`、签名资产和
  `latest.json` 生成契约保持不变。

## 测试与审计边界

- 新增结构测试只读取 workflow 文本，断言事件触发、写权限、Latest 排除、分页、
  Release/tag 删除和非零失败路径；不在本地调用真实 GitHub 删除 API。
- 本地验证覆盖 YAML 关键字/脚本契约、专项 Vitest、`git diff --check`，必要时运行
  完整 `npm run check`。
- 真实远端清理属于发布后操作：只在工作流进入默认分支后执行/观察，记录删除前后的
  Release/tag 列表和失败，不把本地结构测试当作远端删除成功证据。

## 回滚与故障处理

- 若一致性检查失败，工作流不删除任何资源；修复并重新运行 `workflow_dispatch`。
- 若删除中途失败，保留已删除和未删除的真实状态，修复权限/网络后重试；脚本的
  存在性检查避免重复删除已不存在的对象。
- 已删除的公开 Release、资产和 tag 不承诺可恢复；若误删，必须从离线构建/备份和
  更高 SemVer 重新发布，不能在已公开版本上原地替换 updater 资产。
