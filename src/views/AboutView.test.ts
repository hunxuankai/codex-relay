import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AboutView from './AboutView.vue'

describe('AboutView', () => {
  it('explains how Codex Relay works and what it modifies', async () => {
    const wrapper = mount(AboutView, {
      props: {
        appVersion: '0.1.2',
        configDirectory: 'C:\\safe-test\\codex',
      },
    })

    expect(wrapper.get('[aria-label="关于 Codex Relay"]').text()).toContain('关于 Codex Relay')
    expect(wrapper.text()).toContain('当前版本：0.1.2')
    expect(wrapper.text()).toContain('C:\\safe-test\\codex')
    expect(wrapper.text()).toContain('工作原理')
    expect(wrapper.text()).toContain('config.toml')
    expect(wrapper.text()).toContain('model_providers')
    expect(wrapper.text()).toContain('model_provider')
    expect(wrapper.text()).toContain('cli_auth_credentials_store')
    expect(wrapper.text()).toContain('auth.json')
    expect(wrapper.text()).toContain('providers.json')
    expect(wrapper.text()).toContain('settings.json')
    expect(wrapper.text()).toContain('transaction.json')
    expect(wrapper.text()).toContain('明文')
    expect(wrapper.text()).toContain('卸载')

    await wrapper.get('[aria-label="打开当前 Codex 配置目录"]').trigger('click')

    expect(wrapper.emitted('openDirectory')).toHaveLength(1)
  })
})
