import { mount } from '@vue/test-utils'
import { ElAlert, ElButton } from 'element-plus'
import { describe, expect, it } from 'vitest'
import UpdateAvailableBanner from './UpdateAvailableBanner.vue'

describe('UpdateAvailableBanner', () => {
  it('announces the available version and emits the update navigation action', async () => {
    const wrapper = mount(UpdateAvailableBanner, { props: { version: '0.2.0' } })

    expect(wrapper.get('[aria-label="软件更新提示"]').attributes('role')).toBe('status')
    expect(wrapper.findComponent(ElAlert).exists()).toBe(true)
    expect(wrapper.findComponent(ElButton).exists()).toBe(true)
    expect(wrapper.text()).toContain('发现新版本 0.2.0')

    await wrapper.get('[aria-label="前往软件更新设置"]').trigger('click')

    expect(wrapper.emitted('viewUpdate')).toHaveLength(1)
  })
})
