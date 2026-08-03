import { flushPromises, mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type {
  CommandError,
  ReleaseFailureEvidence,
  ReleaseLogEntry,
  ReleaseLogPage,
  ReleaseLogViewMode,
} from '../../types/release'
import ReleaseLogPanel from './ReleaseLogPanel.vue'
import panelSource from './ReleaseLogPanel.vue?raw'

function entry(sequence: number, message = `诊断记录 ${sequence}`): ReleaseLogEntry {
  return {
    sessionId: 'session-logs',
    sequence,
    timestamp: '2026-08-03T12:34:56.789Z',
    stepId: 'full-project-check',
    source: sequence % 2 === 0 ? 'stderr' : 'stdout',
    level: sequence % 2 === 0 ? 'error' : 'info',
    message,
  }
}

function page(entries: readonly ReleaseLogEntry[] = []): ReleaseLogPage {
  return {
    entries,
    nextBeforeSequence: null,
    hasEarlier: false,
    totalEntries: entries.length,
    totalBytes: entries.length * 128,
    truncated: false,
    warning: null,
  }
}

function mountPanel(options: {
  logPage?: ReleaseLogPage
  logViewMode?: ReleaseLogViewMode
  unreadLogCount?: number
  logRequestPending?: boolean
  logError?: CommandError | null
  failure?: ReleaseFailureEvidence | null
} = {}) {
  return mount(ReleaseLogPanel, {
    props: {
      logPage: options.logPage ?? page(),
      logViewMode: options.logViewMode ?? 'latest',
      unreadLogCount: options.unreadLogCount ?? 0,
      logRequestPending: options.logRequestPending ?? false,
      logError: options.logError ?? null,
      failure: options.failure ?? null,
    },
  })
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('ReleaseLogPanel', () => {
  it('renders an accessible empty state and complete diagnostic evidence', async () => {
    const wrapper = mountPanel()
    const viewport = wrapper.get('[aria-label="发布诊断日志"]')

    expect(viewport.attributes('tabindex')).toBe('0')
    expect(wrapper.text()).toContain('尚无发布诊断日志')
    expect(panelSource).not.toContain('v-html')

    await wrapper.setProps({
      logPage: {
        ...page([entry(41, '第一行\n第二行'), entry(42, '编译失败正文')]),
        nextBeforeSequence: 41,
        hasEarlier: true,
        totalEntries: 42,
        totalBytes: 4_096,
        truncated: true,
        warning: '早期普通输出已截断。',
      },
      unreadLogCount: 3,
      failure: {
        phase: 'localChecks',
        stepId: 'full-project-check',
        code: 'RELEASE_LOCAL_VERIFICATION_FAILED',
      },
    })

    expect(wrapper.text()).toContain('12:34:56.789')
    expect(wrapper.text()).toContain('full-project-check')
    expect(wrapper.text()).toContain('stdout')
    expect(wrapper.text()).toContain('stderr')
    expect(wrapper.text()).toContain('第一行\n第二行')
    expect(wrapper.text()).toContain('41–42 / 42')
    expect(wrapper.text()).toContain('4 KiB')
    expect(wrapper.text()).toContain('早期普通输出已截断。')
    expect(wrapper.text()).toContain('早期日志已截断')
    expect(wrapper.text()).toContain('3 条新日志')
    expect(wrapper.text()).toContain(
      'full-project-check · RELEASE_LOCAL_VERIFICATION_FAILED',
    )
  })

  it('emits typed paging actions and disables them while a request is pending', async () => {
    const wrapper = mountPanel({
      logPage: { ...page([entry(2)]), hasEarlier: true, nextBeforeSequence: 2 },
      logViewMode: 'history',
    })

    await wrapper.get('button[aria-label="读取更早日志"]').trigger('click')
    await wrapper.get('button[aria-label="更新当前日志页"]').trigger('click')
    await wrapper.get('button[aria-label="返回最新日志"]').trigger('click')

    expect(wrapper.emitted('load-earlier')).toHaveLength(1)
    expect(wrapper.emitted('refresh-log-page')).toHaveLength(1)
    expect(wrapper.emitted('return-to-latest')).toHaveLength(1)

    await wrapper.setProps({ logRequestPending: true })
    expect(wrapper.get('button[aria-label="读取更早日志"]').attributes()).toHaveProperty(
      'disabled',
    )
    expect(wrapper.get('button[aria-label="更新当前日志页"]').attributes()).toHaveProperty(
      'disabled',
    )
    expect(wrapper.text()).toContain('正在读取日志')
  })

  it('does not steal scroll while reading history and follows after returning latest', async () => {
    const wrapper = mountPanel({ logPage: page([entry(1)]), logViewMode: 'latest' })
    const viewport = wrapper.get<HTMLElement>('[aria-label="发布诊断日志"]')
    Object.defineProperty(viewport.element, 'clientHeight', { value: 100, configurable: true })
    Object.defineProperty(viewport.element, 'scrollHeight', { value: 500, configurable: true })
    viewport.element.scrollTop = 120
    await viewport.trigger('scroll')

    await wrapper.setProps({ logPage: page([entry(1), entry(2)]) })
    await nextTick()
    expect(viewport.element.scrollTop).toBe(120)

    await wrapper.get('button[aria-label="返回最新日志"]').trigger('click')
    await wrapper.setProps({ logViewMode: 'latest', logPage: page([entry(1), entry(2), entry(3)]) })
    await nextTick()
    expect(viewport.element.scrollTop).toBe(500)
  })

  it('copies only the current safe page and shows a safe clipboard error', async () => {
    const writeText = vi.fn().mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error(
      'clipboard implementation detail',
    ))
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    })
    const wrapper = mountPanel({ logPage: page([entry(7, '安全诊断正文')]) })
    const copy = wrapper.get('button[aria-label="复制当前日志页"]')

    await copy.trigger('click')
    await flushPromises()
    expect(writeText).toHaveBeenCalledTimes(1)
    expect(writeText.mock.calls[0]?.[0]).toContain('安全诊断正文')

    await copy.trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('复制失败，请重试。')
    expect(wrapper.text()).not.toContain('clipboard implementation detail')
  })
})
