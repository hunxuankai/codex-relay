import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const cardFiles = [
  'src/components/ProviderList.vue',
  'src/components/BackupCard.vue',
  'src/components/UpdatePanel.vue',
  'src/views/SettingsView.vue',
  'src/views/OnboardingView.vue',
  'src/views/AboutView.vue',
]

describe('Element Plus layout integration', () => {
  it.each(cardFiles)('%s places content layout on the ElCard body', (path) => {
    expect(readFileSync(path, 'utf8')).toContain(':deep(.el-card__body)')
  })

  it('keeps Provider name and ID in an explicit full-width button content row', () => {
    const source = readFileSync('src/components/ProviderList.vue', 'utf8')
    expect(source).toContain('class="provider-select-content"')
    expect(source).toContain('.provider-select-content')
    expect(source).toContain(':deep(.provider-select > span)')
  })

  it('applies navigation icon spacing to the content wrapper created by ElButton', () => {
    expect(readFileSync('src/App.vue', 'utf8')).toContain('.app-nav :deep(.el-button > span)')
  })

  it('keeps the Provider detail summary and common switchers compact on desktop', () => {
    const source = readFileSync('src/views/ProvidersView.vue', 'utf8')

    expect(source).toMatch(
      /\.selected-provider-fields\s*\{[\s\S]*?grid-template-columns:\s*repeat\(3,\s*minmax\(0,\s*1fr\)\)/,
    )
    expect(source).toMatch(
      /\.provider-switch-controls\s*\{[\s\S]*?grid-template-columns:\s*repeat\(2,\s*minmax\(0,\s*1fr\)\)/,
    )
    expect(source).toMatch(/@media \(max-width:\s*620px\)[\s\S]*?\.provider-switch-controls/)
  })

  it('sizes Provider switchers by content instead of stretching every option', () => {
    for (const path of [
      'src/components/ProviderEndpointControls.vue',
      'src/components/ProviderCredentialControls.vue',
    ]) {
      const source = readFileSync(path, 'utf8')
      expect(source).toContain('size="small"')
      expect(source).toMatch(/\.segmented-scroll :deep\(\.el-segmented\)\s*\{[\s\S]*?width:\s*max-content/)
      expect(source).toMatch(/\.control-header :deep\(\.el-button\)\s*\{[\s\S]*?width:\s*auto/)
      expect(source).not.toContain('min-width: 100%')
    }
  })
})
