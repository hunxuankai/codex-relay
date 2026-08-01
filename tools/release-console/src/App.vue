<script setup lang="ts">
import { computed, onMounted, shallowRef, watch } from 'vue'
import { ElConfigProvider, ElMessage, ElMessageBox, ElTag } from 'element-plus'
import DraftAuditPanel from './components/release/DraftAuditPanel.vue'
import PublishConfirmDialog from './components/release/PublishConfirmDialog.vue'
import ProxySettingsPanel from './components/release/ProxySettingsPanel.vue'
import ReleasePlanPanel from './components/release/ReleasePlanPanel.vue'
import ReleaseRecoveryPanel from './components/release/ReleaseRecoveryPanel.vue'
import ReleaseResultPanel from './components/release/ReleaseResultPanel.vue'
import ReleaseStepDetails from './components/release/ReleaseStepDetails.vue'
import ReleaseTimeline from './components/release/ReleaseTimeline.vue'
import RepositorySetupPanel from './components/release/RepositorySetupPanel.vue'
import RepositorySyncConfirmDialog from './components/release/RepositorySyncConfirmDialog.vue'
import { useReleaseNetwork } from './composables/useReleaseNetwork'
import { useReleaseProxyPreference } from './composables/useReleaseProxyPreference'
import { useRepositoryPreference } from './composables/useRepositoryPreference'
import { useReleaseSession } from './composables/useReleaseSession'
import {
  releaseProxyValidationReason,
  type ReleaseProxySettings,
} from './types/network'
import type { CommandError } from './types/release'

const release = useReleaseSession()
const releaseNetwork = useReleaseNetwork()
const releaseProxyPreference = useReleaseProxyPreference()
const repositoryPreference = useRepositoryPreference()
const repositoryPath = computed({
  get: () => repositoryPreference.repositoryPath.value,
  set: (value) => {
    repositoryPreference.update(value)
    release.invalidateRepositoryContext()
  },
})
const targetVersion = shallowRef('')
const notes = shallowRef('')
const exportPath = shallowRef('')
const publishDialogOpen = shallowRef(false)
const repositoryPushDialogOpen = shallowRef(false)
const showTerminalResult = shallowRef(false)

const draftIdentity = computed(() => release.session.value?.draft ?? null)
const isFinished = computed(() =>
  ['completed', 'completedWithWarnings'].includes(release.session.value?.phase ?? ''),
)
const hasActiveSession = computed(() => {
  const phase = release.session.value?.phase
  return phase !== undefined && ![
    'completed',
    'completedWithWarnings',
    'failed',
    'cancelled',
  ].includes(phase)
})
const proxyInvalid = computed(
  () => releaseProxyValidationReason(releaseProxyPreference.settings.value) !== null,
)
const acknowledgementErrorCodes = new Set([
  'RELEASE_ROLLBACK_INCOMPLETE',
  'GIT_PROCESS_TREE_TERMINATION_FAILED',
  'GITHUB_PROCESS_TREE_TERMINATION_FAILED',
])

function presentError(error: CommandError | null) {
  if (!error) return
  if (acknowledgementErrorCodes.has(error.code)) {
    void ElMessageBox.alert(
      `${error.message}\n\n错误码：${error.code}`,
      '发布操作需要处理',
      {
        type: 'error',
        confirmButtonText: '知道了',
        closeOnClickModal: false,
        closeOnPressEscape: true,
        showClose: true,
      },
    ).catch(() => undefined)
    return
  }

  ElMessage({
    type: 'error',
    message: `${error.message}（${error.code}）`,
    duration: 5000,
    grouping: true,
    showClose: true,
  })
}

watch(release.error, presentError)
watch(releaseNetwork.error, presentError)

async function inspectRepository() {
  const inspection = await release.inspect(
    repositoryPath.value,
    releaseProxyPreference.settings.value,
  )
  if (inspection) repositoryPreference.remember(inspection.repositoryPath)
}

function updateProxySettings(settings: ReleaseProxySettings) {
  releaseProxyPreference.update(settings)
  releaseNetwork.invalidate()
}

async function testConnection() {
  await releaseNetwork.test(releaseProxyPreference.settings.value)
}

async function pushRepository() {
  if (!release.inspection.value?.safePush) return
  repositoryPushDialogOpen.value = false
  await release.pushRepository(releaseProxyPreference.settings.value)
}

async function loadSession() {
  const loaded = await release.load(repositoryPath.value)
  showTerminalResult.value = false
  if (loaded) {
    targetVersion.value = loaded.targetVersion
    notes.value = loaded.draft?.manifestNotes ?? notes.value
  }
}

onMounted(() => {
  if (repositoryPath.value.trim().length > 0) void loadSession()
})

async function preparePlan() {
  const prepared = await release.preparePlan(
    repositoryPath.value,
    targetVersion.value,
    releaseProxyPreference.settings.value,
    notes.value.trim().length > 0 ? notes.value : undefined,
  )
  if (prepared) notes.value = prepared.notes
}

async function startRelease() {
  if (!release.plan.value || hasActiveSession.value) return
  await release.start(release.plan.value.id, releaseProxyPreference.settings.value)
}

async function resumeRelease() {
  if (!release.session.value) return
  await release.resume(release.session.value.id, releaseProxyPreference.settings.value)
}

async function cancelRelease() {
  if (!release.session.value) return
  await release.cancel(release.session.value.id)
}

async function publishRelease() {
  if (!release.session.value || !draftIdentity.value) return
  publishDialogOpen.value = false
  await release.publish(release.session.value.id, {
    releaseId: draftIdentity.value.releaseId,
    tagName: draftIdentity.value.tagName,
    targetCommitSha: draftIdentity.value.targetCommitSha,
  }, releaseProxyPreference.settings.value)
}

