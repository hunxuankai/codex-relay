<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElCard, ElTag } from 'element-plus'
import type { ReleasePhase, ReleaseSession } from '../../types/release'

const props = withDefaults(defineProps<{
  session: ReleaseSession
  busy: boolean
  proxyInvalid?: boolean
}>(), {
  proxyInvalid: false,
})

const emit = defineEmits<{
  cancel: []
  resume: []
  reviewPublish: []
  viewResult: []
}>()

type RecoveryAction = 'cancel' | 'resume' | 'reviewPublish' | 'viewResult'

const localPhases = new Set<ReleasePhase>([
  'idle',
  'inspected',
  'planned',
  'applyingCandidate',
  'localChecks',
  'localBuild',
  'sourceAudit',
])
const monitoringPhases = new Set<ReleasePhase>([
  'pushed',
  'workflowQueued',
  'workflowRunning',
  'auditingDraft',
])
const finalizingPhases = new Set<ReleasePhase>([
  'publishing',
  'verifyingPublishedRelease',
  'monitoringCleanup',
])

const action = computed<{ kind: RecoveryAction; label: string }>(() => {
  const phase = props.session.phase
  if (localPhases.has(phase)) return { kind: 'cancel', label: '取消并验证回滚' }
  if (phase === 'committed') return { kind: 'resume', label: '继续 Push' }
  if (monitoringPhases.has(phase)) return { kind: 'resume', label: '继续监控' }
  if (phase === 'awaitingPublishApproval') {
    return { kind: 'reviewPublish', label: '查看并确认公开' }
  }
  if (finalizingPhases.has(phase)) return { kind: 'resume', label: '继续远端收尾' }
  return { kind: 'viewResult', label: '查看上次结果' }
})

const candidateLabel = computed(() => props.session.candidateSha?.slice(0, 12) ?? '尚未提交')
const requiresNetwork = computed(() =>
  action.value.kind === 'resume' || action.value.kind === 'reviewPublish',
)
const proxyBlocked = computed(() => props.proxyInvalid && requiresNetwork.value)
const actionDisabled = computed(() => props.busy || proxyBlocked.value)

function performAction() {
  if (actionDisabled.value) return
  switch (action.value.kind) {
    case 'cancel':
      emit('cancel')
      break
    case 'resume':
      emit('resume')
      break
    case 'reviewPublish':
      emit('reviewPublish')
      break
    case 'viewResult':
      emit('viewResult')
      break
  }
}
</script>

<template>
  <ElCard class="recovery-card" shadow="never">
    <template #header>
      <div class="recovery-heading">
        <div>
          <p class="section-kicker">会话恢复</p>
          <h2>检测到发布会话</h2>
        </div>
        <ElTag effect="plain">{{ session.phase }}</ElTag>
      </div>
    </template>

    <div class="recovery-content">
      <dl class="recovery-summary">
        <div>
          <dt>目标版本</dt>
          <dd>v{{ session.targetVersion }}</dd>
        </div>
        <div>
          <dt>候选提交</dt>
          <dd class="mono">{{ candidateLabel }}</dd>
        </div>
        <div>
          <dt>当前阶段</dt>
          <dd>{{ session.phase }}</dd>
        </div>
      </dl>

      <p v-if="proxyBlocked" class="recovery-warning">
        先修正代理设置，再继续需要 GitHub 网络的恢复动作。
      </p>

      <ElButton
        data-testid="recovery-action-button"
        type="primary"
        plain
        native-type="button"
        :loading="busy"
        :disabled="actionDisabled"
        @click="performAction"
      >
        {{ action.label }}
      </ElButton>
    </div>
  </ElCard>
</template>

<style scoped>
.recovery-card :deep(.el-card__body) {
  padding: 1.15rem;
}

.recovery-heading,
.recovery-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.recovery-heading h2,
.section-kicker,
.recovery-summary,
.recovery-summary dt,
.recovery-summary dd,
.recovery-warning {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.recovery-summary {
  display: grid;
  grid-template-columns: repeat(3, minmax(7rem, auto));
  gap: 1rem;
}

.recovery-summary dt {
  color: var(--text-muted);
  font-size: 0.72rem;
}

.recovery-summary dd {
  margin-top: 0.2rem;
  font-size: 0.84rem;
  font-weight: 700;
}

.mono {
  font-family: var(--font-mono);
}

.recovery-warning {
  color: var(--danger-color);
  font-size: 0.8rem;
}

@media (max-width: 760px) {
  .recovery-content {
    align-items: stretch;
    flex-direction: column;
  }

  .recovery-summary {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
