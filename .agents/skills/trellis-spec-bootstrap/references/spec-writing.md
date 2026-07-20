# 规范编写

Trellis 规范是面向未来代理的编码指南。它们应说明如何在本仓库中工作，而不是介绍通用项目可能如何组织。

## 基于证据编写

每条重要规则都应由以下至少一项支撑：

- 展示首选模式的源码文件。
- 展示预期行为的测试文件。
- 定义约定的项目文档。
- 多个文件中重复出现的模式。

只有在短片段能让规则更清晰时才使用。优先链接文件路径，并指出符号或行为名称。

## 文件结构

保持规范树与项目一致：

- 保持 `index.md` 为规范目录的导航文件。
- 如果开发者会独立查找某些主题，就拆分主题。
- 如果独立文件会重复同一规则，就合并主题。
- 删除不适用的模板文件。
- 为模板遗漏的重要本地模式添加新文件。

## 内容标准

良好的规范章节包括：

- 规则适用时机。
- 应遵循的本地模式。
- 证明该模式的源码或测试文件。
- 常见错误或反模式。
- 具体且可靠的验证命令或检查。

避免：

- 占位说明。
- 通用框架建议。
- 只在一种代理宿主中有效的工具指令。
- 大段复制的代码块。
- 只基于单个偶然实现细节的规则。

## 示例结构

```markdown
## Command Handlers

Command handlers should keep argument parsing, validation, and side effects separate. The local pattern is:

- Parse CLI flags at the command boundary.
- Convert raw inputs into typed task options before invoking core logic.
- Keep filesystem writes in the command or service layer, not in template helpers.

Reference files:
- `packages/cli/src/commands/example.ts`
- `packages/cli/test/commands/example.test.ts`

Avoid passing raw `process.argv` or unvalidated config objects into shared helpers.
```

## 最终检查

完成前：

```bash
grep -R "To be filled\\|TODO: fill\\|placeholder" .trellis/spec
```

同时检查链接、索引文件，以及是否仍有规范描述的是模板而不是本仓库。
