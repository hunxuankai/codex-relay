<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElCard, ElInput, ElTag } from 'element-plus'
import type { ReleasePlanSummary } from '../../types/release'

const props = defineProps<{
  plan: ReleasePlanSummary | null
  busy: boolean
}>()

const emit = defineEmits<{
  regenerate: []
  start: []
}>()

const notes = defineModel<string>('notes', { required: true })
const planMatchesNotes = computed(() => props.plan !== null && props.plan.notes === notes.value)
const startDisabled = computed(() => props.busy || props.plan === null || !planMatchesNotes.value)
</script>

<template>
  <ElCard class="plan-card" shadow="never">
    <template #header>
      <div class="card-heading">
        <div>
          <p class="section-kicker">步骤 2</p>
          <h2>发布候选预览</h2>
        </div>
        <ElTag v-if="plan" type="success" effect="light">
          {{ plan.previousVersion }} → {{ plan.targetVersion }}
        </ElTag>
      </div>
    </template>

    <div class="plan-content">
      <div class="notes-heading">
        <div>
          <label class="field-label" for="release-notes">简体中文发布说明</label>
          <p>
            根据 Git 提交与固定模板生成，不调用 Codex；正文会同时进入 GitHub Release 和
            latest.json.notes。
          </p>
        </div>
        <ElButton size="small" :disabled="busy" @click="emit('regenerate')">重新生成计划</ElButton>
      </div>
      <ElInput
        id="release-notes"
        v-model="notes"
        type="textarea"
        aria-label="发布说明"
        :rows="12"
        resize="vertical"
        placeholder="先生成发布计划，再检查并编辑说明。"
        :disabled="busy"
      />

      <p v-if="plan && !planMatchesNotes" class="plan-warning">
        说明已修改，请重新生成计划以更新六个候选文件和指纹。
      </p>

      <div v-if="plan" class="file-plan" aria-label="计划文件">
        <div class="file-plan-heading">
          <strong>精确计划文件</strong>
          <span>{{ plan.files.length }} 个</span>
        </div>
        <ul>
          <li v-for="file in plan.files" :key="file.relativePath">
            <span class="file-path">{{ file.relativePath }}</span>
            <span class="hash-change mono">
              {{ file.beforeSha256.slice(0, 8) }} → {{ file.afterSha256.slice(0, 8) }}
            </span>
          </li>
        </ul>
      </div>

      <div class="start-row">
        <div class="start-copy">
          <strong>开始后将执行本地门禁、精确提交推送和远端 Draft 审计。</strong>
          <span>正式公开仍需要第二次明确确认。</span>
        </div>
        <ElButton
          data-testid="start-release-button"
          type="primary"
          :disabled="startDisabled"
          :loading="busy"
          @click="emit('start')"
        >
          一键开始候选发布
        </ElButton>
      </div>
    </div>
  </ElCard>
</template>

<style scoped>
.plan-card :deep(.el-card__body) {
  display: grid;
  gap: 1rem;
  padding: 1.15rem;
}

.card-heading,
.notes-heading,
.file-plan-heading,
.start-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.card-heading h2,
.section-kicker,
.notes-heading p,
.plan-warning,
.file-plan ul,
.start-copy strong,
.start-copy span {
  margin: 0;
}

.section-kicker {
  color: var(--accent-color);
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.plan-content,
.start-copy {
  display: grid;
  gap: 0.65rem;
}

.field-label {
  font-size: 0.86rem;
  font-weight: 800;
}

.notes-heading p,
.start-copy span {
  color: var(--text-muted);
  font-size: 0.78rem;
}

.plan-warning {
  color: var(--warning-color);
  font-size: 0.82rem;
  font-weight: 700;
}

.file-plan {
  overflow: hidden;
  border: 1px solid var(--border-color);
  border-radius: 0.8rem;
}

.file-plan-heading {
  padding: 0.65rem 0.8rem;
  background: var(--surface-muted);
  font-size: 0.8rem;
}

.file-plan ul {
  padding: 0;
  list-style: none;
}

.file-plan li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.55rem 0.8rem;
  border-top: 1px solid var(--border-color);
  font-size: 0.78rem;
}

.file-path {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hash-change {
  flex: 0 0 auto;
  color: var(--text-muted);
}

.mono {
  font-family: var(--font-mono);
}

.start-row {
  align-items: flex-end;
  padding-top: 0.4rem;
}

@media (max-width: 760px) {
  .notes-heading,
  .start-row {
    align-items: stretch;
    flex-direction: column;
  }

  .file-plan li {
    align-items: start;
    flex-direction: column;
    gap: 0.25rem;
  }
}
</style>
