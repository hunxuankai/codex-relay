import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import DraftAuditPanel from './DraftAuditPanel.vue'

describe('DraftAuditPanel', () => {
  it('shows the audited identity and assets before requesting publish confirmation', async () => {
    const wrapper = mount(DraftAuditPanel, {
      props: {
        busy: false,
        draft: {
          releaseId: 42,
          tagName: 'v0.5.0',
          targetCommitSha: 'a'.repeat(40),
          assets: [
            {
              id: 501,
              name: 'Codex.Relay_0.5.0_x64-setup.exe',
              size: 1024,
              sha256: 'b'.repeat(64),
            },
          ],
          manifestVersion: '0.5.0',
          manifestNotes: '最终发布说明',
          signature: 'signature-test-not-real',
        },
      },
    })

    expect(wrapper.text()).toContain('Draft 审计已通过')
    expect(wrapper.text()).toContain('Release ID42')
    expect(wrapper.text()).toContain('Codex.Relay_0.5.0_x64-setup.exe')
    expect(wrapper.text()).toContain('再次审计同一 Release ID')

    await wrapper.get('button').trigger('click')
    expect(wrapper.emitted('publish')).toHaveLength(1)
  })
})
