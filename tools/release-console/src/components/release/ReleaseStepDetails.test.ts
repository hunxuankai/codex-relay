import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ReleaseSession } from '../../types/release'
import ReleaseStepDetails from './ReleaseStepDetails.vue'

const session: ReleaseSession = {
  id: 'session-1',
  repositoryPath: 'D:\\safe-temp\\repository',
  targetVersion: '0.5.0',
  phase: 'committed',
  candidateSha: 'a'.repeat(40),
  remoteMainSha: null,
  workflow: null,
  draft: null,
  published: null,
  cleanup: null,
  cleanupWarning: null,
}

describe('ReleaseStepDetails', () => {
  it('shows session evidence and only the safe log and failure events', () => {
    const wrapper = mount(ReleaseStepDetails, {
      props: {
        session,
        events: [
          { kind: 'sessionUpdated', session },
          { kind: 'stepLog', stepId: 'fullChecks', message: '检查输出已脱敏' },
          {
            kind: 'stepFailed',
            stepId: 'commitPush',
            code: 'RELEASE_PUSH_FAILED',
            message: '推送失败，可从已提交检查点继续。',
          },
        ],
      },
    })

    expect(wrapper.text()).toContain('session-1')
    expect(wrapper.text()).toContain('aaaaaaaaaaaa')
    expect(wrapper.text()).toContain('[fullChecks] 检查输出已脱敏')
    expect(wrapper.text()).toContain(
      '[commitPush] RELEASE_PUSH_FAILED：推送失败，可从已提交检查点继续。',
    )
    expect(wrapper.get('[aria-label="脱敏发布日志"]').attributes('tabindex')).toBe('0')
  })
})
