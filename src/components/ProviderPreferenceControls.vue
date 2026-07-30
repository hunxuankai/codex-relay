<script setup lang="ts">
import { computed } from 'vue'
import { ElButton, ElSegmented, ElSwitch } from 'element-plus'
import type { ModelCatalogItem, ProviderProfile } from '../types/provider'

const props = defineProps<{
  provider: ProviderProfile
  modelCatalog: readonly ModelCatalogItem[]
  busy: boolean
}>()

const emit = defineEmits<{
  select: [model: string, reasoningEffort: string]
  'update-fast': [enabled: boolean]
  configure: []
}>()

const selectedCatalogModel = computed(() =>
  props.modelCatalog.find((model) => model.id === props.provider.selectedModel),
)
const reasoningOptions = computed(() => selectedCatalogModel.value?.reasoningEfforts ?? [])
const fastSupported = computed(() => selectedCatalogModel.value?.supportsFast === true)
const displayedFastEnabled = computed(() => fastSupported.value && props.provider.fastEnabled)
const fastDescription = computed(() =>
  fastSupported.value
    ? 'Fast 使用 priority 服务层，可能产生额外费用。'
    : `${props.provider.selectedModel ?? '当前模型'} 不支持 Fast，Fast 保持关闭。`,
)
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

function updateFast(value: string | number | boolean) {
  if (fastSupported.value && typeof value === 'boolean') emit('update-fast', value)
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
      <div class="preference-field fast-field">
        <div class="fast-control">
          <span class="preference-label">Fast</span>
          <ElSwitch
            :model-value="displayedFastEnabled"
            :disabled="busy || !fastSupported"
            aria-label="Fast"
            aria-describedby="provider-fast-description"
            @change="updateFast"
          />
        </div>
        <p id="provider-fast-description" class="fast-description">
          {{ fastDescription }}
        </p>
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

.fast-control {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.preference-hint,
.fast-description,
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
