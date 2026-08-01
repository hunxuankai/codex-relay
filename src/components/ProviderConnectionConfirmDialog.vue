<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  shallowRef,
  useTemplateRef,
  watch,
} from 'vue'
import { ElButton, ElDialog } from 'element-plus'
import type { ButtonInstance } from 'element-plus'

type ConnectionConfirmationAction = 'apply' | 'update' | 'restore'

const props = defineProps<{
  open: boolean
  action: ConnectionConfirmationAction
  sourceProviderName: string | null
  targetProviderId: string
  baseUrlName: string
  apiKeyName: string
  busy: boolean
}>()

const emit = defineEmits<{
  confirm: []
  cancel: []
  showRisk: []
  closed: []
}>()

const title = computed(() => {
  if (props.action === 'apply') return '确认仅应用连接'
  if (props.action === 'update') return '确认更新连接'
  return '确认恢复自身连接'
})
const confirmLabel = computed(() => {
  if (props.action === 'apply') return '应用连接'
  if (props.action === 'update') return '更新连接'
  return '恢复连接'
})
const confirmAriaLabel = computed(() => `确认${confirmLabel.value}`)

const cancelButton = useTemplateRef<ButtonInstance>('cancelButton')
const previousFocus = shallowRef<HTMLElement | null>(null)
let focusTimer: ReturnType<typeof setTimeout> | null = null

function clearFocusTimer() {
  if (focusTimer !== null) {
    clearTimeout(focusTimer)
    focusTimer = null
  }
}

function scheduleCancelFocus() {
  clearFocusTimer()
  if (typeof document === 'undefined') return
  focusTimer = setTimeout(() => {
    focusTimer = null
    const element = cancelButton.value?.$el
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
      if (props.open) scheduleCancelFocus()
      return
    }
    clearFocusTimer()
    await nextTick()
    previousFocus.value?.focus()
    previousFocus.value = null
  },
  { immediate: true },
)

onBeforeUnmount(clearFocusTimer)

function handleModelValue(value: boolean) {
  if (!value && props.open && !props.busy) emit('cancel')
}
</script>

<template>
  <ElDialog
    class="provider-connection-confirm-dialog"
    :model-value="open"
    :title="title"
    width="min(32rem, calc(100vw - 2rem))"
    :show-close="false"
    :close-on-click-modal="false"
    :close-on-press-escape="!busy"
    destroy-on-close
    role="alertdialog"
    :aria-describedby="
      action === 'restore'
        ? 'provider-connection-impact'
        : 'provider-connection-impact provider-connection-compatibility-warning'
    "
    @update:model-value="handleModelValue"
    @open-auto-focus="scheduleCancelFocus"
    @opened="scheduleCancelFocus"
    @closed="emit('closed')"
  >
    <dl class="connection-summary">
      <div v-if="action !== 'restore'">
        <dt>来源 Provider</dt>
        <dd>{{ sourceProviderName ?? '未知 Provider' }}</dd>
      </div>
      <div>
        <dt>目标 Provider ID</dt>
        <dd><code>{{ targetProviderId }}</code></dd>
      </div>
      <div>
        <dt>{{ action === 'restore' ? '恢复 Base URL' : '已选 Base URL' }}</dt>
        <dd>{{ baseUrlName }}</dd>
      </div>
      <div>
        <dt>{{ action === 'restore' ? '恢复 API Key' : '已选 API Key' }}</dt>
        <dd>{{ apiKeyName }}</dd>
      </div>
    </dl>
    <p id="provider-connection-impact" class="connection-impact">
      只更新目标身份的连接地址与当前认证，顶层 <code>model_provider</code> 保持不变。
    </p>
    <aside
      v-if="action !== 'restore'"
      id="provider-connection-compatibility-warning"
      class="connection-compatibility-warning"
      role="note"
    >
      <p>旧会话的加密推理或压缩上下文可能与新连接不兼容，恢复会话时可能失败。</p>
      <ElButton
        class="connection-risk-details"
        type="warning"
        text
        native-type="button"
        aria-label="查看旧会话兼容性详细说明"
        :disabled="busy"
        @click="emit('showRisk')"
      >
        查看详细说明
      </ElButton>
    </aside>
    <template #footer>
      <div class="dialog-actions">
        <ElButton
          ref="cancelButton"
          native-type="button"
          aria-label="取消连接确认"
          :disabled="busy"
          @click="emit('cancel')"
        >
          取消
        </ElButton>
        <ElButton
          :type="action === 'restore' ? 'primary' : 'warning'"
          native-type="button"
          :aria-label="confirmAriaLabel"
          :loading="busy"
          :disabled="busy"
          @click="emit('confirm')"
        >
          {{ confirmLabel }}
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.connection-summary {
  display: grid;
  gap: 0;
  margin: 0;
}

.connection-summary div {
  display: grid;
  grid-template-columns: minmax(7rem, 0.75fr) minmax(0, 1.25fr);
  gap: 0.75rem;
  padding: 0.6rem 0;
  border-bottom: 1px solid var(--border);
}

.connection-summary dt {
  color: var(--text-secondary);
}

.connection-summary dd {
  min-width: 0;
  margin: 0;
  overflow-wrap: anywhere;
  font-weight: 600;
}

.connection-impact {
  margin: 0.85rem 0 0;
  color: var(--text-primary);
  line-height: 1.6;
}

.connection-compatibility-warning {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  margin-top: 0.75rem;
  border: 1px solid var(--warning-border);
  border-radius: 0.65rem;
  padding: 0.6rem 0.7rem;
  background: var(--warning-soft);
}

.connection-compatibility-warning p {
  margin: 0;
  color: var(--warning-text);
  line-height: 1.55;
}

.connection-risk-details {
  flex: 0 0 auto;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

@media (max-width: 480px) {
  .connection-summary div {
    grid-template-columns: 1fr;
    gap: 0.2rem;
  }

  .connection-compatibility-warning {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
