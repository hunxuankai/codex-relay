<script setup lang="ts">
import { ElButton, ElTag } from 'element-plus'
import type { UnavailableBackup } from '../types/backup'

defineProps<{
  backups: readonly UnavailableBackup[]
  busy: boolean
}>()

const emit = defineEmits<{
  openMetadata: [directoryName: string]
}>()
</script>

<template>
  <section class="unavailable-backups" aria-labelledby="unavailable-backups-title">
    <div class="unavailable-heading">
      <div>
        <h2 id="unavailable-backups-title">无法安全恢复的备份</h2>
        <p>这些备份已保留，当前版本不会修改或删除它们。</p>
      </div>
      <ElTag type="warning" effect="plain">{{ backups.length }} 份</ElTag>
    </div>
    <ul class="unavailable-list">
      <li v-for="backup in backups" :key="backup.directoryName" class="unavailable-item">
        <div>
          <strong>{{ backup.directoryName }}</strong>
          <p>{{ backup.message }}</p>
        </div>
        <ElButton
          v-if="backup.canOpenMetadata"
          native-type="button"
          :aria-label="`打开不可用备份 ${backup.directoryName} 的元数据`"
          :disabled="busy"
          @click="emit('openMetadata', backup.directoryName)"
        >
          打开元数据
        </ElButton>
        <span v-else class="metadata-unavailable">元数据文件不可用，无法打开。</span>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.unavailable-backups,
.unavailable-list {
  display: grid;
  gap: 0.75rem;
}

.unavailable-backups {
  border-block: 1px solid var(--warning-border);
  padding: 1rem;
  background: var(--warning-soft);
}

.unavailable-heading,
.unavailable-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.unavailable-heading h2,
.unavailable-heading p,
.unavailable-item p {
  margin: 0;
}

.unavailable-heading h2 {
  font-size: 1rem;
}

.unavailable-heading p,
.unavailable-item p,
.metadata-unavailable {
  color: var(--text-secondary);
  line-height: 1.55;
}

.unavailable-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.unavailable-item {
  align-items: start;
  border-top: 1px solid var(--warning-border);
  padding-top: 0.75rem;
}

@media (max-width: 620px) {
  .unavailable-heading,
  .unavailable-item {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
