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
    }

    const wrapper = mount(ReleaseResultPanel, {
      props: { session, exportPath: '', busy: false },
    })

    expect(wrapper.text()).toContain('Release 已公开')
    expect(wrapper.text()).toContain('历史清理失败')
    expect(wrapper.text()).toContain('Sandbox / 安装 / UAC / 应用内升级：未执行')
    expect(wrapper.text()).not.toContain('升级验证成功')
  })
})
