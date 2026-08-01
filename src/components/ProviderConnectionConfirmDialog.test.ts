import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { describe, expect, it } from 'vitest'
import ProviderConnectionConfirmDialog from './ProviderConnectionConfirmDialog.vue'

describe('ProviderConnectionConfirmDialog', () => {
  it('shows a structured safe apply summary and emits explicit actions', async () => {
    const wrapper = mount(ProviderConnectionConfirmDialog, {
      attachTo: document.body,
      props: {
        open: true,
        action: 'apply',
        sourceProviderName: 'Provider B',
        targetProviderId: 'provider-a',
        baseUrlName: '主用地址',
        apiKeyName: '主用密钥',
        busy: false,
      },
    })
    await flushPromises()

    const text = document.body.textContent ?? ''
    expect(text).toContain('仅应用连接')
    expect(text).toContain('来源 Provider')
    expect(text).toContain('Provider B')
    expect(text).toContain('已选 Base URL')
    expect(text).toContain('主用地址')
    expect(text).toContain('已选 API Key')
    expect(text).toContain('主用密钥')
    expect(text).toContain('目标 Provider ID')
    expect(text).toContain('provider-a')
    expect(text).toContain('顶层 model_provider 保持不变')
    expect(text).not.toContain('https://')
    expect(text).not.toContain('test-key')

    await wrapper.get('[aria-label="取消连接确认"]').trigger('click')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
    await wrapper.get('[aria-label="确认应用连接"]').trigger('click')
    expect(wrapper.emitted('confirm')).toHaveLength(1)
  })

  it.each(['apply', 'update'] as const)(
    'warns about old-session compatibility before the %s mutation',
    async (action) => {
      const wrapper = mount(ProviderConnectionConfirmDialog, {
        attachTo: document.body,
        props: {
          open: true,
          action,
          sourceProviderName: 'Provider B',
          targetProviderId: 'provider-a',
          baseUrlName: '主用地址',
          apiKeyName: '主用密钥',
          busy: false,
        },
      })
      await flushPromises()

      const text = document.body.textContent ?? ''
      expect(text).toContain(
        '旧会话的加密推理或压缩上下文可能与新连接不兼容，恢复会话时可能失败。',
      )

      await wrapper.get('[aria-label="查看旧会话兼容性详细说明"]').trigger('click')
      expect(wrapper.emitted('showRisk')).toHaveLength(1)
      wrapper.unmount()
    },
  )

  it('labels restore entries as the fixed recovery point', async () => {
    const wrapper = mount(ProviderConnectionConfirmDialog, {
      attachTo: document.body,
      props: {
        open: true,
        action: 'restore',
        sourceProviderName: null,
        targetProviderId: 'provider-a',
        baseUrlName: 'Provider A 原地址',
        apiKeyName: 'Provider A 原密钥',
        busy: false,
      },
    })
    await flushPromises()
    const text = document.body.textContent ?? ''

    expect(text).toContain('恢复自身连接')
    expect(text).toContain('恢复 Base URL')
    expect(text).toContain('Provider A 原地址')
    expect(text).toContain('恢复 API Key')
    expect(text).toContain('Provider A 原密钥')
    expect(text).not.toContain('旧会话的加密推理或压缩上下文可能与新连接不兼容')
    expect(wrapper.find('[aria-label="查看旧会话兼容性详细说明"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('restores the opener focus when the parent removes the dialog', async () => {
    const opener = document.createElement('button')
    document.body.append(opener)
    opener.focus()
    const wrapper = mount(ProviderConnectionConfirmDialog, {
      attachTo: document.body,
      props: {
        open: true,
        action: 'apply',
        sourceProviderName: 'Provider B',
        targetProviderId: 'provider-a',
        baseUrlName: '主用地址',
        apiKeyName: '主用密钥',
        busy: false,
      },
    })
    await flushPromises()
    await nextTick()
    await new Promise((resolve) => setTimeout(resolve, 0))
    const cancel = document.body.querySelector<HTMLButtonElement>(
      '[aria-label="取消连接确认"]',
    )

    expect(cancel).not.toBeNull()
    expect(document.activeElement).toBe(cancel)

    wrapper.unmount()
    await nextTick()
    expect(document.activeElement).toBe(opener)
    opener.remove()
  })

  it('forwards the dialog closed event for parent-level sequencing', async () => {
    const wrapper = mount(ProviderConnectionConfirmDialog, {
      props: {
        open: false,
        action: 'apply',
        sourceProviderName: 'Provider B',
        targetProviderId: 'provider-a',
        baseUrlName: '主用地址',
        apiKeyName: '主用密钥',
        busy: false,
      },
    })

    wrapper.getComponent({ name: 'ElDialog' }).vm.$emit('closed')
    await nextTick()

    expect(wrapper.emitted('closed')).toHaveLength(1)
  })
})
