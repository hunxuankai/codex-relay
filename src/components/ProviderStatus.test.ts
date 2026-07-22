import { mount } from '@vue/test-utils'
import { ElTag } from 'element-plus'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import ProviderStatus from './ProviderStatus.vue'

const provider: ProviderProfile = {
  id: 'provider-a',
  name: 'Provider A',
  baseUrl: 'https://provider-a.example.test/v1',
  wireApi: 'responses',
  models: ['gpt-5.6-sol'],
  selectedModel: 'gpt-5.6-sol',
  reasoningEfforts: { 'gpt-5.6-sol': 'medium' },
  preferenceConfigured: true,
  apiKeyConfigured: false,
  isActive: true,
  isValid: false,
  validationMessage: '配置无效。',
}

describe('ProviderStatus', () => {
  it('renders textual Element Plus tags for every active state', () => {
    const wrapper = mount(ProviderStatus, { props: { provider } })

    expect(wrapper.findAllComponents(ElTag)).toHaveLength(3)
    expect(wrapper.text()).toContain('当前')
    expect(wrapper.text()).toContain('未配置密钥')
    expect(wrapper.text()).toContain('配置无效')
  })
})
