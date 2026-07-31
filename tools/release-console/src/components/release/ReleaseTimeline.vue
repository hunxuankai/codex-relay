<script setup lang="ts">
import { computed } from 'vue'
import { ElTag } from 'element-plus'
import type { ReleaseEvent, ReleasePhase, ReleaseSession } from '../../types/release'

const props = defineProps<{
  session: ReleaseSession | null
  events: readonly ReleaseEvent[]
}>()

interface TimelineStep {
  id: string
  label: string
  phase: ReleasePhase
  eventAliases?: string[]
}

const steps: TimelineStep[] = [
  { id: 'preflight', label: '仓库预检', phase: 'inspected' },
  { id: 'plan', label: '候选预览', phase: 'planned' },
  { id: 'candidate', label: '版本事务', phase: 'applyingCandidate' },
  { id: 'releaseTests', label: '发布专项', phase: 'localChecks' },
  { id: 'fullChecks', label: '完整检查', phase: 'localBuild', eventAliases: ['localChecks'] },
  { id: 'ordinaryBuild', label: '普通构建', phase: 'sourceAudit' },
  { id: 'sourceAudit', label: '源码审计', phase: 'committed' },
  { id: 'commitPush', label: '提交推送', phase: 'pushed' },
  { id: 'remoteRun', label: '远端 Run', phase: 'workflowRunning' },
  { id: 'draftAudit', label: 'Draft 审计', phase: 'auditingDraft' },
  { id: 'publishApproval', label: '等待公开', phase: 'awaitingPublishApproval' },
  { id: 'onlineVerification', label: '在线复核', phase: 'verifyingPublishedRelease' },
  { id: 'cleanup', label: '历史清理', phase: 'monitoringCleanup' },
]

const phaseOrder: ReleasePhase[] = [
  'idle',
  'inspected',
  'planned',
  'applyingCandidate',
  'localChecks',
  'localBuild',
  'sourceAudit',
  'committed',
  'pushed',
  'workflowQueued',
  'workflowRunning',
  'auditingDraft',
  'awaitingPublishApproval',
  'publishing',
  'verifyingPublishedRelease',
  'monitoringCleanup',
  'completed',
  'completedWithWarnings',
]

function phaseIndex(phase: ReleasePhase) {
  return phaseOrder.indexOf(phase)
}

function completedEvent(step: TimelineStep) {
  const ids = [step.id, ...(step.eventAliases ?? [])]
  return [...props.events]
    .reverse()
    .find(
      (event) => event.kind === 'stepCompleted' && ids.includes(event.stepId),
    ) as Extract<ReleaseEvent, { kind: 'stepCompleted' }> | undefined
}

function stepState(step: TimelineStep) {
  const current = props.session?.phase ?? 'idle'
  if (current === 'failed') return 'waiting'
  if (current === 'cancelled') return 'waiting'
  const currentIndex = phaseIndex(current)
  const targetIndex = phaseIndex(step.phase)
  if (currentIndex > targetIndex) return 'completed'
  if (currentIndex === targetIndex || (step.id === 'remoteRun' && current === 'workflowQueued')) {
    return 'current'
  }
  return 'waiting'
}

function stateLabel(step: TimelineStep) {
  const state = stepState(step)
  if (state === 'completed') return '已完成'
  if (state === 'current') return '进行中'
  return '未开始'
}

function tagType(step: TimelineStep) {
  const state = stepState(step)
  if (state === 'completed') return 'success'
  if (state === 'current') return 'primary'
  return 'info'
}

function formatDuration(milliseconds: number) {
  const seconds = Math.floor(milliseconds / 1000)
  const minutes = Math.floor(seconds / 60)
  const rest = seconds % 60
  return minutes > 0 ? `${minutes}分${rest.toString().padStart(2, '0')}秒` : `${rest}秒`
}

const visibleSteps = computed(() =>
  steps.map((step) => ({
    ...step,
    state: stepState(step),
    labelText: stateLabel(step),
    duration: completedEvent(step)?.durationMillis ?? null,
  })),
)
</script>

<template>
  <nav class="timeline" aria-label="发布阶段">
    <div class="timeline-heading">
      <div>
        <p class="section-kicker">发布进度</p>
        <h2>阶段时间线</h2>
      </div>
      <ElTag v-if="session" effect="plain">{{ session.targetVersion }}</ElTag>
    </div>

    <ol class="step-list">
      <li
        v-for="(step, index) in visibleSteps"
        :key="step.id"
        class="release-step"
        :class="`is-${step.state}`"
        data-release-step
      >
        <span class="step-index" aria-hidden="true">{{ index + 1 }}</span>
        <span class="step-copy">
          <strong>{{ step.label }}</strong>
          <small v-if="step.duration !== null">{{ formatDuration(step.duration) }}</small>
        </span>
        <ElTag :type="tagType(step)" size="small" effect="light">{{ step.labelText }}</ElTag>
      </li>
    </ol>
  </nav>
</template>

<style scoped>
.timeline {
  display: grid;
  align-content: start;
  gap: 1rem;
  padding: 1.1rem;
  border: 1px solid var(--border-color);
  border-radius: 1rem;
  background: var(--surface-color);
}

.timeline-heading,
.release-step {
  display: flex;
  align-items: center;
}

.timeline-heading {
  justify-content: space-between;
  gap: 1rem;
}

.timeline-heading h2,
.section-kicker,
.step-list {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.step-list {
  display: grid;
  gap: 0.35rem;
  padding: 0;
  list-style: none;
}

.release-step {
  min-height: 2.65rem;
  gap: 0.65rem;
  padding: 0.45rem 0.55rem;
  border-radius: 0.7rem;
}

.release-step.is-current {
  background: var(--accent-soft);
}

.step-index {
  display: grid;
  flex: 0 0 1.65rem;
  width: 1.65rem;
  height: 1.65rem;
  place-items: center;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  color: var(--text-muted);
  font-size: 0.72rem;
  font-weight: 800;
}

.is-completed .step-index,
.is-current .step-index {
  border-color: var(--accent-color);
  color: var(--accent-color);
}

.step-copy {
  display: grid;
  flex: 1;
  min-width: 0;
  gap: 0.1rem;
}

.step-copy strong {
  font-size: 0.82rem;
}

.step-copy small {
  color: var(--text-muted);
  font-size: 0.72rem;
}
</style>
