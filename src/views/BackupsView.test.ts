import { flushPromises, mount } from '@vue/test-utils'
import { ref, shallowRef } from 'vue'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { BackupSummary, UnavailableBackup } from '../types/backup'
import BackupsView from './BackupsView.vue'

const mockUseBackups = vi.hoisted(() => vi.fn())
vi.mock('../composables/useBackups', () => ({ useBackups: mockUseBackups }))

const backup: BackupSummary = {
  directoryName: '20260720-transaction-1',
  files: ['config.toml', 'auth.json', 'providers.json', 'metadata.json'],
  metadata: {
    schemaVersion: 2,
    transactionId: 'transaction-1',
    createdAt: '2026-07-20T00:00:00+08:00',
    operation: 'switch_provider',
    providerId: 'provider-a',
    configExisted: true,
    authExisted: true,
    providersExisted: true,
    preferencesExisted: false,
    appVersion: '0.1.0',
  },
  compatibility: 'current',
}

const unavailableBackup: UnavailableBackup = {
  directoryName: 'unavailable-backup',
  code: 'INVALID_BACKUP_METADATA',
  message: '无法读取此备份的元数据，已保留，无法安全恢复。',
  canOpenMetadata: true,
}

function controller(
  options: { fail?: boolean; loading?: boolean; openFail?: boolean; unavailable?: boolean } = {},
) {
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
    unavailableBackups: ref(options.unavailable ? [unavailableBackup] : []),
    loading: shallowRef(options.loading ?? false),
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
    expect(wrapper.text()).not.toContain('test-key-backup-not-real')
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
        preferencesExisted: false,
      },
      compatibility: 'current',
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

  it('labels legacy backups and explains their preference restore impact before confirmation', async () => {
    const state = controller()
    state.backups.value = [{
      ...backup,
      compatibility: 'legacyWithoutPreferences',
      metadata: { ...backup.metadata, schemaVersion: 1, preferencesExisted: false },
    }]
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView, { attachTo: document.body })

    expect(wrapper.text()).toContain('旧版备份')
    await wrapper.get('[aria-label="恢复备份 transaction-1"]').trigger('click')

    expect(wrapper.text()).toContain('不包含命名地址、模型与推理偏好')
    expect(wrapper.text()).toContain('恢复前会再次备份当前状态')
  })

  it('keeps unavailable backups visible and allows only their metadata to be opened', async () => {
    const state = controller({ unavailable: true })
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView)

    expect(wrapper.text()).toContain('无法安全恢复的备份')
    expect(wrapper.text()).toContain('无法读取此备份的元数据，已保留，无法安全恢复。')
    expect(wrapper.find('[aria-label="恢复备份 unavailable-backup"]').exists()).toBe(false)

    await wrapper.get('[aria-label="打开不可用备份 unavailable-backup 的元数据"]').trigger('click')
    expect(state.openFile).toHaveBeenCalledWith('unavailable-backup', 'metadata.json')
  })

  it('shows unavailable backup guidance instead of an empty state when no backup is recoverable', () => {
    const state = controller({ unavailable: true })
    state.backups.value = []
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView)

    expect(wrapper.text()).toContain('无法安全恢复的备份')
    expect(wrapper.text()).not.toContain('暂无可恢复的事务备份。')
  })

  it('hides stale backup sections while the inventory is refreshing', () => {
    const state = controller({ loading: true, unavailable: true })
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView)

    expect(wrapper.find('[aria-label="正在加载备份"]').exists()).toBe(true)
    expect(wrapper.text()).not.toContain('transaction-1')
    expect(wrapper.text()).not.toContain('无法安全恢复的备份')
  })

  it('explains when unavailable backup metadata cannot be opened', () => {
    const state = controller({ unavailable: true })
    state.backups.value = []
    state.unavailableBackups.value = [{ ...unavailableBackup, canOpenMetadata: false }]
    mockUseBackups.mockReturnValue(state)
    const wrapper = mount(BackupsView)

    expect(wrapper.text()).toContain('元数据文件不可用，无法打开。')
    expect(wrapper.find('[aria-label="打开不可用备份 unavailable-backup 的元数据"]').exists()).toBe(false)
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
