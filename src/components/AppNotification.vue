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
  color: #087443;
  background: #ecfdf3;
}

.notification-error {
  color: #b42318;
  background: #fef3f2;
}
</style>
