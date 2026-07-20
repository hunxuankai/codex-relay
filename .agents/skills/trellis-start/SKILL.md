---
name: trellis-start
description: "通过读取 .trellis/ 中的工作流指南、开发者身份、Git 状态、活动任务和项目规范来初始化 AI 开发会话。对传入任务分类，并路由到需求探索、直接编辑或任务工作流。开始新编码会话、恢复工作、启动新任务或重建项目上下文时使用。"
---

# 启动会话

初始化由 Trellis 管理的开发会话。此平台没有会话启动钩子，因此按以下步骤手动加载等价的精简上下文。

---

## 步骤 1：当前状态
开发者身份、Git 状态、当前任务、活动任务和日志位置。

```bash
python ./.trellis/scripts/get_context.py
```

如果输出包含以 `Trellis update available:` 开头的行，汇总会话上下文时原样复制整行，不要缩短其中的操作命令提示。

## 步骤 2：工作流概览
精简阶段索引、请求分流规则、规划材料契约和步骤细则命令。

```bash
python ./.trellis/scripts/get_context.py --mode phase
```

完整指南位于 `.trellis/workflow.md`（按需读取）。

## 步骤 3：规范索引
发现包和规范层，然后读取每个相关索引文件。

```bash
python ./.trellis/scripts/get_context.py --mode packages
cat .trellis/spec/guides/index.md
cat .trellis/spec/<package>/<layer>/index.md   # for each relevant layer
```

索引文件会列出真正开始编码时需要读取的具体规范文档。

## 步骤 4：决定下一步
步骤 1 已给出当前任务及其状态。检查任务目录：

- **活动任务状态为 `planning` 且没有 `prd.md`** → 阶段 1.1。加载 `trellis-brainstorm` Skill。
- **活动任务状态为 `planning` 且已有 `prd.md`** → 留在阶段 1。轻量任务可以只有 PRD；复杂任务需要 `design.md` 和 `implement.md`。运行 `task.py start` 前加载对应的阶段 1 步骤细则。
- **活动任务状态为 `in_progress`** → 阶段 2 的步骤 2.1。加载步骤细则：
  ```bash
  python ./.trellis/scripts/get_context.py --mode phase --step 2.1 --platform codex
  ```
- **没有活动任务** → 先分类。对于简单对话或小任务，只询问本轮是否创建 Trellis 任务。对于复杂工作，询问是否可以创建 Trellis 任务并进入规划。如果用户拒绝，本会话跳过 Trellis。

---

## Skill 路由（快速参考）

| 用户意图 | Skill |
|---|---|
| 新功能 / 需求不清晰 | `trellis-brainstorm` |
| 即将编写代码 | `trellis-before-dev` |
| 编码完成 / 质量检查 | `trellis-check` |
| 卡住 / 多次修复同一缺陷 | `trellis-break-loop` |
| 学到值得沉淀的内容 | `trellis-update-spec` |

完整规则和防止自我合理化的对照表见 `.trellis/workflow.md`。
