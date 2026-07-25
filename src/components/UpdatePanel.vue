<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElCard, ElProgress } from 'element-plus'
import ConfirmDialog from './ConfirmDialog.vue'
import type { UpdaterController } from '../composables/useUpdater'
import { formatReleaseDate, renderReleaseNotes } from '../utils/updatePresentation'

const props = defineProps<{ updater: UpdaterController }>()
const updater = props.updater

const releaseDate = computed(() => {
  const value = updater.release.value?.date
  return value ? formatReleaseDate(value) : null
})

const releaseNotes = computed(() => {
  const value = updater.release.value?.notes
  return value ? renderReleaseNotes(value) : ''
})
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
      <p v-if="releaseDate">发布日期：{{ releaseDate }}</p>
      <section v-if="releaseNotes" class="release-notes" aria-label="版本更新说明">
        <!-- renderReleaseNotes() sanitizes the generated HTML before it reaches v-html. -->
        <div class="release-notes-content" v-html="releaseNotes"></div>
      </section>
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

.version-text {
  color: var(--text-secondary);
}

.release-notes-content {
  min-width: 0;
  overflow-wrap: anywhere;
  color: var(--text-secondary);
}

.release-notes-content :deep(p),
.release-notes-content :deep(ul),
.release-notes-content :deep(ol),
.release-notes-content :deep(blockquote),
.release-notes-content :deep(pre),
.release-notes-content :deep(table) {
  margin: 0 0 0.65rem;
}

.release-notes-content :deep(p:last-child),
.release-notes-content :deep(ul:last-child),
.release-notes-content :deep(ol:last-child),
.release-notes-content :deep(blockquote:last-child),
.release-notes-content :deep(pre:last-child),
.release-notes-content :deep(table:last-child) {
  margin-bottom: 0;
}

.release-notes-content :deep(h1),
.release-notes-content :deep(h2),
.release-notes-content :deep(h3),
.release-notes-content :deep(h4),
.release-notes-content :deep(h5),
.release-notes-content :deep(h6) {
  margin: 0 0 0.5rem;
  color: var(--text-primary);
  font-size: 1rem;
}

.release-notes-content :deep(ul),
.release-notes-content :deep(ol) {
  padding-left: 1.35rem;
}

.release-notes-content :deep(blockquote) {
  border-left: 3px solid var(--border-strong);
  padding-left: 0.75rem;
}

.release-notes-content :deep(code) {
  border-radius: 0.25rem;
  padding: 0.1rem 0.3rem;
  background: var(--surface-muted);
  font: 0.9em ui-monospace, SFMono-Regular, Consolas, monospace;
}

.release-notes-content :deep(pre) {
  overflow-x: auto;
  border: 1px solid var(--border);
  border-radius: 0.45rem;
  padding: 0.7rem;
  background: var(--surface-muted);
}

.release-notes-content :deep(table) {
  display: block;
  max-width: 100%;
  overflow-x: auto;
}

.release-notes-content :deep(pre code) {
  padding: 0;
  background: transparent;
}

.release-notes-content :deep(a) {
  color: var(--accent-strong);
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
