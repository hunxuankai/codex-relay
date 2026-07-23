import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import ImportCurrentApiKeyDialog from './ImportCurrentApiKeyDialog.vue'

describe('ImportCurrentApiKeyDialog', () => {
  it('requires a name and emits only the trimmed name', async () => {
    const wrapper = mount(ImportCurrentApiKeyDialog, {
      props: { open: true, providerName: 'Provider A', busy: false },
      global: { stubs: { teleport: true } },
    })
    await flushPromises()

    await wrapper.get('[aria-label="确认导入当前密钥"]').trigger('click')
    expect(wrapper.text()).toContain('密钥名称为必填项')
    expect(wrapper.emitted('import')).toBeUndefined()

    await wrapper.get('[name="import-api-key-name"]').setValue('  从 Codex 导入  ')
    await wrapper.get('[aria-label="确认导入当前密钥"]').trigger('click')

    expect(wrapper.emitted('import')?.[0]).toEqual(['从 Codex 导入'])
    expect(JSON.stringify(wrapper.emitted())).not.toContain('test-key')
  })

  it('requests close without importing', async () => {
    const wrapper = mount(ImportCurrentApiKeyDialog, {
      props: { open: true, providerName: 'Provider A', busy: false },
      global: { stubs: { teleport: true } },
    })
    await flushPromises()

    await wrapper.get('[aria-label="取消导入当前密钥"]').trigger('click')

    expect(wrapper.emitted('close')).toEqual([[]])
    expect(wrapper.emitted('import')).toBeUndefined()
  })
})
