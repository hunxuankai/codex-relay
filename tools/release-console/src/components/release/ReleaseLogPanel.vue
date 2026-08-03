<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, shallowRef, useTemplateRef, watch } from 'vue'
import { ElButton, ElTag } from 'element-plus'
import type {
  CommandError,
  ReleaseFailureEvidence,
  ReleaseLogEntry,
  ReleaseLogPage,
  ReleaseLogViewMode,
} from '../../types/release'

const props = defineProps<{
  logPage: ReleaseLogPage
  logViewMode: ReleaseLogViewMode
  unreadLogCount: number
  logRequestPending: boolean
  logError: CommandError | null
  failure: ReleaseFailureEvidence | null
}>()

const emit = defineEmits<{
  'load-earlier': []
  'refresh-log-page': []
  'return-to-latest': []
}>()

const viewport = useTemplateRef<HTMLElement>('viewport')
const followingLatest = shallowRef(true)
const copyStatus = shallowRef<string | null>(null)
let scrollGeneration = 0
let unmounted = false

const visibleRange = computed(() => {
  const first = props.logPage.entries[0]?.sequence
  const last = props.logPage.entries[props.logPage.entries.length - 1]?.sequence
  if (first === undefined || last === undefined) return `0 / ${props.logPage.totalEntries}`
  return `${first}–${last} / ${props.logPage.totalEntries}`
})

const totalBytes = computed(() => formatBytes(props.logPage.totalBytes))
const showReturnLatest = computed(
  () => props.logViewMode === 'history' || props.unreadLogCount > 0 || !followingLatest.value,
)

function formatBytes(bytes: number) {
  if (bytes < 1_024) return `${bytes} B`
  const kibibytes = bytes / 1_024
  if (kibibytes < 1_024) return `${formatAmount(kibibytes)} KiB`
  return `${formatAmount(kibibytes / 1_024)} MiB`
}

function formatAmount(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1)
}

function displayTime(timestamp: string) {
  const time = timestamp.slice(11, 23)
  return time.length > 0 ? time : timestamp
}

function entryKey(entry: ReleaseLogEntry) {
  return `${entry.sessionId}:${entry.sequence}`
}

function onScroll() {
  const element = viewport.value
  if (!element || props.logViewMode === 'history') return
  const remaining = element.scrollHeight - element.clientHeight - element.scrollTop
  const isFollowing = remaining <= 24
  if (!isFollowing) scrollGeneration += 1
  followingLatest.value = isFollowing
}

async function scrollToBottom() {
  const generation = ++scrollGeneration
  await nextTick()
  if (unmounted || generation !== scrollGeneration) return
  const element = viewport.value
  if (element) element.scrollTop = element.scrollHeight
}

function returnToLatest() {
  followingLatest.value = true
  copyStatus.value = null
  emit('return-to-latest')
  void scrollToBottom()
}

function formatEntryForCopy(entry: ReleaseLogEntry) {
  return `${entry.timestamp} [${entry.stepId}] [${entry.source}/${entry.level}] ${entry.message}`
}

async function copyCurrentPage() {
  copyStatus.value = null
  try {
    if (!navigator.clipboard?.writeText) throw new Error('clipboard unavailable')
    await navigator.clipboard.writeText(props.logPage.entries.map(formatEntryForCopy).join('\n'))
    copyStatus.value = '已复制当前页。'
  } catch {
    copyStatus.value = '复制失败，请重试。'
  }
}

watch(
  [() => props.logPage.entries, () => props.logViewMode],
  ([, viewMode]) => {
    if (viewMode === 'history') {
      followingLatest.value = false
      return
    }
    if (followingLatest.value) void scrollToBottom()
  },
  { flush: 'post', immediate: true },
)

onBeforeUnmount(() => {
  unmounted = true
  scrollGeneration += 1
})
</script>

