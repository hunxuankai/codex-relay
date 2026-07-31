import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import App from './App.vue'
import appSource from './App.vue?raw'

describe('release console shell', () => {
  it('keeps App as a composition surface for the visual release workflow', () => {
    const wrapper = mount(App)

    expect(wrapper.get('h1').text()).toBe('Codex Relay 发布控制台')
    expect(wrapper.text()).toContain('仓库与版本')
    expect(wrapper.text()).toContain('阶段时间线')
    expect(wrapper.text()).toContain('加载活动会话')
    expect(wrapper.get('.release-console-layout')).toBeTruthy()
    expect(wrapper.text()).toContain('不执行 Sandbox、真实安装、UAC 或应用内升级')
  })

  it('switches to a single column before the configured minimum window width', () => {
    expect(appSource).toContain('@media (max-width: 820px)')
    expect(appSource).toMatch(
      /@media \(max-width: 820px\)[\s\S]*?\.release-console-layout\s*\{[\s\S]*?grid-template-columns:\s*minmax\(0, 1fr\)/,
    )
  })

  it('keeps planning and start controls locked while a release session is active', () => {
    expect(appSource).toContain('const hasActiveSession = computed(')
    expect(appSource).toContain(':busy="release.busy.value || hasActiveSession"')
    expect(appSource).toMatch(/const canCancel = computed\([\s\S]*?'idle'/)
  })
})
