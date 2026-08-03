import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { ReleaseSession } from '../../types/release'
import ReleaseStepDetails from './ReleaseStepDetails.vue'
import detailsSource from './ReleaseStepDetails.vue?raw'

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
  failure: null,
}

describe('ReleaseStepDetails', () => {
  it('shows session facts without owning the diagnostic log view', () => {
    const wrapper = mount(ReleaseStepDetails, {
      props: {
        session,
      },
    })

    expect(wrapper.text()).toContain('session-1')
    expect(wrapper.text()).toContain('aaaaaaaaaaaa')
    expect(wrapper.find('[aria-label="脱敏发布日志"]').exists()).toBe(false)
    expect(detailsSource).not.toContain('stepLog')
    expect(detailsSource).not.toContain('log-view')
  })
})
