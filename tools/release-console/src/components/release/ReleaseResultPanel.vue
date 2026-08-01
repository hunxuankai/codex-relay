<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElCard, ElInput, ElTag } from 'element-plus'
import type { ReleaseSession } from '../../types/release'

const props = defineProps<{
  session: ReleaseSession
  busy: boolean
}>()

const emit = defineEmits<{
  export: []
}>()

const exportPath = defineModel<string>('exportPath', { required: true })
const cleanupFailed = computed(
  () => props.session.cleanup?.succeeded === false || props.session.cleanupWarning !== null,
)
const title = computed(() => {
  if (props.session.phase === 'failed') {
    return props.session.published ? 'Release 已公开，但发布收尾失败' : '发布失败'
  }
  if (props.session.phase === 'cancelled') return '发布已取消'
  if (props.session.published) return 'Release 已公开'
  return '发布会话结果'
})
const status = computed(() => {
  if (props.session.phase === 'failed') return { label: '失败', type: 'danger' as const }
  if (props.session.phase === 'cancelled') return { label: '已取消', type: 'info' as const }
  if (props.session.phase === 'completedWithWarnings' || cleanupFailed.value) {
    return { label: '完成但有警告', type: 'warning' as const }
  }
  return { label: '已完成', type: 'success' as const }
})
const onlineVerificationStatus = computed(() => {
  if (!props.session.published) {
    return {
      text: '— Release 与公开资产在线复核：未执行。',
      className: 'status-neutral',
    }
  }
  if (!['completed', 'completedWithWarnings'].includes(props.session.phase)) {
    return {
      text: '⚠ Release 已公开，但在线复核未完成。',
      className: 'status-warning',
    }
  }
  return {
    text: '✓ Release 与公开资产在线复核：已完成。',
    className: 'status-success',
  }
})
const cleanupStatus = computed(() => {
  if (!props.session.published) {
    return { text: '— 历史 Release 清理：未执行。', className: 'status-neutral' }
  }
  if (props.session.cleanup?.succeeded === true) {
    return { text: '✓ 历史 Release 清理已完成。', className: 'status-success' }
  }
  if (cleanupFailed.value) {
    return {
      text: '⚠ 历史清理失败或未能确认，请查看 cleanup Run。',
      className: 'status-warning',
    }
  }
  return {
    text: '— 历史 Release 清理：未执行或未确认。',
    className: 'status-neutral',
  }
})
</script>

<template>
  <ElCard class="result-card" shadow="never">
    <template #header>
      <div class="card-heading">
        <div>
          <p class="section-kicker">发布结果</p>
          <h2>{{ title }}</h2>
        </div>
        <ElTag :type="status.type" effect="dark">
          {{ status.label }}
        </ElTag>
      </div>
    </template>

    <div class="result-content">
      <dl class="result-facts">
        <div><dt>版本</dt><dd>{{ session.targetVersion }}</dd></div>
        <div><dt>Release ID</dt><dd>{{ session.published?.releaseId ?? '未公开' }}</dd></div>
        <div><dt>Tag</dt><dd>{{ session.published?.tagName ?? '未公开' }}</dd></div>
        <div><dt>候选提交</dt><dd class="mono">{{ session.candidateSha?.slice(0, 12) ?? '—' }}</dd></div>
      </dl>

      <div class="result-statuses">
        <p :class="onlineVerificationStatus.className">{{ onlineVerificationStatus.text }}</p>
        <p :class="cleanupStatus.className">{{ cleanupStatus.text }}</p>
        <p class="status-neutral">— Sandbox / 安装 / UAC / 应用内升级：未执行。</p>
      </div>

      <div class="export-row">
        <ElInput
          v-model="exportPath"
          aria-label="摘要导出路径"
          placeholder="例如 D:\\release-evidence\\v0.5.0.json"
          :disabled="busy"
        />
        <ElButton :disabled="busy || exportPath.trim().length === 0" @click="emit('export')">
          导出非秘密摘要
        </ElButton>
      </div>
    </div>
  </ElCard>
</template>

<style scoped>
.result-card :deep(.el-card__body) {
  display: grid;
  gap: 1rem;
  padding: 1.15rem;
}

.card-heading,
.export-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.card-heading h2,
.section-kicker,
.result-facts,
.result-facts dt,
.result-facts dd,
.result-statuses p {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.result-content,
.result-statuses {
  display: grid;
  gap: 0.75rem;
}

.result-facts {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.65rem;
}

.result-facts div {
  padding: 0.65rem 0.75rem;
  border: 1px solid var(--border-color);
  border-radius: 0.7rem;
}

.result-facts dt {
  color: var(--text-muted);
  font-size: 0.72rem;
}

.result-facts dd {
  margin-top: 0.2rem;
  font-size: 0.82rem;
  font-weight: 800;
}

.mono {
  font-family: var(--font-mono);
}

.result-statuses {
  padding: 0.85rem;
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
}

.status-success {
  color: var(--success-color);
}

.status-warning {
  color: var(--warning-color);
  font-weight: 700;
}

.status-neutral {
  color: var(--text-muted);
}

.export-row :deep(.el-input) {
  flex: 1;
}

@media (max-width: 760px) {
  .result-facts {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .export-row {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
