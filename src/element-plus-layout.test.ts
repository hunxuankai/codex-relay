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
})
