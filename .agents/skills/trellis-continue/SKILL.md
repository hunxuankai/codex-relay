---
name: trellis-continue
description: "恢复当前任务。加载工作流阶段索引，判断应从哪个阶段/步骤继续，再通过 get_context.py --mode phase 获取步骤级细则。回到进行中任务并需要确认下一步时使用。"
---

# 继续当前任务

恢复当前任务，并从 `.trellis/workflow.md` 中正确的阶段/步骤继续。

---

## 步骤 1：加载当前上下文

```bash
python ./.trellis/scripts/get_context.py
```

确认当前任务、Git 状态和最近提交。

## 步骤 2：加载阶段索引

```bash
python ./.trellis/scripts/get_context.py --mode phase
```

显示包含路由和 Skill 映射的阶段索引（规划 / 实施 / 完成）。

## 步骤 3：判断当前位置

`get_context.py` 会显示活动任务的 `status` 字段。根据 `status` 和任务材料是否存在进行路由。该命令让用户不必记住 Trellis 流程，但它本身不代表批准实施。

- `status=planning` 且没有 `prd.md` → **1.1**（加载 `trellis-brainstorm`）
- `status=planning` 且只有 `prd.md` → 判断任务是轻量还是复杂。轻量任务可进入 **1.4** 审查；复杂任务返回 **1.1**，补充 `design.md` 和 `implement.md`。
- `status=planning`，复杂任务材料完整，但子代理 JSONL 未整理（只有种子 `_example` 行）→ **1.3**
- `status=planning`，必需材料完整，所需 JSONL 已整理或处于 inline 模式 → **1.4**（请求启动审查；仅在用户确认后运行 `task.py start`）
- `status=in_progress` 且尚未开始实施 → **2.1**
- `status=in_progress` 且实施完成、尚未检查 → **2.2**
- `status=in_progress` 且检查通过 → **3.3**（更新规范）→ **3.4**（提交）
- `status=completed`（很少出现，通常会立即归档）→ 归档流程

阶段规则（完整细则见 `.trellis/workflow.md`）：

1. 在阶段内**按顺序**执行步骤，不能跳过 `[required]` 步骤
2. 若必需输出已存在，则 `[once]` 步骤视为已完成。只有轻量任务可以仅有 `prd.md`；复杂任务还需要 `design.md` 和 `implement.md`
3. 如果新发现有此需要，可以返回更早的阶段

## 步骤 4：加载具体步骤

确定从哪个步骤恢复后：

```bash
python ./.trellis/scripts/get_context.py --mode phase --step <X.X> --platform codex
```

遵循加载的指令。完成每个 `[required]` 步骤后，再进入下一步。

---

## 参考

完整工作流和阶段步骤细则位于 `.trellis/workflow.md`。本命令只是入口，权威指南以该文件为准。
