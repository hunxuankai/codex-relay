import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PublishConfirmDialog from './PublishConfirmDialog.vue'

describe('PublishConfirmDialog', () => {
  it('shows the exact irreversible identity and defaults keyboard focus to cancel', async () => {
    const wrapper = mount(PublishConfirmDialog, {
      attachTo: document.body,
      props: {
        modelValue: true,
        busy: false,
        identity: {
          releaseId: 42,
          tagName: 'v0.5.0',
          targetCommitSha: 'a'.repeat(40),
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(document.body.textContent).toContain('Release ID 42')
    expect(document.body.textContent).toContain('v0.5.0')
    expect(document.body.textContent).toContain('不可撤销')
    expect((document.activeElement as HTMLElement | null)?.textContent).toContain('取消')

    const confirm = wrapper.find('[data-testid="confirm-publish-button"]')
    await confirm.trigger('click')
    expect(wrapper.emitted('confirm')).toHaveLength(1)
    wrapper.unmount()
  })
})
