# 发布更新操作文档实施记录

## 当前进度

- 发布指南、规范索引和 README 入口已完成；专项检查、失败修复复测和项目完整检查均通过，准备最终审查与收尾。

## 实施清单

- [x] 新增 `.trellis/spec/release/publishing.md`。
- [x] 更新 `.trellis/spec/release/index.md` 导航和检查项。
- [x] 在 README 发布章节增加文档入口。
- [x] 检查文档中的版本、Secret、Draft、签名和回滚契约。
- [x] 运行 Markdown/链接检查、`git diff --check` 和 `npm run check`。
- [x] 运行 Trellis 质量检查并完成规范更新判断。

## 验证命令

```powershell
rg -n -S "publishing.md|TAURI_SIGNING_PRIVATE_KEY|releaseDraft|latest.json" README.md .trellis/spec/release .github/workflows/release.yml
git diff --check
npm run check
```

## 风险与回滚点

- 不编辑 `.github/workflows/release.yml`、版本文件或签名配置。
- 若文档与当前配置不一致，以工作流、配置和 release 规范的现行契约为准修正文档。
- 回滚只删除本任务新增文档和入口链接，不触碰发布资产或远端状态。

## 验证证据

- RED：检查 `.trellis/spec/release/publishing.md`、发布规范索引入口和 README 入口的 PowerShell 命令退出 1，三项均按预期缺失。
- GREEN：同一入口检查退出 0；新文档、发布规范索引和 README 三处均存在有效入口。
- 相对链接专项检查退出 0；`publishing.md`、发布规范索引和 README 中的本地 Markdown 链接均可解析到现有文件。
- 契约关键词检查命中版本文件、结构测试、Secret 名称、手动 Draft、`latest.json`、Authenticode、数据路径和更高 SemVer 规则；`git diff --check` 退出 0，仅报告 Git 的 LF/CRLF 工作区提示。
- 首轮 Trellis 一致性检查退出 1：指南使用了“手动触发”和“Draft Release”中文描述，但没有显式写出工作流键 `workflow_dispatch:` 与 `releaseDraft: true`。复现确认工作流包含两项字面量而文档不包含，根因是机器配置键未落盘，不是工作流漂移；已最小补充两个键名，等待重跑。
- 修复后工作流字面契约专项检查退出 0，指南和工作流同时包含 `workflow_dispatch:`、`releaseDraft: true` 及两个签名 Secret 名称。
- `npm run check` 于本轮退出 0，耗时 43.2 秒：8 项 Trellis 测试、18 个前端测试文件共 87 项、107 项 Rust 单元测试、2 项路径安全测试和 1 项 Provider 工作流测试通过；类型检查、Rust fmt 与 Clippy 同时通过。
- 完成前再次运行完整 `npm run check`，于 59.1 秒后退出 0；测试数量保持为 8 项 Trellis、18 个前端文件共 87 项、107 项 Rust 单元、2 项路径安全和 1 项 Provider 工作流，类型检查、Rust fmt 与 Clippy 均通过。
- 本任务只修改文档和 Trellis 任务材料，没有修改构建、签名、安装、升级或卸载逻辑，因此未运行 Release/NSIS 构建、GitHub Actions、Draft、公开发布或 Sandbox/VM 真实升级，也不对这些行为作本轮成功声明。
- 规范更新判断：用户要求的长期发布操作步骤已直接写入 `.trellis/spec/release/publishing.md`，并由 release 索引和 README 导航；没有额外独立规则需要再写入其他 spec。

## 尚未解决的问题

- 无。
