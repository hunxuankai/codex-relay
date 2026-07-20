---
name: trellis-spec-bootstrap
description: "使用平台无关的单代理工作流引导项目专属 Trellis 编码规范。创建或刷新 .trellis/spec 规范、使用 GitNexus、ABCoder 或源码检查分析代码库、拆分包/层规范工作，以及编写由真实代码库支撑且没有占位文本的规范文档时使用。"
---

# Trellis 规范引导

使用此 Skill 基于真实代码库创建或刷新 `.trellis/spec/` 规范。由一个有能力的代理负责完整闭环：分析仓库、选择规范边界、编写文档并验证结果。该工作流不依赖特定宿主、CLI 或代理品牌。

## 工作流

1. 确认 Trellis 已初始化，并检查当前 `.trellis/spec/` 树。
2. 使用可用的最佳工具分析仓库架构：GitNexus、ABCoder、语言工具和直接读取源码。
3. 只有在符合真实代码库时，才按包和层拆分规范工作。
4. 使用项目中的具体模式、文件路径、示例和反模式填充或重塑规范文件。
5. 验证最终规范内部一致，且不包含模板占位符。

## 参考资料路由

| 需求 | 读取 |
|------|------|
| 仓库架构分析 | [references/repository-analysis.md](references/repository-analysis.md) |
| 规范工作拆分和任务规划 | [references/spec-task-planning.md](references/spec-task-planning.md) |
| 编写高信号 Trellis 规范文件 | [references/spec-writing.md](references/spec-writing.md) |
| GitNexus 和 ABCoder MCP 设置 | [references/mcp-setup.md](references/mcp-setup.md) |

## 操作规则

- 把模板视为起点，而不是契约。仓库实际需要时，可以删除、重命名、拆分或添加规范文件。
- 优先编写有源码依据的规则，而非通用建议。每条重要建议都应指向真实文件或重复出现的本地模式。
- 默认由单一负责人执行。可选辅助代理只是实现细节，不是必需项或用户可见依赖。
- 除非目标项目已将某个平台标准化，否则不要编写平台专属指令。
- 不要在 `.trellis/spec/` 中遗留占位文本、空标题或复制的样板内容。

## 完成标准

- `.trellis/spec/` 描述项目当前的真实状态。
- 每个相关包或层都有包含真实示例的实用编码指南。
- 已删除不适用的模板章节。
- `index.md` 文件与最终规范文件集合一致。
- 所有必要设置或分析假设都记录在相关规范或任务注记中。
