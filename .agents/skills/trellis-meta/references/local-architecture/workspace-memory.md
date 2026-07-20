# 本地工作区记忆系统

`.trellis/workspace/` 存储跨会话记忆，目的是让 AI 和人类理解不同窗口、不同日期中发生过什么。

## 目录结构

```text
.trellis/workspace/
├── index.md
└── <developer>/
    ├── index.md
    ├── journal-1.md
    └── journal-2.md
```

| 文件 | 用途 |
| --- | --- |
| `.trellis/.developer` | 当前开发者身份。 |
| `.trellis/workspace/index.md` | 全局工作区概览。 |
| `.trellis/workspace/<developer>/index.md` | 某开发者的会话索引。 |
| `.trellis/workspace/<developer>/journal-N.md` | 会话日志。 |

## 开发者身份

首次运行：

```bash
python ./.trellis/scripts/init_developer.py <name>
```

该命令创建 `.trellis/.developer` 和对应工作区目录。AI 不应随意更改开发者身份；如果身份错误，先确认当前项目的使用者。

## 日志

`journal-N.md` 记录每次会话已完成或部分完成的工作。默认每个日志约容纳 2000 行，超过后轮换到下一个文件。

记录会话的常用命令：

```bash
python ./.trellis/scripts/add_session.py \
  --title "Session title" \
  --summary "What changed" \
  --commit "abc1234"
```

没有提交的规划或审查工作也可以使用 `--no-commit` 或空提交值记录。

## 工作区记忆与任务的关系

| 系统 | 存储内容 |
| --- | --- |
| `.trellis/tasks/` | 特定任务的需求、设计、研究和状态。 |
| `.trellis/workspace/` | 跨任务和会话的工作记录。 |
| `.trellis/spec/` | 作为长期约定保存的工程知识。 |

如果信息只对当前任务有用，放入任务目录。
如果信息描述当前会话发生了什么，放入工作区日志。
如果信息在未来每次编写代码时都应遵循，放入规范。

## 本地定制点

| 需求 | 编辑位置 |
| --- | --- |
| 修改日志最大行数 | `.trellis/config.yaml` 中的 `max_journal_lines`。 |
| 修改会话自动提交信息 | `.trellis/config.yaml` 中的 `session_commit_message`。 |
| 修改会话内容格式 | `.trellis/scripts/add_session.py`。 |
| 修改工作区在上下文中的显示方式 | `.trellis/scripts/common/session_context.py`。 |

## AI 使用规则

AI 不应把工作区视为唯一事实来源。恢复任务时先读取当前任务，再用工作区补充背景。任务完成后，把重要过程注记记录到工作区；如果产生长期规则，更新规范。
