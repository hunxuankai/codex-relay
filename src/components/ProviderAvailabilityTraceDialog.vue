<script setup lang="ts">
import { nextTick, onBeforeUnmount, shallowRef, useTemplateRef, watch } from 'vue'
import { ElButton, ElDialog } from 'element-plus'
import type { ButtonInstance } from 'element-plus'
import type { ProviderAvailabilityTrace } from '../types/providerAvailability'

const props = defineProps<{
  open: boolean
  providerName: string
  trace: ProviderAvailabilityTrace
  durationMs: number
}>()

const emit = defineEmits<{
  close: []
}>()

const closeButton = useTemplateRef<ButtonInstance>('closeButton')
const previousFocus = shallowRef<HTMLElement | null>(null)
let focusTimer: ReturnType<typeof setTimeout> | undefined
let componentActive = true

function formatDuration(durationMs: number) {
  if (durationMs < 1_000) return `${Math.max(0, Math.round(durationMs))} ms`
  return `${(durationMs / 1_000).toFixed(1)} s`
}

function focusCloseButton() {
  focusTimer = undefined
  if (!componentActive || typeof HTMLButtonElement === 'undefined') return
  const element = closeButton.value?.$el
  if (element instanceof HTMLButtonElement) element.focus()
}

function scheduleFocus() {
  if (!componentActive) return
  if (focusTimer !== undefined) clearTimeout(focusTimer)
  focusTimer = setTimeout(focusCloseButton, 0)
}

onBeforeUnmount(() => {
  componentActive = false
  if (focusTimer !== undefined) clearTimeout(focusTimer)
  focusTimer = undefined
  previousFocus.value = null
})

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previousFocus.value = document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null
      await nextTick()
      if (!componentActive || !props.open) return
      scheduleFocus()
      return
    }
    await nextTick()
    if (!componentActive) return
    previousFocus.value?.focus()
    previousFocus.value = null
  },
  { immediate: true },
)

function handleModelValue(value: boolean) {
  if (!value && props.open) emit('close')
}
</script>

<template>
  <ElDialog
    class="provider-trace-dialog"
    :model-value="open"
    :title="`${providerName} 的 API 请求与响应`"
    width="min(56rem, calc(100vw - 2rem))"
    :close-on-click-modal="false"
    :close-on-press-escape="true"
    destroy-on-close
    @update:model-value="handleModelValue"
    @open-auto-focus="scheduleFocus"
    @opened="scheduleFocus"
  >
    <div class="trace-dialog-content">
      <section class="trace-section" aria-labelledby="provider-trace-request-title">
        <div class="trace-heading">
          <h3 id="provider-trace-request-title">请求</h3>
          <span class="trace-summary">{{ trace.request.method }} {{ trace.request.url }}</span>
        </div>
        <pre class="trace-body" aria-label="API 请求正文">{{ trace.request.body }}</pre>
      </section>

      <section class="trace-section" aria-labelledby="provider-trace-response-title">
        <div class="trace-heading">
          <h3 id="provider-trace-response-title">响应</h3>
          <span class="trace-summary">耗时 {{ formatDuration(durationMs) }}</span>
        </div>
        <template v-if="trace.response">
          <p class="trace-response-status">HTTP {{ trace.response.status }}</p>
          <p v-if="trace.response.bodyTruncated" class="trace-truncated" role="note">
            响应正文已截断，仅显示安全上限内的内容。
          </p>
          <pre class="trace-body" aria-label="API 响应正文">{{ trace.response.body || '（空响应正文）' }}</pre>
        </template>
        <p v-else class="trace-no-response" role="status">
          未收到 HTTP 响应。
        </p>
      </section>
    </div>

    <template #footer>
      <div class="trace-dialog-actions">
        <ElButton
          ref="closeButton"
          native-type="button"
          aria-label="关闭 API 请求与响应详情"
          @click="emit('close')"
        >
          关闭
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.trace-dialog-content,
.trace-section {
  display: grid;
  gap: 0.75rem;
}

.trace-heading,
.trace-dialog-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.trace-heading h3,
.trace-response-status,
.trace-no-response,
.trace-truncated {
  margin: 0;
}

.trace-summary {
  min-width: 0;
  overflow-wrap: anywhere;
  color: var(--text-secondary);
  font-size: 0.85rem;
}

.trace-body {
  max-height: 18rem;
  margin: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 0.55rem;
  padding: 0.75rem;
  background: var(--surface-muted);
  color: var(--text-primary);
  font: 0.82rem/1.55 ui-monospace, SFMono-Regular, Consolas, monospace;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.trace-response-status {
  color: var(--text-secondary);
  font-weight: 600;
}

.trace-no-response,
.trace-truncated {
  color: var(--text-secondary);
}

@media (max-width: 620px) {
  .trace-heading {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
