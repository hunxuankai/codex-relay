import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it } from 'vitest'
import ProviderConnectionRiskDialog from './ProviderConnectionRiskDialog.vue'

describe('ProviderConnectionRiskDialog', () => {
  it('presents a safe read-only explanation and exposes all supported close paths', async () => {
    const wrapper = mount(ProviderConnectionRiskDialog, {
      attachTo: document.body,
      props: { open: true },
    })
    await flushPromises()

    const dialog = wrapper.getComponent({ name: 'ElDialog' })
    expect(dialog.props('showClose')).toBe(false)
    expect(dialog.props('closeOnClickModal')).toBe(true)
    expect(dialog.props('closeOnPressEscape')).toBe(true)

    const text = document.body.textContent ?? ''
    expect(text).toContain('旧会话兼容性说明')
    expect(text).toContain('新连接不保证能识别这些不透明内容')
    expect(text).toContain('OpenAI-compatible API 不等于会话上下文兼容')
    expect(text).not.toContain('https://')
    expect(text).not.toContain('test-key')

    await wrapper.get('[aria-label="关闭旧会话兼容性说明"]').trigger('click')
    expect(wrapper.emitted('close')).toHaveLength(1)

    dialog.vm.$emit('update:modelValue', false)
    await nextTick()
    expect(wrapper.emitted('close')).toHaveLength(2)

    dialog.vm.$emit('closed')
    await nextTick()
    expect(wrapper.emitted('closed')).toHaveLength(1)
    wrapper.unmount()
  })

  it('focuses the acknowledgement action and restores the opener on close', async () => {
    const opener = document.createElement('button')
    document.body.append(opener)
    opener.focus()
    const wrapper = mount(ProviderConnectionRiskDialog, {
      attachTo: document.body,
      props: { open: true },
    })
    await flushPromises()
    await new Promise((resolve) => setTimeout(resolve, 0))

    const close = document.body.querySelector<HTMLButtonElement>(
      '[aria-label="关闭旧会话兼容性说明"]',
    )
    expect(close).not.toBeNull()
    expect(document.activeElement).toBe(close)

    await wrapper.setProps({ open: false })
    await nextTick()
    expect(document.activeElement).toBe(opener)
    wrapper.unmount()
    opener.remove()
  })
})
