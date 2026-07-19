<script setup lang="ts">
import type { ProviderProfile } from '../types/provider'
import ProviderStatus from './ProviderStatus.vue'

defineProps<{
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
}>()
</script>

<template>
  <section class="provider-list" aria-label="Provider 列表">
    <header class="provider-list-header">
      <div>
        <p class="eyebrow">Providers</p>
        <h2 class="provider-list-title">模型服务</h2>
      </div>
      <button
        class="primary-button"
        type="button"
        aria-label="新增 Provider"
        :disabled="busy"
        @click="emit('create')"
      >
        新增
      </button>
    </header>

    <p v-if="providers.length === 0" class="empty-state">还没有 Provider。</p>
    <ul v-else class="provider-items">
      <li v-for="provider in providers" :key="provider.id">
        <article
          class="provider-card"
          :class="{ selected: selectedProviderId === provider.id }"
        >
          <button
            class="provider-select"
            type="button"
            :aria-label="`选择 ${provider.name}`"
            @click="emit('select', provider.id)"
          >
            <span class="provider-name">{{ provider.name }}</span>
            <span class="provider-id">{{ provider.id }}</span>
          </button>

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
              <dt>默认模型</dt>
              <dd>{{ provider.model || '跟随 Codex 当前设置' }}</dd>
            </div>
          </dl>
          <p v-if="provider.validationMessage" class="validation-message">
            {{ provider.validationMessage }}
          </p>

          <div class="provider-actions">
            <button
              type="button"
              :aria-label="`编辑 ${provider.name}`"
              :disabled="busy"
              @click="emit('edit', provider.id)"
            >
              编辑
            </button>
            <button
              type="button"
              :aria-label="`使用 ${provider.name}`"
              :disabled="busy || provider.isActive || !provider.isValid || !provider.apiKeyConfigured"
              @click="emit('use', provider.id)"
            >
              使用
            </button>
            <button
              type="button"
              :aria-label="`删除 ${provider.name}`"
              :disabled="busy || provider.isActive"
              @click="emit('delete', provider.id)"
            >
              删除
            </button>
          </div>
        </article>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.provider-list {
  display: grid;
  gap: 1rem;
}

.provider-list-header,
.provider-actions,
.provider-select {
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
  color: #667085;
  font-size: 0.75rem;
}

.provider-items {
  display: grid;
  gap: 0.75rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.provider-card {
  display: grid;
  gap: 0.75rem;
  border: 1px solid #d0d5dd;
  border-radius: 0.8rem;
  padding: 0.9rem;
}

.provider-card.selected {
  border-color: #356ae6;
  box-shadow: 0 0 0 2px rgb(53 106 230 / 15%);
}

.provider-select {
  justify-content: space-between;
  gap: 1rem;
  border: 0;
  padding: 0;
  background: transparent;
  text-align: left;
}

.provider-name {
  font-weight: 700;
}

.provider-details {
  display: grid;
  gap: 0.4rem;
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
  color: #b42318;
}

.provider-actions {
  gap: 0.5rem;
}

.primary-button {
  font-weight: 700;
}

.empty-state {
  color: #667085;
}
</style>
