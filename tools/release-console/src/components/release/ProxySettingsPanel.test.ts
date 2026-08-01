import { mount } from '@vue/test-utils'
import { ElInput, ElInputNumber, ElSelect, ElSwitch } from 'element-plus'
import { describe, expect, it } from 'vitest'
import ProxySettingsPanel from './ProxySettingsPanel.vue'

describe('ProxySettingsPanel', () => {
  it('edits the explicit proxy switch, type, address and port through typed events', async () => {
    const wrapper = mount(ProxySettingsPanel, {
      props: {
        settings: {
          enabled: false,
          proxyType: 'http',
          host: '',
          port: null,
        },
        result: null,
        busy: false,
        error: null,
      },
    })

    expect(wrapper.text()).toContain('网络代理')
    expect(wrapper.text()).toContain('关闭时强制直连')
    expect(wrapper.get('[data-testid="test-connection-button"]').attributes('disabled')).toBeUndefined()

    wrapper.findComponent(ElSwitch).vm.$emit('change', true)
    wrapper.findComponent(ElSelect).vm.$emit('update:modelValue', 'socks5')
    wrapper.findComponent(ElInput).vm.$emit('update:modelValue', '127.0.0.1')
    wrapper.findComponent(ElInputNumber).vm.$emit('update:modelValue', 1080)
    await wrapper.vm.$nextTick()

    expect(wrapper.emitted('update:settings')).toEqual([
      [{ enabled: true, proxyType: 'http', host: '', port: null }],
      [{ enabled: false, proxyType: 'socks5', host: '', port: null }],
      [{ enabled: false, proxyType: 'http', host: '127.0.0.1', port: null }],
      [{ enabled: false, proxyType: 'http', host: '', port: 1080 }],
    ])
  })

  it('shows enabled-mode validation and independent Git and GitHub results', async () => {
    const wrapper = mount(ProxySettingsPanel, {
      props: {
        settings: {
          enabled: true,
          proxyType: 'http',
          host: '',
          port: null,
        },
        result: null,
        busy: false,
        error: null,
      },
    })

    expect(wrapper.text()).toContain('填写代理地址')
    expect(wrapper.get('[data-testid="test-connection-button"]').attributes('disabled')).toBeDefined()

    await wrapper.setProps({
      settings: {
        enabled: true,
        proxyType: 'http',
        host: 'http://127.0.0.1',
        port: 7890,
      },
    })
    expect(wrapper.text()).toContain('只填写主机名、IPv4 或 IPv6')
    expect(wrapper.get('[data-testid="test-connection-button"]').attributes('disabled')).toBeDefined()

    await wrapper.setProps({
      settings: {
        enabled: true,
        proxyType: 'http',
        host: '999.999.999.999',
        port: 7890,
      },
    })
    expect(wrapper.get('[data-testid="test-connection-button"]').attributes('disabled')).toBeDefined()

    await wrapper.setProps({
      settings: {
        enabled: true,
        proxyType: 'http',
        host: '::::',
        port: 7890,
      },
    })
    expect(wrapper.get('[data-testid="test-connection-button"]').attributes('disabled')).toBeDefined()

    await wrapper.setProps({
      settings: {
        enabled: true,
        proxyType: 'http',
        host: '127.0.0.1',
        port: 7890,
      },
      result: {
        git: {
          success: true,
          code: null,
          message: 'Git 远端连接正常。',
          durationMillis: 12,
        },
        github: {
          success: false,
          code: 'GITHUB_PROCESS_TIMEOUT',
          message: 'GitHub API 连接超时。',
          durationMillis: 30_000,
        },
      },
    })

    expect(wrapper.get('[data-testid="test-connection-button"]').attributes('disabled')).toBeUndefined()
    expect(wrapper.text()).toContain('Git 远端')
    expect(wrapper.text()).toContain('12 ms')
    expect(wrapper.text()).toContain('GitHub API')
    expect(wrapper.text()).toContain('GITHUB_PROCESS_TIMEOUT')
    expect(wrapper.text()).toContain('30000 ms')
  })
})
