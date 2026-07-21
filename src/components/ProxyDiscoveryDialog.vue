<script setup lang="ts">
import { nextTick, shallowRef, useTemplateRef, watch } from 'vue'

const props = defineProps<{
  open: boolean
  candidates: readonly string[]
  selected: string | null
}>()

const emit = defineEmits<{
  select: [proxy: string]
  confirm: []
  cancel: []
}>()

const dialog = useTemplateRef<HTMLElement>('dialog')
const closeButton = useTemplateRef<HTMLButtonElement>('closeButton')
const previousFocus = shallowRef<HTMLElement | null>(null)

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previousFocus.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
      await nextTick()
      closeButton.value?.focus()
    } else {
      await nextTick()
      previousFocus.value?.focus()
      previousFocus.value = null
    }
  },
)

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('cancel')
  if (event.key !== 'Tab' || !dialog.value) return
  const focusable = Array.from(dialog.value.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled])'))
  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault()
    last?.focus()
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault()
    first?.focus()
  }
}
</script>

<template>
  <div v-if="open" class="dialog-backdrop" role="presentation">
    <section
      ref="dialog"
      class="proxy-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="proxy-dialog-title"
      @keydown="handleKeydown"
    >
      <h2 id="proxy-dialog-title" class="dialog-title">本机代理检测结果</h2>
      <fieldset v-if="candidates.length" class="proxy-options">
        <legend>选择要启用的代理</legend>
        <label v-for="candidate in candidates" :key="candidate" class="proxy-option">
          <input
            type="radio"
            name="detected-proxy"
            :value="candidate"
            :checked="candidate === selected"
            @change="emit('select', candidate)"
          />
          <code>{{ candidate }}</code>
        </label>
      </fieldset>
      <p v-else>未检测到可用于访问更新源的本机代理。</p>
      <div class="dialog-actions">
        <button ref="closeButton" type="button" @click="emit('cancel')">关闭</button>
        <button
          v-if="candidates.length"
          data-action="apply-proxy"
          type="button"
          :disabled="!selected"
          @click="emit('confirm')"
        >
          一键填入并启用
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
  background: var(--overlay);
}

.proxy-dialog {
  width: min(32rem, 100%);
  border-radius: 1rem;
  padding: 1.25rem;
  background: var(--surface);
  box-shadow: var(--shadow);
}

.dialog-title {
  margin-top: 0;
}

.proxy-options {
  display: grid;
  gap: 0.75rem;
  border: 0;
  padding: 0;
}

.proxy-option {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  margin-top: 1rem;
}
</style>
