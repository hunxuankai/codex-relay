import { flushPromises, mount } from '@vue/test-utils'
import { ElDialog } from 'element-plus'
import { describe, expect, it, vi } from 'vitest'
import type { ProviderAvailabilityTrace } from '../types/providerAvailability'
import ProviderAvailabilityTraceDialog from './ProviderAvailabilityTraceDialog.vue'

const trace: ProviderAvailabilityTrace = {
  request: {
    method: 'POST',
    url: 'https://provider.example.test/v1/responses',
    body: '{"model":"gpt-5.6-sol","stream":false}',
  },
  response: {
    status: 401,
    body: '{"error":{"message":"Provider rejected the request"}}',
    bodyTruncated: false,
  },
}

describe('ProviderAvailabilityTraceDialog', () => {
  it('shows request, response, status and duration without rendering headers', async () => {
    const wrapper = mount(ProviderAvailabilityTraceDialog, {
      attachTo: document.body,
      props: {
        open: true,
        providerName: 'Provider A',
        trace,
        durationMs: 125,
      },
    })
    await flushPromises()

    expect(wrapper.findComponent(ElDialog).exists()).toBe(true)
    expect(document.body.textContent).toContain('Provider A 的 API 请求与响应')
    expect(document.body.textContent).toContain('POST')
    expect(document.body.textContent).toContain('/v1/responses')
    expect(document.body.textContent).toContain('HTTP 401')
    expect(document.body.textContent).toContain('125 ms')
    expect(document.body.textContent).toContain('Provider rejected the request')
    expect(document.body.textContent).not.toContain('Authorization')
    wrapper.unmount()
  })

  it('explains when no HTTP response was received and marks truncated bodies', async () => {
    const wrapper = mount(ProviderAvailabilityTraceDialog, {
      attachTo: document.body,
      props: {
        open: true,
        providerName: 'Provider A',
        trace: {
          ...trace,
          response: null,
        },
        durationMs: 30_000,
      },
    })
    await flushPromises()
    expect(document.body.textContent).toContain('未收到 HTTP 响应')

    await wrapper.setProps({
      trace: {
        ...trace,
        response: { ...trace.response!, bodyTruncated: true },
      },
    })
    await flushPromises()
    expect(document.body.textContent).toContain('正文已截断')
    wrapper.unmount()
  })

  it('emits close from the explicit button and the Escape model contract', async () => {
    const buttonWrapper = mount(ProviderAvailabilityTraceDialog, {
      attachTo: document.body,
      props: { open: true, providerName: 'Provider A', trace, durationMs: 1 },
    })
    await flushPromises()

    const closeButton = document.querySelector<HTMLButtonElement>(
      '[aria-label="关闭 API 请求与响应详情"]',
    )
    closeButton?.click()
    await flushPromises()
    expect(buttonWrapper.emitted('close')).toEqual([[]])
    buttonWrapper.unmount()

    const escapeWrapper = mount(ProviderAvailabilityTraceDialog, {
      props: { open: true, providerName: 'Provider A', trace, durationMs: 1 },
    })

    const dialog = escapeWrapper.getComponent(ElDialog)
    expect(dialog.props('closeOnPressEscape')).toBe(true)
    dialog.vm.$emit('update:modelValue', false)
    expect(escapeWrapper.emitted('close')).toEqual([[]])
    escapeWrapper.unmount()
  })

  it('cancels deferred focus when the dialog is unmounted', async () => {
    vi.useFakeTimers()
    const setTimeoutSpy = vi.spyOn(globalThis, 'setTimeout')
    const clearTimeoutSpy = vi.spyOn(globalThis, 'clearTimeout')

    try {
      const wrapper = mount(ProviderAvailabilityTraceDialog, {
        attachTo: document.body,
        props: { open: true, providerName: 'Provider A', trace, durationMs: 1 },
      })
      await flushPromises()

      const focusTimerIndex = setTimeoutSpy.mock.calls.findIndex(
        ([handler]) => typeof handler === 'function' && handler.name === 'focusCloseButton',
      )
      expect(focusTimerIndex).toBeGreaterThanOrEqual(0)
      const focusTimerId = setTimeoutSpy.mock.results[focusTimerIndex]?.value

      wrapper.unmount()

      expect(clearTimeoutSpy.mock.calls.some(([timerId]) => timerId === focusTimerId)).toBe(true)
    } finally {
      vi.restoreAllMocks()
      vi.useRealTimers()
    }
  })
})
