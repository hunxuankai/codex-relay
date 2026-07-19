import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const styles = readFileSync('src/style.css', 'utf8')
const confirmDialog = readFileSync('src/components/ConfirmDialog.vue', 'utf8')

describe('global Windows visual system', () => {
  it('uses reusable color tokens for light and dark themes', () => {
    expect(styles).toContain('--surface:')
    expect(styles).toContain('--text-primary:')
    expect(styles).toContain('@media (prefers-color-scheme: dark)')
  })

  it('separates danger text from the high-contrast danger button background', () => {
    expect(styles).toContain('--danger-button-background:')
    expect(styles).toContain('--on-danger:')
    expect(confirmDialog).toContain('background: var(--danger-button-background)')
    expect(confirmDialog).toContain('color: var(--on-danger)')
  })

  it('keeps interactive targets large and keyboard focus visible', () => {
    expect(styles).toMatch(/button[\s\S]*min-height:\s*44px/)
    expect(styles).toContain(':focus-visible')
  })

  it('provides a narrow-window layout without a fixed desktop width', () => {
    expect(styles).toMatch(/@media \(max-width:\s*\d+px\)/)
    expect(styles).not.toMatch(/\.app-shell[\s\S]*width:\s*9\d\dpx/)
  })
})
