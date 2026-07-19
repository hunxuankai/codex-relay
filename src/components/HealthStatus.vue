<script setup lang="ts">
import { computed } from 'vue'
import type { DeepReadonly } from 'vue'
import type { HealthReport } from '../types/health'

const props = defineProps<{
  report: DeepReadonly<HealthReport> | null
  loading: boolean
  busy: boolean
  errorMessage: string | null
}>()

const emit = defineEmits<{
  rerun: []
}>()

const summaryLabel = computed(() => {
  if (props.loading && !props.report) return '检查中'
  if (props.report?.level === 'normal') return '正常'
  if (props.report?.level === 'warning') return '警告'
  if (props.report?.level === 'error') return '错误'
  return '尚未检查'
})
</script>

<template>
  <section class="health-status" aria-label="自检状态" aria-live="polite">
    <header class="health-header">
      <div>
        <p class="eyebrow">Health</p>
        <h1>系统自检</h1>
      </div>
      <span class="health-summary" :data-level="report?.level ?? 'unknown'">
        {{ summaryLabel }}
      </span>
    </header>

    <p v-if="errorMessage" class="health-error" role="alert">{{ errorMessage }}</p>
    <p v-if="loading && !report">正在执行启动自检…</p>

    <ul v-if="report" class="health-checks">
      <li
        v-for="check in report.checks"
        :key="check.id"
        class="health-check"
        :data-check-id="check.id"
        :data-level="check.level"
      >
        <div class="check-heading">
          <strong>{{ check.label }}</strong>
          <span>{{ check.level === 'normal' ? '正常' : check.level === 'warning' ? '警告' : '错误' }}</span>
        </div>
        <p>{{ check.message }}</p>
      </li>
    </ul>

    <button
      type="button"
      aria-label="重新运行完整自检"
      :disabled="busy"
      @click="emit('rerun')"
    >
      {{ busy ? '正在检查…' : '重新运行完整自检' }}
    </button>
  </section>
</template>

<style scoped>
.health-status,
.health-checks {
  display: grid;
  gap: 1rem;
}

.health-header,
.check-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.health-header h1,
.eyebrow,
.health-check p {
  margin: 0;
}

.eyebrow {
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.health-summary,
.health-check {
  border: 1px solid var(--border);
  background: var(--surface);
  border-radius: 0.75rem;
  padding: 0.75rem;
}

.health-summary[data-level='normal'],
.health-check[data-level='normal'] {
  border-color: var(--success);
}

.health-summary[data-level='warning'],
.health-check[data-level='warning'] {
  border-color: var(--warning-border);
}

.health-summary[data-level='error'],
.health-check[data-level='error'],
.health-error {
  color: var(--danger);
}

.health-checks {
  margin: 0;
  padding: 0;
  list-style: none;
}
</style>
