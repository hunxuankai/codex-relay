import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import RepositorySyncConfirmDialog from './RepositorySyncConfirmDialog.vue'

describe('RepositorySyncConfirmDialog', () => {
  it('shows the exact safe push scope and defaults keyboard focus to cancel', async () => {
    const wrapper = mount(RepositorySyncConfirmDialog, {
      attachTo: document.body,
      props: {
        modelValue: true,
        busy: false,
        remoteUrl: 'https://github.com/hunxuankai/codex-relay.git',
        preview: {
          expectedHeadSha: 'b'.repeat(40),
          expectedRemoteMainSha: 'a'.repeat(40),
          commitCount: 2,
          commits: [
            { sha: 'b'.repeat(40), subject: 'fix: second reviewed commit' },
            { sha: 'c'.repeat(40), subject: 'feat: first reviewed commit' },
          ],
        },
      },
    })

    await new Promise((resolve) => setTimeout(resolve, 0))
    expect(document.body.textContent).toContain('hunxuankai/codex-relay')
    expect(document.body.textContent).toContain('bbbbbbbbbbbb')
    expect(document.body.textContent).toContain('aaaaaaaaaaaa')
    expect(document.body.textContent).toContain('2 个提交')
    expect(document.body.textContent).toContain('fix: second reviewed commit')
    expect(document.body.textContent).toContain('不会推送 Tag 或其他分支')
    expect((document.activeElement as HTMLElement | null)?.textContent).toContain('取消')

    await wrapper.get('[data-testid="confirm-repository-push-button"]').trigger('click')
    expect(wrapper.emitted('confirm')).toHaveLength(1)
    wrapper.unmount()
  })
})
