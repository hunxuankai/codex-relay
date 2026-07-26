import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import type {
  ProviderAvailabilityTrace,
  ProviderAvailabilityResult,
  ProviderTestKind,
  ProviderTestStatus,
} from '../types/providerAvailability'
import ProviderAvailabilityPanel from './ProviderAvailabilityPanel.vue'

const provider: ProviderProfile = {
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
  models: ['gpt-5.6-sol'],
  selectedModel: 'gpt-5.6-sol',
  reasoningEfforts: { 'gpt-5.6-sol': 'medium' },
  preferenceConfigured: true,
  apiKeyConfigured: true,
  configurationComplete: true,
  disabledReason: null,
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
    trace: null,
    ...overrides,
  }
}

const apiTrace: ProviderAvailabilityTrace = {
  request: {
    method: 'POST',
    url: 'https://provider-a.example.test/v1/responses',
    body: '{\n  "model": "gpt-5.6-sol",\n  "stream": false\n}',
  },
  response: {
    status: 200,
    body: '{\n  "status": "completed"\n}',
    bodyTruncated: false,
  },
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
    expect(wrapper.emitted('testApi')).toEqual([[false]])
    expect(wrapper.emitted('requestCodexTest')).toEqual([[false]])
  })

  it('defaults to bypassing proxies and blocks proxy tests until network proxy is enabled', async () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      props: {
        provider,
        apiResult: null,
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
        networkProxyEnabled: false,
      },
    })

    const skipProxy = wrapper.get('[name="provider-test-skip-proxy"]')
    expect((skipProxy.element as HTMLInputElement).checked).toBe(true)

    await skipProxy.setValue(false)

    expect(wrapper.text()).toContain('设置中的“网络代理”尚未启用，无法使用代理测试。')
    expect(wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[aria-label="运行 Provider A 的 Codex 兼容性测试"]').attributes('disabled')).toBeDefined()
  })

  it('requests the configured proxy for both tests when enabled', async () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      props: {
        provider,
        apiResult: null,
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
        networkProxyEnabled: true,
      },
    })

    await wrapper.get('[name="provider-test-skip-proxy"]').setValue(false)
    await wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').trigger('click')
    await wrapper.get('[aria-label="运行 Provider A 的 Codex 兼容性测试"]').trigger('click')

    expect(wrapper.emitted('testApi')).toEqual([[true]])
    expect(wrapper.emitted('requestCodexTest')).toEqual([[true]])
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

  it('opens the API request and response trace in a separate dialog', async () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      attachTo: document.body,
      props: {
        provider,
        apiResult: result('api', 'passed', { trace: apiTrace }),
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    const openTrace = wrapper.get('[aria-label="查看 Provider A 的 API 请求与响应"]')
    await openTrace.trigger('click')

    expect(document.body.textContent).toContain('请求')
    expect(document.body.textContent).toContain('POST')
    expect(document.body.textContent).toContain('/v1/responses')
    expect(document.body.textContent).toContain('响应')
    expect(document.body.textContent).toContain('HTTP 200')
    expect(document.body.textContent).toContain('"status": "completed"')
  })

  it('reopens an existing trace without starting another API test', async () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      attachTo: document.body,
      props: {
        provider,
        apiResult: result('api', 'passed', { trace: apiTrace }),
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    const openTrace = wrapper.get('[aria-label="查看 Provider A 的 API 请求与响应"]')
    await openTrace.trigger('click')
    const dialog = wrapper.findComponent({ name: 'ProviderAvailabilityTraceDialog' })
    expect(dialog.props('open')).toBe(true)

    dialog.vm.$emit('close')
    await flushPromises()
    expect(dialog.props('open')).toBe(false)

    await openTrace.trigger('click')
    expect(dialog.props('open')).toBe(true)
    expect(wrapper.emitted('testApi')).toBeUndefined()
  })

  it('opens the request and response dialog immediately with loading sections', async () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      attachTo: document.body,
      props: {
        provider,
        apiResult: null,
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    await wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').trigger('click')

    const dialog = wrapper.findComponent({ name: 'ProviderAvailabilityTraceDialog' })
    expect(dialog.exists()).toBe(true)
    expect(dialog.props('open')).toBe(true)
    expect(document.body.textContent).toContain('正在生成请求')
    expect(document.body.textContent).toContain('正在等待响应')
    wrapper.unmount()
  })

  it('clears the old trace and automatically opens the new trace after an API test', async () => {
    const newTrace: ProviderAvailabilityTrace = {
      request: {
        ...apiTrace.request,
        body: '{\n  "model": "gpt-5.6-sol",\n  "stream": false,\n  "request": "new"\n}',
      },
      response: {
        ...apiTrace.response!,
        body: '{\n  "status": "new-response"\n}',
      },
    }
    const wrapper = mount(ProviderAvailabilityPanel, {
      attachTo: document.body,
      props: {
        provider,
        apiResult: result('api', 'passed', { trace: apiTrace }),
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    await wrapper.get('[aria-label="查看 Provider A 的 API 请求与响应"]').trigger('click')
    expect(document.body.textContent).toContain('"status": "completed"')

    await wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').trigger('click')
    await wrapper.setProps({ apiResult: null, runningKind: 'api' })
    expect(document.body.textContent).not.toContain('"status": "completed"')

    await wrapper.setProps({
      apiResult: result('api', 'passed', { trace: newTrace }),
      runningKind: null,
    })
    await flushPromises()

    expect(document.body.textContent).toContain('"status": "new-response"')
    expect(document.body.textContent).not.toContain('"status": "completed"')
  })

  it('consumes the auto-open request when the new API result has no trace', async () => {
    const wrapper = mount(ProviderAvailabilityPanel, {
      attachTo: document.body,
      props: {
        provider,
        apiResult: result('api', 'passed', { trace: apiTrace }),
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    await wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').trigger('click')
    await wrapper.setProps({ apiResult: null, runningKind: 'api' })
    await wrapper.setProps({
      apiResult: result('api', 'failed', { trace: null }),
      runningKind: null,
    })
    await flushPromises()

    expect(document.body.textContent).not.toContain('API 请求与响应')

    await wrapper.setProps({ apiResult: result('api', 'passed', { trace: apiTrace }) })
    await flushPromises()
    expect(document.body.textContent).not.toContain('"status": "completed"')
  })

  it('does not auto-open a cancelled trace', async () => {
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

    await wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').trigger('click')
    await wrapper.setProps({ runningKind: 'api' })
    await wrapper.setProps({
      apiResult: result('api', 'cancelled', { trace: apiTrace }),
      runningKind: null,
    })
    await flushPromises()

    const dialog = wrapper.findComponent({ name: 'ProviderAvailabilityTraceDialog' })
    expect(dialog.exists()).toBe(false)
  })

  it('drops a pending auto-open when the Provider changes', async () => {
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

    await wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').trigger('click')
    await wrapper.setProps({ runningKind: 'api' })
    await wrapper.setProps({
      provider: { ...provider, id: 'provider-b', name: 'Provider B' },
      apiResult: result('api', 'passed', { trace: apiTrace }),
      runningKind: null,
    })
    await flushPromises()

    const dialog = wrapper.findComponent({ name: 'ProviderAvailabilityTraceDialog' })
    expect(dialog.exists()).toBe(true)
    expect(dialog.props('open')).toBe(false)
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
        provider: {
          ...provider,
          apiKeyStatus: 'external',
          configurationComplete: false,
          disabledReason: '外部密钥尚未纳管。',
        },
        apiResult: null,
        codexResult: null,
        runningKind: null,
        disabled: false,
        cancelling: false,
      },
    })

    expect(wrapper.text()).toContain('外部密钥尚未纳管。')
    expect(wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[aria-label="运行 Provider A 的 Codex 兼容性测试"]').attributes('disabled')).toBeDefined()
  })
})
