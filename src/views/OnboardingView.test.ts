import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import OnboardingView from './OnboardingView.vue'

describe('OnboardingView', () => {
  it('offers exactly the required first-run actions', async () => {
    const wrapper = mount(OnboardingView, {
      props: {
        busy: false,
        currentProviderName: null,
        canImportCurrentKey: false,
        successMessage: null,
        errorMessage: null,
      },
    })

    const actions = wrapper.findAll('[data-onboarding-action]').map((button) => button.text())
    expect(actions).toEqual(['打开 Codex 配置目录', '新增第一个 Provider', '稍后设置', '退出'])

    for (const [event, label] of [
      ['openDirectory', '打开 Codex 配置目录'],
      ['addProvider', '新增第一个 Provider'],
      ['later', '稍后设置'],
      ['exit', '退出'],
    ] as const) {
      await wrapper.get(`[aria-label="${label}"]`).trigger('click')
      expect(wrapper.emitted(event)).toHaveLength(1)
    }
  })

  it('imports the current auth.json key only after naming it', async () => {
    const wrapper = mount(OnboardingView, {
      attachTo: document.body,
      props: {
        busy: false,
        currentProviderName: 'Provider A',
        canImportCurrentKey: true,
        successMessage: null,
        errorMessage: null,
      },
    })

    await wrapper.get('[aria-label="保存当前 auth.json 密钥"]').trigger('click')
    expect(wrapper.text()).toContain('是否将其保存到当前 Provider')
    await wrapper.get('[aria-label="确认导入当前密钥"]').trigger('click')
    expect(wrapper.text()).toContain('密钥名称为必填项')
    await wrapper.get('[aria-label="取消导入当前密钥"]').trigger('click')
    expect(wrapper.emitted('importCurrentKey')).toBeUndefined()

    await wrapper.get('[aria-label="保存当前 auth.json 密钥"]').trigger('click')
    await wrapper.get('[name="import-api-key-name"]').setValue('从 Codex 导入')
    await wrapper.get('[aria-label="确认导入当前密钥"]').trigger('click')
    expect(wrapper.emitted('importCurrentKey')).toEqual([['从 Codex 导入']])
  })
})
