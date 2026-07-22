<script setup lang="ts">
import { nextTick, shallowRef, useTemplateRef, watch } from 'vue'
import { ElButton, ElDialog } from 'element-plus'
import type { ButtonInstance } from 'element-plus'

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    message: string
    confirmLabel?: string
    tone?: 'danger' | 'neutral'
  }>(),
  { confirmLabel: '确认', tone: 'danger' },
)

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()

const cancelButton = useTemplateRef<ButtonInstance>('cancelButton')
const previousFocus = shallowRef<HTMLElement | null>(null)

function focusCancelButton() {
  const element = cancelButton.value?.$el
  if (element instanceof HTMLButtonElement) element.focus()
}

function scheduleCancelFocus() {
  setTimeout(focusCancelButton, 0)
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previousFocus.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
      await nextTick()
      scheduleCancelFocus()
      return
    }
    await nextTick()
    previousFocus.value?.focus()
    previousFocus.value = null
  },
  { immediate: true },
)

function handleModelValue(value: boolean) {
  if (!value && props.open) emit('cancel')
}
</script>

<template>
  <ElDialog
    class="confirm-dialog"
    :model-value="open"
    :title="title"
    width="min(28rem, calc(100vw - 2rem))"
    :show-close="false"
    :close-on-click-modal="false"
    destroy-on-close
    role="alertdialog"
    aria-describedby="confirm-dialog-message"
    @update:model-value="handleModelValue"
    @open-auto-focus="scheduleCancelFocus"
    @opened="scheduleCancelFocus"
  >
    <p id="confirm-dialog-message" class="dialog-message">{{ message }}</p>
    <template #footer>
      <div class="dialog-actions">
        <ElButton
          ref="cancelButton"
          native-type="button"
          aria-label="取消确认"
          @click="emit('cancel')"
        >
          取消
        </ElButton>
        <ElButton
          :class="props.tone === 'danger' ? 'danger-button' : 'primary-button'"
          :type="props.tone === 'danger' ? 'danger' : 'primary'"
          native-type="button"
          aria-label="确认操作"
          @click="emit('confirm')"
        >
          {{ confirmLabel }}
        </ElButton>
      </div>
    </template>
  </ElDialog>
</template>

<style scoped>
.dialog-message {
  margin: 0;
  color: var(--text-primary);
  line-height: 1.65;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}
</style>
