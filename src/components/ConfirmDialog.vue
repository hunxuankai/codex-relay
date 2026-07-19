<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, shallowRef, useTemplateRef, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    open: boolean
    title: string
    message: string
    confirmLabel?: string
  }>(),
  { confirmLabel: '确认' },
)

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()

const dialog = useTemplateRef<HTMLElement>('dialog')
const cancelButton = useTemplateRef<HTMLButtonElement>('cancelButton')
const previousFocus = shallowRef<HTMLElement | null>(null)

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previousFocus.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
      await nextTick()
      cancelButton.value?.focus()
    } else {
      await nextTick()
      previousFocus.value?.focus()
      previousFocus.value = null
    }
  },
  { immediate: true },
)

function handleKeydown(event: KeyboardEvent) {
  if (!props.open) return
  if (event.key === 'Escape') {
    event.preventDefault()
    emit('cancel')
    return
  }
  if (event.key !== 'Tab' || !dialog.value) return
  const focusable = Array.from(
    dialog.value.querySelectorAll<HTMLElement>('button:not([disabled]), [href], input:not([disabled])'),
  )
  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  if (!first || !last) return
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault()
    last.focus()
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault()
    first.focus()
  }
}

onMounted(() => document.addEventListener('keydown', handleKeydown))
onBeforeUnmount(() => document.removeEventListener('keydown', handleKeydown))
</script>

<template>
  <div v-if="open" class="dialog-backdrop" role="presentation">
    <section
      ref="dialog"
      class="confirm-dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirm-dialog-title"
      aria-describedby="confirm-dialog-message"
    >
      <h2 id="confirm-dialog-title" class="dialog-title">{{ title }}</h2>
      <p id="confirm-dialog-message" class="dialog-message">{{ message }}</p>
      <div class="dialog-actions">
        <button
          ref="cancelButton"
          type="button"
          aria-label="取消确认"
          @click="emit('cancel')"
        >
          取消
        </button>
        <button
          class="danger-button"
          type="button"
          aria-label="确认操作"
          @click="emit('confirm')"
        >
          {{ confirmLabel }}
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.dialog-backdrop {
  position: fixed;
  z-index: 100;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 1rem;
  background: rgb(16 24 40 / 55%);
}

.confirm-dialog {
  width: min(28rem, 100%);
  border-radius: 1rem;
  padding: 1.25rem;
  background: #fff;
  box-shadow: 0 1.5rem 4rem rgb(16 24 40 / 28%);
}

.dialog-title,
.dialog-message {
  margin-top: 0;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
}

.danger-button {
  color: #fff;
  background: #b42318;
}
</style>
