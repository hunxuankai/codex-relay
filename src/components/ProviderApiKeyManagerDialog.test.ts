import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ProviderApiKeyDraft } from '../types/provider'
import ProviderApiKeyManagerDialog from './ProviderApiKeyManagerDialog.vue'

const entries: ProviderApiKeyDraft[] = [
  { id: 'key-primary', name: '主用密钥', apiKey: 'test-key-primary-not-real' },
  { id: 'key-backup', name: '备用密钥', apiKey: 'test-key-backup-not-real' },
]

function mountDialog(overrides: Record<string, unknown> = {}) {
  return mount(ProviderApiKeyManagerDialog, {
    props: {
      open: true,
      providerName: 'Provider A',
      entries,
      selectedApiKeyId: 'key-primary',
      apiKeyStatus: 'managed',
      loading: false,
      busy: false,
      errorMessage: null,
      ...overrides,
    },
    global: { stubs: { teleport: true } },
  })
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('ProviderApiKeyManagerDialog', () => {
  it('opens in plaintext, hides or shows all, and copies one key with safe feedback', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })
    const wrapper = mountDialog()
    await flushPromises()

    const keyInputs = wrapper.findAll('[name^="api-key-value-"]')
    expect(keyInputs.map((input) => input.attributes('type'))).toEqual(['text', 'text'])
    expect((keyInputs[0]!.element as HTMLInputElement).value === 'test-key-primary-not-real').toBe(true)

    await wrapper.get('[aria-label="隐藏全部 API Key"]').trigger('click')
    expect(wrapper.findAll('[name^="api-key-value-"]').map((input) => input.attributes('type')))
      .toEqual(['password', 'password'])
    await wrapper.get('[aria-label="显示全部 API Key"]').trigger('click')

    await wrapper.get('[aria-label="复制 主用密钥"]').trigger('click')
    await flushPromises()
    expect(writeText).toHaveBeenCalledOnce()
    expect(writeText.mock.calls[0]?.[0] === 'test-key-primary-not-real').toBe(true)
    expect(wrapper.text()).toContain('主用密钥已复制。')
    expect(wrapper.text()).not.toContain('test-key-primary-not-real')
  })

  it('emits complete ordered drafts while protecting the current and final key', async () => {
    const wrapper = mountDialog()
    await flushPromises()

    expect(wrapper.get('[aria-label="删除密钥 主用密钥"]').attributes('disabled')).toBeDefined()
    await wrapper.get('[aria-label="新增 API Key"]').trigger('click')
    const addedEvents = wrapper.emitted('replaceEntries') ?? []
    const added = addedEvents[addedEvents.length - 1]?.[0] as ProviderApiKeyDraft[]
    await wrapper.setProps({ entries: added })
    const nameInputs = wrapper.findAll('[name^="api-key-name-"]')
    await nameInputs[2]!.setValue('灾备密钥')
    const renamedEvents = wrapper.emitted('replaceEntries') ?? []
    const renamed = renamedEvents[renamedEvents.length - 1]?.[0] as ProviderApiKeyDraft[]
    await wrapper.setProps({ entries: renamed })
    const updatedKeyInputs = wrapper.findAll('[name^="api-key-value-"]')
    await updatedKeyInputs[2]!.setValue('test-key-disaster-not-real')

    const replacementEvents = wrapper.emitted('replaceEntries') ?? []
    const replacement = replacementEvents[replacementEvents.length - 1]?.[0] as ProviderApiKeyDraft[]
    await wrapper.setProps({ entries: replacement })
    expect(replacement).toHaveLength(3)
    expect(replacement[0]?.id).toBe('key-primary')
    expect(replacement[2]?.id).toBeNull()
    expect(replacement[2]?.apiKey === 'test-key-disaster-not-real').toBe(true)

    await wrapper.get('[aria-label="保存 API Key 列表"]').trigger('click')
    expect(wrapper.emitted('save')).toHaveLength(1)

    const single = mountDialog({ entries: [entries[0]], selectedApiKeyId: null })
    await flushPromises()
    expect(single.get('[aria-label="删除密钥 主用密钥"]').attributes('disabled')).toBeDefined()
  })

  it('requests close without returning any key in the close payload', async () => {
    const wrapper = mountDialog()
    await flushPromises()

    await wrapper.get('[aria-label="关闭 API Key 管理器"]').trigger('click')

    expect(wrapper.emitted('close')).toEqual([[]])
  })
})
