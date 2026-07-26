<script setup lang="ts">
import { shallowRef } from 'vue'
import { ElButton, ElCard, ElEmpty } from 'element-plus'
import type { ProviderProfile } from '../types/provider'
import ProviderStatus from './ProviderStatus.vue'

const props = defineProps<{
  providers: readonly ProviderProfile[]
  selectedProviderId: string | null
  busy: boolean
}>()

const emit = defineEmits<{
  create: []
  select: [providerId: string]
  edit: [providerId: string]
  use: [providerId: string]
  delete: [providerId: string]
  reorder: [providerIds: string[]]
}>()

const draggedProviderId = shallowRef<string | null>(null)
const dropTargetId = shallowRef<string | null>(null)

function resetDrag() {
  draggedProviderId.value = null
  dropTargetId.value = null
}

function startDrag(event: DragEvent, providerId: string) {
  if (props.busy || props.providers.length < 2) {
    event.preventDefault()
    return
  }
  draggedProviderId.value = providerId
  dropTargetId.value = providerId
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', providerId)
  }
}

function allowDrop(event: DragEvent, providerId: string) {
  if (props.busy || !draggedProviderId.value) return
  event.preventDefault()
  dropTargetId.value = providerId
  if (event.dataTransfer) event.dataTransfer.dropEffect = 'move'
}

function emitMovedOrder(sourceProviderId: string, targetIndex: number) {
  const providerIds = props.providers.map((provider) => provider.id)
  const sourceIndex = providerIds.indexOf(sourceProviderId)
  if (
    sourceIndex < 0 ||
    targetIndex < 0 ||
    targetIndex >= providerIds.length ||
    sourceIndex === targetIndex
  ) {
    resetDrag()
    return
  }

  const [movedProviderId] = providerIds.splice(sourceIndex, 1)
  if (!movedProviderId) {
    resetDrag()
    return
  }
  providerIds.splice(targetIndex, 0, movedProviderId)
  resetDrag()
  emit('reorder', providerIds)
}

function dropProvider(event: DragEvent, targetProviderId: string) {
  if (props.busy) {
    resetDrag()
    return
  }
  event.preventDefault()
  const sourceProviderId = draggedProviderId.value
  const providerIds = props.providers.map((provider) => provider.id)
  const targetIndex = providerIds.indexOf(targetProviderId)
  if (!sourceProviderId || targetIndex < 0) {
    resetDrag()
    return
  }
  emitMovedOrder(sourceProviderId, targetIndex)
}

function moveProviderBy(providerId: string, offset: -1 | 1) {
  if (props.busy) return
  const sourceIndex = props.providers.findIndex((provider) => provider.id === providerId)
  emitMovedOrder(providerId, sourceIndex + offset)
}
</script>

