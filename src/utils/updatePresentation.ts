import DOMPurify from 'dompurify'
import { marked, Renderer } from 'marked'

const releaseDateFormatter = new Intl.DateTimeFormat('zh-CN', {
  dateStyle: 'medium',
  timeStyle: 'short',
})

const releaseNotesRenderer = new Renderer()
releaseNotesRenderer.html = ({ text }) => escapeHtml(text)

const releaseNotesSanitizeOptions = {
  USE_PROFILES: { html: true },
  FORBID_ATTR: ['style'],
  FORBID_TAGS: [
    'audio',
    'embed',
    'form',
    'iframe',
    'img',
    'input',
    'object',
    'script',
    'style',
    'svg',
    'video',
  ],
}

function escapeHtml(value: string) {
  return value.replace(/[&<>'"]/g, (character) => {
    switch (character) {
      case '&':
        return '&amp;'
      case '<':
        return '&lt;'
      case '>':
        return '&gt;'
      case "'":
        return '&#39;'
      default:
        return '&quot;'
    }
  })
}

export function formatReleaseDate(value: string): string {
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? value : releaseDateFormatter.format(date)
}

export function renderReleaseNotes(notes: string): string {
  const html = marked.parse(notes, {
    async: false,
    breaks: true,
    gfm: true,
    renderer: releaseNotesRenderer,
  })

  return DOMPurify.sanitize(html, releaseNotesSanitizeOptions)
}
