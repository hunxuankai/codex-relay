import { mount } from '@vue/test-utils'
import { ElSelect, ElSwitch } from 'element-plus'
import { nextTick } from 'vue'
import { describe, expect, it } from 'vitest'
import type { CreateProviderInput, FileSetFingerprint, ProviderProfile } from '../types/provider'
import ProviderEditor from './ProviderEditor.vue'

const fingerprints: FileSetFingerprint = {
  config: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'config' },
  auth: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'auth' },
  providers: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'providers' },
  preferences: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'preferences' },
}

const modelCatalog = [
  { id: 'gpt-5.6-sol', reasoningEfforts: ['none', 'low', 'medium', 'high', 'xhigh', 'max'], defaultReasoningEffort: 'medium', supportsFast: true },
  { id: 'gpt-5.4-mini', reasoningEfforts: ['none', 'low', 'medium', 'high', 'xhigh'], defaultReasoningEffort: 'none', supportsFast: false },
] as const

const existing: ProviderProfile = {
  id: 'provider-a',
  name: 'Provider A',
  baseUrl: 'https://provider-a.example.test/v1',
  baseUrls: [
    {
      id: 'provider-a-url',
      name: '主用地址',
      url: 'https://provider-a.example.test/v1',
    },
  ],
  selectedBaseUrlId: 'provider-a-url',
  baseUrlStatus: 'managed',
  apiKeys: [{ id: 'provider-a-key', name: '主用密钥' }],
  selectedApiKeyId: 'provider-a-key',
  apiKeyStatus: 'managed',
  wireApi: 'responses',
  models: ['gpt-5.6-sol', 'gpt-5.4-mini'],
  selectedModel: 'gpt-5.6-sol',
  reasoningEfforts: { 'gpt-5.6-sol': 'high', 'gpt-5.4-mini': 'none' },
  fastEnabled: false,
  preferenceConfigured: true,
  apiKeyConfigured: true,
  configurationComplete: true,
  disabledReason: null,
  isActive: true,
  isValid: true,
  validationMessage: null,
}

