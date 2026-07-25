import { describe, expect, it } from 'vitest'
import { formatReleaseDate, renderReleaseNotes } from './updatePresentation'

describe('update presentation utilities', () => {
  it('keeps an invalid release date readable as a fallback', () => {
    expect(formatReleaseDate('not-a-date')).toBe('not-a-date')
  })

  it('renders common markdown while removing unsafe elements and URLs', () => {
    const html = renderReleaseNotes(
      '# 更新\n\n- 修复\n\n<script>alert(1)</script>\n\n[危险链接](javascript:alert(1))\n\n![外部图片](https://example.com/image.png)',
    )
    const container = document.createElement('div')
    container.innerHTML = html

    expect(container.querySelector('h1')?.textContent).toBe('更新')
    expect(container.querySelectorAll('li')).toHaveLength(1)
    expect(container.querySelector('script')).toBeNull()
    expect(container.querySelector('img')).toBeNull()
    expect(container.querySelector('a[href^="javascript:"]')).toBeNull()
  })
})