<template>
  <section class="provider-list" aria-label="Provider 列表">
    <header class="provider-list-header">
      <div>
        <p class="eyebrow">Providers</p>
        <h2 class="provider-list-title">模型服务</h2>
      </div>
      <ElButton
        class="primary-button"
        type="primary"
        native-type="button"
        aria-label="新增 Provider"
        :disabled="busy"
        @click="emit('create')"
      >
        新增
      </ElButton>
    </header>

    <ElEmpty v-if="providers.length === 0" class="empty-state" description="还没有 Provider。" />
    <ul v-else class="provider-items">
      <li
        v-for="provider in providers"
        :key="provider.id"
        :data-provider-id="provider.id"
        :class="{
          'provider-item-dragging': draggedProviderId === provider.id,
          'provider-item-drop-target': dropTargetId === provider.id && draggedProviderId !== provider.id,
        }"
        @dragover="allowDrop($event, provider.id)"
        @drop="dropProvider($event, provider.id)"
      >
        <ElCard
          class="provider-card"
          :class="{ selected: selectedProviderId === provider.id }"
          shadow="never"
        >
          <div class="provider-card-heading">
            <ElButton
              class="provider-drag-handle"
              text
              native-type="button"
              :aria-label="`拖动 ${provider.name} 排序`"
              :title="`拖动 ${provider.name} 排序`"
              :disabled="busy || providers.length < 2"
              :draggable="!busy && providers.length > 1"
              @dragstart="startDrag($event, provider.id)"
              @dragend="resetDrag"
              @keydown.up.prevent="moveProviderBy(provider.id, -1)"
              @keydown.down.prevent="moveProviderBy(provider.id, 1)"
              @click.prevent
            >
              <span aria-hidden="true">⠿</span>
            </ElButton>
            <ElButton
              class="provider-select"
              text
              native-type="button"
              :aria-label="`选择 ${provider.name}`"
              @click="emit('select', provider.id)"
            >
              <span class="provider-select-content">
                <span class="provider-name">{{ provider.name }}</span>
                <span class="provider-id">{{ provider.id }}</span>
              </span>
            </ElButton>
          </div>

          <ProviderStatus :provider="provider" />
          <dl class="provider-details">
            <div>
              <dt>Base URL</dt>
              <dd>{{ provider.baseUrl || '未设置' }}</dd>
            </div>
            <div>
              <dt>Wire API</dt>
              <dd>{{ provider.wireApi }}</dd>
            </div>
            <div>
              <dt>偏好模型</dt>
              <dd>
                {{
                  provider.selectedModel ||
                  (provider.preferenceConfigured === false
                    ? '模型偏好未配置'
                    : '未指定（切换时保留现有模型）')
                }}
              </dd>
            </div>
          </dl>
          <p v-if="provider.validationMessage" class="validation-message">
            {{ provider.validationMessage }}
          </p>

          <div class="provider-actions">
            <ElButton
              native-type="button"
              :aria-label="`编辑 ${provider.name}`"
              :disabled="busy"
              @click="emit('edit', provider.id)"
            >
              编辑
            </ElButton>
            <ElButton
              type="primary"
              native-type="button"
              :aria-label="`使用 ${provider.name}`"
              :disabled="busy || provider.isActive || !provider.isValid || !provider.configurationComplete"
              @click="emit('use', provider.id)"
            >
              使用
            </ElButton>
            <ElButton
              type="danger"
              plain
              native-type="button"
              :aria-label="`删除 ${provider.name}`"
              :disabled="busy || provider.isActive"
              @click="emit('delete', provider.id)"
            >
              删除
            </ElButton>
          </div>
        </ElCard>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.provider-list {
  display: grid;
  align-content: start;
  gap: 0.75rem;
}

.provider-list-header,
.provider-actions,
.provider-select,
.provider-card-heading {
  display: flex;
  align-items: center;
}

.provider-list-header {
  justify-content: space-between;
}

.eyebrow,
.provider-list-title {
  margin: 0;
}

.eyebrow,
.provider-id,
.provider-details dt {
  color: var(--text-secondary);
  font-size: 0.75rem;
}

.provider-items {
  display: grid;
  gap: 0.6rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.provider-card-heading {
  gap: 0.35rem;
  min-width: 0;
}

.provider-drag-handle {
  flex: 0 0 auto;
  cursor: grab;
  font-size: 1.05rem;
}

.provider-drag-handle:active {
  cursor: grabbing;
}

.provider-item-dragging {
  opacity: 0.55;
}

.provider-item-drop-target .provider-card {
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 38%, transparent);
}

.provider-card {
  border: 1px solid var(--border);
  border-radius: 0.8rem;
  background: var(--surface);
}

.provider-card :deep(.el-card__body) {
  display: grid;
  gap: 0.6rem;
  padding: 0.75rem;
}

.provider-card.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 18%, transparent);
}

.provider-select {
  flex: 1 1 auto;
  min-width: 0;
  width: 100%;
  justify-content: stretch;
  border: 0;
  padding: 0;
  text-align: left;
}

:deep(.provider-select > span) {
  display: block;
  width: 100%;
  min-width: 0;
}

.provider-select-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  width: 100%;
  min-width: 0;
}

.provider-name {
  min-width: 0;
  font-weight: 700;
  overflow-wrap: anywhere;
}

.provider-id {
  flex: 0 1 auto;
  min-width: 0;
  overflow-wrap: anywhere;
}

.provider-details {
  display: grid;
  gap: 0.3rem;
  margin: 0;
}

.provider-details div {
  display: grid;
  grid-template-columns: 5rem minmax(0, 1fr);
  gap: 0.5rem;
}

.provider-details dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.validation-message {
  margin: 0;
  color: var(--danger);
}

.provider-actions {
  flex-wrap: wrap;
  gap: 0.4rem;
}

.primary-button {
  font-weight: 700;
}

.empty-state {
  color: var(--text-secondary);
}
</style>
