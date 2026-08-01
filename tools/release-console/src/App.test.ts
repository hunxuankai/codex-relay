import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReleasePreflightResult } from './types/release'

const { invokeMock, messageMock, messageBoxAlertMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  messageMock: vi.fn(),
  messageBoxAlertMock: vi.fn(),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
  Channel: class<T> {
    onmessage?: (message: T) => void
  },
}))

vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')
  return {
    ...actual,
    ElMessage: messageMock,
    ElMessageBox: {
      ...actual.ElMessageBox,
      alert: messageBoxAlertMock,
    },
  }
})

import App from './App.vue'
import appSource from './App.vue?raw'
import ProxySettingsPanel from './components/release/ProxySettingsPanel.vue'
import ReleaseRecoveryPanel from './components/release/ReleaseRecoveryPanel.vue'
import RepositorySyncConfirmDialog from './components/release/RepositorySyncConfirmDialog.vue'
import { REPOSITORY_PREFERENCE_KEY } from './composables/useRepositoryPreference'
import { RELEASE_PROXY_PREFERENCE_KEY } from './composables/useReleaseProxyPreference'

function inspection(repositoryPath: string): ReleasePreflightResult {
  return {
    repositoryPath,
    repository: {
      localBranch: 'master',
      defaultBranch: 'main',
      headSha: 'a'.repeat(40),
      remoteMainSha: 'a'.repeat(40),
      remoteUrl: 'https://github.com/hunxuankai/codex-relay.git',
      clean: true,
      sync: {
        status: 'synced',
        aheadCount: 0,
        behindCount: 0,
        aheadCommits: [],
      },
    },
    external: {
      tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
      activeReleaseRuns: 0,
      conflictingDrafts: 0,
      latestReleaseTag: 'v0.4.0',
    },
    releaseReady: true,
    blockingReasons: [],
    safePush: null,
  }
}

function aheadInspection(repositoryPath: string): ReleasePreflightResult {
  return {
    ...inspection(repositoryPath),
    repository: {
      ...inspection(repositoryPath).repository,
      headSha: 'b'.repeat(40),
      remoteMainSha: 'a'.repeat(40),
      sync: {
        status: 'ahead',
        aheadCount: 1,
        behindCount: 0,
        aheadCommits: [{ sha: 'b'.repeat(40), subject: 'feat: reviewed local commit' }],
      },
    },
    releaseReady: false,
    blockingReasons: ['本地领先远端 main 1 个提交；请先推送当前提交。'],
    safePush: {
      expectedHeadSha: 'b'.repeat(40),
      expectedRemoteMainSha: 'a'.repeat(40),
      commitCount: 1,
      commits: [{ sha: 'b'.repeat(40), subject: 'feat: reviewed local commit' }],
    },
  }
}

