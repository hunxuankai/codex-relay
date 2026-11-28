import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { RelayCommandError } from '../services/tauri'
import type { CliProfileList } from '../types/cliProfile'
import CliProfilesView from './CliProfilesView.vue'

const commands = vi.hoisted(() => ({
  listCliProfiles: vi.fn(),
  prepareProfileKeyEdit: vi.fn(),
  saveProfileEnvKey: vi.fn(),
  applyProfileProviderKey: vi.fn(),
  openProfileEnvironmentSettings: vi.fn(),
}))

vi.mock('../services/tauri', async (importOriginal) => ({
  ...await importOriginal<typeof import('../services/tauri')>(),
  ...commands,
}))

const inventory = {
  codexHome: 'C:\\safe-test\\codex',
  pendingWrite: false,
  profiles: [
    {
      name: 'agentrouter', command: 'codex -p agentrouter', providerId: 'regional-route',
      host: 'regional.example.test', variable: 'REGIONAL_API_KEY', inheritsDefault: false,
      problem: null, keyStatus: { user: 'notDetected', relayProcess: 'set', matchedUserKeyName: null, restriction: null, sharedWith: [] },
    },
    {
      name: 'qiandao', command: 'codex -p qiandao', providerId: 'shared-route',
      host: 'shared.example.test', variable: 'SHARED_TOKEN', inheritsDefault: true,
      problem: null, keyStatus: { user: 'set', relayProcess: 'unavailable', matchedUserKeyName: '共享备用', restriction: 'sharedVariable', sharedWith: ['默认供应商'] },
    },
  ],
} as const satisfies CliProfileList

const edit = {
  profileName: 'agentrouter', providerId: 'regional-route', host: 'regional.example.test',
  variable: 'REGIONAL_API_KEY', token: 'edit-token', replacesValue: false, relayProcessHasValue: true,
  providerKeys: [],
} as const

const draft = 'test-key-profile-ui-not-real'
const writeText = vi.fn()

