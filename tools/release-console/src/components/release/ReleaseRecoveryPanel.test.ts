import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ReleasePhase, ReleaseSession } from '../../types/release'
import ReleaseRecoveryPanel from './ReleaseRecoveryPanel.vue'

function session(phase: ReleasePhase): ReleaseSession {
  return {
    id: 'session-1',
    repositoryPath: 'D:\\safe-temp\\repository',
    targetVersion: '0.5.0',
    phase,
    candidateSha: 'a'.repeat(40),
    remoteMainSha: null,
    workflow: null,
    draft: null,
    published: null,
    cleanup: null,
    cleanupWarning: null,
    failure: null,
  }
}

function failedMonitorSession(
  code = 'RELEASE_REMOTE_FAILED',
  stepId = 'releasePipeline',
): ReleaseSession {
  const failed = session('failed')
  failed.remoteMainSha = failed.candidateSha
  failed.workflow = {
    runId: 42,
    url: 'https://github.com/hunxuankai/codex-relay/actions/runs/42',
  }
  failed.failure = { phase: 'workflowRunning', stepId, code }
  return failed
}

describe('ReleaseRecoveryPanel', () => {
  it.each([
    ['localChecks', '取消并验证回滚'],
    ['committed', '继续 Push'],
    ['workflowRunning', '继续监控'],
    ['awaitingPublishApproval', '查看并确认公开'],
    ['completed', '查看上次结果'],
    ['failed', '查看上次结果'],
  ] as const)('projects phase %s to the correct recovery action', (phase, label) => {
    const wrapper = mount(ReleaseRecoveryPanel, {
      props: { session: session(phase), busy: false },
    })

    expect(wrapper.text()).toContain('v0.5.0')
    expect(wrapper.text()).toContain('aaaaaaaaaaaa')
    expect(wrapper.text()).toContain(label)
  })

  it('emits the committed recovery action without owning async state', async () => {
    const wrapper = mount(ReleaseRecoveryPanel, {
      props: { session: session('committed'), busy: false },
    })

    await wrapper.get('[data-testid="recovery-action-button"]').trigger('click')
    expect(wrapper.emitted('resume')).toHaveLength(1)
  })

  it('offers same-run monitoring recovery for a legacy failed session', async () => {
    const wrapper = mount(ReleaseRecoveryPanel, {
      props: { session: failedMonitorSession(), busy: false },
    })

    const button = wrapper.get('[data-testid="recovery-action-button"]')
    expect(button.text()).toBe('继续监控')
    await button.trigger('click')
    expect(wrapper.emitted('resume')).toHaveLength(1)
    expect(wrapper.emitted('viewResult')).toBeUndefined()
  })

  it.each(['GITHUB_COMMAND_FAILED', 'GITHUB_PROCESS_TIMEOUT', 'GITHUB_RUN_TIMEOUT'])(
    'offers recovery for the recorded %s monitoring interruption', (code) => {
      const wrapper = mount(ReleaseRecoveryPanel, {
        props: { session: failedMonitorSession(code, 'remoteRun'), busy: false },
      })
      expect(wrapper.get('[data-testid="recovery-action-button"]').text()).toBe('继续监控')
    },
  )

  it.each(['GITHUB_RUN_FAILED', 'GITHUB_RESPONSE_INVALID', 'GITHUB_PROCESS_TREE_TERMINATION_FAILED'])(
    'keeps %s failures at the result action', (code) => {
      const wrapper = mount(ReleaseRecoveryPanel, {
        props: { session: failedMonitorSession(code, 'remoteRun'), busy: false },
      })
      expect(wrapper.get('[data-testid="recovery-action-button"]').text()).toBe('查看上次结果')
    },
  )

  it('hides recovery when the saved run evidence is incomplete or belongs to a later phase', async () => {
    const current = failedMonitorSession()
    const wrapper = mount(ReleaseRecoveryPanel, { props: { session: current, busy: false } })
    const variants: ReleaseSession[] = [
      { ...current, remoteMainSha: null },
      { ...current, workflow: null },
      { ...current, workflow: { runId: 42, url: 'https://example.invalid/run/42' } },
      { ...current, cleanupWarning: '已进入清理阶段' },
      { ...current, cleanupWarning: '' },
      { ...current, failure: { phase: 'auditingDraft', stepId: 'releasePipeline', code: 'RELEASE_REMOTE_FAILED' } },
    ]
    for (const value of variants) {
      await wrapper.setProps({ session: value })
      expect(wrapper.get('[data-testid="recovery-action-button"]').text()).toBe('查看上次结果')
    }
  })

  it.each([
    { busy: true, proxyInvalid: false },
    { busy: false, proxyInvalid: true },
  ])('blocks recovered monitoring for busy or invalid proxy state: %o', async (state) => {
    const wrapper = mount(ReleaseRecoveryPanel, {
      props: { session: failedMonitorSession(), ...state },
    })
    const button = wrapper.get('[data-testid="recovery-action-button"]')
    expect(button.attributes('disabled')).toBeDefined()
    await button.trigger('click')
    expect(wrapper.emitted('resume')).toBeUndefined()
  })

  it('blocks only network recovery actions when the proxy settings are invalid', async () => {
    const committed = mount(ReleaseRecoveryPanel, {
      props: { session: session('committed'), busy: false, proxyInvalid: true },
    })

    expect(committed.get('[data-testid="recovery-action-button"]').attributes('disabled')).toBeDefined()
    expect(committed.text()).toContain('先修正代理设置')

    const local = mount(ReleaseRecoveryPanel, {
      props: { session: session('localChecks'), busy: false, proxyInvalid: true },
    })
    const cancel = local.get('[data-testid="recovery-action-button"]')

    expect(cancel.attributes('disabled')).toBeUndefined()
    await cancel.trigger('click')
    expect(local.emitted('cancel')).toHaveLength(1)
  })
})
