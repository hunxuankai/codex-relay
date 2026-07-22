import { flushPromises, mount } from '@vue/test-utils'
import { ElDialog, ElRadioGroup } from 'element-plus'
import { describe, expect, it } from 'vitest'
import ProxyDiscoveryDialog from './ProxyDiscoveryDialog.vue'

describe('ProxyDiscoveryDialog', () => {
  it('lists every candidate and confirms the selected proxy', async () => {
    const wrapper = mount(ProxyDiscoveryDialog, {
      attachTo: document.body,
      props: {
        open: true,
        candidates: ['http://127.0.0.1:7890', 'http://127.0.0.1:7897'],
        selected: 'http://127.0.0.1:7890',
      },
    })
    await flushPromises()

    expect(wrapper.findComponent(ElDialog).exists()).toBe(true)
    const radios = Array.from(document.body.querySelectorAll<HTMLInputElement>('input[type="radio"]'))
    expect(radios).toHaveLength(2)
    wrapper.getComponent(ElRadioGroup).vm.$emit('change', 'http://127.0.0.1:7897')
    expect(wrapper.emitted('select')).toEqual([['http://127.0.0.1:7897']])
    document.body.querySelector<HTMLButtonElement>('[data-action="apply-proxy"]')?.click()
    expect(wrapper.emitted('confirm')).toEqual([[]])
  })

  it('shows an explicit empty result without an apply action', async () => {
    const wrapper = mount(ProxyDiscoveryDialog, {
      attachTo: document.body,
      props: { open: true, candidates: [], selected: null },
    })
    await flushPromises()

    expect(wrapper.findComponent(ElDialog).exists()).toBe(true)
    expect(document.body.textContent).toContain('未检测到可用于访问更新源的本机代理')
    expect(document.body.querySelector('[data-action="apply-proxy"]')).toBeNull()
  })
})
