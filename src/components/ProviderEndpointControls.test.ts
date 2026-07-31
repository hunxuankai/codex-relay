import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import { providerConnection } from '../test-utils/provider'
import ProviderEndpointControls from './ProviderEndpointControls.vue'

function provider(overrides: Partial<ProviderProfile> = {}): ProviderProfile {
  return {
    id: 'provider-a',
    name: 'Provider A',
    baseUrl: 'https://primary.example.test/v1',
    baseUrls: [
      { id: 'url-primary', name: '主用地址', url: 'https://primary.example.test/v1' },
      { id: 'url-backup', name: '备用地址', url: 'https://backup.example.test/v1' },
    ],
    selectedBaseUrlId: 'url-primary',
    baseUrlStatus: 'managed',
    apiKeys: [{ id: 'key-primary', name: '主用密钥' }],
    selectedApiKeyId: 'key-primary',
    apiKeyStatus: 'managed',
    wireApi: 'responses',
    models: ['gpt-5.6-sol'],
    selectedModel: 'gpt-5.6-sol',
    reasoningEfforts: { 'gpt-5.6-sol': 'medium' },
    fastEnabled: false,
    preferenceConfigured: true,
    apiKeyConfigured: true,
    configurationComplete: true,
    disabledReason: null,
    connection: providerConnection(),
    isActive: true,
    isValid: true,
    validationMessage: null,
    ...overrides,
  }
}

describe('ProviderEndpointControls', () => {
  it('shows named URLs, visible current state, and emits selection independently', async () => {
    const wrapper = mount(ProviderEndpointControls, {
      props: { provider: provider(), busy: false },
    })
    const segmented = wrapper.getComponent({ name: 'ElSegmented' })

    expect(segmented.props('options')).toEqual([
      { label: '主用地址', value: 'url-primary' },
      { label: '备用地址', value: 'url-backup' },
    ])
    expect(segmented.props('modelValue')).toBe('url-primary')
    expect(segmented.props('size')).toBe('small')
    expect(wrapper.text()).toContain('当前地址：主用地址')
    expect(wrapper.text()).toContain('https://primary.example.test/v1')
    expect(wrapper.get('.segmented-scroll').attributes('role')).toBe('group')
    expect(wrapper.get('[aria-label="管理 Base URL"]').text()).toBe('管理')

    segmented.vm.$emit('change', 'url-backup')
    await wrapper.get('[aria-label="管理 Base URL"]').trigger('click')

    expect(wrapper.emitted('select')?.[0]).toEqual(['url-backup'])
    expect(wrapper.emitted('manage')).toHaveLength(1)
  })

  it('labels an unmatched external URL without inventing a managed selection', () => {
    const wrapper = mount(ProviderEndpointControls, {
      props: {
        provider: provider({
          baseUrl: 'https://external.example.test/v1',
          selectedBaseUrlId: null,
          baseUrlStatus: 'external',
        }),
        busy: false,
      },
    })

    expect(wrapper.getComponent({ name: 'ElSegmented' }).props('modelValue')).toBeUndefined()
    expect(wrapper.text()).toContain('当前使用外部地址')
    expect(wrapper.text()).toContain('https://external.example.test/v1')
  })

  it('shows and locks the routed connection source until identity restore', async () => {
    const wrapper = mount(ProviderEndpointControls, {
      props: {
        provider: provider({
          selectedBaseUrlId: null,
          baseUrlStatus: 'routed',
          connection: providerConnection({
            role: 'identity',
            status: 'active',
            action: 'restore',
            sourceProviderName: 'Provider B',
            appliedBaseUrlName: 'B 主用地址',
          }),
        }),
        busy: false,
      },
    })
    const segmented = wrapper.getComponent({ name: 'ElSegmented' })
    const manage = wrapper.get('[aria-label="管理 Base URL"]')

    expect(wrapper.text()).toContain('Provider B')
    expect(wrapper.text()).toContain('B 主用地址')
    expect(wrapper.text()).toContain('恢复自身连接后可管理')
    expect(segmented.props('disabled')).toBe(true)
    expect(manage.attributes('disabled')).toBeDefined()

    segmented.vm.$emit('change', 'url-backup')
    await manage.trigger('click')
    expect(wrapper.emitted('select')).toBeUndefined()
    expect(wrapper.emitted('manage')).toBeUndefined()
  })
})
