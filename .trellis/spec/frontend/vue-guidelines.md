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
