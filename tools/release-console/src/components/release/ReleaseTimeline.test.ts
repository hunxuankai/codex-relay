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
    failure: null,
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

  it.each([
    {
      stepId: 'release-structure-tests',
      previousLabel: '版本事务',
      failedLabel: '发布专项',
      nextLabel: '完整检查',
    },
    {
      stepId: 'release-console-rust-tests',
      previousLabel: '版本事务',
      failedLabel: '发布专项',
      nextLabel: '完整检查',
    },
    {
      stepId: 'full-project-check',
      previousLabel: '发布专项',
      failedLabel: '完整检查',
      nextLabel: '普通构建',
    },
    {
      stepId: 'ordinary-build',
      previousLabel: '完整检查',
      failedLabel: '普通构建',
      nextLabel: '源码审计',
    },
  ])(
    'restores persisted $stepId failure at $failedLabel without live events',
    ({ stepId, previousLabel, failedLabel, nextLabel }) => {
      const failedSession: ReleaseSession = {
        ...session('failed'),
        failure: {
          phase: 'localChecks',
          stepId,
          code: 'RELEASE_LOCAL_VERIFICATION_FAILED',
        },
      }
      const wrapper = mount(ReleaseTimeline, {
        props: {
          session: failedSession,
          events: [],
        },
      })
      const stepText = Object.fromEntries(
        wrapper.findAll('[data-release-step]').map((step) => {
          const label = step.get('strong').text()
          return [label, step.text()]
        }),
      )

      expect(stepText[previousLabel]).toContain('已完成')
      expect(stepText[failedLabel]).toContain('失败')
      expect(stepText[nextLabel]).toContain('未开始')
      expect(wrapper.get('.is-failed').text()).toContain(failedLabel)
    },
  )

  it('uses live failure events only as a fallback for legacy sessions', () => {
    const wrapper = mount(ReleaseTimeline, {
      props: {
        session: session('failed'),
        events: [
          { kind: 'sessionUpdated', session: session('localChecks') },
          {
            kind: 'stepFailed',
            stepId: 'release-console-rust-tests',
            code: 'RELEASE_LOCAL_VERIFICATION_FAILED',
            message: '发布控制台 Rust 测试失败。',
          },
        ],
      },
    })

    expect(wrapper.get('.is-failed').text()).toContain('发布专项')
    expect(wrapper.get('.is-failed').text()).toContain('失败')
  })

  it('does not invent a failed step for legacy sessions without evidence', () => {
    const wrapper = mount(ReleaseTimeline, {
      props: {
        session: session('failed'),
        events: [],
      },
    })

    expect(wrapper.find('.is-failed').exists()).toBe(false)
    expect(wrapper.findAll('[data-release-step]').every((step) => step.text().includes('未开始')))
      .toBe(true)
  })
})
