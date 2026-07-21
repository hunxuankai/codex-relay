import { mount } from '@vue/test-utils'
import { shallowRef, type ShallowRef } from 'vue'
import { describe, expect, it, vi } from 'vitest'
import type { RelayUiError } from '../types/command'
import type { UpdateProgress, UpdateReleaseInfo } from '../types/update'
import type { UpdateStatus } from '../composables/useUpdater'
import type { UpdaterController } from '../composables/useUpdater'
import UpdatePanel from './UpdatePanel.vue'

function controller(): UpdaterController {
  return {
    status: shallowRef<UpdateStatus>('idle'),
    currentVersion: shallowRef<string | null>('0.1.0'),
    release: shallowRef<UpdateReleaseInfo | null>(null),
    error: shallowRef<RelayUiError | null>(null),
    progress: shallowRef<UpdateProgress | null>(null),
    check: vi.fn(),
    checkSilently: vi.fn(),
    reset: vi.fn(),
    requestInstall: vi.fn(),
    cancelInstall: vi.fn(),
    confirmInstall: vi.fn(),
  } as unknown as UpdaterController
}

describe('UpdatePanel', () => {
  it('checks only after a click and renders release notes as plain text', async () => {
    const updater = controller()
    const wrapper = mount(UpdatePanel, { props: { updater } })

    expect(updater.check).not.toHaveBeenCalled()
    await wrapper.get('button').trigger('click')
    expect(updater.check).toHaveBeenCalledOnce()

    ;(updater.status as ShallowRef<UpdateStatus>).value = 'available'
    ;(updater.release as ShallowRef<UpdateReleaseInfo | null>).value = {
      currentVersion: '0.1.0',
      version: '0.2.0',
      date: '2026-07-21T00:00:00Z',
      notes: '<img src=x onerror=alert(1)>安全更新',
    }
    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('0.2.0')
    expect(wrapper.text()).toContain('<img src=x onerror=alert(1)>安全更新')
    expect(wrapper.find('img').exists()).toBe(false)
  })
})
