---
name: trellis-session-insight
description: "通过 `trellis mem` CLI 检索过去的 AI 对话历史。用户询问“上次如何解决 X”“之前讨论过吗”“关于 X 的决定是什么”“提醒我这个任务做过什么”“上次怎么解的”“想起一段对话”，或开始与过往工作重叠的需求探索、调试熟悉缺陷、跨会话继续任务、进行收尾回顾时使用。返回原始历史对话；根据当前情境决定更新规范、追加任务注记、在回答中引用或仅内部吸收。"
---

# Trellis 会话洞察

此 Skill 说明 AI **如何调用 `trellis mem`**（项目的跨会话记忆原料），以及**何时适合使用它**。

它刻意被设计为**能力型 Skill，而不是工作流**。没有固定输出文件、没有必需的写回步骤，也没有“完成工作后始终运行”的规则。如何处理 `mem` 返回的内容，应根据当前对话即时判断。此 Skill 的作用是让 AI 知道该能力存在并能作出选择。

## `trellis mem` 是什么

这是一个本地 CLI，会索引用户过去的 Claude Code、Codex 和 Pi Agent 对话日志，并支持列出、搜索、按 Trellis 任务边界切片，以及导出清理后的对话。Claude 和 Codex 分别使用 `~/.claude/projects/` 与 `~/.codex/sessions/`。Pi 使用默认或环境配置的会话根目录、全局 `~/.pi/agent/settings.json` 和目标项目的 `.pi/settings.json`；相对 `sessionDir` 从设置文件所在目录解析。项目本地 Pi 设置需要通过当前工作目录或 `--cwd` 进行项目范围查找。OpenCode 日志目前还不能索引（Provider 适配器尚未完成）；当明显需要查找 OpenCode 会话时，应说明此限制，不要猜测。

`mem` 不会上传任何内容，所有读取都在本机完成。

## 何时使用

判断标准是：“资深同事会不会问‘我们之前不是讨论过吗？’”——这正是应使用它的时刻。具体模式包括：

- **重复需求探索的风险。** 新任务涉及用户以前处理过的领域，希望在再次询问用户前确认是否已有决定。
- **熟悉缺陷的调试。** 当前缺陷模式像是用户以前报告或修复过的问题。拉取相关历史会话可省去完整调试循环。
- **跨会话继续。** 用户间隔一段时间后恢复工作，只说“我们做到哪了”或“继续上次的”，没有给出细节。
- **检索决定。** 用户提到“我们对 X 作出的决定”，但该决定存在于旧需求探索对话中，而不在任何 `prd.md` 或 `spec/` 中。
- **完成工作回顾。** 用户明确要求总结本任务中作出的决定、困难或意外；不要把它强制成每次收尾都执行的步骤。
- **识别过往工作模式。** 用户询问“我是不是总在 X 上犯同样错误”或“我每次都踩这个坑吗”；跨会话搜索可以回答。

如果这些情况都不适用，不要调用 `mem`。它是工具，不是仪式。

## 何时不要使用

- 相关上下文已在当前轮次、`prd.md`、`design.md`、最近的 `git log` 或已打开文件中。`mem` 用于找回已经离开即时上下文的内容。
- 用户询问的是代码事实，而非过去对话中的事实。使用 `git log -p`、`grep` 或直接读取文件更快且更权威。
- 当前处于子代理（`trellis-implement` / `trellis-check`），其派发提示已包含整理后的 `implement.jsonl` / `check.jsonl` 上下文。额外调用 `mem` 通常只会增加噪声。
- 用户已明确表示“不要翻历史，只回答我问的内容”。

## 如何处理 `mem` 的返回内容

把输出视为**原始材料**，而不是交付物。获得结果后，根据当前对话决定：

- 如果某段过去交流能回答用户当前问题，**在回复中直接引用**，并注明会话 ID / 阶段供用户核验。
- 如果 `mem` 找到了本应记录却遗漏的关键决定，**更新 `<task>/prd.md` 或 `<task>/design.md`**。先向用户说明拟议改动。
- 如果发现属于当前任务记录但不适合放入 PRD，**追加到任务本地注记文件**（例如 `<task>/notes.md`，或扩展现有注记）。
- 如果发现是有助于未来任务的项目级约定或易错点，**更新 `.trellis/spec/`**。为此运行 `trellis-update-spec` Skill；`session-insight` 在发现阶段结束。
- **仅吸收内容**，在接下来几轮中改进回答，不写入任何文件。对于一次性回忆，这通常是正确做法。

Trellis 不规定唯一去向。强迫每次回忆都写入固定文件只会让文件膨胀成噪声，应由具体情境决定。

## 如何调用

完整 CLI 参考见 `references/cli-quick-reference.md`。80% 的场景可使用以下命令之一：

```bash
# Find sessions whose contents mention a keyword (project-scope is default;
# add --global to search every project on this machine).
trellis mem search "<keyword>"

# Dump dialogue from one session, optionally filtered by phase or keyword.
trellis mem extract <session-id> --phase brainstorm
trellis mem extract <session-id> --grep "<keyword>"

# Drill into a session: top-N hit turns + surrounding context.
trellis mem context <session-id> --turns 3 --around 2

# When you do not know the session id yet, start with list + filter.
trellis mem list --cwd <project-path>
trellis mem projects   # → list active project cwds, then narrow
```

阶段切片（`--phase brainstorm|implement|all`）会在 `task.py create` 和 `task.py start` 边界切分会话。回顾当前任务的完成工作时，`--phase brainstorm` 恢复规划讨论，`--phase implement` 恢复实施循环。默认值为 `all`。

## 触发模式

`references/triggering-patterns.md` 列出更多应让你想到“使用 `mem`”的用户原话（英文和中文）；训练判断直觉时可随时查阅。

## 范围外事项

- `mem` 不编辑代码或更新文件。是否写回由你根据当前情境决定。
- `mem` 对平台 JSONL 存储只读，不会推送或同步到远端。
- 此 Skill 不替代 `trellis-update-spec`（把发现提升为项目级指南的正确工具），也不替代平台原生任务/规范工作流。