describe('ProviderEditor', () => {
  it('validates and normalizes a new Provider', async () => {
    const wrapper = mount(ProviderEditor, {
      attachTo: document.body,
      props: { mode: 'create', provider: null, fingerprints, busy: false, modelCatalog },
    })

    expect(wrapper.getComponent(ElSwitch).props('modelValue')).toBe(false)

    await wrapper.get('form').trigger('submit')
    await nextTick()
    expect(wrapper.text()).toContain('Provider ID 为必填项')
    expect(wrapper.text()).toContain('名称为必填项')
    expect(wrapper.text()).toContain('地址名称为必填项')
    expect(wrapper.text()).toContain('Base URL 为必填项')
    expect(wrapper.text()).toContain('密钥名称为必填项')
    expect(wrapper.text()).toContain('API Key 为必填项')
    expect(document.activeElement).toBe(wrapper.get('[name="provider-id"]').element)
    expect(wrapper.get('[name="provider-id"]').attributes('aria-invalid')).toBe('true')

    await wrapper.get('[name="provider-id"]').setValue('PROVIDER-A')
    expect((wrapper.get('[name="provider-id"]').element as HTMLInputElement).value).toBe('provider-a')
    await wrapper.get('[name="provider-name"]').setValue('  Provider A  ')
    await wrapper.get('[name="base-url-name"]').setValue('  主用地址  ')
    await wrapper.get('[name="base-url"]').setValue('ftp://invalid.test')
    wrapper.getComponent(ElSelect).vm.$emit('update:modelValue', ['gpt-5.6-sol', 'gpt-5.4-mini'])
    await nextTick()
    await wrapper.get('[name="api-key-name"]').setValue('  主用密钥  ')
    await wrapper.get('#provider-api-key').setValue('test-key-provider-not-real')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.text()).toContain('Base URL 必须使用 HTTP 或 HTTPS')

    await wrapper.get('[name="base-url"]').setValue('https://provider-a.example.test/v1')
    await wrapper.get('form').trigger('submit')

    const submitted = wrapper.emitted('submit')?.[0]?.[0] as CreateProviderInput
    expect({ ...submitted, apiKey: '<redacted>' }).toEqual({
      id: 'provider-a',
      name: 'Provider A',
      baseUrlName: '主用地址',
      baseUrl: 'https://provider-a.example.test/v1',
      wireApi: 'responses',
      models: ['gpt-5.6-sol', 'gpt-5.4-mini'],
      apiKeyName: '主用密钥',
      apiKey: '<redacted>',
      fastEnabled: false,
      activateAfterSave: false,
      expectedFiles: fingerprints,
    })
    expect(submitted.apiKey === 'test-key-provider-not-real').toBe(true)
  })

  it('rejects a duplicate Provider ID before submission', async () => {
    const wrapper = mount(ProviderEditor, {
      attachTo: document.body,
      props: {
        mode: 'create',
        provider: null,
        fingerprints,
        busy: false,
        existingIds: ['provider-a'],
        modelCatalog,
      },
    })
    await wrapper.get('[name="provider-id"]').setValue('provider-a')
    await wrapper.get('[name="provider-name"]').setValue('Provider A')
    await wrapper.get('[name="base-url-name"]').setValue('主用地址')
    await wrapper.get('[name="base-url"]').setValue('https://provider-a.example.test/v1')
    wrapper.getComponent(ElSelect).vm.$emit('update:modelValue', ['gpt-5.6-sol'])
    await nextTick()
    await wrapper.get('[name="api-key-name"]').setValue('主用密钥')
    await wrapper.get('#provider-api-key').setValue('test-key-provider-not-real')

    await wrapper.get('form').trigger('submit')

    expect(wrapper.text()).toContain('Provider ID 已存在')
    expect(wrapper.emitted('submit')).toBeUndefined()
  })

  it('keeps ID immutable and excludes URL and API Key management during edit', async () => {
    const wrapper = mount(ProviderEditor, {
      props: { mode: 'edit', provider: existing, fingerprints, busy: false, modelCatalog },
    })

    expect(wrapper.get('[name="provider-id"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[name="wire-api"]').attributes('disabled')).toBeDefined()
    expect((wrapper.get('[name="wire-api"]').element as HTMLInputElement).value).toBe('responses')
    expect(wrapper.find('[name="base-url-name"]').exists()).toBe(false)
    expect(wrapper.find('[name="base-url"]').exists()).toBe(false)
    expect(wrapper.find('[name="api-key-name"]').exists()).toBe(false)
    expect(wrapper.find('#provider-api-key').exists()).toBe(false)
    expect(wrapper.getComponent(ElSelect).props('modelValue')).toEqual(['gpt-5.6-sol', 'gpt-5.4-mini'])
    expect(wrapper.text()).toContain('当前偏好：gpt-5.6-sol')
    expect(wrapper.text()).toContain('当前 Provider')
    expect(wrapper.find('[name="sync-if-active"]').exists()).toBe(false)
    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toEqual({
      id: 'provider-a',
      name: 'Provider A',
      wireApi: 'responses',
      models: ['gpt-5.6-sol', 'gpt-5.4-mini'],
      fastEnabled: false,
      syncIfActive: false,
      expectedFiles: fingerprints,
    })
  })

  it('restores and submits the saved Fast preference during edit', async () => {
    const wrapper = mount(ProviderEditor, {
      props: {
        mode: 'edit',
        provider: { ...existing, fastEnabled: true },
        fingerprints,
        busy: false,
        modelCatalog,
      },
    })

    expect(wrapper.getComponent(ElSwitch).props('modelValue')).toBe(true)

    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toMatchObject({
      id: 'provider-a',
      fastEnabled: true,
    })
  })

  it('uses the actual saved preference model to determine Fast support', () => {
    const wrapper = mount(ProviderEditor, {
      props: {
        mode: 'edit',
        provider: { ...existing, selectedModel: 'gpt-5.4-mini' },
        fingerprints,
        busy: false,
        modelCatalog,
      },
    })
    const fastSwitch = wrapper.getComponent(ElSwitch)

    expect(fastSwitch.props('modelValue')).toBe(false)
    expect(fastSwitch.props('disabled')).toBe(true)
    expect(wrapper.get('#provider-editor-fast-description').text()).toContain(
      'gpt-5.4-mini 不支持 Fast',
    )
  })

  it('turns Fast off when the edited model set falls back to an unsupported model', async () => {
    const wrapper = mount(ProviderEditor, {
      props: {
        mode: 'edit',
        provider: { ...existing, fastEnabled: true },
        fingerprints,
        busy: false,
        modelCatalog,
      },
    })

    wrapper.getComponent(ElSelect).vm.$emit('update:modelValue', ['gpt-5.4-mini'])
    await nextTick()

    const fastSwitch = wrapper.getComponent(ElSwitch)
    expect(fastSwitch.props('modelValue')).toBe(false)
    expect(fastSwitch.props('disabled')).toBe(true)

    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toMatchObject({
      models: ['gpt-5.4-mini'],
      fastEnabled: false,
    })
  })

  it('offers immediate sync only after active fields change', async () => {
    const wrapper = mount(ProviderEditor, {
      props: { mode: 'edit', provider: existing, fingerprints, busy: false, modelCatalog },
    })

    await wrapper.get('[name="provider-name"]').setValue('Provider A Updated')
    expect(wrapper.find('[name="sync-if-active"]').exists()).toBe(true)
    await wrapper.get('[name="sync-if-active"]').setValue(true)
    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toMatchObject({
      name: 'Provider A Updated',
      syncIfActive: true,
    })
  })

  it('offers immediate sync when only Fast changes on the active Provider', async () => {
    const wrapper = mount(ProviderEditor, {
      props: { mode: 'edit', provider: existing, fingerprints, busy: false, modelCatalog },
    })

    wrapper.getComponent(ElSwitch).vm.$emit('update:modelValue', true)
    await nextTick()

    expect(wrapper.find('[name="sync-if-active"]').exists()).toBe(true)
    await wrapper.get('[name="sync-if-active"]').setValue(true)
    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toMatchObject({
      fastEnabled: true,
      syncIfActive: true,
    })
  })
})
