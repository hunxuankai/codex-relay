import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import { providerConnection } from '../test-utils/provider'
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

  it.each([
    ['apply', '仅应用连接', false],
    ['applied', '已应用', true],
    ['update', '更新连接', false],
    ['restore', '恢复自身连接', false],
  ] as const)(
    'renders the %s connection action with an accessible Provider name',
    async (action, label, disabled) => {
      const wrapper = mount(ProviderList, {
        props: {
          providers: [provider({
            isActive: action === 'restore',
            connection: providerConnection({
              role: action === 'restore' ? 'identity' : 'source',
              status: action === 'apply' ? 'none' : action === 'restore' ? 'stale' : 'active',
              action,
              disabledReason: action === 'restore' ? '当前连接已失效；请恢复自身连接。' : null,
            }),
          })],
          selectedProviderId: 'provider-a',
          busy: false,
        },
      })

      const button = wrapper.get(`[aria-label="${label} Provider A"]`)
      expect(button.text()).toBe(label)
      expect(button.attributes('disabled') !== undefined).toBe(disabled)
      if (!disabled) {
        await button.trigger('click')
        expect(wrapper.emitted('connection')?.[0]).toEqual(['provider-a'])
      }
    },
  )

  it.each(['apply', 'applied', 'update'] as const)(
    'keeps the %s compatibility explanation independent from the connection mutation',
    async (action) => {
      const wrapper = mount(ProviderList, {
        props: {
          providers: [provider({
            configurationComplete: action !== 'apply',
            connection: providerConnection({
              role: 'source',
              status: action === 'apply' ? 'none' : 'active',
              action,
              disabledReason: action === 'apply' ? '缺少受管 API Key。' : null,
            }),
          })],
          selectedProviderId: 'provider-a',
          busy: false,
        },
      })

      const explanation = wrapper.get(
        '[aria-label="查看 Provider A 的旧会话兼容性说明"]',
      )
      expect(explanation.attributes('disabled')).toBeUndefined()

      await explanation.trigger('click')

      expect(wrapper.emitted('connectionRisk')?.[0]).toEqual(['provider-a'])
      expect(wrapper.emitted('connection')).toBeUndefined()

      await wrapper.setProps({ busy: true })
      expect(explanation.attributes('disabled')).toBeDefined()
    },
  )

  it('does not show the compatibility explanation for restore or absent connection actions', () => {
    const wrapper = mount(ProviderList, {
      props: {
        providers: [
          provider({
            connection: providerConnection({
              role: 'identity',
              status: 'stale',
              action: 'restore',
            }),
          }),
          provider({
            id: 'provider-b',
            name: 'Provider B',
            isActive: false,
          }),
        ],
        selectedProviderId: null,
        busy: false,
      },
    })

    expect(wrapper.findAll('[aria-label$="旧会话兼容性说明"]')).toHaveLength(0)
  })

  it('prevents deleting an identity that still owns a stale connection recovery point', () => {
    const wrapper = mount(ProviderList, {
      props: {
        providers: [provider({
          isActive: false,
          connection: providerConnection({
            role: 'identity',
            status: 'stale',
            action: 'restore',
            disabledReason: '当前连接已失效；请恢复自身连接。',
          }),
        })],
        selectedProviderId: 'provider-a',
        busy: false,
      },
    })

    expect(wrapper.get('[aria-label="删除 Provider A"]').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('当前连接已失效；请恢复自身连接。')
  })

  it('prevents deleting the Provider used as the active connection source', () => {
    const wrapper = mount(ProviderList, {
      props: {
        providers: [provider({
          isActive: false,
          connection: providerConnection({
            role: 'source',
            status: 'active',
            action: 'applied',
          }),
        })],
        selectedProviderId: 'provider-a',
        busy: false,
      },
    })

    expect(wrapper.get('[aria-label="删除 Provider A"]').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('当前连接')
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
