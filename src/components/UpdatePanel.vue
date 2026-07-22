<script setup lang="ts">
import { ElButton, ElCard, ElProgress } from 'element-plus'
import ConfirmDialog from './ConfirmDialog.vue'
import type { UpdaterController } from '../composables/useUpdater'

defineProps<{ updater: UpdaterController }>()
</script>

<template>
  <ElCard class="update-panel" shadow="never" aria-labelledby="update-panel-title">
    <div class="update-header">
      <div>
        <h2 id="update-panel-title">应用更新</h2>
        <p class="version-text">当前版本：{{ updater.currentVersion.value ?? '检查后显示' }}</p>
      </div>
      <ElButton
        native-type="button"
        :disabled="['checking', 'downloading', 'launching'].includes(updater.status.value)"
        @click="updater.check"
      >
        {{ updater.status.value === 'checking' ? '正在检查…' : '检查更新' }}
      </ElButton>
    </div>

    <p v-if="updater.status.value === 'upToDate'" role="status">当前已是最新版本。</p>

    <div v-if="updater.status.value === 'available' || updater.status.value === 'confirming'" class="release-info">
      <p>发现新版本 {{ updater.release.value?.version }}</p>
      <p v-if="updater.release.value?.date">发布日期：{{ updater.release.value.date }}</p>
      <p v-if="updater.release.value?.notes" class="release-notes">{{ updater.release.value.notes }}</p>
      <ElButton type="primary" native-type="button" @click="updater.requestInstall">下载并安装</ElButton>
    </div>

    <div v-if="updater.status.value === 'downloading'" class="download-status" role="status" aria-live="polite">
      <ElProgress
        v-if="updater.progress.value?.totalBytes !== null && updater.progress.value?.totalBytes !== undefined"
        :percentage="Math.round(updater.progress.value.percent ?? 0)"
      />
      <p>
        {{ updater.progress.value?.percent === null || updater.progress.value?.percent === undefined
          ? '正在下载更新…'
          : `正在下载更新… ${Math.round(updater.progress.value.percent)}%` }}
      </p>
    </div>

    <p v-if="updater.status.value === 'launching'" role="status" aria-live="polite">
      正在启动安装器，应用即将退出…
    </p>
    <p v-if="updater.status.value === 'error'" class="error-text" role="alert">
      {{ updater.error.value?.message }}
    </p>

    <ConfirmDialog
      :open="updater.status.value === 'confirming'"
      title="下载并安装更新"
      message="下载完成后应用将退出，per-machine 安装可能触发 Windows UAC。是否继续？"
      confirm-label="继续安装"
      tone="neutral"
      @confirm="updater.confirmInstall"
      @cancel="updater.cancelInstall"
    />
  </ElCard>
</template>

<style scoped>
.release-info,
.download-status {
  display: grid;
  gap: 0.75rem;
}

.update-panel {
  border: 1px solid var(--border);
  border-radius: 0.8rem;
}

.update-panel :deep(.el-card__body) {
  display: grid;
  gap: 0.75rem;
  padding: 1rem;
}

.update-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.update-header h2,
.update-header p,
.release-info p,
.download-status p,
.error-text {
  margin: 0;
}

.version-text,
.release-notes {
  color: var(--text-secondary);
}

.release-notes {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.error-text {
  color: var(--danger);
}

.update-panel :deep(.el-progress) {
  width: 100%;
}

@media (max-width: 42rem) {
  .update-header {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
