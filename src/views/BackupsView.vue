<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import AppNotification from '../components/AppNotification.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import { useBackups } from '../composables/useBackups'

const emit = defineEmits<{
  restored: []
}>()

const backupState = useBackups()
const restoreDirectoryName = shallowRef<string | null>(null)
const selectedBackup = computed(
  () =>
    backupState.backups.value.find(
      (backup) => backup.directoryName === restoreDirectoryName.value,
    ) ?? null,
)

async function confirmRestore() {
  const directoryName = restoreDirectoryName.value
  if (!directoryName) return
  restoreDirectoryName.value = null
  await backupState.restore(directoryName)
  if (!backupState.error.value && backupState.successMessage.value) emit('restored')
}
</script>

<template>
  <main class="backups-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Backups</p>
        <h1>备份与恢复</h1>
      </div>
      <button type="button" :disabled="backupState.loading.value" @click="backupState.refresh">
        刷新列表
      </button>
    </header>

    <AppNotification :message="backupState.successMessage.value" level="success" />
    <AppNotification :message="backupState.error.value?.message ?? null" level="error" />

    <p v-if="backupState.loading.value">正在加载备份…</p>
    <p v-else-if="backupState.backups.value.length === 0">暂无可恢复的事务备份。</p>
    <ul v-else class="backup-list">
      <li
        v-for="backup in backupState.backups.value"
        :key="backup.directoryName"
        class="backup-card"
      >
        <div class="backup-details">
          <strong>{{ backup.metadata.transactionId }}</strong>
          <span>{{ backup.metadata.createdAt }}</span>
          <span>操作：{{ backup.metadata.operation }}</span>
          <span>Provider：{{ backup.metadata.providerId ?? '无' }}</span>
          <span>应用版本：{{ backup.metadata.appVersion }}</span>
        </div>
        <button
          type="button"
          :aria-label="`恢复备份 ${backup.metadata.transactionId}`"
          :disabled="backupState.busy.value"
          @click="restoreDirectoryName = backup.directoryName"
        >
          恢复
        </button>
      </li>
    </ul>

    <ConfirmDialog
      :open="Boolean(restoreDirectoryName)"
      title="确认恢复备份"
      :message="`确定恢复事务 ${selectedBackup?.metadata.transactionId ?? ''} 吗？恢复前会再次备份当前状态，完成后将刷新 Provider 与自检状态。`"
      confirm-label="恢复"
      @confirm="confirmRestore"
      @cancel="restoreDirectoryName = null"
    />
  </main>
</template>

<style scoped>
.backups-view,
.backup-list {
  display: grid;
  gap: 1rem;
}

.backups-view {
  padding: 1.25rem;
}

.view-header,
.backup-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.view-header h1,
.eyebrow {
  margin: 0;
}

.eyebrow {
  color: var(--accent);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.backup-list {
  margin: 0;
  padding: 0;
  list-style: none;
}

.backup-card {
  border: 1px solid var(--border);
  background: var(--surface);
  border-radius: 0.8rem;
  padding: 1rem;
}

.backup-details {
  display: grid;
  gap: 0.25rem;
}

@media (max-width: 620px) {
  .backup-card {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
