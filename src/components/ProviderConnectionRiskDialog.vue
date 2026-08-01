<script setup lang="ts">
import { nextTick, onBeforeUnmount, shallowRef, useTemplateRef, watch } from 'vue'
import { ElButton, ElDialog } from 'element-plus'
import type { ButtonInstance } from 'element-plus'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  close: []
  closed: []
}>()

const closeButton = useTemplateRef<ButtonInstance>('closeButton')
const previousFocus = shallowRef<HTMLElement | null>(null)
let focusTimer: ReturnType<typeof setTimeout> | null = null

function clearFocusTimer() {
  if (focusTimer === null) return
  clearTimeout(focusTimer)
  focusTimer = null
}

function scheduleCloseFocus() {
  clearFocusTimer()
  if (typeof document === 'undefined') return
  focusTimer = setTimeout(() => {
    focusTimer = null
    const element = closeButton.value?.$el
    if (props.open && element instanceof HTMLButtonElement) element.focus()
  }, 0)
}

watch(
  () => props.open,
  async (open) => {
    if (typeof document === 'undefined') return
    if (open) {
      previousFocus.value = document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null
      await nextTick()
      if (props.open) scheduleCloseFocus()
      return
    }

    clearFocusTimer()
    await nextTick()
    if (previousFocus.value?.isConnected) previousFocus.value.focus()
    previousFocus.value = null
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  clearFocusTimer()
  if (previousFocus.value?.isConnected) previousFocus.value.focus()
  previousFocus.value = null
})

function handleModelValue(value: boolean) {
  if (!value && props.open) emit('close')
}
</script>

<template>
  <ElDialog
    class="provider-connection-risk-dialog"
    :model-value="open"
    title="旧会话兼容性说明"
    width="min(36rem, calc(100vw - 2rem))"
    :show-close="false"
    :close-on-click-modal="true"
    :close-on-press-escape="true"
    destroy-on-close
    role="dialog"
    aria-describedby="provider-connection-risk-content"
    @update:model-value="handleModelValue"
    @open-auto-focus="scheduleCloseFocus"
    @opened="scheduleCloseFocus"
    @closed="emit('closed')"
  >
    <div id="provider-connection-risk-content" class="connection-risk-content">
      <p>
        “仅应用连接”只替换 Base URL 与认证，不会改变顶层 <code>model_provider</code> 身份。
      </p>
      <ol>
        <li>旧会话可能保存原上游生成的加密推理、加密压缩上下文或响应状态。</li>
        <li>新连接不保证能识别这些不透明内容，恢复旧会话时可能失败或丢失推理上下文。</li>
        <li>OpenAI-compatible API 不等于会话上下文兼容。</li>
      </ol>
      <p>
        遇到问题时，可先恢复自身连接后重试，或使用新连接创建新会话。
      </p>
    </div>

    <template #footer>
      <div class="dialog-actions">
        <ElButton
          ref="closeButton"
          type="primary"
          native-type="button"
          aria-label="关闭旧会话兼容性说明"
          @click="emit('close')"
        >
          我知道了
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.connection-risk-content {
  display: grid;
  gap: 0.75rem;
  color: var(--text-primary);
  line-height: 1.65;
}

.connection-risk-content p,
.connection-risk-content ol {
  margin: 0;
}

.connection-risk-content ol {
  display: grid;
  gap: 0.55rem;
  padding-left: 1.4rem;
}

.connection-risk-content code {
  overflow-wrap: anywhere;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
}
</style>
