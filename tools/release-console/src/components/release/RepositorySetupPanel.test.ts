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
          },
          external: {
            tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
            activeReleaseRuns: 0,
            conflictingDrafts: 0,
            latestReleaseTag: 'v0.4.0',
          },
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
          },
          external: {
            tools: { git: '2.50', node: '24', npm: '11', cargo: '1.90', gh: '2.76' },
            activeReleaseRuns: 0,
            conflictingDrafts: 0,
            latestReleaseTag: null,
          },
        },
        busy: false,
      },
    })

    expect(wrapper.text()).toContain('线上 Latest')
    expect(wrapper.text()).toContain('尚无正式版本')
  })
})
