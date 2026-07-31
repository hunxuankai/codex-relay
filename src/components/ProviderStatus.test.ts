import { mount } from '@vue/test-utils'
import { ElTag } from 'element-plus'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import { providerConnection } from '../test-utils/provider'
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
  fastEnabled: false,
  preferenceConfigured: true,
  apiKeyConfigured: false,
  configurationComplete: false,
  disabledReason: '缺少受管 API Key。',
  connection: providerConnection(),
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

  it.each([
    [providerConnection({ role: 'identity', status: 'active', action: 'restore' }), '当前身份'],
    [providerConnection({ role: 'source', status: 'active', action: 'applied' }), '当前连接'],
    [providerConnection({ role: 'source', status: 'active', action: 'update' }), '选择已变化'],
    [providerConnection({ role: 'identity', status: 'stale', action: 'restore' }), '连接已失效'],
  ] as const)('renders the backend connection state as %s', (connection, label) => {
    const wrapper = mount(ProviderStatus, {
      props: { provider: { ...provider, connection } },
    })

    expect(wrapper.text()).toContain(label)
  })
})
