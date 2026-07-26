import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import ProviderList from './ProviderList.vue'

function provider(overrides: Partial<ProviderProfile> = {}): ProviderProfile {
  return {
    id: 'provider-a',
    name: 'Provider A',
    baseUrl: 'https://provider-a.example.test/v1',
    baseUrls: [{ id: 'url-primary', name: '主用地址', url: 'https://provider-a.example.test/v1' }],
    selectedBaseUrlId: 'url-primary',
    baseUrlStatus: 'managed',
    apiKeys: [{ id: 'key-primary', name: '主用密钥' }],
    selectedApiKeyId: 'key-primary',
    apiKeyStatus: 'managed',
    wireApi: 'responses',
    models: ['model-a'],
    selectedModel: 'model-a',
    reasoningEfforts: { 'model-a': 'medium' },
    preferenceConfigured: true,
    apiKeyConfigured: true,
    configurationComplete: true,
    disabledReason: null,
    isActive: true,
    isValid: true,
    validationMessage: null,
    ...overrides,
  }
}

describe('ProviderList', () => {
  it('renders Provider details and status', () => {
    const wrapper = mount(ProviderList, {
      props: {
        providers: [
          provider(),
          provider({
            id: 'provider-b',
            name: 'Provider B',
            selectedModel: null,
            apiKeyConfigured: false,
            apiKeys: [],
            selectedApiKeyId: null,
            apiKeyStatus: 'missing',
            configurationComplete: false,
            disabledReason: '缺少受管 API Key。',
            isActive: false,
            isValid: false,
            validationMessage: 'Base URL 无效。',
          }),
        ],
        selectedProviderId: 'provider-a',
        busy: false,
      },
    })

    expect(wrapper.text()).toContain('Provider A')
    expect(wrapper.text()).toContain('provider-a')
    expect(wrapper.text()).toContain('https://provider-a.example.test/v1')
    expect(wrapper.text()).toContain('responses')
    expect(wrapper.text()).toContain('model-a')
    expect(wrapper.text()).toContain('未指定（切换时保留现有模型）')
    expect(wrapper.text()).toContain('当前')
    expect(wrapper.text()).toContain('Base URL 无效。')
    expect(wrapper.text()).toContain('配置不完整')
  })

  it('disables invalid or keyless use and current deletion', () => {
    const wrapper = mount(ProviderList, {
      props: {
        providers: [
          provider(),
          provider({
            id: 'provider-b',
            name: 'Provider B',
            apiKeyConfigured: false,
            apiKeys: [],
            selectedApiKeyId: null,
            apiKeyStatus: 'missing',
            configurationComplete: false,
            disabledReason: '缺少受管 API Key。',
            isActive: false,
          }),
        ],
        selectedProviderId: null,
        busy: false,
      },
    })

    expect(wrapper.get('[aria-label="使用 Provider A"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[aria-label="删除 Provider A"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[aria-label="使用 Provider B"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[aria-label="删除 Provider B"]').attributes('disabled')).toBeUndefined()
  })

  it('emits create, select, edit, use, and delete actions', async () => {
    const wrapper = mount(ProviderList, {
      props: {
        providers: [provider({ isActive: false })],
        selectedProviderId: null,
        busy: false,
      },
    })

    await wrapper.get('[aria-label="新增 Provider"]').trigger('click')
    await wrapper.get('[aria-label="选择 Provider A"]').trigger('click')
    await wrapper.get('[aria-label="编辑 Provider A"]').trigger('click')
    await wrapper.get('[aria-label="使用 Provider A"]').trigger('click')
    await wrapper.get('[aria-label="删除 Provider A"]').trigger('click')

    expect(wrapper.emitted('create')).toHaveLength(1)
    expect(wrapper.emitted('select')?.[0]).toEqual(['provider-a'])
    expect(wrapper.emitted('edit')?.[0]).toEqual(['provider-a'])
    expect(wrapper.emitted('use')?.[0]).toEqual(['provider-a'])
    expect(wrapper.emitted('delete')?.[0]).toEqual(['provider-a'])
  })

  it('emits the complete order when a Provider is dropped and disables dragging while busy', async () => {
    const wrapper = mount(ProviderList, {
      props: {
        providers: [
          provider({ id: 'provider-a', name: 'Provider A' }),
          provider({ id: 'provider-b', name: 'Provider B', isActive: false }),
          provider({ id: 'provider-c', name: 'Provider C', isActive: false }),
        ],
        selectedProviderId: null,
        busy: false,
      },
    })

    const handle = wrapper.get('[aria-label="拖动 Provider A 排序"]')
    expect(handle.attributes('draggable')).toBe('true')
    await handle.trigger('dragstart')
    await wrapper.get('[data-provider-id="provider-c"]').trigger('drop')

    expect(wrapper.emitted('reorder')?.[0]).toEqual([
      ['provider-b', 'provider-c', 'provider-a'],
    ])

    await wrapper.get('[aria-label="拖动 Provider C 排序"]').trigger('keydown', {
      key: 'ArrowUp',
    })
    expect(wrapper.emitted('reorder')?.[1]).toEqual([
      ['provider-a', 'provider-c', 'provider-b'],
    ])

    await wrapper.setProps({ busy: true })
    expect(wrapper.get('[aria-label="拖动 Provider A 排序"]').attributes('draggable')).toBe('false')
  })
})
