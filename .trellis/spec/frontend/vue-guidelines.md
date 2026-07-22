# Vue 与类型约定

## 目录职责

- `src/views/`：页面级编排，不直接调用 `invoke`。
- `src/components/`：可复用展示和局部交互，通过 typed props/emits 协作。
- `src/composables/`：资源状态、事件订阅和显式业务动作。
- `src/services/tauri.ts`：唯一 Tauri IPC 边界。
- `src/types/`：与 Rust command DTO 对应的 TypeScript 类型。

## 组件形式

所有 Vue 文件使用 Composition API：

```vue
<script setup lang="ts">
const props = defineProps<{ provider: ProviderSummary }>()
const emit = defineEmits<{ select: [providerId: string] }>()
</script>
```

不得新增 Options API、隐式 `any`、未类型化 emit 或组件内散落的字符串 command 名。

使用 Element Plus 组件前，必须查阅当前安装版本的官方文档和类型声明，核对组件名、Props、Events、Slots 及 `v-model` 类型；不得凭记忆猜测 API。当前项目的模型编辑使用 `ElSelect` 多选，详情偏好使用两行 `ElSegmented`。

## Element Plus 使用边界

- 交互控件、表单、对话框、反馈、状态标签、空状态和进度优先使用 Element Plus；`main`、`nav`、`section`、`header`、`dl`、`ul/ol` 等有意义的文档结构保留原生语义。
- `src/style.css` 的项目变量是视觉唯一事实来源，必须同步映射到 `--el-*` 变量；新增 Element Plus 组件时同时检查浅色、`prefers-color-scheme: dark`、约 44px 交互目标和窄窗口布局。
- 使用 Element Plus 后不得再用全局 `button:hover`、`button:disabled`、`input:not(...)` 或 `input[type=checkbox]` 重设背景、边框、文字和尺寸；这些选择器会命中组件内部原生节点，产生白字浅底、双层输入框和开关错位。应改用 `.el-button`、`.el-input__wrapper`、`.el-switch` 等明确组件边界。
- `ElCard` 的插槽内容位于 `.el-card__body`；grid、gap、padding、对齐等内容布局必须写在 `:deep(.el-card__body)`，卡片根类只负责边框、背景、圆角和选中态。
- `ElButton` 会为默认插槽增加内容 `<span>`；按钮内包含图标与文字或多个字段时，必须增加显式内容容器，或对 `.el-button > span` 设置布局，不能假设按钮的直接子节点仍是业务元素。
- 继续显式导入组件并由 `unplugin-element-plus` 按需注入样式；不得改成 `app.use(ElementPlus)` 全量导入。
- 危险确认通过共享 `ConfirmDialog` 使用 `ElDialog`；必须关闭遮罩点击关闭，默认聚焦安全动作，并验证 Escape 与关闭后焦点恢复。
- 测试优先使用可见文本、`aria-label`、公开 props/emits 或组件类型，不锁定 `.el-*` 私有 DOM 层级。

```vue
<!-- 正确：Element Plus 提供交互，原生元素保留页面语义 -->
<nav aria-label="主导航">
  <ElButton text native-type="button">Providers</ElButton>
</nav>

<ElCard class="provider-card">
  <!-- 内容 -->
</ElCard>

<style scoped>
.provider-card :deep(.el-card__body) {
  display: grid;
  gap: 0.75rem;
  padding: 1rem;
}
</style>

<!-- 错误：为了组件覆盖率把所有语义结构机械替换成通用容器 -->
```

## IPC 契约

只有 `src/services/tauri.ts` 可以导入 `@tauri-apps/api/core` 的 `invoke`。它负责：

- 精确命令名和 camelCase 参数；
- `CommandResult<T>` 解包；
- 把稳定错误码和安全中文消息映射给调用方；
- 不把密钥放入普通 Provider DTO。

```ts
// 错误：组件直接调用后端
await invoke('switch_provider', { providerId })

// 正确：组件/composable 只使用 typed service
await tauri.switchProvider(providerId, fingerprint)
```

## 表单与密钥

- Provider ID 创建后只读。
- 名称、HTTP(S) Base URL、固定 `responses`、模型和密钥动作在前端即时校验，Rust 必须再次验证。
- API Key 默认密码显示；显示/隐藏按钮必须可访问。
- 编辑时未触碰密钥提交 `unchanged`；明确清空提交 `clear` 并二次确认。
- 密钥只短暂存在编辑器局部内存，不进入 localStorage、日志、通知、快照或普通 composable 状态。

## 测试

组件和视图使用 Vitest + Vue Test Utils，通过 mock `src/services/tauri.ts` 或 composable 边界验证用户行为，不访问真实文件系统。
