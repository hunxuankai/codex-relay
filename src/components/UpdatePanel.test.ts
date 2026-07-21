import { mount } from '@vue/test-utils'
import { shallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { RelayUiError } from '../types/command'
import type { UpdateProgress, UpdateReleaseInfo } from '../types/update'
import type { UpdateStatus } from '../composables/useUpdater'
import UpdatePanel from './UpdatePanel.vue'

const mockUseUpdater = vi.hoisted(() => vi.fn())
vi.mock('../composables/useUpdater', () => ({ useUpdater: mockUseUpdater }))

function controller() {
  return {
    status: shallowRef<UpdateStatus>('idle'),
    currentVersion: shallowRef<string | null>('0.1.0'),
    release: shallowRef<UpdateReleaseInfo | null>(null),
    error: shallowRef<RelayUiError | null>(null),
    progress: shallowRef<UpdateProgress | null>(null),
    check: vi.fn(),
    reset: vi.fn(),
    requestInstall: vi.fn(),
    cancelInstall: vi.fn(),
    confirmInstall: vi.fn(),
  }
}

describe('UpdatePanel', () => {
  beforeEach(() => mockUseUpdater.mockReset())

  it('checks only after a click and renders release notes as plain text', async () => {
    const updater = controller()
    mockUseUpdater.mockReturnValue(updater)
    const wrapper = mount(UpdatePanel)

    expect(updater.check).not.toHaveBeenCalled()
    await wrapper.get('button').trigger('click')
    expect(updater.check).toHaveBeenCalledOnce()

    updater.status.value = 'available'
    updater.release.value = {
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
