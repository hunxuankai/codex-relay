import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ReleasePlanPanel from './ReleasePlanPanel.vue'

describe('ReleasePlanPanel', () => {
  it('shows all planned files and requires regeneration after notes are edited', async () => {
    const wrapper = mount(ReleasePlanPanel, {
      props: {
        notes: '最终说明',
        busy: false,
        plan: {
          id: 'plan-1',
          repositoryPath: 'D:\\safe-temp\\repository',
          previousVersion: '0.4.0',
          targetVersion: '0.5.0',
          notes: '最终说明',
          files: [
            {
              relativePath: 'package.json',
              beforeSha256: 'a'.repeat(64),
              afterSha256: 'b'.repeat(64),
            },
            {
              relativePath: '.github/release-notes.md',
              beforeSha256: 'c'.repeat(64),
              afterSha256: 'd'.repeat(64),
            },
          ],
        },
      },
    })

    expect(wrapper.text()).toContain('package.json')
    expect(wrapper.text()).toContain('.github/release-notes.md')
    expect(wrapper.get('[data-testid="start-release-button"]').attributes('disabled')).toBeUndefined()

    await wrapper.get('textarea[aria-label="发布说明"]').setValue('维护者编辑后的说明')
    await wrapper.setProps({ notes: '维护者编辑后的说明' })
    expect(wrapper.text()).toContain('说明已修改，请重新生成计划')
    expect(wrapper.get('[data-testid="start-release-button"]').attributes('disabled')).toBeDefined()
  })
})
