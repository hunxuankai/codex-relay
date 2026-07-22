import { mount } from '@vue/test-utils'
import { ElAlert, ElButton } from 'element-plus'
import { describe, expect, it } from 'vitest'
import SelfCheckErrorBanner from './SelfCheckErrorBanner.vue'

describe('SelfCheckErrorBanner', () => {
  it('uses consistent Element Plus feedback and keeps alert semantics', async () => {
    const wrapper = mount(SelfCheckErrorBanner, { props: { errorCount: 2 } })

    expect(wrapper.get('[aria-label="系统自检错误提示"]').attributes('role')).toBe('alert')
    expect(wrapper.findComponent(ElAlert).exists()).toBe(true)
    expect(wrapper.findComponent(ElButton).exists()).toBe(true)
    await wrapper.get('[aria-label="查看自检详情"]').trigger('click')
    expect(wrapper.emitted('viewDetails')).toHaveLength(1)
  })
})
