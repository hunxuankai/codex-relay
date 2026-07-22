<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElSegmented } from 'element-plus'
import type { ModelCatalogItem, ProviderProfile } from '../types/provider'

const props = defineProps<{
  provider: ProviderProfile
  modelCatalog: readonly ModelCatalogItem[]
  busy: boolean
}>()

const emit = defineEmits<{
  select: [model: string, reasoningEffort: string]
  configure: []
}>()

const selectedCatalogModel = computed(() =>
  props.modelCatalog.find((model) => model.id === props.provider.selectedModel),
)
const reasoningOptions = computed(() => selectedCatalogModel.value?.reasoningEfforts ?? [])
const selectedEffort = computed(() =>
  props.provider.selectedModel
    ? props.provider.reasoningEfforts?.[props.provider.selectedModel]
    : undefined,
)

function selectModel(value: string | number | boolean) {
  const model = String(value)
  const entry = props.modelCatalog.find((item) => item.id === model)
  if (!entry) return
  const effort = props.provider.reasoningEfforts?.[model] ?? entry.defaultReasoningEffort
  emit('select', model, effort)
}

function selectReasoningEffort(value: string | number | boolean) {
  if (!props.provider.selectedModel) return
  emit('select', props.provider.selectedModel, String(value))
}
</script>

<template>
  <section class="preference-controls" aria-label="Provider 模型偏好">
    <template v-if="provider.preferenceConfigured && provider.selectedModel">
      <div class="preference-field">
        <span class="preference-label">模型</span>
        <ElSegmented
          :model-value="provider.selectedModel"
          :options="[...(provider.models ?? [])]"
          :disabled="busy"
          aria-label="模型"
          @change="selectModel"
        />
      </div>
      <div class="preference-field">
        <span class="preference-label">推理强度</span>
        <ElSegmented
          :model-value="selectedEffort"
          :options="[...reasoningOptions]"
          :disabled="busy"
          aria-label="推理强度"
          @change="selectReasoningEffort"
        />
      </div>
      <p class="preference-hint">
        {{
          provider.isActive
            ? '修改后立即写入当前 Codex 配置；请重启 Codex 后生效。'
            : '这里只保存偏好，将在应用此 Provider 时生效。'
        }}
      </p>
    </template>
    <div v-else class="preference-missing" role="note">
      <p>模型偏好未配置，完成配置前不能应用此 Provider。</p>
      <ElButton type="primary" plain native-type="button" :disabled="busy" @click="emit('configure')">
        编辑并配置模型
      </ElButton>
    </div>
  </section>
</template>

<style scoped>
.preference-controls,
.preference-field {
  display: grid;
  gap: 0.6rem;
}

.preference-controls {
  gap: 1rem;
}

.preference-label {
  color: var(--text-secondary);
  font-weight: 700;
}

.preference-field :deep(.el-segmented) {
  max-width: 100%;
  overflow-x: auto;
}

.preference-hint,
.preference-missing p {
  margin: 0;
  color: var(--text-secondary);
}

.preference-missing {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}
</style>
