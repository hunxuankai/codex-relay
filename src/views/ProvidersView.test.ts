import { flushPromises, mount } from '@vue/test-utils'
import { nextTick, ref, shallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ProviderProfile } from '../types/provider'
import type { ProviderAvailabilityResult, ProviderTestKind } from '../types/providerAvailability'
import ProvidersView from './ProvidersView.vue'

const mockUseProviders = vi.hoisted(() => vi.fn())
vi.mock('../composables/useProviders', () => ({ useProviders: mockUseProviders }))
const mockUseProviderAvailability = vi.hoisted(() => vi.fn())
vi.mock('../composables/useProviderAvailability', () => ({
  useProviderAvailability: mockUseProviderAvailability,
}))

const fingerprints = {
  config: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'config' },
  auth: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'auth' },
  providers: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'providers' },
  preferences: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'preferences' },
}

const modelCatalog = [
  { id: 'gpt-5.6-sol', reasoningEfforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'], defaultReasoningEffort: 'medium' },
  { id: 'gpt-5.4-mini', reasoningEfforts: ['none', 'low', 'medium', 'high', 'xhigh'], defaultReasoningEffort: 'none' },
] as const

function provider(overrides: Partial<ProviderProfile> = {}): ProviderProfile {
  return {
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
    ...overrides,
  }
}

function controller() {
  const providers = ref([provider()])
  const selectedProviderId = shallowRef<string | null>('provider-a')
  const selectedProvider = ref<ProviderProfile | null>(providers.value[0] ?? null)
  const successMessage = shallowRef<string | null>(null)
  const error = shallowRef<{ code: string; message: string } | null>(null)
  const create = vi.fn().mockImplementation(async () => {
    successMessage.value = 'Provider 已保存。'
    return { providers: providers.value, message: successMessage.value }
  })
  const update = vi.fn().mockImplementation(async () => {
    successMessage.value = 'Provider 已更新。'
  })
  const remove = vi.fn().mockImplementation(async () => {
    providers.value = []
    successMessage.value = 'Provider 已删除。'
  })
  const switchTo = vi.fn().mockImplementation(async () => {
    successMessage.value = '已切换到「Provider A」。请重启 Codex 后生效。'
  })
  return {
    providers,
    fingerprints: shallowRef(fingerprints),
    modelCatalog: ref([...modelCatalog]),
    currentAuthImportAvailable: shallowRef(false),
    selectedProviderId,
    selectedProvider,
    activeProvider: ref<ProviderProfile | null>(null),
    loading: shallowRef(false),
    busy: shallowRef(false),
    error,
    successMessage,
    refresh: vi.fn(),
    create,
    update,
    remove,
    switchTo,
    importCurrentKey: vi.fn(),
    updatePreference: vi.fn(),
    selectProvider: vi.fn((id: string) => {
      selectedProviderId.value = id
      selectedProvider.value = providers.value.find((item) => item.id === id) ?? null
    }),
  }
}

function availabilityController() {
  const results = shallowRef<Record<string, Partial<Record<ProviderTestKind, ProviderAvailabilityResult>>>>({})
  const busy = shallowRef(false)
  const runningKind = shallowRef<ProviderTestKind | null>(null)
  const runningProviderId = shallowRef<string | null>(null)
  const cancelling = shallowRef(false)
  const error = shallowRef<{ code: string; message: string } | null>(null)
  return {
    results,
    busy,
    runningKind,
    runningProviderId,
    cancelling,
    error,
    resultFor: vi.fn((providerId: string, kind: ProviderTestKind) => results.value[providerId]?.[kind] ?? null),
    testApi: vi.fn(),
    testCodex: vi.fn(),
    cancel: vi.fn(),
    invalidateAll: vi.fn(),
  }
}

describe('ProvidersView', () => {
  beforeEach(() => {
    mockUseProviders.mockReset()
    mockUseProviderAvailability.mockReset()
    mockUseProviderAvailability.mockReturnValue(availabilityController())
  })

  it('submits create and edit flows through the composable', async () => {
    const state = controller()
    mockUseProviders.mockReturnValue(state)
    const wrapper = mount(ProvidersView)

    await wrapper.get('[aria-label="新增 Provider"]').trigger('click')
    await wrapper.get('[name="provider-id"]').setValue('provider-b')
    await wrapper.get('[name="provider-name"]').setValue('Provider B')
    await wrapper.get('[name="base-url"]').setValue('https://provider-b.example.test/v1')
    wrapper.getComponent({ name: 'ElSelect' }).vm.$emit('update:modelValue', ['gpt-5.6-sol'])
    await nextTick()
    await wrapper.get('#provider-api-key').setValue('test-key-not-real')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(state.create).toHaveBeenCalledOnce()
    expect(wrapper.emitted('providerCreated')).toHaveLength(1)

    await wrapper.get('[aria-label="编辑 Provider A"]').trigger('click')
    await wrapper.get('[name="provider-name"]').setValue('Provider A Updated')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(state.update).toHaveBeenCalledOnce()
  })

  it('opens the create editor when requested by onboarding', () => {
    const state = controller()
    mockUseProviders.mockReturnValue(state)

    const wrapper = mount(ProvidersView, { props: { startCreating: true } })

    expect(wrapper.find('[aria-label="新增 Provider"]').exists()).toBe(true)
    expect(wrapper.find('[name="provider-id"]').exists()).toBe(true)
  })

  it('reports onboarding create cancellation without completing setup', async () => {
    const state = controller()
    mockUseProviders.mockReturnValue(state)
    const wrapper = mount(ProvidersView, { props: { startCreating: true } })

    const cancelButton = wrapper.findAll('button').find((button) => button.text() === '取消')
    expect(cancelButton).toBeDefined()
    await cancelButton?.trigger('click')

    expect(wrapper.emitted('createCancelled')).toHaveLength(1)
  })

  it('requires delete confirmation before refreshing the list', async () => {
    const state = controller()
    mockUseProviders.mockReturnValue(state)
    const wrapper = mount(ProvidersView, { attachTo: document.body })

    await wrapper.get('[aria-label="删除 Provider A"]').trigger('click')
    expect(state.remove).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('确认删除')
    expect(wrapper.text()).toContain('config.toml')
    expect(wrapper.text()).toContain('providers.json')

    await wrapper.get('[aria-label="确认操作"]').trigger('click')
    await flushPromises()
    expect(state.remove).toHaveBeenCalledWith('provider-a')
  })

  it('shows the selected Provider details and actions in the right pane', () => {
    const state = controller()
    mockUseProviders.mockReturnValue(state)
    const wrapper = mount(ProvidersView)
    const detail = wrapper.get('[aria-label="所选 Provider 详情"]')

    expect(detail.text()).toContain('Provider A')
    expect(detail.text()).toContain('provider-a')
    expect(detail.text()).toContain('https://provider-a.example.test/v1')
    expect(detail.text()).toContain('responses')
    expect(detail.text()).toContain('gpt-5.6-sol')
    expect(detail.text()).toContain('密钥已配置')
    expect(detail.find('[aria-label="编辑所选 Provider"]').exists()).toBe(true)
  })

  it('confirms before importing the current auth.json key', async () => {
    const state = controller()
    const active = provider({ isActive: true, apiKeyConfigured: false })
    state.providers.value = [active]
    state.activeProvider.value = active
    state.selectedProvider.value = active
    state.currentAuthImportAvailable.value = true
    mockUseProviders.mockReturnValue(state)
    const wrapper = mount(ProvidersView, { attachTo: document.body })

    await wrapper.get('[aria-label="导入当前 auth.json 密钥"]').trigger('click')
    expect(state.importCurrentKey).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('确认导入')
    await wrapper.get('[aria-label="确认操作"]').trigger('click')
    await flushPromises()

    expect(state.importCurrentKey).toHaveBeenCalledWith('provider-a')
  })

  it('shows switch success, failure, and external conflict messages', async () => {
    const state = controller()
    mockUseProviders.mockReturnValue(state)
    const wrapper = mount(ProvidersView)

    await wrapper.get('[aria-label="使用 Provider A"]').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('请重启 Codex 后生效')

    state.successMessage.value = null
    state.error.value = { code: 'SWITCH_FAILED', message: '切换失败。' }
    await nextTick()
    expect(wrapper.text()).toContain('切换失败。')

    state.error.value = { code: 'EXTERNAL_MODIFICATION', message: '配置已被外部修改，请重新加载。' }
    await nextTick()
    expect(wrapper.text()).toContain('配置已被外部修改，请重新加载。')
  })

  it('starts API testing directly but requires confirmation before starting Codex testing', async () => {
    const state = controller()
    const availability = availabilityController()
    mockUseProviders.mockReturnValue(state)
    mockUseProviderAvailability.mockReturnValue(availability)
    const wrapper = mount(ProvidersView, { attachTo: document.body })

    await wrapper.get('[aria-label="测试 Provider A 的 API 可用性"]').trigger('click')
    expect(availability.testApi).toHaveBeenCalledWith('provider-a')

    await wrapper.get('[aria-label="运行 Provider A 的 Codex 兼容性测试"]').trigger('click')
    expect(availability.testCodex).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('启动本机 Codex')
    expect(wrapper.text()).toContain('token 消耗')

    await wrapper.get('[aria-label="确认操作"]').trigger('click')
    expect(availability.testCodex).toHaveBeenCalledWith('provider-a')
  })

  it('shares the availability busy state with Provider mutations while keeping selection available', () => {
    const state = controller()
    const availability = availabilityController()
    availability.busy.value = true
    availability.runningKind.value = 'api'
    availability.runningProviderId.value = 'provider-a'
    mockUseProviders.mockReturnValue(state)
    mockUseProviderAvailability.mockReturnValue(availability)
    const wrapper = mount(ProvidersView)

    expect(wrapper.get('[aria-label="编辑所选 Provider"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[aria-label="编辑 Provider A"]').attributes('disabled')).toBeDefined()
    expect(wrapper.find('[aria-label="取消 Provider A 的 API 可用性测试"]').exists()).toBe(true)
    expect(wrapper.get('[aria-label="选择 Provider A"]').attributes('disabled')).toBeUndefined()
  })

  it('invalidates session test results when the Provider fingerprint changes', async () => {
    const state = controller()
    const availability = availabilityController()
    mockUseProviders.mockReturnValue(state)
    mockUseProviderAvailability.mockReturnValue(availability)
    mount(ProvidersView)

    state.fingerprints.value = {
      ...fingerprints,
      config: { ...fingerprints.config, sha256: 'changed' },
    }
    await nextTick()

    expect(availability.invalidateAll).toHaveBeenCalledOnce()
  })
})
