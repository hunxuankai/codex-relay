<script setup lang="ts">
import { ElAlert } from 'element-plus'

defineProps<{
  message: string | null
  level: 'success' | 'error'
}>()
</script>

<template>
  <ElAlert
    v-if="message"
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
