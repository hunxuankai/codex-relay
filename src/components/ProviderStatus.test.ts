import { mount } from '@vue/test-utils'
import { ElTag } from 'element-plus'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import ProviderStatus from './ProviderStatus.vue'

const provider: ProviderProfile = {
  id: 'provider-a',
  name: 'Provider A',
  baseUrl: 'https://provider-a.example.test/v1',
  baseUrls: [{ id: 'url-primary', name: '主用地址', url: 'https://provider-a.example.test/v1' }],
  selectedBaseUrlId: 'url-primary',
  baseUrlStatus: 'managed',
  apiKeys: [],
  selectedApiKeyId: null,
  apiKeyStatus: 'missing',
  wireApi: 'responses',
  models: ['gpt-5.6-sol'],
  selectedModel: 'gpt-5.6-sol',
  reasoningEfforts: { 'gpt-5.6-sol': 'medium' },
  preferenceConfigured: true,
  apiKeyConfigured: false,
  configurationComplete: false,
  disabledReason: '缺少受管 API Key。',
  isActive: true,
  isValid: false,
  validationMessage: '配置无效。',
}

describe('ProviderStatus', () => {
  it('renders textual Element Plus tags for every active state', () => {
    const wrapper = mount(ProviderStatus, { props: { provider } })

    expect(wrapper.findAllComponents(ElTag)).toHaveLength(3)
    expect(wrapper.text()).toContain('当前')
    expect(wrapper.text()).toContain('配置不完整')
    expect(wrapper.text()).toContain('配置无效')
  })
})
