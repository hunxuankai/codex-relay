import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it } from 'vitest'
import type { FileSetFingerprint, ProviderProfile } from '../types/provider'
import ProviderEditor from './ProviderEditor.vue'

const fingerprints: FileSetFingerprint = {
  config: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'config' },
  auth: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'auth' },
  providers: { exists: true, len: 1, modifiedUnixMillis: 1, sha256: 'providers' },
}

const existing: ProviderProfile = {
  id: 'provider-a',
  name: 'Provider A',
  baseUrl: 'https://provider-a.example.test/v1',
  wireApi: 'responses',
  model: 'model-a',
  apiKeyConfigured: true,
  isActive: true,
  isValid: true,
  validationMessage: null,
}

describe('ProviderEditor', () => {
  it('validates and normalizes a new Provider', async () => {
    const wrapper = mount(ProviderEditor, {
      attachTo: document.body,
      props: { mode: 'create', provider: null, fingerprints, busy: false },
    })

    await wrapper.get('form').trigger('submit')
    await nextTick()
    expect(wrapper.text()).toContain('Provider ID 为必填项')
    expect(wrapper.text()).toContain('名称为必填项')
    expect(wrapper.text()).toContain('Base URL 为必填项')
    expect(wrapper.text()).toContain('API Key 为必填项')
    expect(document.activeElement).toBe(wrapper.get('[name="provider-id"]').element)
    expect(wrapper.get('[name="provider-id"]').attributes('aria-invalid')).toBe('true')

    await wrapper.get('[name="provider-id"]').setValue('PROVIDER-A')
    expect((wrapper.get('[name="provider-id"]').element as HTMLInputElement).value).toBe('provider-a')
    await wrapper.get('[name="provider-name"]').setValue('  Provider A  ')
    await wrapper.get('[name="base-url"]').setValue('ftp://invalid.test')
    await wrapper.get('[name="model"]').setValue('  model-a  ')
    await wrapper.get('#provider-api-key').setValue('test-key-not-real')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.text()).toContain('Base URL 必须使用 HTTP 或 HTTPS')

    await wrapper.get('[name="base-url"]').setValue('https://provider-a.example.test/v1')
    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toEqual({
      id: 'provider-a',
      name: 'Provider A',
      baseUrl: 'https://provider-a.example.test/v1',
      wireApi: 'responses',
      model: 'model-a',
      apiKey: 'test-key-not-real',
      activateAfterSave: false,
      expectedFiles: fingerprints,
    })
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
      },
    })
    await wrapper.get('[name="provider-id"]').setValue('provider-a')
    await wrapper.get('[name="provider-name"]').setValue('Provider A')
    await wrapper.get('[name="base-url"]').setValue('https://provider-a.example.test/v1')
    await wrapper.get('#provider-api-key').setValue('test-key-not-real')

    await wrapper.get('form').trigger('submit')

    expect(wrapper.text()).toContain('Provider ID 已存在')
    expect(wrapper.emitted('submit')).toBeUndefined()
  })

  it('keeps ID immutable and omits an untouched key during edit', async () => {
    const wrapper = mount(ProviderEditor, {
      props: { mode: 'edit', provider: existing, fingerprints, busy: false },
    })

    expect(wrapper.get('[name="provider-id"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[name="wire-api"]').attributes('disabled')).toBeDefined()
    expect((wrapper.get('[name="wire-api"]').element as HTMLInputElement).value).toBe('responses')
    expect(wrapper.text()).toContain('当前 Provider')
    expect(wrapper.find('[name="sync-if-active"]').exists()).toBe(false)
    await wrapper.get('[name="model"]').setValue('   ')
    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toEqual({
      id: 'provider-a',
      name: 'Provider A',
      baseUrl: 'https://provider-a.example.test/v1',
      wireApi: 'responses',
      model: null,
      apiKeyChange: { action: 'unchanged' },
      syncIfActive: false,
      expectedFiles: fingerprints,
    })
  })

  it('emits explicit key clearing without trying to sync a missing key', async () => {
    const wrapper = mount(ProviderEditor, {
      props: { mode: 'edit', provider: existing, fingerprints, busy: false },
    })

    await wrapper.get('[aria-label="清空 API Key"]').trigger('click')
    await wrapper.get('[aria-label="确认清空 API Key"]').trigger('click')
    expect(wrapper.find('[name="sync-if-active"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('清除当前 Provider 的 API Key 后无法立即同步')
    expect(wrapper.text()).toContain('当前 auth.json 不会被改写，Codex 可能继续使用现有生效密钥')
    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toMatchObject({
      apiKeyChange: { action: 'clear' },
      syncIfActive: false,
    })
  })

  it('offers immediate sync only after active fields change', async () => {
    const wrapper = mount(ProviderEditor, {
      props: { mode: 'edit', provider: existing, fingerprints, busy: false },
    })

    await wrapper.get('[name="provider-name"]').setValue('Provider A Updated')
    expect(wrapper.find('[name="sync-if-active"]').exists()).toBe(true)
    await wrapper.get('[name="sync-if-active"]').setValue(true)
    await wrapper.get('form').trigger('submit')

    expect(wrapper.emitted('submit')?.[0]?.[0]).toMatchObject({
      name: 'Provider A Updated',
      apiKeyChange: { action: 'unchanged' },
      syncIfActive: true,
    })
  })
})
