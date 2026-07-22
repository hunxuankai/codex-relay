import { mount } from '@vue/test-utils'
import { ElAlert } from 'element-plus'
import { describe, expect, it } from 'vitest'
import AppNotification from './AppNotification.vue'

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
  })
})
