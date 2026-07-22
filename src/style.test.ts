import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const styles = readFileSync('src/style.css', 'utf8')
const app = readFileSync('src/App.vue', 'utf8')
const confirmDialog = readFileSync('src/components/ConfirmDialog.vue', 'utf8')
const selfCheckErrorBanner = readFileSync('src/components/SelfCheckErrorBanner.vue', 'utf8')

describe('global Windows visual system', () => {
  it('uses reusable color tokens for light and dark themes', () => {
    expect(styles).toContain('--surface:')
    expect(styles).toContain('--text-primary:')
    expect(styles).toContain('--el-color-primary: var(--accent)')
    expect(styles).toContain('--el-bg-color: var(--surface)')
    expect(styles).toContain('@media (prefers-color-scheme: dark)')
  })

  it('separates danger text from the high-contrast danger button background', () => {
    expect(styles).toContain('--danger-button-background:')
    expect(styles).toContain('--on-danger:')
    expect(confirmDialog).toContain("props.tone === 'danger' ? 'danger' : 'primary'")
  })

  it('keeps interactive targets large and keyboard focus visible', () => {
    expect(styles).toMatch(/\.el-button[\s\S]*min-height:\s*44px/)
    expect(styles).toMatch(/\.el-input__wrapper[\s\S]*min-height:\s*44px/)
    expect(styles).toContain(':focus-visible')
  })

  it('does not override Element Plus internal buttons and inputs with legacy element selectors', () => {
    expect(styles).not.toMatch(/\nbutton\s*\{/)
    expect(styles).not.toContain('button:hover:not(:disabled)')
    expect(styles).not.toContain("input:not([type='checkbox'])")
    expect(styles).not.toContain("input[type='checkbox']")
  })

  it('keeps disabled semantic buttons readable instead of using white text on pale fills', () => {
    expect(styles).toContain('.el-button--primary.is-disabled')
    expect(styles).toContain('--el-button-disabled-text-color: var(--accent-strong)')
    expect(styles).toContain('.el-button--danger.is-disabled')
    expect(styles).toContain('--el-button-disabled-text-color: var(--danger)')
  })

  it('provides a narrow-window layout without a fixed desktop width', () => {
    expect(styles).toMatch(/@media \(max-width:\s*\d+px\)/)
    expect(styles).not.toMatch(/\.app-shell[\s\S]*width:\s*9\d\dpx/)
  })

  it('keeps the self-check error banner prominent across themes and narrow windows', () => {
    expect(selfCheckErrorBanner).toContain('color: var(--danger)')
    expect(selfCheckErrorBanner).toContain('background: var(--danger-soft)')
    expect(selfCheckErrorBanner).toMatch(/@media \(max-width:\s*\d+px\)/)
  })

  it('keeps the top notification borders inset from the window edges', () => {
    expect(app).toMatch(/\.app-notification-slot\s*{[\s\S]*?margin-inline:\s*1\.25rem/)
  })
})
