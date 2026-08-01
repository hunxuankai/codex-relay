<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import { ElAlert, ElButton, ElConfigProvider, ElTag } from 'element-plus'
import DraftAuditPanel from './components/release/DraftAuditPanel.vue'
import PublishConfirmDialog from './components/release/PublishConfirmDialog.vue'
import ReleasePlanPanel from './components/release/ReleasePlanPanel.vue'
import ReleaseResultPanel from './components/release/ReleaseResultPanel.vue'
import ReleaseStepDetails from './components/release/ReleaseStepDetails.vue'
import ReleaseTimeline from './components/release/ReleaseTimeline.vue'
import RepositorySetupPanel from './components/release/RepositorySetupPanel.vue'
import { useRepositoryPreference } from './composables/useRepositoryPreference'
import { useReleaseSession } from './composables/useReleaseSession'

const release = useReleaseSession()
const repositoryPreference = useRepositoryPreference()
const repositoryPath = computed({
  get: () => repositoryPreference.repositoryPath.value,
  set: repositoryPreference.update,
})
const targetVersion = shallowRef('')
const notes = shallowRef('')
const exportPath = shallowRef('')
const publishDialogOpen = shallowRef(false)

const draftIdentity = computed(() => release.session.value?.draft ?? null)
const canCancel = computed(() =>
  [
    'idle',
    'inspected',
    'planned',
    'applyingCandidate',
    'localChecks',
    'localBuild',
    'sourceAudit',
  ].includes(release.session.value?.phase ?? ''),
)
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

async function inspectRepository() {
  const inspection = await release.inspect(repositoryPath.value)
  if (inspection) repositoryPreference.remember(inspection.repositoryPath)
}

async function loadSession() {
  const loaded = await release.load(repositoryPath.value)
  if (loaded) {
    targetVersion.value = loaded.targetVersion
    notes.value = loaded.draft?.manifestNotes ?? notes.value
  }
}

async function preparePlan() {
  const prepared = await release.preparePlan(
    repositoryPath.value,
    targetVersion.value,
    notes.value.trim().length > 0 ? notes.value : undefined,
  )
  if (prepared) notes.value = prepared.notes
}

async function startRelease() {
  if (!release.plan.value || hasActiveSession.value) return
  await release.start(release.plan.value.id)
}

async function resumeRelease() {
  if (!release.session.value) return
  await release.resume(release.session.value.id)
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
  })
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
          <ElButton
            :disabled="release.busy.value || repositoryPath.trim().length === 0"
            @click="loadSession"
          >
            加载活动会话
          </ElButton>
          <ElButton
            v-if="release.session.value && !isFinished"
            :disabled="release.busy.value"
            @click="resumeRelease"
          >
            继续当前阶段
          </ElButton>
          <ElButton
            v-if="canCancel"
            type="danger"
            plain
            :disabled="release.busy.value"
            @click="cancelRelease"
          >
            取消并回滚
          </ElButton>
        </div>
      </header>

      <ElAlert
        v-if="release.error.value"
        class="app-error"
        type="error"
        :closable="false"
        show-icon
        :title="release.error.value.message"
      >
        <template #default>
          <span class="mono">{{ release.error.value.code }}</span>
        </template>
      </ElAlert>

      <div class="release-console-layout">
        <ReleaseTimeline :session="release.session.value" :events="release.events.value" />

        <section class="workspace" aria-label="发布工作区">
          <RepositorySetupPanel
            v-model:repository-path="repositoryPath"
            v-model:target-version="targetVersion"
            :inspection="release.inspection.value"
            :busy="release.busy.value || hasActiveSession"
            @inspect="inspectRepository"
            @prepare-plan="preparePlan"
          />

          <ReleasePlanPanel
            v-model:notes="notes"
            :plan="release.plan.value"
            :busy="release.busy.value || hasActiveSession"
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
            :busy="release.busy.value"
            @publish="publishDialogOpen = true"
          />

          <ReleaseResultPanel
            v-if="release.session.value && isFinished"
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
        :busy="release.busy.value"
        @confirm="publishRelease"
      />
    </main>
  </ElConfigProvider>
</template>

<style scoped>
.app-shell {
  display: grid;
  align-content: start;
  min-height: 100vh;
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
  align-items: start;
  gap: 1rem;
}

.workspace {
  display: grid;
  min-width: 0;
  gap: 1rem;
}

.app-error {
  overflow-wrap: anywhere;
}

.mono {
  font-family: var(--font-mono);
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
    padding: 0.75rem;
  }

  .release-console-layout {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
