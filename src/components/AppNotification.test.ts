import { mount } from '@vue/test-utils'
import { ElAlert } from 'element-plus'
import { nextTick } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'
import AppNotification from './AppNotification.vue'

afterEach(() => {
  vi.clearAllTimers()
  vi.useRealTimers()
})

describe('AppNotification', () => {
  it('uses an Element Plus alert while preserving live-region semantics', () => {
    const success = mount(AppNotification, {
      props: { message: '保存成功。', level: 'success' },
    })
    const error = mount(AppNotification, {
      props: { message: '保存失败。', level: 'error' },
    })

    expect(success.getComponent(ElAlert).props()).toMatchObject({
      type: 'success',
      closable: false,
    })
    expect(success.get('[role="status"]').text()).toContain('保存成功。')
    expect(error.get('[role="alert"]').text()).toContain('保存失败。')

    success.unmount()
    error.unmount()
  })

  it('automatically dismisses success feedback after five seconds while keeping errors visible', async () => {
    vi.useFakeTimers()
    const success = mount(AppNotification, {
      props: { message: '保存成功。', level: 'success' },
    })
    const error = mount(AppNotification, {
      props: { message: '保存失败。', level: 'error' },
    })

    await vi.advanceTimersByTimeAsync(5_000)
    await nextTick()

    expect(success.find('[role="status"]').exists()).toBe(false)
    expect(error.find('[role="alert"]').exists()).toBe(true)

    success.unmount()
    error.unmount()
  })

  it('restarts the dismiss timer for a new notification identifier', async () => {
    vi.useFakeTimers()
    const wrapper = mount(AppNotification, {
      props: { message: '保存成功。', level: 'success', messageId: 0 },
    })

    await vi.advanceTimersByTimeAsync(4_000)
    await wrapper.setProps({ messageId: 1 })
    await vi.advanceTimersByTimeAsync(1_000)
    await nextTick()

    expect(wrapper.find('[role="status"]').exists()).toBe(true)

    await vi.advanceTimersByTimeAsync(4_000)
    await nextTick()

    expect(wrapper.find('[role="status"]').exists()).toBe(false)
    wrapper.unmount()
  })
})