async function exportSummary() {
  if (!release.session.value || exportPath.value.trim().length === 0) return
  await release.exportSummary(release.session.value.id, exportPath.value)
}
</script>

<template>
  <ElConfigProvider size="default">
    <main class="app-shell">
      <header class="app-header">
        <div class="brand-copy">
          <p class="eyebrow">Codex Relay · 维护者工具</p>
          <h1>Codex Relay 发布控制台</h1>
          <p class="subtitle">可视化准备、检查、审计并公开 Windows 更新。</p>
        </div>
        <div class="header-actions">
          <ElTag v-if="release.session.value" effect="plain">
            {{ release.session.value.phase }}
          </ElTag>
        </div>
      </header>

      <div class="release-console-layout">
        <ReleaseTimeline
          class="release-timeline-panel"
          :session="release.session.value"
          :events="release.events.value"
        />

        <section class="workspace" aria-label="发布工作区">
          <ReleaseRecoveryPanel
            v-if="release.session.value"
            :session="release.session.value"
            :busy="release.busy.value"
            :proxy-invalid="proxyInvalid"
            @cancel="cancelRelease"
            @resume="resumeRelease"
            @review-publish="publishDialogOpen = true"
            @view-result="showTerminalResult = true"
          />

          <ProxySettingsPanel
            :settings="releaseProxyPreference.settings.value"
            :result="releaseNetwork.result.value"
            :busy="releaseNetwork.busy.value"
            @update:settings="updateProxySettings"
            @test="testConnection"
          />

          <RepositorySetupPanel
            v-model:repository-path="repositoryPath"
            v-model:target-version="targetVersion"
            :inspection="release.inspection.value"
            :busy="release.busy.value || hasActiveSession || proxyInvalid"
            @inspect="inspectRepository"
            @prepare-plan="preparePlan"
            @request-push="repositoryPushDialogOpen = true"
          />

          <ReleasePlanPanel
            v-model:notes="notes"
            :plan="release.plan.value"
            :busy="release.busy.value || hasActiveSession || proxyInvalid"
            @regenerate="preparePlan"
            @start="startRelease"
          />

          <ReleaseStepDetails
            :session="release.session.value"
            :events="release.events.value"
          />

          <DraftAuditPanel
            v-if="release.session.value?.draft && release.session.value.phase === 'awaitingPublishApproval'"
            :draft="release.session.value.draft"
            :busy="release.busy.value || proxyInvalid"
            @publish="publishDialogOpen = true"
          />

          <ReleaseResultPanel
            v-if="release.session.value && (isFinished || showTerminalResult)"
            v-model:export-path="exportPath"
            :session="release.session.value"
            :busy="release.busy.value"
            @export="exportSummary"
          />
        </section>
      </div>

      <PublishConfirmDialog
        v-if="draftIdentity"
        v-model="publishDialogOpen"
        :identity="{
          releaseId: draftIdentity.releaseId,
          tagName: draftIdentity.tagName,
          targetCommitSha: draftIdentity.targetCommitSha,
        }"
        :busy="release.busy.value || proxyInvalid"
        @confirm="publishRelease"
      />

      <RepositorySyncConfirmDialog
        v-if="release.inspection.value?.safePush"
        v-model="repositoryPushDialogOpen"
        :remote-url="release.inspection.value.repository.remoteUrl"
        :preview="release.inspection.value.safePush"
        :busy="release.busy.value || proxyInvalid"
        @confirm="pushRepository"
      />
    </main>
  </ElConfigProvider>
</template>

<style scoped>
.app-shell {
  display: grid;
  grid-template-rows: auto minmax(0, 1fr);
  height: 100vh;
  min-height: 0;
  overflow: hidden;
  gap: 1rem;
  padding: 1.25rem;
}

.app-header,
.header-actions {
  display: flex;
  align-items: center;
}

.app-header {
  justify-content: space-between;
  gap: 1.5rem;
}

.brand-copy {
  display: grid;
  gap: 0.25rem;
}

.eyebrow,
.subtitle,
.brand-copy h1 {
  margin: 0;
}

.eyebrow {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.brand-copy h1 {
  font-size: clamp(1.65rem, 3vw, 2.35rem);
  line-height: 1.1;
}

.subtitle {
  color: var(--text-muted);
  font-size: 0.88rem;
}

.header-actions {
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.6rem;
}

.header-actions :deep(.el-button) {
  margin-left: 0;
}

.release-console-layout {
  display: grid;
  grid-template-columns: minmax(15.5rem, 0.72fr) minmax(0, 2.1fr);
  align-items: stretch;
  min-height: 0;
  overflow: hidden;
  gap: 1rem;
}

.release-timeline-panel,
.workspace {
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
}

.workspace {
  display: grid;
  align-content: start;
  grid-auto-rows: max-content;
  min-width: 0;
  gap: 1rem;
  padding-right: 0.25rem;
}

@media (max-width: 900px) {
  .app-header {
    align-items: flex-start;
    flex-direction: column;
  }

  .header-actions {
    justify-content: flex-start;
  }
}

@media (max-width: 820px) {
  .app-shell {
    grid-template-rows: auto;
    height: auto;
    min-height: 100vh;
    overflow: visible;
    padding: 0.75rem;
  }

  .release-console-layout {
    grid-template-columns: minmax(0, 1fr);
    align-items: start;
    overflow: visible;
  }

  .release-timeline-panel,
  .workspace {
    overflow-y: visible;
    scrollbar-gutter: auto;
  }

  .workspace {
    padding-right: 0;
  }
}
</style>
