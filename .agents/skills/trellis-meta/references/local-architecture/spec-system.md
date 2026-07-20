# 本地规范系统

`.trellis/spec/` 是用户项目专属的工程规范库。Trellis 不是让 AI 背诵约定，而是在正确时机注入相关规范或要求 AI 读取。

## 目录模型

常见的单仓库结构：

```text
.trellis/spec/
├── backend/
│   ├── index.md
│   └── ...
├── frontend/
│   ├── index.md
│   └── ...
└── guides/
    ├── index.md
    └── ...
```

常见的 Monorepo 结构：

```text
.trellis/spec/
├── cli/
│   ├── backend/
│   │   ├── index.md
│   │   └── ...
│   └── unit-test/
│       ├── index.md
│       └── ...
├── docs-site/
│   └── docs/
│       ├── index.md
│       └── ...
└── guides/
    ├── index.md
    └── ...
```

`index.md` 是每层的入口。它应列出开发前检查和质量检查。具体规范位于同目录的其他 Markdown 文件中。

## 包配置

`.trellis/config.yaml` 可以声明包：

```yaml
packages:
  cli:
    path: packages/cli
  docs-site:
    path: docs-site
    type: submodule
default_package: cli
```

AI 可以运行：

```bash
python ./.trellis/scripts/get_context.py --mode packages
```

该命令列出当前项目的包和规范层。配置上下文 JSONL 时以此输出为参考。

## 规范如何进入任务

任务进入实施前，如果需要任务材料之外的规范或研究上下文，规划阶段可以把相关规范写入 `implement.jsonl` / `check.jsonl`：

```jsonl
{"file": ".trellis/spec/cli/backend/index.md", "reason": "CLI backend conventions"}
{"file": ".trellis/spec/cli/unit-test/conventions.md", "reason": "Test expectations"}
```

子代理或平台前导指令读取这些 JSONL 文件并加载所引用的规范。在不支持子代理的平台上，AI 应按工作流直接读取相关规范。

## 规范应包含什么

规范应包含项目可执行的工程约定，而不是通用最佳实践：

- 文件应放在哪里。
- 错误处理应如何表达。
- API、钩子和命令的输入/输出契约。
- 禁止的模式。
- 必须测试的情况。
- 项目专属陷阱及规避方式。

AI 在实施或调试中学到新规则时，应更新 `.trellis/spec/`，而不是只在聊天中总结。

## 本地定制点

| 需求 | 编辑位置 |
| --- | --- |
| 添加新规范层 | `.trellis/spec/<package>/<layer>/index.md` 和对应规范文件。 |
| 修改 Monorepo 规范映射 | `.trellis/config.yaml` 中的 `packages` / `default_package` / `spec_scope`。 |
| 修改 AI 实施前读取的规范 | 任务的 `implement.jsonl`。 |
| 修改 AI 检查时读取的规范 | 任务的 `check.jsonl`。 |
| 修改规范更新时机 | `.trellis/workflow.md` 的阶段 3.3 和 `trellis-update-spec` Skill。 |

## 边界

`.trellis/spec/` 是用户的项目规范，而不是 Trellis 内置模板的永久副本。AI 应鼓励用户根据真实项目代码更新它，不要把 Trellis 默认模板视为不可变文档。
