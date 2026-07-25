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
  it('checks only after a click, formats the release date, and renders safe markdown notes', async () => {
    const updater = controller()
    const wrapper = mount(UpdatePanel, { props: { updater } })
    const releaseDate = '2026-07-25T14:57:33.219Z'
    const expectedReleaseDate = new Intl.DateTimeFormat('zh-CN', {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(new Date(releaseDate))

    expect(updater.check).not.toHaveBeenCalled()
    await wrapper.get('button').trigger('click')
    expect(updater.check).toHaveBeenCalledOnce()

    ;(updater.status as ShallowRef<UpdateStatus>).value = 'available'
    ;(updater.release as ShallowRef<UpdateReleaseInfo | null>).value = {
      currentVersion: '0.1.0',
      version: '0.2.0',
      date: releaseDate,
      notes: '## 安全更新\n\n- 修复问题\n- **加固**\n\n<script>alert(1)</script>\n\n[危险链接](javascript:alert(1))',
    }
    await wrapper.vm.$nextTick()

    expect(wrapper.text()).toContain('0.2.0')
    expect(wrapper.text()).toContain(`发布日期：${expectedReleaseDate}`)
    expect(wrapper.text()).not.toContain(releaseDate)
    expect(wrapper.find('.release-notes-content h2').text()).toBe('安全更新')
    expect(wrapper.findAll('.release-notes-content li')).toHaveLength(2)
    expect(wrapper.find('.release-notes-content strong').text()).toBe('加固')
    expect(wrapper.find('.release-notes-content script').exists()).toBe(false)
    expect(wrapper.find('.release-notes-content a[href^="javascript:"]').exists()).toBe(false)
  })
})
