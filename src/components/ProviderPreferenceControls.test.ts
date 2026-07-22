import { mount } from '@vue/test-utils'
import { ElSegmented } from 'element-plus'
import { describe, expect, it } from 'vitest'
import type { ModelCatalogItem, ProviderProfile } from '../types/provider'
import ProviderPreferenceControls from './ProviderPreferenceControls.vue'

const modelCatalog: ModelCatalogItem[] = [
  { id: 'gpt-5.6-sol', reasoningEfforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'], defaultReasoningEffort: 'medium' },
  { id: 'gpt-5.4-mini', reasoningEfforts: ['none', 'low', 'medium', 'high', 'xhigh'], defaultReasoningEffort: 'none' },
]

function provider(overrides: Partial<ProviderProfile> = {}): ProviderProfile {
  return {
    id: 'provider-a', name: 'Provider A', baseUrl: 'https://provider-a.example.test/v1', wireApi: 'responses',
    models: ['gpt-5.6-sol', 'gpt-5.4-mini'], selectedModel: 'gpt-5.6-sol',
    reasoningEfforts: { 'gpt-5.6-sol': 'high', 'gpt-5.4-mini': 'low' }, preferenceConfigured: true,
    apiKeyConfigured: true, isActive: false, isValid: true, validationMessage: null, ...overrides,
  }
}

describe('ProviderPreferenceControls', () => {
  it('uses two segmented controls and restores the selected model effort', async () => {
    const wrapper = mount(ProviderPreferenceControls, { props: { provider: provider(), modelCatalog, busy: false } })
    const controls = wrapper.findAllComponents(ElSegmented) as any[]
    expect(controls).toHaveLength(2)
    expect(controls[0]?.props('options')).toEqual(['gpt-5.6-sol', 'gpt-5.4-mini'])
    expect(controls[1]?.props('modelValue')).toBe('high')

    controls[0]?.vm.$emit('change', 'gpt-5.4-mini')
    expect(wrapper.emitted('select')?.[0]).toEqual(['gpt-5.4-mini', 'low'])
    controls[1]?.vm.$emit('change', 'xhigh')
    expect(wrapper.emitted('select')?.[1]).toEqual(['gpt-5.6-sol', 'xhigh'])
    expect(wrapper.text()).toContain('将在应用此 Provider 时生效')
  })

  it('describes immediate writes for the active Provider and disables controls while busy', () => {
    const wrapper = mount(ProviderPreferenceControls, { props: { provider: provider({ isActive: true }), modelCatalog, busy: true } })
    expect(wrapper.text()).toContain('立即写入当前 Codex 配置')
    for (const control of wrapper.findAllComponents(ElSegmented) as any[]) expect(control.props('disabled')).toBe(true)
  })

  it('shows a configuration entry when preference metadata is missing', async () => {
    const wrapper = mount(ProviderPreferenceControls, {
      props: { provider: provider({ models: [], selectedModel: null, reasoningEfforts: {}, preferenceConfigured: false }), modelCatalog, busy: false },
    })
    expect(wrapper.findComponent(ElSegmented).exists()).toBe(false)
    expect(wrapper.text()).toContain('模型偏好未配置')
    await wrapper.get('button').trigger('click')
    expect(wrapper.emitted('configure')).toHaveLength(1)
  })
})