describe('CLI Profiles', () => {
  beforeEach(() => {
    vi.resetAllMocks()
    commands.listCliProfiles.mockResolvedValue(inventory)
    commands.prepareProfileKeyEdit.mockResolvedValue(edit)
    commands.openProfileEnvironmentSettings.mockResolvedValue(undefined)
    writeText.mockResolvedValue(undefined)
    vi.stubGlobal('navigator', { clipboard: { writeText } })
  })

  afterEach(() => vi.unstubAllGlobals())

  it('lists native launch commands and explains read-only and process status', async () => {
    const wrapper = mount(CliProfilesView)
    await flushPromises()
    expect(wrapper.text()).toContain(inventory.codexHome)
    expect(wrapper.text()).toContain('仅当前 Relay 进程可见')
    expect(wrapper.text()).toContain('继承默认配置')
    expect(wrapper.text()).toContain('共享变量：默认供应商')
    expect(wrapper.text()).toContain('无法检测')
    expect(wrapper.find('[aria-label="更换 qiandao 的 Key"]').exists()).toBe(false)
    await wrapper.get('[aria-label="复制 agentrouter 的启动命令"]').trigger('click')
    await wrapper.get('[aria-label="复制 qiandao 的启动命令"]').trigger('click')
    await wrapper.get('[aria-label="复制 qiandao 的变量名"]').trigger('click')
    expect(writeText.mock.calls.map(([value]) => value)).toEqual([
      'codex -p agentrouter', 'codex -p qiandao', 'SHARED_TOKEN',
    ])
    await wrapper.get('[aria-label="打开 Windows 环境变量"]').trigger('click')
    expect(commands.openProfileEnvironmentSettings).toHaveBeenCalledOnce()
    expect(commands.prepareProfileKeyEdit).not.toHaveBeenCalled()
    expect(commands.saveProfileEnvKey).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('shows the Windows user key name and updates matches on refresh', async () => {
    const wrapper = mount(CliProfilesView)
    await flushPromises()
    const agentrouter = wrapper.get('[aria-label="CLI Profile 列表"] > li:nth-child(1)')
    const qiandao = wrapper.get('[aria-label="CLI Profile 列表"] > li:nth-child(2)')
    expect(agentrouter.text()).not.toContain('当前 Key')
    expect(qiandao.text()).toContain('当前 Key（Windows 用户变量）：共享备用')

    commands.listCliProfiles.mockResolvedValueOnce({
      ...inventory, profiles: [
        { ...inventory.profiles[0], keyStatus: { ...inventory.profiles[0].keyStatus, user: 'set' } },
        { ...inventory.profiles[1], keyStatus: { ...inventory.profiles[1].keyStatus, matchedUserKeyName: '共享主用' } },
      ],
    })
    const refresh = wrapper.findAll('button').find((button) => button.text() === '刷新')
    if (!refresh) throw new Error('找不到刷新按钮')
    await refresh.trigger('click')
    await flushPromises()
    expect(commands.listCliProfiles).toHaveBeenCalledTimes(2)
    expect(agentrouter.text()).toContain('当前 Key（Windows 用户变量）：未能匹配已保存的 Key')
    expect(qiandao.text()).toContain('当前 Key（Windows 用户变量）：共享主用')
    expect(qiandao.text()).not.toContain('共享备用')
    wrapper.unmount()
  })

  it('keeps a new key in the masked field only and clears it on cancel and unmount', async () => {
    const wrapper = mount(CliProfilesView, { attachTo: document.body })
    await flushPromises()
    const opener = wrapper.get<HTMLButtonElement>('[aria-label="设置 agentrouter 的 Key"]')
    opener.element.focus()
    await opener.trigger('click')
    await flushPromises()
    const input = wrapper.get<HTMLInputElement>('[name="profile-new-key"]')
    expect(input.attributes('type')).toBe('password')
    expect(input.element.value).toBe('')
    expect(wrapper.text()).toContain('没有可复用的同供应商 Key')
    expect(wrapper.text()).toContain('不会导入当前 Relay 进程中的值')
    await input.setValue(draft)
    expect(wrapper.text()).not.toContain(draft)
    await wrapper.get('[aria-label="取消编辑 Key"]').trigger('click')
    await flushPromises()
    expect(input.element.value).toBe('')
    expect(commands.saveProfileEnvKey).not.toHaveBeenCalled()

    await opener.trigger('click')
    await flushPromises()
    const reopened = wrapper.get<HTMLInputElement>('[name="profile-new-key"]')
    expect(reopened.element.value).toBe('')
    await reopened.setValue(draft)
    wrapper.unmount()
    expect(reopened.element.value).toBe('')
  })

  it('confirms one scoped save, clears the draft immediately and refreshes persisted status', async () => {
    let completeSave!: (outcome: { windowsNotified: boolean }) => void
    commands.saveProfileEnvKey.mockReturnValue(new Promise((resolve) => { completeSave = resolve }))
    const wrapper = mount(CliProfilesView)
    await flushPromises()
    await wrapper.get('[aria-label="设置 agentrouter 的 Key"]').trigger('click')
    await flushPromises()
    const input = wrapper.get<HTMLInputElement>('[name="profile-new-key"]')
    await input.setValue(draft)
    await wrapper.get('form[aria-label="设置 Profile Key"]').trigger('submit')
    expect(input.element.value).toBe('')
    expect(commands.saveProfileEnvKey).toHaveBeenCalledWith({
      profileName: 'agentrouter', token: 'edit-token', newKey: draft,
    })
    expect(commands.listCliProfiles).toHaveBeenCalledOnce()
    commands.listCliProfiles.mockResolvedValueOnce({
      ...inventory, profiles: [
        { ...inventory.profiles[0], keyStatus: { ...inventory.profiles[0].keyStatus, user: 'set' } },
        inventory.profiles[1],
      ],
    })
    completeSave({ windowsNotified: false })
    await flushPromises()
    expect(commands.listCliProfiles).toHaveBeenCalledTimes(2)
    expect(wrapper.text()).toContain('已保存到 Windows 用户环境变量')
    expect(wrapper.text()).toContain('完全退出终端后重新打开')
    expect(wrapper.find('[aria-label="更换 agentrouter 的 Key"]').exists()).toBe(true)
    expect(writeText).not.toHaveBeenCalled()
    wrapper.unmount()
  })

  it('shows an unfinished transaction without retaining the key', async () => {
    commands.saveProfileEnvKey.mockRejectedValue(new RelayCommandError(
      'ROLLBACK_INCOMPLETE', '无法确认原用户变量已恢复。',
    ))
    const wrapper = mount(CliProfilesView)
    await flushPromises()
    await wrapper.get('[aria-label="设置 agentrouter 的 Key"]').trigger('click')
    await flushPromises()
    await wrapper.get('[name="profile-new-key"]').setValue(draft)
    commands.listCliProfiles.mockResolvedValueOnce({ ...inventory, pendingWrite: true })
    await wrapper.get('form[aria-label="设置 Profile Key"]').trigger('submit')
    await flushPromises()
    expect(wrapper.text()).toContain('无法确认原用户变量已恢复')
    expect(wrapper.text()).toContain('有一次保存结果未确认')
    expect(wrapper.text()).not.toContain(draft)
    expect(wrapper.text()).not.toContain('已保存到 Windows 用户环境变量')
    wrapper.unmount()
  })

  it('applies a named key in one click by entry ID and refreshes persisted status', async () => {
    commands.prepareProfileKeyEdit.mockResolvedValueOnce({
      ...edit,
      providerKeys: [{ id: 'primary-id', name: '主用' }, { id: 'backup-id', name: '备用' }],
    })
    let completeSave!: (outcome: { windowsNotified: boolean }) => void
    commands.applyProfileProviderKey.mockReturnValue(new Promise((resolve) => { completeSave = resolve }))
    const wrapper = mount(CliProfilesView)
    await flushPromises()
    await wrapper.get('[aria-label="设置 agentrouter 的 Key"]').trigger('click')
    await flushPromises()
    expect(commands.applyProfileProviderKey).not.toHaveBeenCalled()
    const primary = wrapper.get<HTMLButtonElement>('[aria-label="使用已保存的 Key：主用"]')
    const backup = wrapper.get<HTMLButtonElement>('[aria-label="使用已保存的 Key：备用"]')
    const input = wrapper.get<HTMLInputElement>('[name="profile-new-key"]')
    await input.setValue(draft)
    await backup.trigger('click')
    expect(input.element.value).toBe('')
    expect(primary.element.disabled).toBe(true)
    expect(backup.element.disabled).toBe(true)
    await primary.trigger('click')
    expect(commands.applyProfileProviderKey).toHaveBeenCalledExactlyOnceWith({
      profileName: 'agentrouter', token: 'edit-token', apiKeyId: 'backup-id',
    })
    expect(commands.saveProfileEnvKey).not.toHaveBeenCalled()
    expect(writeText).not.toHaveBeenCalled()

    commands.listCliProfiles.mockResolvedValueOnce({
      ...inventory, profiles: [
        { ...inventory.profiles[0], keyStatus: { ...inventory.profiles[0].keyStatus, user: 'set', matchedUserKeyName: '备用' } },
        inventory.profiles[1],
      ],
    })
    completeSave({ windowsNotified: true })
    await flushPromises()
    expect(wrapper.find('[aria-label="更换 agentrouter 的 Key"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('当前 Key（Windows 用户变量）：备用')
    expect(wrapper.text()).toContain('请从已刷新的终端环境重新启动 Codex')
    wrapper.unmount()
  })
})