describe('release console shell', () => {
  beforeEach(() => {
    window.localStorage.clear()
    invokeMock.mockReset()
    invokeMock.mockResolvedValue({ success: true, data: null })
    messageMock.mockReset()
    messageBoxAlertMock.mockReset()
    messageBoxAlertMock.mockResolvedValue('confirm')
  })

  it('keeps App as a composition surface for the visual release workflow', () => {
    window.localStorage.setItem(
      REPOSITORY_PREFERENCE_KEY,
      JSON.stringify({
        version: 1,
        repositoryPath: '\\\\?\\D:\\safe-temp\\repository',
      }),
    )
    const wrapper = mount(App)

    expect(wrapper.get('h1').text()).toBe('Codex Relay 发布控制台')
    expect(wrapper.get<HTMLInputElement>('input[aria-label="仓库路径"]').element.value).toBe(
      'D:\\safe-temp\\repository',
    )
    expect(wrapper.text()).toContain('仓库与版本')
    expect(wrapper.text()).toContain('阶段时间线')
    expect(wrapper.text()).not.toContain('加载活动会话')
    expect(wrapper.get('.release-console-layout')).toBeTruthy()
    expect(wrapper.text()).not.toContain('首版只完成可视化一键发布与在线复核')
  })

  it('switches to a single column before the configured minimum window width', () => {
    expect(appSource).toContain('@media (max-width: 820px)')
    expect(appSource).toMatch(
      /@media \(max-width: 820px\)[\s\S]*?\.release-console-layout\s*\{[\s\S]*?grid-template-columns:\s*minmax\(0, 1fr\)/,
    )
  })

  it('keeps the release progress panel fixed while the desktop workspace scrolls independently', () => {
    expect(appSource).toMatch(
      /\.app-shell\s*\{[\s\S]*?grid-template-rows:\s*auto minmax\(0, 1fr\);[\s\S]*?height:\s*100vh;[\s\S]*?overflow:\s*hidden;/,
    )
    expect(appSource).toMatch(
      /\.release-console-layout\s*\{[\s\S]*?min-height:\s*0;[\s\S]*?overflow:\s*hidden;/,
    )
    expect(appSource).toMatch(
      /\.release-timeline-panel,[\s\S]*?\.workspace\s*\{[\s\S]*?min-height:\s*0;[\s\S]*?overflow-y:\s*auto;/,
    )
    expect(appSource).toMatch(
      /\.workspace\s*\{[\s\S]*?grid-auto-rows:\s*max-content;/,
    )
    expect(appSource).toMatch(
      /@media \(max-width: 820px\)[\s\S]*?\.app-shell\s*\{[\s\S]*?height:\s*auto;[\s\S]*?overflow:\s*visible;/,
    )
  })

  it('shows ordinary release command failures as temporary Element Plus messages', async () => {
    invokeMock.mockResolvedValueOnce({
      success: false,
      error: { code: 'GIT_REPOSITORY_INVALID', message: '无法读取 Git 仓库。' },
    })
    const wrapper = mount(App)

    await wrapper.get<HTMLInputElement>('input[aria-label="仓库路径"]').setValue(
      'D:\\safe-temp\\missing-repository',
    )
    await wrapper.get('[data-testid="inspect-button"]').trigger('click')
    await flushPromises()

    expect(messageMock).toHaveBeenCalledWith({
      type: 'error',
      message: '无法读取 Git 仓库。（GIT_REPOSITORY_INVALID）',
      duration: 5000,
      grouping: true,
      showClose: true,
    })
    expect(messageBoxAlertMock).not.toHaveBeenCalled()
    expect(wrapper.find('.app-error').exists()).toBe(false)
    expect(appSource).not.toContain('<ElAlert')
  })

  it('shows connection command failures as temporary messages instead of persistent panel text', async () => {
    invokeMock.mockResolvedValueOnce({
      success: false,
      error: { code: 'GITHUB_COMMAND_FAILED', message: 'GitHub API 连接失败。' },
    })
    const wrapper = mount(App)

    await wrapper.get('[data-testid="test-connection-button"]').trigger('click')
    await flushPromises()

    expect(messageMock).toHaveBeenCalledWith({
      type: 'error',
      message: 'GitHub API 连接失败。（GITHUB_COMMAND_FAILED）',
      duration: 5000,
      grouping: true,
      showClose: true,
    })
    expect(wrapper.text()).not.toContain('GitHub API 连接失败。')
  })

  it.each([
    {
      code: 'RELEASE_ROLLBACK_INCOMPLETE',
      message: '候选文件未能完整回滚，请人工检查仓库。',
    },
    {
      code: 'GIT_PROCESS_TREE_TERMINATION_FAILED',
      message: 'Git 进程树未能完整终止，请人工检查。',
    },
    {
      code: 'GITHUB_PROCESS_TREE_TERMINATION_FAILED',
      message: 'GitHub CLI 进程树未能完整终止，请人工检查。',
    },
  ])('requires acknowledgement for $code failures', async ({ code, message }) => {
    invokeMock.mockResolvedValueOnce({
      success: false,
      error: { code, message },
    })
    const wrapper = mount(App)

    await wrapper.get<HTMLInputElement>('input[aria-label="仓库路径"]').setValue(
      'D:\\safe-temp\\repository',
    )
    await wrapper.get('[data-testid="inspect-button"]').trigger('click')
    await flushPromises()

    expect(messageBoxAlertMock).toHaveBeenCalledWith(
      `${message}\n\n错误码：${code}`,
      '发布操作需要处理',
      {
        type: 'error',
        confirmButtonText: '知道了',
        closeOnClickModal: false,
        closeOnPressEscape: true,
        showClose: true,
      },
    )
    expect(messageMock).not.toHaveBeenCalled()
  })

  it('keeps planning and start controls locked while a release session is active', () => {
    expect(appSource).toContain('const hasActiveSession = computed(')
    expect(appSource).toContain(
      ':busy="release.busy.value || hasActiveSession || proxyInvalid"',
    )
    expect(appSource).toContain('<ReleaseRecoveryPanel')
    expect(appSource).not.toContain('加载活动会话')
  })

  it('remembers the canonical repository path only after a successful inspection', async () => {
    invokeMock.mockResolvedValueOnce({
      success: true,
      data: inspection('\\\\?\\D:\\canonical\\repository'),
    })
    const wrapper = mount(App)
    const input = wrapper.get<HTMLInputElement>('input[aria-label="仓库路径"]')

    await input.setValue('D:\\typed\\repository')
    await wrapper.get('[data-testid="inspect-button"]').trigger('click')
    await flushPromises()

    expect(input.element.value).toBe('D:\\canonical\\repository')
    expect(JSON.parse(window.localStorage.getItem(REPOSITORY_PREFERENCE_KEY) ?? '')).toEqual({
      version: 1,
      repositoryPath: 'D:\\canonical\\repository',
    })
  })

  it('does not overwrite the remembered repository when inspection fails', async () => {
    window.localStorage.setItem(
      REPOSITORY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, repositoryPath: 'D:\\remembered\\repository' }),
    )
    invokeMock
      .mockResolvedValueOnce({ success: true, data: null })
      .mockResolvedValueOnce({
        success: false,
        error: { code: 'GIT_REPOSITORY_INVALID', message: '无法读取 Git 仓库。' },
      })
    const wrapper = mount(App)
    const input = wrapper.get<HTMLInputElement>('input[aria-label="仓库路径"]')

    await input.setValue('D:\\invalid\\repository')
    await wrapper.get('[data-testid="inspect-button"]').trigger('click')
    await flushPromises()

    expect(input.element.value).toBe('D:\\invalid\\repository')
    expect(JSON.parse(window.localStorage.getItem(REPOSITORY_PREFERENCE_KEY) ?? '')).toEqual({
      version: 1,
      repositoryPath: 'D:\\remembered\\repository',
    })
  })

  it('restores, persists and tests the current release proxy snapshot', async () => {
    const storedProxy = {
      enabled: true,
      proxyType: 'http' as const,
      host: '127.0.0.1',
      port: 7890,
    }
    const updatedProxy = {
      enabled: true,
      proxyType: 'socks5' as const,
      host: 'localhost',
      port: 1080,
    }
    window.localStorage.setItem(
      RELEASE_PROXY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, settings: storedProxy }),
    )
    invokeMock.mockResolvedValueOnce({
      success: true,
      data: {
        git: {
          success: true,
          code: null,
          message: 'Git 远端连接正常。',
          durationMillis: 12,
        },
        github: {
          success: true,
          code: null,
          message: 'GitHub API 连接正常。',
          durationMillis: 18,
        },
      },
    })

    const wrapper = mount(App)
    const panel = wrapper.getComponent(ProxySettingsPanel)
    expect(panel.props('settings')).toEqual(storedProxy)

    panel.vm.$emit('update:settings', updatedProxy)
    await wrapper.vm.$nextTick()
    expect(JSON.parse(window.localStorage.getItem(RELEASE_PROXY_PREFERENCE_KEY) ?? '')).toEqual({
      version: 1,
      settings: updatedProxy,
    })

    panel.vm.$emit('test')
    await flushPromises()

    expect(invokeMock).toHaveBeenCalledWith('test_release_connection', { proxy: updatedProxy })
    expect(wrapper.text()).toContain('Git 远端连接正常。')
    expect(wrapper.text()).toContain('GitHub API 连接正常。')
  })

  it('confirms and safely pushes the backend preview with the current proxy', async () => {
    const repositoryPath = 'D:\\safe-temp\\repository'
    const proxy = {
      enabled: true,
      proxyType: 'socks5' as const,
      host: '127.0.0.1',
      port: 1080,
    }
    window.localStorage.setItem(
      REPOSITORY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, repositoryPath }),
    )
    window.localStorage.setItem(
      RELEASE_PROXY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, settings: proxy }),
    )
    invokeMock
      .mockResolvedValueOnce({ success: true, data: null })
      .mockResolvedValueOnce({ success: true, data: aheadInspection(repositoryPath) })
      .mockResolvedValueOnce({ success: true, data: inspection(repositoryPath) })
    const wrapper = mount(App)

    await wrapper.get('[data-testid="inspect-button"]').trigger('click')
    await flushPromises()
    expect(invokeMock.mock.calls[1]).toEqual([
      'inspect_release_repository',
      { repositoryPath, proxy },
    ])
    await wrapper.get('[data-testid="request-push-button"]').trigger('click')
    const dialog = wrapper.getComponent(RepositorySyncConfirmDialog)
    expect(dialog.props('modelValue')).toBe(true)

    dialog.vm.$emit('confirm')
    await flushPromises()

    expect(invokeMock).toHaveBeenLastCalledWith('push_release_repository', {
      request: {
        repositoryPath,
        expectedHeadSha: 'b'.repeat(40),
        expectedRemoteMainSha: 'a'.repeat(40),
        proxy,
      },
    })
    expect(wrapper.find('[data-testid="request-push-button"]').exists()).toBe(false)
  })

  it('invalidates the reviewed push preview when the repository path changes', async () => {
    const repositoryPath = 'D:\\safe-temp\\repository'
    invokeMock.mockResolvedValueOnce({
      success: true,
      data: aheadInspection(repositoryPath),
    })
    const wrapper = mount(App)
    const input = wrapper.get<HTMLInputElement>('input[aria-label="仓库路径"]')

    await input.setValue(repositoryPath)
    await wrapper.get('[data-testid="inspect-button"]').trigger('click')
    await flushPromises()
    expect(wrapper.find('[data-testid="request-push-button"]').exists()).toBe(true)

    await input.setValue('D:\\safe-temp\\other-clone')

    expect(wrapper.find('[data-testid="request-push-button"]').exists()).toBe(false)
  })

  it('automatically detects a remembered local session without inspecting the repository', async () => {
    const repositoryPath = 'D:\\safe-temp\\repository'
    window.localStorage.setItem(
      REPOSITORY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, repositoryPath }),
    )
    const wrapper = mount(App)

    await flushPromises()

    expect(invokeMock).toHaveBeenCalledTimes(1)
    expect(invokeMock).toHaveBeenCalledWith('get_release_session', { repositoryPath })
    expect(wrapper.findComponent(ReleaseRecoveryPanel).exists()).toBe(false)
    expect(wrapper.text()).not.toContain('加载活动会话')
  })

  it('shows the detected committed session and resumes it with the current proxy', async () => {
    const repositoryPath = 'D:\\safe-temp\\repository'
    const proxy = {
      enabled: true,
      proxyType: 'http' as const,
      host: '127.0.0.1',
      port: 7890,
    }
    const committed = {
      id: 'session-1',
      repositoryPath,
      targetVersion: '0.5.0',
      phase: 'committed',
      candidateSha: 'b'.repeat(40),
      remoteMainSha: 'a'.repeat(40),
      workflow: null,
      draft: null,
      published: null,
      cleanup: null,
      cleanupWarning: null,
      failure: null,
    }
    window.localStorage.setItem(
      REPOSITORY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, repositoryPath }),
    )
    window.localStorage.setItem(
      RELEASE_PROXY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, settings: proxy }),
    )
    invokeMock
      .mockResolvedValueOnce({ success: true, data: committed })
      .mockResolvedValueOnce({
        success: true,
        data: { ...committed, phase: 'workflowRunning' },
      })
    const wrapper = mount(App)
    await flushPromises()
    const recovery = wrapper.getComponent(ReleaseRecoveryPanel)
    expect(recovery.text()).toContain('继续 Push')

    recovery.vm.$emit('resume')
    await flushPromises()

    expect(invokeMock).toHaveBeenLastCalledWith('resume_release', {
      sessionId: 'session-1',
      proxy,
      onEvent: expect.anything(),
    })
  })
})
