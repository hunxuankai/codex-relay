import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ApiKeyInput from './ApiKeyInput.vue'

describe('ApiKeyInput', () => {
  it('uses password mode by default and toggles visibility', async () => {
    const wrapper = mount(ApiKeyInput, {
      props: { modelValue: 'test-key-not-real', configured: false },
    })
    const input = wrapper.get('input')

    expect(input.attributes('type')).toBe('password')
    await wrapper.get('[aria-label="显示 API Key"]').trigger('click')
    expect(input.attributes('type')).toBe('text')
    await wrapper.get('[aria-label="隐藏 API Key"]').trigger('click')
    expect(input.attributes('type')).toBe('password')
  })

  it('requires explicit confirmation before clearing a configured key', async () => {
    const wrapper = mount(ApiKeyInput, {
      props: { modelValue: '', configured: true },
    })

    await wrapper.get('[aria-label="清空 API Key"]').trigger('click')
    expect(wrapper.text()).toContain('确认清空')
    expect(wrapper.emitted('clear')).toBeUndefined()

    await wrapper.get('[aria-label="确认清空 API Key"]').trigger('click')
    expect(wrapper.emitted('clear')).toHaveLength(1)
  })
})
