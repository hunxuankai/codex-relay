---
name: trellis-before-dev
description: "在实施开始前发现并加载 .trellis/spec/ 中的项目专属编码规范。读取目标包的规范索引、开发前检查清单和共享思考指南。开始新编码任务、编写代码前、切换到其他包或需要刷新项目约定与标准时使用。"
---

开始任务前，读取相关开发规范。

执行以下步骤：

1. **读取当前任务材料**：
   - 从 `prd.md` 读取需求和验收标准
   - 如果存在 `design.md`，从中读取技术设计
   - 如果存在 `implement.md`，从中读取实施顺序和验证计划

2. **发现包及其规范层**：
   ```bash
   python ./.trellis/scripts/get_context.py --mode packages
   ```

3. **确定适用规范**，依据包括：
   - 正在修改哪个包（例如 `cli/`、`docs-site/`）
   - 工作类型（后端、前端、单元测试、文档等）
   - 任务材料中引用的任何规范或研究路径

4. **读取每个相关模块的规范索引**：
   ```bash
   cat .trellis/spec/<package>/<layer>/index.md
   ```
   遵循索引中的**“开发前检查”**部分。

5. **读取开发前检查中列出的具体规范文件**，只选择与当前任务相关的文件。索引本身不是终点，它会指向实际规范文件（例如 `error-handling.md`、`conventions.md`、`mock-strategies.md`）。阅读这些文件以理解编码标准和模式。

6. **始终读取共享指南**：
   ```bash
   cat .trellis/spec/guides/index.md
   ```

7. 理解需要遵循的编码标准和模式，然后继续实施计划。

编写任何代码前都**必须**完成此步骤。
