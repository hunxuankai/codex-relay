# 修改本地规范结构

当用户希望修改 AI 遵循的工程约定、添加新的规范层或调整 monorepo 包映射时，编辑 `.trellis/spec/` 和 `.trellis/config.yaml`。

## 编辑前先读取这些文件

1. `.trellis/config.yaml`
2. `.trellis/spec/`
3. `.trellis/workflow.md` 中的规划产物指南和 Phase 3.3
4. 当前任务的 `implement.jsonl` / `check.jsonl`

## 常见需求

| 需求 | 编辑位置 |
| --- | --- |
| 添加 backend/frontend/docs/test 规范层 | `.trellis/spec/<layer>/` 或 `.trellis/spec/<package>/<layer>/` |
| 添加共享思考指南 | `.trellis/spec/guides/` |
| 调整 monorepo 包 | `.trellis/config.yaml` 中的 `packages` |
| 修改默认包 | `.trellis/config.yaml` 中的 `default_package` |
| 控制规范扫描范围 | `.trellis/config.yaml` 中的 `spec_scope` |
| 让任务读取新规范 | 任务的 `implement.jsonl` / `check.jsonl` |

## 添加规范层

单仓库示例：

```text
.trellis/spec/security/
├── index.md
└── auth.md
```

Monorepo 示例：

```text
.trellis/spec/webapp/security/
├── index.md
└── auth.md
```

`index.md` 应包含：

- 该规范层适用于哪些代码。
- 开发前检查。
- 质量检查。
- 指向具体规范文件的链接。

## 更新上下文

添加规范并不意味着每个任务都会自动读取它。当前任务必须在 JSONL 中引用该规范：

```bash
python ./.trellis/scripts/task.py add-context <task> implement ".trellis/spec/webapp/security/index.md" "Security conventions"
python ./.trellis/scripts/task.py add-context <task> check ".trellis/spec/webapp/security/index.md" "Security review rules"
```

## 修改 Monorepo 包

`.trellis/config.yaml` 示例：

```yaml
packages:
  webapp:
    path: apps/web
  api:
    path: apps/api
default_package: webapp
```

编辑后运行：

```bash
python ./.trellis/scripts/get_context.py --mode packages
```

使用该输出来确认 AI 能看到正确的包和规范层。

## 注意事项

- 规范是用户项目约定，可以根据项目需要修改。
- 不要把临时任务信息放入规范；临时信息应放在任务中。
- 不要只在 Agent 或命令中记录长期约定；应将其保存在规范中。
- 修改规范结构后，检查现有任务 JSONL 是否仍指向实际存在的文件。
