<script setup lang="ts">
import { ElAlert } from 'element-plus'
import { shallowRef, watch } from 'vue'

const SUCCESS_DISMISS_DELAY_MS = 5_000

const props = withDefaults(defineProps<{
  message: string | null
  level: 'success' | 'error'
  messageId?: number
}>(), {
  messageId: 0,
})

const visible = shallowRef(false)

watch(
  () => [props.message, props.level, props.messageId] as const,
  ([message, level], _previous, onCleanup) => {
    visible.value = Boolean(message)
    if (!message || level !== 'success') return

    const timer = setTimeout(() => {
      visible.value = false
    }, SUCCESS_DISMISS_DELAY_MS)
    onCleanup(() => clearTimeout(timer))
  },
  { immediate: true },
)
</script>

<template>
  <ElAlert
    v-if="message && visible"
    class="app-notification"
    :type="level"
    :title="message"
    :closable="false"
    show-icon
    :role="level === 'error' ? 'alert' : 'status'"
    aria-live="polite"
  />
</template>

<style scoped>
.app-notification {
  margin: 0;
}
</style>
