import { flushPromises, mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { ReleasePreflightResult } from './types/release'

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
  Channel: class<T> {
    onmessage?: (message: T) => void
  },
}))

import App from './App.vue'
import appSource from './App.vue?raw'
import { REPOSITORY_PREFERENCE_KEY } from './composables/useRepositoryPreference'

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
    },
    external: {
      tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
      activeReleaseRuns: 0,
      conflictingDrafts: 0,
      latestReleaseTag: 'v0.4.0',
    },
  }
}

describe('release console shell', () => {
  beforeEach(() => {
    window.localStorage.clear()
    invokeMock.mockReset()
  })

  it('keeps App as a composition surface for the visual release workflow', () => {
    window.localStorage.setItem(
      REPOSITORY_PREFERENCE_KEY,
      JSON.stringify({ version: 1, repositoryPath: 'D:\\safe-temp\\repository' }),
    )
    const wrapper = mount(App)

    expect(wrapper.get('h1').text()).toBe('Codex Relay 发布控制台')
    expect(wrapper.get<HTMLInputElement>('input[aria-label="仓库路径"]').element.value).toBe(
      'D:\\safe-temp\\repository',
    )
    expect(wrapper.text()).toContain('仓库与版本')
    expect(wrapper.text()).toContain('阶段时间线')
    expect(wrapper.text()).toContain('加载活动会话')
    expect(wrapper.get('.release-console-layout')).toBeTruthy()
    expect(wrapper.text()).not.toContain('首版只完成可视化一键发布与在线复核')
  })

  it('switches to a single column before the configured minimum window width', () => {
    expect(appSource).toContain('@media (max-width: 820px)')
    expect(appSource).toMatch(
      /@media \(max-width: 820px\)[\s\S]*?\.release-console-layout\s*\{[\s\S]*?grid-template-columns:\s*minmax\(0, 1fr\)/,
    )
  })

  it('keeps planning and start controls locked while a release session is active', () => {
    expect(appSource).toContain('const hasActiveSession = computed(')
    expect(appSource).toContain(':busy="release.busy.value || hasActiveSession"')
    expect(appSource).toMatch(/const canCancel = computed\([\s\S]*?'idle'/)
  })

  it('remembers the canonical repository path only after a successful inspection', async () => {
    invokeMock.mockResolvedValueOnce({
      success: true,
      data: inspection('D:\\canonical\\repository'),
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
    invokeMock.mockResolvedValueOnce({
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
})
