import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it } from 'vitest'
import ConfirmDialog from './ConfirmDialog.vue'

describe('ConfirmDialog', () => {
  it('keeps danger as the default and supports a neutral confirmation style', () => {
    const danger = mount(ConfirmDialog, {
      props: { open: true, title: '确认删除', message: '不可撤销。' },
    })
    const neutral = mount(ConfirmDialog, {
      props: { open: true, title: '安装更新', message: '应用将退出。', tone: 'neutral' },
    })

    expect(danger.get('[aria-label="确认操作"]').classes()).toContain('danger-button')
    expect(neutral.get('[aria-label="确认操作"]').classes()).toContain('primary-button')
  })

  it('traps focus while open and restores the previous focus', async () => {
    const opener = document.createElement('button')
    document.body.append(opener)
    opener.focus()
    const wrapper = mount(ConfirmDialog, {
      attachTo: document.body,
      props: {
        open: true,
        title: '确认删除',
        message: '此操作不可撤销。',
        confirmLabel: '删除',
      },
    })
    await nextTick()
    const cancel = wrapper.get('[aria-label="取消确认"]')
    const confirm = wrapper.get('[aria-label="确认操作"]')

    expect(document.activeElement).toBe(cancel.element)
    ;(confirm.element as HTMLButtonElement).focus()
    await confirm.trigger('keydown', { key: 'Tab' })
    expect(document.activeElement).toBe(cancel.element)

    await cancel.trigger('click')
    await wrapper.setProps({ open: false })
    await nextTick()
    expect(document.activeElement).toBe(opener)
  })
})
