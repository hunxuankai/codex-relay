<script setup lang="ts">
import type { BackupFileName, BackupSummary } from '../types/backup'

const props = defineProps<{
  backup: BackupSummary
  expanded: boolean
  busy: boolean
}>()

const emit = defineEmits<{
  toggle: [directoryName: string]
  openFile: [directoryName: string, fileName: BackupFileName]
  restore: [directoryName: string]
}>()

const fileListId = `backup-files-${encodeURIComponent(props.backup.directoryName)}`
</script>

<template>
  <li class="backup-card">
    <div class="backup-card-summary">
      <div class="backup-details">
        <strong>{{ backup.metadata.transactionId }}</strong>
        <span>{{ backup.metadata.createdAt }}</span>
        <span>操作：{{ backup.metadata.operation }}</span>
        <span>Provider：{{ backup.metadata.providerId ?? '无' }}</span>
        <span>应用版本：{{ backup.metadata.appVersion }}</span>
      </div>
      <div class="backup-actions">
        <button
          type="button"
          :aria-label="`${expanded ? '收起' : '查看'}备份文件 ${backup.metadata.transactionId}`"
          :aria-expanded="expanded"
          :aria-controls="fileListId"
          @click="emit('toggle', backup.directoryName)"
        >
          {{ expanded ? '收起文件' : '查看文件' }}
        </button>
        <button
          type="button"
          :aria-label="`恢复备份 ${backup.metadata.transactionId}`"
          :disabled="busy"
          @click="emit('restore', backup.directoryName)"
        >
          恢复
        </button>
      </div>
    </div>

    <ul v-if="expanded" :id="fileListId" class="backup-files">
      <li v-for="fileName in backup.files" :key="fileName">
        <button
          type="button"
          class="backup-file-button"
          :aria-label="`打开备份文件 ${fileName}`"
          :disabled="busy"
          @click="emit('openFile', backup.directoryName, fileName)"
        >
          {{ fileName }}
        </button>
      </li>
    </ul>
  </li>
</template>

<style scoped>
.backup-card {
  display: grid;
  gap: 0.85rem;
  border: 1px solid var(--border);
  background: var(--surface);
  border-radius: 0.8rem;
  padding: 1rem;
}

.backup-card-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.backup-details,
.backup-actions {
  display: grid;
  gap: 0.25rem;
}

.backup-actions {
  grid-auto-flow: column;
  gap: 0.5rem;
}

.backup-files {
  display: grid;
  gap: 0.5rem;
  margin: 0;
  padding: 0.75rem 0 0;
  border-top: 1px solid var(--border);
  list-style: none;
}

.backup-file-button {
  width: 100%;
  text-align: left;
}

@media (max-width: 620px) {
  .backup-card-summary {
    align-items: stretch;
    flex-direction: column;
  }

  .backup-actions {
    grid-auto-flow: row;
  }
}
</style>
