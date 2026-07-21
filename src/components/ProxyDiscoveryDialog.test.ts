import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ProxyDiscoveryDialog from './ProxyDiscoveryDialog.vue'

describe('ProxyDiscoveryDialog', () => {
  it('lists every candidate and confirms the selected proxy', async () => {
    const wrapper = mount(ProxyDiscoveryDialog, {
      props: {
        open: true,
        candidates: ['http://127.0.0.1:7890', 'http://127.0.0.1:7897'],
        selected: 'http://127.0.0.1:7890',
      },
    })

    const radios = wrapper.findAll('input[type="radio"]')
    expect(radios).toHaveLength(2)
    await radios[1]?.setValue()
    expect(wrapper.emitted('select')).toEqual([['http://127.0.0.1:7897']])
    await wrapper.get('[data-action="apply-proxy"]').trigger('click')
    expect(wrapper.emitted('confirm')).toEqual([[]])
  })

  it('shows an explicit empty result without an apply action', () => {
    const wrapper = mount(ProxyDiscoveryDialog, {
      props: { open: true, candidates: [], selected: null },
    })

    expect(wrapper.text()).toContain('未检测到可用于访问更新源的本机代理')
    expect(wrapper.find('[data-action="apply-proxy"]').exists()).toBe(false)
  })
})
