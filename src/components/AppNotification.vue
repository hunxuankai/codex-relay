<script setup lang="ts">
defineProps<{
  message: string | null
  level: 'success' | 'error'
}>()
</script>

<template>
  <p
    v-if="message"
    class="app-notification"
    :class="`notification-${level}`"
    :role="level === 'error' ? 'alert' : 'status'"
    aria-live="polite"
  >
    {{ message }}
  </p>
</template>

<style scoped>
.app-notification {
  margin: 0;
  border: 1px solid currentColor;
  border-radius: 0.7rem;
  padding: 0.7rem 0.9rem;
}

.notification-success {
  color: var(--success);
  background: var(--success-soft);
}

.notification-error {
  color: var(--danger);
  background: var(--danger-soft);
}
</style>
