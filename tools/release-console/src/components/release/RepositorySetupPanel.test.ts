import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import RepositorySetupPanel from './RepositorySetupPanel.vue'

describe('RepositorySetupPanel', () => {
  it('keeps repository/version input explicit and explains why release actions are disabled', async () => {
    const wrapper = mount(RepositorySetupPanel, {
      props: {
        repositoryPath: '',
        targetVersion: '',
        inspection: null,
        busy: false,
      },
    })

    expect(wrapper.text()).toContain('请选择 Codex Relay 仓库')
    expect(wrapper.get('[data-testid="inspect-button"]').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).toContain('先填写仓库路径')

    await wrapper.get('input[aria-label="仓库路径"]').setValue('D:\\safe-temp\\repository')
    const updates = wrapper.emitted('update:repositoryPath') ?? []
    expect(updates[updates.length - 1]).toEqual(['D:\\safe-temp\\repository'])
    expect(wrapper.emitted('inspect')).toBeUndefined()
  })

  it('shows the verified remote and emits inspect/plan actions without mutating props', async () => {
    const wrapper = mount(RepositorySetupPanel, {
      props: {
        repositoryPath: 'D:\\safe-temp\\repository',
        targetVersion: '0.5.0',
        inspection: {
          repositoryPath: 'D:\\safe-temp\\repository',
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
        },
        busy: false,
      },
    })

    expect(wrapper.text()).toContain('hunxuankai/codex-relay')
    expect(wrapper.text()).toContain('master → main')
    expect(wrapper.text()).toContain('线上 Latest')
    expect(wrapper.text()).toContain('v0.4.0')
    await wrapper.get('[data-testid="inspect-button"]').trigger('click')
    await wrapper.get('[data-testid="plan-button"]').trigger('click')
    expect(wrapper.emitted('inspect')).toHaveLength(1)
    expect(wrapper.emitted('preparePlan')).toHaveLength(1)
  })

  it('shows an explicit empty state when the repository has no published release', () => {
    const wrapper = mount(RepositorySetupPanel, {
      props: {
        repositoryPath: 'D:\\safe-temp\\repository',
        targetVersion: '0.1.0',
        inspection: {
          repositoryPath: 'D:\\safe-temp\\repository',
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
            latestReleaseTag: null,
          },
          releaseReady: true,
          blockingReasons: [],
          safePush: null,
        },
        busy: false,
      },
    })

    expect(wrapper.text()).toContain('线上 Latest')
    expect(wrapper.text()).toContain('尚无正式版本')
  })

  it('shows authoritative ahead state and keeps planning blocked until the repository is synced', async () => {
    const wrapper = mount(RepositorySetupPanel, {
      props: {
        repositoryPath: 'D:\\safe-temp\\repository',
        targetVersion: '0.5.0',
        inspection: {
          repositoryPath: 'D:\\safe-temp\\repository',
          repository: {
            localBranch: 'master',
            defaultBranch: 'main',
            headSha: 'b'.repeat(40),
            remoteMainSha: 'a'.repeat(40),
            remoteUrl: 'https://github.com/hunxuankai/codex-relay.git',
            clean: true,
            sync: {
              status: 'ahead',
              aheadCount: 2,
              behindCount: 0,
              aheadCommits: [
                { sha: 'b'.repeat(40), subject: 'fix: second reviewed commit' },
                { sha: 'c'.repeat(40), subject: 'feat: first reviewed commit' },
              ],
            },
          },
          external: {
            tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
            activeReleaseRuns: 0,
            conflictingDrafts: 0,
            latestReleaseTag: 'v0.4.0',
          },
          releaseReady: false,
          blockingReasons: ['本地领先远端 main 2 个提交；请先推送当前提交。'],
          safePush: {
            expectedHeadSha: 'b'.repeat(40),
            expectedRemoteMainSha: 'a'.repeat(40),
            commitCount: 2,
            commits: [
              { sha: 'b'.repeat(40), subject: 'fix: second reviewed commit' },
              { sha: 'c'.repeat(40), subject: 'feat: first reviewed commit' },
            ],
          },
        },
        busy: false,
      },
    })

    expect(wrapper.text()).toContain('本地领先 2 个提交')
    expect(wrapper.text()).toContain('本地领先远端 main 2 个提交')
    expect(wrapper.text()).toContain('fix: second reviewed commit')
    expect(wrapper.get('[data-testid="plan-button"]').attributes('disabled')).toBeDefined()
    const push = wrapper.get('[data-testid="request-push-button"]')
    expect(push.text()).toContain('推送当前 2 个提交')
    await push.trigger('click')
    expect(wrapper.emitted('requestPush')).toHaveLength(1)
  })

  it('reports missing toolchain facts instead of claiming every tool is ready', () => {
    const wrapper = mount(RepositorySetupPanel, {
      props: {
        repositoryPath: 'D:\\safe-temp\\repository',
        targetVersion: '0.5.0',
        inspection: {
          repositoryPath: 'D:\\safe-temp\\repository',
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
            tools: { git: null, node: '24', npm: '11', cargo: '1.90', gh: null },
            activeReleaseRuns: 0,
            conflictingDrafts: 0,
            latestReleaseTag: 'v0.4.0',
          },
          releaseReady: false,
          blockingReasons: ['发布所需工具尚未全部就绪。'],
          safePush: null,
        },
        busy: false,
      },
    })

    expect(wrapper.text()).toContain('缺少：Git、gh')
    expect(wrapper.text()).not.toContain('Git / Node / npm / Cargo / gh 已就绪')
  })
})
