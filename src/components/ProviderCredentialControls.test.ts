import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import ProviderCredentialControls from './ProviderCredentialControls.vue'

function provider(overrides: Partial<ProviderProfile> = {}): ProviderProfile {
  return {
    id: 'provider-a',
    name: 'Provider A',
    baseUrl: 'https://primary.example.test/v1',
    baseUrls: [{ id: 'url-primary', name: '主用地址', url: 'https://primary.example.test/v1' }],
    selectedBaseUrlId: 'url-primary',
    baseUrlStatus: 'managed',
    apiKeys: [
      { id: 'key-primary', name: '主用密钥' },
      { id: 'key-backup', name: '备用密钥' },
    ],
    selectedApiKeyId: 'key-primary',
    apiKeyStatus: 'managed',
    wireApi: 'responses',
    models: ['gpt-5.6-sol'],
    selectedModel: 'gpt-5.6-sol',
    reasoningEfforts: { 'gpt-5.6-sol': 'medium' },
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

describe('ProviderCredentialControls', () => {
  it('shows only key names and emits key selection independently', async () => {
    const wrapper = mount(ProviderCredentialControls, {
      props: { provider: provider(), busy: false },
    })
    const segmented = wrapper.getComponent({ name: 'ElSegmented' })

    expect(segmented.props('options')).toEqual([
      { label: '主用密钥', value: 'key-primary' },
      { label: '备用密钥', value: 'key-backup' },
    ])
    expect(wrapper.text()).toContain('当前密钥：主用密钥')
    expect(wrapper.text()).not.toContain('test-key')

    segmented.vm.$emit('change', 'key-backup')
    await wrapper.get('[aria-label="管理 API Key"]').trigger('click')

    expect(wrapper.emitted('select')?.[0]).toEqual(['key-backup'])
    expect(wrapper.emitted('manage')).toHaveLength(1)
  })

  it('explains external and missing key states without exposing a value', async () => {
    const external = mount(ProviderCredentialControls, {
      props: {
        provider: provider({ selectedApiKeyId: null, apiKeyStatus: 'external' }),
        busy: false,
      },
    })
    expect(external.text()).toContain('当前使用外部密钥')

    await external.setProps({
      provider: provider({
        apiKeys: [],
        selectedApiKeyId: null,
        apiKeyStatus: 'missing',
        apiKeyConfigured: false,
        configurationComplete: false,
        disabledReason: '缺少受管 API Key。',
      }),
    })
    expect(external.text()).toContain('尚未配置受管密钥')
    expect(external.text()).toContain('缺少受管 API Key。')
  })
})
