<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import { ElButton, ElEmpty, ElSkeleton } from 'element-plus'
import AppNotification from '../components/AppNotification.vue'
import BackupCard from '../components/BackupCard.vue'
import ConfirmDialog from '../components/ConfirmDialog.vue'
import UnavailableBackupList from '../components/UnavailableBackupList.vue'
import { useBackups } from '../composables/useBackups'

const emit = defineEmits<{
  restored: []
}>()

const backupState = useBackups()
const restoreDirectoryName = shallowRef<string | null>(null)
const expandedDirectoryName = shallowRef<string | null>(null)
const selectedBackup = computed(
  () =>
    backupState.backups.value.find(
      (backup) => backup.directoryName === restoreDirectoryName.value,
    ) ?? null,
)
const restoreMessage = computed(() => {
  const transactionId = selectedBackup.value?.metadata.transactionId ?? ''
  const legacyNote = selectedBackup.value?.compatibility === 'legacyWithoutPreferences'
    ? '该旧版备份不包含命名地址、模型与推理偏好，恢复会回到旧版状态。'
    : ''
  return `确定恢复事务 ${transactionId} 吗？${legacyNote}恢复前会再次备份当前状态，完成后将刷新 Provider 与自检状态。`
})

async function confirmRestore() {
  const directoryName = restoreDirectoryName.value
  if (!directoryName) return
  restoreDirectoryName.value = null
  await backupState.restore(directoryName)
  if (!backupState.error.value && backupState.successMessage.value) emit('restored')
}

function toggleFiles(directoryName: string) {
  expandedDirectoryName.value = expandedDirectoryName.value === directoryName
    ? null
    : directoryName
}
</script>

<template>
  <main class="backups-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Backups</p>
        <h1>备份与恢复</h1>
      </div>
      <ElButton native-type="button" :disabled="backupState.loading.value" @click="backupState.refresh">
        刷新列表
      </ElButton>
    </header>

    <AppNotification :message="backupState.successMessage.value" level="success" />
    <AppNotification :message="backupState.error.value?.message ?? null" level="error" />

    <ElSkeleton v-if="backupState.loading.value" :rows="3" animated aria-label="正在加载备份" />
    <ElEmpty
      v-else-if="backupState.backups.value.length === 0 && backupState.unavailableBackups.value.length === 0"
      description="暂无可恢复的事务备份。"
    />
    <ul
      v-if="!backupState.loading.value && backupState.backups.value.length > 0"
      class="backup-list"
    >
      <BackupCard
        v-for="backup in backupState.backups.value"
        :key="backup.directoryName"
        :backup="backup"
        :expanded="expandedDirectoryName === backup.directoryName"
        :busy="backupState.busy.value"
        @toggle="toggleFiles"
        @open-file="backupState.openFile"
        @restore="restoreDirectoryName = $event"
      />
    </ul>

    <UnavailableBackupList
      v-if="!backupState.loading.value && backupState.unavailableBackups.value.length > 0"
      :backups="backupState.unavailableBackups.value"
      :busy="backupState.busy.value"
      @open-metadata="backupState.openFile($event, 'metadata.json')"
    />

    <ConfirmDialog
      :open="Boolean(restoreDirectoryName)"
      title="确认恢复备份"
      :message="restoreMessage"
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

.view-header {
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

@media (max-width: 620px) {
  .view-header {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
