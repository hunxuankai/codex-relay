<script setup lang="ts">
import { ElButton, ElCard, ElTag } from 'element-plus'
import type { DraftAuditEvidence } from '../../types/release'

defineProps<{
  draft: DraftAuditEvidence
  busy: boolean
}>()

const emit = defineEmits<{
  publish: []
}>()
</script>

<template>
  <ElCard class="audit-card" shadow="never">
    <template #header>
      <div class="card-heading">
        <div>
          <p class="section-kicker">人工门禁</p>
          <h2>Draft 审计已通过</h2>
        </div>
        <ElTag type="success" effect="dark">可以公开</ElTag>
      </div>
    </template>

    <div class="audit-content">
      <dl class="audit-summary">
        <div><dt>Release ID</dt><dd>{{ draft.releaseId }}</dd></div>
        <div><dt>Tag</dt><dd>{{ draft.tagName }}</dd></div>
        <div><dt>候选提交</dt><dd class="mono">{{ draft.targetCommitSha.slice(0, 12) }}</dd></div>
        <div><dt>Manifest</dt><dd>{{ draft.manifestVersion }}</dd></div>
      </dl>

      <div class="audit-matrix">
        <div><span>Release 身份与说明</span><ElTag type="success" size="small">通过</ElTag></div>
        <div><span>Tag 与候选 SHA</span><ElTag type="success" size="small">通过</ElTag></div>
        <div><span>NSIS / .sig / latest.json</span><ElTag type="success" size="small">通过</ElTag></div>
        <div><span>size / SHA-256 / GitHub digest</span><ElTag type="success" size="small">通过</ElTag></div>
        <div><span>平台 URL 与签名关联</span><ElTag type="success" size="small">通过</ElTag></div>
      </div>

      <ul class="asset-list" aria-label="Draft 资产证据">
        <li v-for="asset in draft.assets" :key="asset.id">
          <span>{{ asset.name }}</span>
          <span class="asset-meta mono">{{ asset.size }} B · {{ asset.sha256.slice(0, 12) }}</span>
        </li>
      </ul>

      <div class="publish-row">
        <p>点击后会再次审计同一 Release ID；身份或资产漂移会立即停止。</p>
        <ElButton type="danger" :loading="busy" @click="emit('publish')">确认正式公开</ElButton>
      </div>
    </div>
  </ElCard>
</template>

<style scoped>
.audit-card :deep(.el-card__body) {
  display: grid;
  gap: 1rem;
  padding: 1.15rem;
}

.card-heading,
.audit-matrix div,
.asset-list li,
.publish-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.card-heading h2,
.section-kicker,
.audit-summary,
.audit-summary dt,
.audit-summary dd,
.asset-list,
.publish-row p {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.audit-content,
.audit-matrix {
  display: grid;
  gap: 0.75rem;
}

.audit-summary {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.75rem;
}

.audit-summary div,
.audit-matrix div {
  padding: 0.65rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 0.7rem;
}

.audit-summary dt {
  color: var(--text-muted);
  font-size: 0.72rem;
}

.audit-summary dd {
  margin-top: 0.2rem;
  font-size: 0.82rem;
  font-weight: 800;
}

.audit-matrix div {
  font-size: 0.8rem;
}

.asset-list {
  padding: 0;
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  list-style: none;
}

.asset-list li {
  padding: 0.55rem 0.75rem;
  font-size: 0.78rem;
}

.asset-list li + li {
  border-top: 1px solid var(--border-color);
}

.asset-meta,
.publish-row p {
  color: var(--text-muted);
}

.mono {
  font-family: var(--font-mono);
}

.publish-row {
  align-items: end;
}

.publish-row p {
  font-size: 0.8rem;
}

@media (max-width: 760px) {
  .audit-summary {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .publish-row {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
