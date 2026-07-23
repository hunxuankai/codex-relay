import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ProviderBaseUrlSummary } from '../types/provider'
import ProviderBaseUrlManagerDialog from './ProviderBaseUrlManagerDialog.vue'

const entries: ProviderBaseUrlSummary[] = [
  { id: 'url-primary', name: '主用地址', url: 'https://primary.example.test/v1' },
  { id: 'url-backup', name: '备用地址', url: 'https://backup.example.test/v1' },
]

function mountDialog(overrides: Record<string, unknown> = {}) {
  return mount(ProviderBaseUrlManagerDialog, {
    props: {
      open: true,
      providerName: 'Provider A',
      entries,
      selectedBaseUrlId: 'url-primary',
      externalUrl: null,
      busy: false,
      ...overrides,
    },
    global: { stubs: { teleport: true } },
  })
}

describe('ProviderBaseUrlManagerDialog', () => {
  it('keeps ordered stable IDs and appends a new URL in one save', async () => {
    const wrapper = mountDialog()
    await flushPromises()

    await wrapper.get('[aria-label="新增 Base URL"]').trigger('click')
    const nameInputs = wrapper.findAll('[name^="base-url-name-"]')
    const urlInputs = wrapper.findAll('[name^="base-url-value-"]')
    await nameInputs[2]!.setValue('  灾备地址  ')
    await urlInputs[2]!.setValue('https://disaster.example.test/v1')
    await wrapper.get('[aria-label="保存 Base URL 列表"]').trigger('click')

    expect(wrapper.emitted('save')?.[0]?.[0]).toEqual([
      { id: 'url-primary', name: '主用地址', url: 'https://primary.example.test/v1' },
      { id: 'url-backup', name: '备用地址', url: 'https://backup.example.test/v1' },
      { id: null, name: '灾备地址', url: 'https://disaster.example.test/v1' },
    ])
  })

  it('rejects duplicates and prevents deleting the current or final URL', async () => {
    const wrapper = mountDialog()
    await flushPromises()

    expect(wrapper.get('[aria-label="删除地址 主用地址"]').attributes('disabled')).toBeDefined()
    await wrapper.get('[name="base-url-name-1"]').setValue('主用地址')
    await wrapper.get('[name="base-url-value-1"]').setValue('https://primary.example.test/v1')
    await wrapper.get('[aria-label="保存 Base URL 列表"]').trigger('click')

    expect(wrapper.text()).toContain('地址名称不能重复')
    expect(wrapper.text()).toContain('Base URL 不能重复')
    expect(wrapper.emitted('save')).toBeUndefined()

    const single = mountDialog({
      entries: [entries[0]],
      selectedBaseUrlId: null,
    })
    await flushPromises()
    expect(single.get('[aria-label="删除地址 主用地址"]').attributes('disabled')).toBeDefined()
  })

  it('can append an unmatched external URL only after the user names it', async () => {
    const wrapper = mountDialog({ externalUrl: 'https://external.example.test/v1' })
    await flushPromises()

    expect(wrapper.text()).toContain('当前外部地址')
    await wrapper.get('[aria-label="保存当前外部地址为命名项"]').trigger('click')
    const nameInputs = wrapper.findAll('[name^="base-url-name-"]')
    const urlInputs = wrapper.findAll('[name^="base-url-value-"]')
    expect((urlInputs[2]!.element as HTMLInputElement).value).toBe('https://external.example.test/v1')
    await nameInputs[2]!.setValue('外部地址')
    await wrapper.get('[aria-label="保存 Base URL 列表"]').trigger('click')

    expect(wrapper.emitted('save')?.[0]?.[0]).toMatchObject([
      {},
      {},
      { id: null, name: '外部地址', url: 'https://external.example.test/v1' },
    ])
  })
})
