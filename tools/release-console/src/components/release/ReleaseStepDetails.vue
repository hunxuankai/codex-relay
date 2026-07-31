<script setup lang="ts">
import { computed } from 'vue'
import { ElCard, ElTag } from 'element-plus'
import type { ReleaseEvent, ReleaseSession } from '../../types/release'

const props = defineProps<{
  session: ReleaseSession | null
  events: readonly ReleaseEvent[]
}>()

const logs = computed(() =>
  props.events.filter(
    (event): event is Extract<ReleaseEvent, { kind: 'stepLog' | 'stepFailed' }> =>
      event.kind === 'stepLog' || event.kind === 'stepFailed',
  ),
)
</script>

<template>
  <ElCard class="details-card" shadow="never">
    <template #header>
      <div class="details-heading">
        <div>
          <p class="section-kicker">实时证据</p>
          <h2>当前会话</h2>
        </div>
        <ElTag v-if="session" effect="plain">{{ session.phase }}</ElTag>
      </div>
    </template>
    <div class="details-content">
      <dl v-if="session" class="session-facts">
        <div><dt>会话</dt><dd>{{ session.id }}</dd></div>
        <div><dt>版本</dt><dd>{{ session.targetVersion }}</dd></div>
        <div><dt>候选</dt><dd class="mono">{{ session.candidateSha?.slice(0, 12) ?? '尚未提交' }}</dd></div>
        <div><dt>Run</dt><dd>{{ session.workflow?.runId ?? '尚未触发' }}</dd></div>
      </dl>
      <div class="log-view" tabindex="0" aria-label="脱敏发布日志">
        <p v-if="logs.length === 0">尚无脱敏日志；阶段变化会显示在左侧时间线。</p>
        <p v-for="(event, index) in logs" :key="`${event.stepId}-${index}`">
          <strong>[{{ event.stepId }}]</strong>
          {{ event.kind === 'stepLog' ? event.message : `${event.code}：${event.message}` }}
        </p>
      </div>
    </div>
  </ElCard>
</template>

<style scoped>
.details-card :deep(.el-card__body) {
  padding: 1.15rem;
}

.details-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.details-heading h2,
.section-kicker,
.session-facts,
.session-facts dt,
.session-facts dd,
.log-view p {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.details-content {
  display: grid;
  gap: 0.85rem;
}

.session-facts {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.65rem;
}

.session-facts div {
  min-width: 0;
  padding: 0.6rem 0.7rem;
  border: 1px solid var(--border-color);
  border-radius: 0.65rem;
}

.session-facts dt {
  color: var(--text-muted);
  font-size: 0.7rem;
}

.session-facts dd {
  overflow: hidden;
  margin-top: 0.2rem;
  font-size: 0.78rem;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mono {
  font-family: var(--font-mono);
}

.log-view {
  display: grid;
  max-height: 12rem;
  gap: 0.35rem;
  overflow: auto;
  padding: 0.8rem;
  border: 1px solid var(--border-color);
  border-radius: 0.7rem;
  background: var(--log-background);
  color: var(--log-text);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  line-height: 1.5;
}

@media (max-width: 760px) {
  .session-facts {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
