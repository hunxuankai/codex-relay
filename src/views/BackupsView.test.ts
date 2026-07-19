import { flushPromises, mount } from '@vue/test-utils'
import { ref, shallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { BackupSummary } from '../types/backup'
import BackupsView from './BackupsView.vue'

const mockUseBackups = vi.hoisted(() => vi.fn())
vi.mock('../composables/useBackups', () => ({ useBackups: mockUseBackups }))

const backup: BackupSummary = {
  directoryName: '20260720-transaction-1',
  metadata: {
    transactionId: 'transaction-1',
    createdAt: '2026-07-20T00:00:00+08:00',
    operation: 'switch_provider',
    providerId: 'provider-a',
    configExisted: true,
    authExisted: true,
    providersExisted: true,
    appVersion: '0.1.0',
  },
}

function controller(options: { fail?: boolean } = {}) {
  const error = shallowRef<{ code: string; message: string } | null>(null)
  const successMessage = shallowRef<string | null>(null)
  const restore = vi.fn().mockImplementation(async () => {
    if (options.fail) {
      error.value = { code: 'RESTORE_FAILED', message: '恢复失败，未显示任何密钥。' }
      return
    }
    successMessage.value = '配置备份已恢复。'
  })
  return {
    backups: ref([backup]),
    loading: shallowRef(false),
    busy: shallowRef(false),
    error,
    successMessage,
    refresh: vi.fn(),
    restore,
  }
}

describe('BackupsView', () => {
  beforeEach(() => mockUseBackups.mockReset())

  it('shows backup metadata without rendering secret fields', () => {
    mockUseBackups.mockReturnValue(controller())
    const wrapper = mount(BackupsView)

    expect(wrapper.text()).toContain('transaction-1')
    expect(wrapper.text()).toContain('provider-a')
    expect(wrapper.text()).toContain('switch_provider')
    expect(wrapper.text()).not.toContain('apiKey')
    expect(wrapper.text()).not.toContain('test-key-not-real')
  })

  it('requires confirmation, restores, and requests Provider and health refreshes', async () => {
    const state = controller()
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView, { attachTo: document.body })

    await wrapper.get('[aria-label="恢复备份 transaction-1"]').trigger('click')
    expect(state.restore).not.toHaveBeenCalled()
    expect(wrapper.text()).toContain('恢复前会再次备份当前状态')

    await wrapper.get('[aria-label="确认操作"]').trigger('click')
    await flushPromises()

    expect(state.restore).toHaveBeenCalledWith('20260720-transaction-1')
    expect(wrapper.emitted('restored')).toHaveLength(1)
  })

  it('shows a safe restore failure without reporting a refresh event', async () => {
    const state = controller({ fail: true })
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView, { attachTo: document.body })

    await wrapper.get('[aria-label="恢复备份 transaction-1"]').trigger('click')
    await wrapper.get('[aria-label="确认操作"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('恢复失败，未显示任何密钥。')
    expect(wrapper.emitted('restored')).toBeUndefined()
  })
})
