import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ProxySettingsPanel from './ProxySettingsPanel.vue'

describe('ProxySettingsPanel', () => {
  it('edits proxy settings and exposes test or discovery actions at the right time', async () => {
    const wrapper = mount(ProxySettingsPanel, {
      props: {
        modelValue: { enabled: false, url: '' },
        busy: false,
        testing: false,
        discovering: false,
      },
    })

    expect(wrapper.get('[data-action="discover-proxy"]').isVisible()).toBe(true)
    expect(wrapper.find('[data-action="test-proxy"]').exists()).toBe(false)

    await wrapper.get('[name="proxy-url"]').setValue('http://127.0.0.1:7897')
    const updates = wrapper.emitted('update:modelValue') ?? []
    expect(updates[updates.length - 1]).toEqual([
      { enabled: false, url: 'http://127.0.0.1:7897' },
    ])

    await wrapper.setProps({ modelValue: { enabled: true, url: 'http://127.0.0.1:7897' } })
    expect(wrapper.find('[data-action="discover-proxy"]').exists()).toBe(false)
    await wrapper.get('[data-action="test-proxy"]').trigger('click')
    expect(wrapper.emitted('test')).toEqual([[]])
  })
})
