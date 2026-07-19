import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import type { HealthReport } from '../types/health'
import HealthStatus from './HealthStatus.vue'

function report(level: HealthReport['level']): HealthReport {
  return {
    level,
    configDirectory: 'C:\\test\\codex',
    currentProvider: 'provider-a',
    generatedAt: '2026-07-20T00:00:00+08:00',
    checks: [
      {
        id: 'codex-cli',
        label: 'Codex CLI',
        level: 'warning',
        message: '未找到 codex 命令，但不影响 Provider 管理。',
      },
    ],
  }
}

describe('HealthStatus', () => {
  it.each([
    ['normal', '正常'],
    ['warning', '警告'],
    ['error', '错误'],
  ] as const)('renders the %s summary state', (level, label) => {
    const wrapper = mount(HealthStatus, {
      props: { report: report(level), loading: false, busy: false, errorMessage: null },
    })

    expect(wrapper.get('[aria-label="自检状态"]').text()).toContain(label)
  })

  it('keeps a missing Codex CLI as a visible warning and can rerun checks', async () => {
    const wrapper = mount(HealthStatus, {
      props: { report: report('warning'), loading: false, busy: false, errorMessage: null },
    })

    const cliCheck = wrapper.get('[data-check-id="codex-cli"]')
    expect(cliCheck.attributes('data-level')).toBe('warning')
    expect(cliCheck.text()).toContain('不影响 Provider 管理')

    await wrapper.get('[aria-label="重新运行完整自检"]').trigger('click')
    expect(wrapper.emitted('rerun')).toHaveLength(1)
  })
})
