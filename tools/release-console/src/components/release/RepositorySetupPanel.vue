<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElCard, ElInput, ElTag } from 'element-plus'
import type { ReleasePreflightResult } from '../../types/release'

const props = defineProps<{
  inspection: ReleasePreflightResult | null
  busy: boolean
}>()

const emit = defineEmits<{
  inspect: []
  preparePlan: []
}>()

const repositoryPath = defineModel<string>('repositoryPath', { required: true })
const targetVersion = defineModel<string>('targetVersion', { required: true })

const inspectDisabled = computed(() => props.busy || repositoryPath.value.trim().length === 0)
const planDisabled = computed(
  () => props.busy || props.inspection === null || targetVersion.value.trim().length === 0,
)
const disabledReason = computed(() => {
  if (repositoryPath.value.trim().length === 0) return '先填写仓库路径。'
  if (props.inspection === null) return '先完成仓库预检。'
  if (targetVersion.value.trim().length === 0) return '填写严格更高的 SemVer。'
  return ''
})
const repositoryName = computed(() => {
  const remote = props.inspection?.repository.remoteUrl ?? ''
  return remote.replace(/^.*github\.com[/:]/, '').replace(/\.git$/, '')
})
</script>

<template>
  <ElCard class="setup-card" shadow="never">
    <template #header>
      <div class="card-heading">
        <div>
          <p class="section-kicker">步骤 1</p>
          <h2>仓库与版本</h2>
        </div>
        <ElTag v-if="inspection" type="success" effect="light">预检通过</ElTag>
      </div>
    </template>

    <div class="setup-content">
      <label class="field-label" for="release-repository">Codex Relay 仓库</label>
      <ElInput
        id="release-repository"
        v-model="repositoryPath"
        aria-label="仓库路径"
        autocomplete="off"
        placeholder="例如 D:\\Kai\\Project\\codex-relay"
        :disabled="busy"
      />
      <p class="field-help">请选择 Codex Relay 仓库；控制台只会操作固定的 hunxuankai/codex-relay。</p>

      <div class="field-grid">
        <div class="field-block">
          <label class="field-label" for="release-version">目标版本</label>
          <ElInput
            id="release-version"
            v-model="targetVersion"
            aria-label="目标版本"
            autocomplete="off"
            placeholder="例如 0.5.0"
            :disabled="busy"
          />
        </div>
        <div class="action-row">
          <ElButton
            data-testid="inspect-button"
            :disabled="inspectDisabled"
            :loading="busy"
            @click="emit('inspect')"
          >
            检查仓库
          </ElButton>
          <ElButton
            data-testid="plan-button"
            type="primary"
            :disabled="planDisabled"
            :loading="busy"
            @click="emit('preparePlan')"
          >
            生成发布计划
          </ElButton>
        </div>
      </div>

      <p v-if="disabledReason" class="disabled-reason">{{ disabledReason }}</p>

      <dl v-if="inspection" class="inspection-summary">
        <div>
          <dt>远端</dt>
          <dd>{{ repositoryName }}</dd>
        </div>
        <div>
          <dt>分支</dt>
          <dd>
            {{ inspection.repository.localBranch }} → {{ inspection.repository.defaultBranch }}
          </dd>
        </div>
        <div>
          <dt>候选基线</dt>
          <dd class="mono">{{ inspection.repository.headSha.slice(0, 12) }}</dd>
        </div>
        <div>
          <dt>工具链</dt>
          <dd>Git / Node / npm / Cargo / gh 已就绪</dd>
        </div>
      </dl>
    </div>
  </ElCard>
</template>

<style scoped>
.setup-card :deep(.el-card__body) {
  display: grid;
  gap: 1rem;
  padding: 1.15rem;
}

.card-heading,
.field-grid,
.action-row,
.inspection-summary {
  display: flex;
  align-items: center;
}

.card-heading {
  justify-content: space-between;
  gap: 1rem;
}

.card-heading h2,
.section-kicker,
.field-help,
.disabled-reason,
.inspection-summary dt,
.inspection-summary dd {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.setup-content,
.field-block {
  display: grid;
  gap: 0.45rem;
}

.field-label {
  font-size: 0.84rem;
  font-weight: 700;
}

.field-help,
.disabled-reason {
  color: var(--text-muted);
  font-size: 0.8rem;
}

.field-grid {
  align-items: end;
  justify-content: space-between;
  gap: 1rem;
}

.field-block {
  flex: 1 1 14rem;
}

.action-row {
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.65rem;
}

.inspection-summary {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.75rem;
  padding: 0.85rem;
  border: 1px solid var(--border-color);
  border-radius: 0.75rem;
  background: var(--surface-muted);
}

.inspection-summary div {
  min-width: 0;
}

.inspection-summary dt {
  color: var(--text-muted);
  font-size: 0.72rem;
}

.inspection-summary dd {
  overflow: hidden;
  margin-top: 0.2rem;
  font-size: 0.82rem;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mono {
  font-family: var(--font-mono);
}

@media (max-width: 760px) {
  .field-grid {
    display: grid;
  }

  .action-row {
    justify-content: stretch;
  }

  .action-row :deep(.el-button) {
    flex: 1;
    margin-left: 0;
  }

  .inspection-summary {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
