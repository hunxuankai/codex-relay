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
    expect(wrapper.text()).toContain('model_reasoning_effort')
    expect(wrapper.text()).toContain('cli_auth_credentials_store')
    expect(wrapper.text()).toContain('auth.json')
    expect(wrapper.text()).toContain('providers.json')
    expect(wrapper.text()).toContain('provider-preferences.json')
    expect(wrapper.text()).toContain('多个命名 Base URL')
    expect(wrapper.text()).toContain('多个命名 API Key')
    expect(wrapper.text()).toContain('持久化 Provider 列表顺序')
    expect(wrapper.text()).toContain('独立切换')
    expect(wrapper.text()).toContain('打开管理器后默认明文显示')
    expect(wrapper.text()).toContain('settings.json')
    expect(wrapper.text()).toContain('transaction.json')
    expect(wrapper.text()).toContain('Windows 记事本打开所选文件')
    expect(wrapper.text()).toContain('明文')
    expect(wrapper.text()).toContain('启动时及运行期间每小时自动检查一次更新')
    expect(wrapper.text()).toContain('自动下载或安装')
    expect(wrapper.text()).toContain('已安装版本升级会沿用原安装目录')
    expect(wrapper.text()).toContain('只有用户显式点击测试时才访问 Provider 模型网络')
    expect(wrapper.text()).toContain('API 可用性测试')
    expect(wrapper.text()).toContain('无工具、非流式、最多 16 个输出 token')
    expect(wrapper.text()).toContain('点击 API 测试后详情弹窗立即打开')
    expect(wrapper.text()).toContain('测试过程中分别显示 loading')
    expect(wrapper.text()).toContain('点击遮罩或关闭按钮关闭')
    expect(wrapper.text()).toContain('不显示 Header、API Key 或代理地址')
    expect(wrapper.text()).toContain('Codex 兼容性测试')
    expect(wrapper.text()).toContain('一次正常 Codex 回合')
    expect(wrapper.text()).toContain('不会修改当前 config.toml 或 auth.json')
    expect(wrapper.text()).toContain('测试结果只保存在本次会话内')
    expect(wrapper.text()).toContain('卸载')
    expect(wrapper.text()).not.toContain('从界面清空密钥')

    await wrapper.get('[aria-label="打开当前 Codex 配置目录"]').trigger('click')

    expect(wrapper.emitted('openDirectory')).toHaveLength(1)
  })
})
