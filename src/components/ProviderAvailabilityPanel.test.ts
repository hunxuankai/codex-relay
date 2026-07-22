import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import type {
  ProviderAvailabilityResult,
  ProviderTestKind,
  ProviderTestStatus,
} from '../types/providerAvailability'
import ProviderAvailabilityPanel from './ProviderAvailabilityPanel.vue'

const provider: ProviderProfile = {
  id: 'provider-a',
  name: 'Provider A',
  baseUrl: 'https://provider-a.example.test/v1',
  wireApi: 'responses',
  models: ['gpt-5.6-sol'],
  selectedModel: 'gpt-5.6-sol',
  reasoningEfforts: { 'gpt-5.6-sol': 'medium' },
  preferenceConfigured: true,
  apiKeyConfigured: true,
  isActive: false,
  isValid: true,
  validationMessage: null,
}

function result(
  kind: ProviderTestKind,
  status: ProviderTestStatus,
  overrides: Partial<ProviderAvailabilityResult> = {},
): ProviderAvailabilityResult {
  return {
    providerId: 'provider-a',
    kind,
    status,
    code: status === 'passed' ? `${kind.toUpperCase()}_TEST_PASSED` : 'API_AUTH_FAILED',
    message: status === 'passed' ? '测试通过。' : 'Provider 拒绝了 API Key。',
    model: 'gpt-5.6-sol',
    durationMs: 25,
    testedAt: '2026-07-23T00:00:00Z',
    httpStatus: kind === 'api' ? 401 : null,
    codexVersion: kind === 'codex' ? '0.144.4' : null,
    ...overrides,
  }
}

describe('ProviderAvailabilityPanel', () => {
  it('presents the minimal API probe as the default action and Codex as a separate advanced action', async () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      props: {
        provider,
        apiResult: null,
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    const buttons = wrapper.findAllComponents({ name: 'ElButton' })
    const apiButton = buttons.find(
      (button) => button.attributes('aria-label') === '测试 Provider A 的 API 可用性',
    )
    const codexButton = buttons.find(
      (button) => button.attributes('aria-label') === '运行 Provider A 的 Codex 兼容性测试',
    )

    expect(apiButton?.props('type')).toBe('primary')
    expect(apiButton?.props('plain')).toBe(true)
    expect(apiButton?.text()).toBe('测试 API 可用性')
    expect(codexButton?.text()).toBe('运行 Codex 兼容性测试')
    expect(wrapper.text()).toContain('最多 16 个输出 token')
    expect(wrapper.text()).toContain('一次正常 Codex 回合')
    expect(wrapper.text()).toContain('高级')

    await apiButton?.trigger('click')
    await codexButton?.trigger('click')
    expect(wrapper.emitted('testApi')).toHaveLength(1)
    expect(wrapper.emitted('requestCodexTest')).toHaveLength(1)
  })

  it('shows API and Codex outcomes independently with visible status and bounded metadata', () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      props: {
        provider,
        apiResult: result('api', 'failed'),
        codexResult: result('codex', 'passed'),
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    const apiResult = wrapper.get('[aria-label="API 测试结果"]')
    expect(apiResult.text()).toContain('失败')
    expect(apiResult.text()).toContain('Provider 拒绝了 API Key。')
    expect(apiResult.text()).toContain('gpt-5.6-sol')
    expect(apiResult.text()).toContain('25 ms')
    expect(apiResult.text()).toContain('HTTP 401')
    expect(apiResult.getComponent({ name: 'ElTag' }).props('type')).toBe('danger')

    const codexResult = wrapper.get('[aria-label="Codex 兼容性测试结果"]')
    expect(codexResult.text()).toContain('通过')
    expect(codexResult.text()).toContain('测试通过。')
    expect(codexResult.text()).toContain('Codex 0.144.4')
    expect(codexResult.getComponent({ name: 'ElTag' }).props('type')).toBe('success')
  })

  it('turns the active test into an explicit cancel action and disables the other test', async () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      props: {
        provider,
        apiResult: null,
        codexResult: null,
        runningKind: 'api',
        disabled: false,
        cancelling: false,
      },
    })

    const apiButton = wrapper.get('[aria-label="取消 Provider A 的 API 可用性测试"]')
    const codexButton = wrapper.get('[aria-label="运行 Provider A 的 Codex 兼容性测试"]')
    expect(apiButton.text()).toBe('取消 API 可用性测试')
    expect(apiButton.attributes('disabled')).toBeUndefined()
    expect(codexButton.attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('正在运行 API 可用性测试')

    await apiButton.trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)

    await wrapper.setProps({ cancelling: true })
    expect(wrapper.get('[aria-label="取消 Provider A 的 API 可用性测试"]').text()).toBe('正在取消…')
    expect(wrapper.get('[aria-label="取消 Provider A 的 API 可用性测试"]').attributes('disabled')).toBeDefined()
  })

  it('explains why testing is disabled when the Provider is not ready', () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      props: {
        provider: { ...provider, apiKeyConfigured: false },
        apiResult: null,
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    expect(wrapper.text()).toContain('未配置 API Key，无法测试。')
    expect(wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[aria-label="运行 Provider A 的 Codex 兼容性测试"]').attributes('disabled')).toBeDefined()
  })
})
