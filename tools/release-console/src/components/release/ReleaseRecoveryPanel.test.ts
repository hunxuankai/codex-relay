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
