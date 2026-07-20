# 本地多 Agent Channel 运行时

`trellis channel` 是随 Trellis CLI 提供的本地多 Agent 协作运行时。它让主 AI 会话能够派生对等 Worker（Claude Code、Codex 或 `.trellis/agents/` 下的任意 Agent 定义），通过事件日志交换持久消息，并协调审查或头脑风暴循环，而无需手工拼接 Shell 管道。

本参考资料说明 channel 如何接入用户项目，帮助定制项目的 AI 知道应编辑什么。运行时用法（命令、forum/thread 模式、Worker 派生参数）以捆绑的 `trellis-channel` 能力 Skill 为准。

## 本地系统模型

Channel 运行时跨越三个本地层面：

1. 用户主目录中的**存储层**：持久事件日志和 Worker 状态文件。
2. 项目 `.trellis/agents/` 中的 **Agent 定义**：由 `trellis channel spawn --agent <name>` 使用的平台无关角色卡。
3. `.trellis/config.yaml` 中的**项目配置**：Worker 保护阈值和其他 channel 调节项。

## 核心路径

| 路径 | 用途 |
| --- | --- |
| `~/.trellis/channels/<project>/<channel>/events.jsonl` | 每个 channel 的仅追加事件日志；序列加锁且可安全重放。 |
| `~/.trellis/channels/<project>/<channel>/<channel>.lock` | Channel 级写锁。 |
| `~/.trellis/channels/<project>/<channel>/<worker>.spawnlock` | OOM 保护使用的逐 Worker 派生锁。 |
| `~/.trellis/channels/<project>/<channel>/.seq` | 用于按序分配事件的序列辅助文件。 |
| `~/.trellis/channels/_global/<channel>/...` | 使用 `--scope global` 创建的 channel；项目桶替换为共享键。 |
| `.trellis/agents/check.md` | `--agent check` 使用的默认 Check Agent 角色定义。 |
| `.trellis/agents/implement.md` | `--agent implement` 使用的默认 Implement Agent 角色定义。 |
| `.trellis/config.yaml`（`channel.*` 块） | Worker 保护阈值和 channel 默认值。 |

项目桶名称由项目绝对路径派生（将斜杠扁平化，并把非字母数字字符替换为 `-`），与 Claude Code 的 `~/.claude/projects/<sanitized-cwd>/` 约定一致。测试或沙箱环境可通过 `TRELLIS_CHANNEL_ROOT`（根目录）或 `TRELLIS_CHANNEL_PROJECT`（桶名称）覆盖。

## 何时使用 Channel 运行时

Channel 比单次 Bash 调用或一次性子 Agent 派发更重。只有至少满足以下一个条件时才使用：

- 工作需要**两个或更多 Agent 进行多轮对话**（跨 AI 头脑风暴、同行审查、调度器 + Worker）。
- Worker 应作为**对等进程**运行，主会话可以中断、观察其进度或异步等待。
- 对话必须能够在以后**持久保存并检查**（forum/thread channel、问题看板、决策轨迹）。
- 多个 Worker 必须**共享事件日志**，让每个 Worker 都能看到其他 Worker 的报告。

以下情况优先使用更轻量的原语：

- 单次 Bash 命令或单次 Agent 工具调用已经足够 → 直接使用。
- 用户只需要针对文件进行静态审查 → 读取文件并在当前会话回复。
- 需求是“回忆我们上周讨论了什么” → 使用 `trellis mem`，而不是 channel。

## 定制点

| 需求 | 编辑位置 |
| --- | --- |
| 修改 channel Worker 默认空闲超时 | `.trellis/config.yaml` 中的 `channel.worker_guard.idle_timeout`。接受 `5m`、`30s` 等值；设为 `0` 可禁用空闲清理。 |
| 修改实时 Worker 预算 | `.trellis/config.yaml` 中的 `channel.worker_guard.max_live_workers`。设为 `0` 可禁用派生时预算检查。 |
| 按次派生覆盖 Worker 保护设置 | 向 `trellis channel spawn` 传递 `--idle-timeout` / `--max-live-workers`，或在环境中设置 `TRELLIS_CHANNEL_WORKER_IDLE_TIMEOUT` / `TRELLIS_CHANNEL_MAX_LIVE_WORKERS`。 |
| 修改默认 Check 或 Implement Worker 的行为 | 编辑 `.trellis/agents/check.md` 或 `.trellis/agents/implement.md`。它们是平台无关角色卡；传入 `--agent check|implement` 时由 channel 运行时注入。 |
| 添加新角色卡 | 把 `<name>.md` 放入 `.trellis/agents/`；`trellis channel spawn --agent <name>` 会自动使用。 |
| 迁移 channel 存储位置（CI 沙箱、临时运行） | 设置 `TRELLIS_CHANNEL_ROOT=/path/to/dir`。Channel 事件随之迁移；现有 channel 仍留在旧根目录。 |
| 切换存储范围 | 在每个 channel 子命令上传递 `--scope project`（默认）或 `--scope global`。只有桶目录发生变化。 |

Worker 保护设置优先级为：CLI 参数 > 环境变量 > `.trellis/config.yaml` > 内置默认值。内置默认值为 `idle_timeout: 5m` 和 `max_live_workers: 6`。

## 与其他本地层的关系

- **工作流层**：使用 channel 派发的工作流（例如 `channel-driven-subagent-dispatch`）会指示主 Agent 调用 `trellis channel spawn --agent check` 或 `--agent implement`，而不是平台子 Agent。如果缺少 `.trellis/agents/check.md` 或 `implement.md`，`trellis workflow --template <id>` 会在安装时输出非阻塞警告。误删后可使用 `trellis update` 恢复。
- **任务层**：channel Worker 不拥有任务状态。监督它的主会话通过 Worker 收件箱传递活动任务路径；Worker 从磁盘解析任务产物。
- **Spec 层**：Worker 与主会话以相同方式读取 `.trellis/spec/`。Channel 运行时不会绕过 spec 上下文加载。
- **平台集成层**：channel 运行时与平台无关，不依赖 `.claude/`、`.codex/` 或其他平台目录。标准化 Provider 输出的适配器（Claude `stream-json`、Codex `app-server`）位于 Trellis CLI 二进制中，而不是项目内。
- **平台子 Agent 文件与 channel Worker**：编辑 `.claude/agents/trellis-implement.md`（以及其他平台 `.X/agents/` 目录中的对应文件）**不会**改变 channel 运行时 Worker 的行为——channel Worker 加载 `.trellis/agents/<name>.md`。平台专属 Agent 文件用于主 AI 会话直接派发子 Agent，而不是用于 channel 派生的 Worker。各平台 Agent 层面参见 `platform-files/agents.md`，该区分由 `trellis-meta/SKILL.md` 规则固化。

## 运行时用法

有关命令语法、forum/thread 模式、Worker 句柄、进度检查，以及 `--kind done` / `--kind turn_finished` 调度器等待模式，请加载捆绑的 `trellis-channel` Skill（运行 `trellis init` / `trellis update` 后会自动安装到各平台 Skill 目录）。本参考资料只介绍本地文件布局和定制选项，不重复可能随版本变化的命令语法。
