<script setup lang="ts">
import { ElCard, ElTag } from 'element-plus'
import type { ReleaseSession } from '../../types/release'

defineProps<{
  session: ReleaseSession | null
}>()
</script>

<template>
  <ElCard class="details-card" shadow="never">
    <template #header>
      <div class="details-heading">
        <div>
          <p class="section-kicker">会话证据</p>
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
      <p v-else class="session-empty">尚未开始发布会话。</p>
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
.session-empty {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0;
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

.session-empty {
  color: var(--text-muted);
  font-size: 0.78rem;
}

@media (max-width: 760px) {
  .session-facts {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
