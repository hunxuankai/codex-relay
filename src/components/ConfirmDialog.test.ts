import { flushPromises, mount } from '@vue/test-utils'
import { ElDialog } from 'element-plus'
import { nextTick } from 'vue'
import { describe, expect, it } from 'vitest'
import ConfirmDialog from './ConfirmDialog.vue'

describe('ConfirmDialog', () => {
  it('keeps danger as the default and supports a neutral confirmation style', async () => {
    const danger = mount(ConfirmDialog, {
      attachTo: document.body,
      props: { open: true, title: '确认删除', message: '不可撤销。' },
    })
    const neutral = mount(ConfirmDialog, {
      attachTo: document.body,
      props: { open: true, title: '安装更新', message: '应用将退出。', tone: 'neutral' },
    })
    await flushPromises()

    expect(document.body.querySelector('.danger-button')).not.toBeNull()
    expect(document.body.querySelector('.primary-button')).not.toBeNull()
    expect(danger.getComponent(ElDialog).props('closeOnClickModal')).toBe(false)
    expect(neutral.getComponent(ElDialog).props('closeOnPressEscape')).toBe(true)
  })

  it('starts on the safe action and restores the previous focus', async () => {
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
    await flushPromises()
    await nextTick()
    await new Promise((resolve) => setTimeout(resolve, 0))
    const cancel = document.body.querySelector<HTMLButtonElement>('[aria-label="取消确认"]')

    expect(cancel).not.toBeNull()
    expect(document.activeElement).toBe(cancel)

    cancel?.click()
    await wrapper.setProps({ open: false })
    await nextTick()
    expect(document.activeElement).toBe(opener)
  })
})
