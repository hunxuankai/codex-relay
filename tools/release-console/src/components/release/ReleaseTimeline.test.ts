import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ReleaseSession } from '../../types/release'
import ReleaseTimeline from './ReleaseTimeline.vue'

function session(phase: ReleaseSession['phase']): ReleaseSession {
  return {
    id: 'session-1',
    repositoryPath: 'D:\\safe-temp\\repository',
    targetVersion: '0.5.0',
    phase,
    candidateSha: 'a'.repeat(40),
    remoteMainSha: 'a'.repeat(40),
    workflow: null,
    draft: null,
    published: null,
    cleanup: null,
    cleanupWarning: null,
  }
}

describe('ReleaseTimeline', () => {
  it('shows textual status for every release stage instead of relying on color', () => {
    const wrapper = mount(ReleaseTimeline, {
      props: {
        session: session('workflowRunning'),
        events: [
          { kind: 'stepStarted', stepId: 'remoteRun', startedAt: '2026-07-31T10:00:00Z' },
          {
            kind: 'stepCompleted',
            stepId: 'localChecks',
            completedAt: '2026-07-31T10:02:00Z',
            durationMillis: 120_000,
          },
        ],
      },
    })

    expect(wrapper.text()).toContain('远端 Run')
    expect(wrapper.text()).toContain('进行中')
    expect(wrapper.text()).toContain('完整检查')
    expect(wrapper.text()).toContain('2分00秒')
    expect(wrapper.findAll('[data-release-step]').length).toBeGreaterThanOrEqual(12)
  })
})
