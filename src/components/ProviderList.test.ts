import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import ProviderList from './ProviderList.vue'

function provider(overrides: Partial<ProviderProfile> = {}): ProviderProfile {
  return {
    id: 'provider-a',
    name: 'Provider A',
    baseUrl: 'https://provider-a.example.test/v1',
    wireApi: 'responses',
    model: 'model-a',
    apiKeyConfigured: true,
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
            model: null,
            apiKeyConfigured: false,
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
    expect(wrapper.text()).toContain('当前')
    expect(wrapper.text()).toContain('Base URL 无效。')
    expect(wrapper.text()).toContain('未配置密钥')
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
})
