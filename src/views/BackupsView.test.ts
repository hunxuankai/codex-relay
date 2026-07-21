import { flushPromises, mount } from '@vue/test-utils'
import { ref, shallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { BackupSummary } from '../types/backup'
import BackupsView from './BackupsView.vue'

const mockUseBackups = vi.hoisted(() => vi.fn())
vi.mock('../composables/useBackups', () => ({ useBackups: mockUseBackups }))

const backup: BackupSummary = {
  directoryName: '20260720-transaction-1',
  files: ['config.toml', 'auth.json', 'providers.json', 'metadata.json'],
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

function controller(options: { fail?: boolean; openFail?: boolean } = {}) {
  const error = shallowRef<{ code: string; message: string } | null>(null)
  const successMessage = shallowRef<string | null>(null)
  const restore = vi.fn().mockImplementation(async () => {
    if (options.fail) {
      error.value = { code: 'RESTORE_FAILED', message: '恢复失败，未显示任何密钥。' }
      return
    }
    successMessage.value = '配置备份已恢复。'
  })
  const openFile = vi.fn().mockImplementation(async () => {
    if (options.openFail) {
      error.value = {
        code: 'OPEN_BACKUP_FILE_FAILED',
        message: '无法使用记事本打开备份文件。',
      }
    }
  })
  return {
    backups: ref([backup]),
    loading: shallowRef(false),
    busy: shallowRef(false),
    error,
    successMessage,
    refresh: vi.fn(),
    openFile,
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

  it('expands the backup file list and opens a selected file', async () => {
    const state = controller()
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView)

    await wrapper.get('[aria-label="查看备份文件 transaction-1"]').trigger('click')

    expect(wrapper.text()).toContain('config.toml')
    expect(wrapper.text()).toContain('auth.json')
    expect(wrapper.text()).toContain('providers.json')
    expect(wrapper.text()).toContain('metadata.json')

    await wrapper.get('[aria-label="打开备份文件 auth.json"]').trigger('click')
    expect(state.openFile).toHaveBeenCalledWith('20260720-transaction-1', 'auth.json')
  })

  it('keeps only one backup expanded and shows only files in each summary', async () => {
    const second: BackupSummary = {
      directoryName: '20260720-transaction-2',
      files: ['metadata.json'],
      metadata: {
        ...backup.metadata,
        transactionId: 'transaction-2',
        configExisted: false,
        authExisted: false,
        providersExisted: false,
      },
    }
    const state = controller()
    state.backups.value = [backup, second]
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView)

    await wrapper.get('[aria-label="查看备份文件 transaction-1"]').trigger('click')
    expect(wrapper.find('#backup-files-20260720-transaction-1').exists()).toBe(true)

    await wrapper.get('[aria-label="查看备份文件 transaction-2"]').trigger('click')
    expect(wrapper.find('#backup-files-20260720-transaction-1').exists()).toBe(false)
    expect(wrapper.get('#backup-files-20260720-transaction-2').text()).toBe('metadata.json')

    await wrapper.get('[aria-label="收起备份文件 transaction-2"]').trigger('click')
    expect(wrapper.find('#backup-files-20260720-transaction-2').exists()).toBe(false)
  })

  it('shows an open failure and leaves restore available', async () => {
    const state = controller({ openFail: true })
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView)

    await wrapper.get('[aria-label="查看备份文件 transaction-1"]').trigger('click')
    await wrapper.get('[aria-label="打开备份文件 config.toml"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('无法使用记事本打开备份文件。')
    expect(wrapper.get('[aria-label="恢复备份 transaction-1"]').attributes('disabled')).toBeUndefined()
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
