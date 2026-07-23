import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ApiKeyInput from './ApiKeyInput.vue'

describe('ApiKeyInput', () => {
  it('uses password mode by default and toggles visibility', async () => {
    const wrapper = mount(ApiKeyInput, {
    props: { modelValue: 'test-key-provider-not-real' },
    })
    const input = wrapper.get('input')

    expect(input.attributes('type')).toBe('password')
    await wrapper.get('[aria-label="显示 API Key"]').trigger('click')
    expect(input.attributes('type')).toBe('text')
    await wrapper.get('[aria-label="隐藏 API Key"]').trigger('click')
    expect(input.attributes('type')).toBe('password')
  })

  it('does not expose the removed clear-existing-key workflow', () => {
    const wrapper = mount(ApiKeyInput, {
      props: { modelValue: '', configured: true } as never,
    })

    expect(wrapper.find('[aria-label="清空 API Key"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('确认清空')
  })
})