<template>
  <section class="release-log-panel" aria-labelledby="release-log-title">
    <header class="log-toolbar">
      <div class="log-heading">
        <div class="log-title-line">
          <h2 id="release-log-title">发布诊断日志</h2>
          <ElTag size="small" effect="plain">
            {{ logViewMode === 'latest' ? '最新' : '历史' }}
          </ElTag>
          <ElTag v-if="unreadLogCount > 0" size="small" type="warning" effect="plain">
            {{ unreadLogCount }} 条新日志
          </ElTag>
        </div>
        <p class="log-summary">
          <span>{{ visibleRange }}</span>
          <span>{{ totalBytes }}</span>
          <span v-if="logPage.truncated" class="summary-warning">早期日志已截断</span>
        </p>
      </div>

      <div class="log-actions" aria-label="日志页操作">
        <ElButton
          size="small"
          plain
          native-type="button"
          aria-label="读取更早日志"
          :disabled="logRequestPending || !logPage.hasEarlier"
          @click="emit('load-earlier')"
        >
          更早
        </ElButton>
        <ElButton
          size="small"
          plain
          native-type="button"
          aria-label="更新当前日志页"
          :loading="logRequestPending"
          :disabled="logRequestPending"
          @click="emit('refresh-log-page')"
        >
          更新
        </ElButton>
        <ElButton
          v-if="showReturnLatest"
          size="small"
          type="primary"
          plain
          native-type="button"
          aria-label="返回最新日志"
          :disabled="logRequestPending"
          @click="returnToLatest"
        >
          返回最新
        </ElButton>
        <ElButton
          size="small"
          text
          native-type="button"
          aria-label="复制当前日志页"
          :disabled="logPage.entries.length === 0"
          @click="copyCurrentPage"
        >
          复制当前页
        </ElButton>
      </div>
    </header>

    <div class="log-notices" aria-live="polite">
      <p v-if="logRequestPending" class="notice neutral">正在读取日志...</p>
      <p v-if="logPage.warning" class="notice warning">{{ logPage.warning }}</p>
      <p v-if="logError" class="notice error">
        {{ logError.message }}（{{ logError.code }}）
      </p>
      <p v-if="failure" class="notice error">
        {{ failure.stepId }} · {{ failure.code }}
      </p>
      <p v-if="copyStatus" class="notice neutral">{{ copyStatus }}</p>
    </div>

    <div
      ref="viewport"
      class="log-viewport"
      tabindex="0"
      aria-label="发布诊断日志"
      :aria-busy="logRequestPending"
      @scroll="onScroll"
    >
      <p v-if="logPage.entries.length === 0" class="log-empty">尚无发布诊断日志</p>
      <ol v-else class="log-list">
        <li v-for="entry in logPage.entries" :key="entryKey(entry)" class="log-entry">
          <div class="entry-meta">
            <time :datetime="entry.timestamp">{{ displayTime(entry.timestamp) }}</time>
            <span class="entry-step">{{ entry.stepId }}</span>
            <span class="entry-source">{{ entry.source }}</span>
            <span class="entry-level" :data-level="entry.level">{{ entry.level }}</span>
            <span class="entry-sequence">#{{ entry.sequence }}</span>
          </div>
          <pre class="entry-message">{{ entry.message }}</pre>
        </li>
      </ol>
    </div>
  </section>
</template>

<style scoped>
.release-log-panel {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr);
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  border-top: 1px solid var(--border-color);
  background: var(--surface-color);
}

.log-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-width: 0;
  gap: 0.75rem;
  padding: 0.6rem 0.85rem 0.5rem;
}

.log-heading,
.log-title-line,
.log-summary,
.log-actions,
.entry-meta {
  display: flex;
  align-items: center;
}

.log-heading {
  min-width: 0;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.25rem;
}

.log-title-line,
.log-summary,
.log-actions {
  flex-wrap: wrap;
  gap: 0.45rem;
}

.log-title-line h2,
.log-summary,
.notice,
.log-empty,
.entry-message {
  margin: 0;
}

.log-title-line h2 {
  font-size: 0.92rem;
  line-height: 1.25;
}

.log-summary {
  color: var(--text-muted);
  font-size: 0.72rem;
}

.log-summary span + span::before {
  content: "·";
  margin-right: 0.45rem;
}

.summary-warning {
  color: var(--warning-color);
  font-weight: 700;
}

.log-actions {
  justify-content: flex-end;
}

.log-actions :deep(.el-button) {
  margin-left: 0;
}

.log-notices {
  display: flex;
  min-width: 0;
  flex-wrap: wrap;
  gap: 0.35rem 0.8rem;
  padding: 0 0.85rem 0.45rem;
}

.notice {
  overflow-wrap: anywhere;
  font-size: 0.72rem;
  line-height: 1.35;
}

.notice.neutral {
  color: var(--text-muted);
}

.notice.warning {
  color: var(--warning-color);
}

.notice.error {
  color: var(--danger-color);
  font-weight: 700;
}

.log-viewport {
  min-width: 0;
  min-height: 0;
  overflow: auto;
  overscroll-behavior: contain;
  padding: 0.65rem 0.85rem 0.8rem;
  background: var(--log-background);
  color: var(--log-text);
  font-family: var(--font-mono);
  scrollbar-gutter: stable;
}

.log-empty {
  color: color-mix(in srgb, var(--log-text) 72%, transparent);
  font-size: 0.78rem;
}

.log-list {
  display: grid;
  gap: 0.45rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.log-entry {
  min-width: 0;
  border-bottom: 1px solid color-mix(in srgb, var(--log-text) 14%, transparent);
  padding-bottom: 0.4rem;
}

.entry-meta {
  min-width: 0;
  flex-wrap: wrap;
  gap: 0.35rem 0.65rem;
  color: color-mix(in srgb, var(--log-text) 70%, transparent);
  font-size: 0.68rem;
  line-height: 1.35;
}

.entry-step {
  color: var(--log-text);
  font-weight: 700;
  overflow-wrap: anywhere;
}

.entry-source,
.entry-level,
.entry-sequence {
  white-space: nowrap;
}

.entry-level[data-level="warning"] {
  color: #ffd18a;
}

.entry-level[data-level="error"] {
  color: #ff9aa2;
}

.entry-message {
  margin-top: 0.2rem;
  overflow-wrap: anywhere;
  color: var(--log-text);
  font-family: inherit;
  font-size: 0.74rem;
  line-height: 1.45;
  white-space: pre-wrap;
}

@media (max-width: 620px) {
  .log-toolbar {
    align-items: flex-start;
    flex-direction: column;
    padding-inline: 0.65rem;
  }

  .log-actions {
    width: 100%;
    justify-content: flex-start;
  }

  .log-notices,
  .log-viewport {
    padding-inline: 0.65rem;
  }
}
</style>
