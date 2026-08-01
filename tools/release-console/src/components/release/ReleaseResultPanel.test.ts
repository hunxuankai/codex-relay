import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ReleaseSession } from '../../types/release'
import ReleaseResultPanel from './ReleaseResultPanel.vue'

describe('ReleaseResultPanel', () => {
  it('separates published success, cleanup failure and unexecuted upgrade checks', () => {
    const session: ReleaseSession = {
      id: 'session-1',
      repositoryPath: 'D:\\safe-temp\\repository',
      targetVersion: '0.5.0',
      phase: 'completedWithWarnings',
      candidateSha: 'a'.repeat(40),
      remoteMainSha: 'a'.repeat(40),
      workflow: { runId: 123, url: 'https://github.com/actions/runs/123' },
      draft: null,
      published: {
        releaseId: 42,
        tagName: 'v0.5.0',
        publishedAt: '2026-07-31T11:00:00Z',
      },
      cleanup: {
        runId: 900,
        url: 'https://github.com/actions/runs/900',
        status: 'completed',
        conclusion: 'failure',
        succeeded: false,
        jobs: [],
      },
      cleanupWarning: null,
      failure: null,
    }

    const wrapper = mount(ReleaseResultPanel, {
      props: { session, exportPath: '', busy: false },
    })

    expect(wrapper.text()).toContain('Release 已公开')
    expect(wrapper.text()).toContain('历史清理失败')
    expect(wrapper.text()).toContain('Sandbox / 安装 / UAC / 应用内升级：未执行')
    expect(wrapper.text()).not.toContain('升级验证成功')
  })

  it('keeps failed sessions truthful instead of reporting cleanup success', () => {
    const session: ReleaseSession = {
      id: 'session-failed',
      repositoryPath: 'D:\\safe-temp\\repository',
      targetVersion: '0.5.0',
      phase: 'failed',
      candidateSha: null,
      remoteMainSha: null,
      workflow: null,
      draft: null,
      published: null,
      cleanup: null,
      cleanupWarning: null,
      failure: null,
    }

    const wrapper = mount(ReleaseResultPanel, {
      props: { session, exportPath: '', busy: false },
    })

    expect(wrapper.text()).toContain('发布失败')
    expect(wrapper.text()).toContain('Release 与公开资产在线复核：未执行')
    expect(wrapper.text()).toContain('历史 Release 清理：未执行')
    expect(wrapper.text()).not.toContain('历史 Release 清理已完成')
  })

  it('does not turn a published but failed remote-finalization session into success', () => {
    const session: ReleaseSession = {
      id: 'session-published-failed',
      repositoryPath: 'D:\\safe-temp\\repository',
      targetVersion: '0.5.0',
      phase: 'failed',
      candidateSha: 'a'.repeat(40),
      remoteMainSha: 'a'.repeat(40),
      workflow: { runId: 123, url: 'https://github.com/actions/runs/123' },
      draft: null,
      published: {
        releaseId: 42,
        tagName: 'v0.5.0',
        publishedAt: '2026-07-31T11:00:00Z',
      },
      cleanup: null,
      cleanupWarning: null,
      failure: null,
    }

    const wrapper = mount(ReleaseResultPanel, {
      props: { session, exportPath: '', busy: false },
    })

    expect(wrapper.text()).toContain('Release 已公开，但发布收尾失败')
    expect(wrapper.text()).toContain('在线复核未完成')
    expect(wrapper.text()).toContain('历史 Release 清理：未执行或未确认')
    expect(wrapper.text()).not.toContain('历史 Release 清理已完成')
  })
})
