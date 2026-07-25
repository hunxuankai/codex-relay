import { existsSync, readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'

const cleanupWorkflowPath = '.github/workflows/cleanup-old-releases.yml'
const cleanupWorkflow = existsSync(cleanupWorkflowPath)
  ? readFileSync(cleanupWorkflowPath, 'utf8')
  : ''

describe('GitHub Release 历史版本清理契约', () => {
  it('只在正式 Release 发布后运行，并提供手动重试入口', () => {
    expect(cleanupWorkflow).not.toBe('')
    expect(cleanupWorkflow).toMatch(/release:\s*\n\s+types:\s*\n\s+- published/)
    expect(cleanupWorkflow).toContain('workflow_dispatch:')
    expect(cleanupWorkflow).toContain('contents: write')
    expect(cleanupWorkflow).toContain('github.event.release.id')
    expect(cleanupWorkflow).toContain('github.event.release.tag_name')
    expect(cleanupWorkflow).toContain('github.event.release.prerelease == false')
  })

  it('以 releases/latest 为保留对象并防止 Draft/并发状态误删', () => {
    expect(cleanupWorkflow).toContain('releases/latest')
    expect(cleanupWorkflow).toContain('EVENT_RELEASE_ID')
    expect(cleanupWorkflow).toContain('EVENT_RELEASE_TAG')
    expect(cleanupWorkflow).toContain('concurrency:')
    expect(cleanupWorkflow).toContain('cancel-in-progress: false')
    expect(cleanupWorkflow).toContain('keep_id')
    expect(cleanupWorkflow).toContain('keep_tag')
    expect(cleanupWorkflow).toContain('releases/latest')
    expect(cleanupWorkflow).toContain('releases/latest 不是可保留的正式 Release')
    expect(cleanupWorkflow).toMatch(/draft|prerelease/i)
  })

  it('保留 GitHub API 的 false 布尔值，不用 jq 的 // 把它误判为 true', () => {
    expect(cleanupWorkflow).toContain("keep_draft=\"$(jq -r '.draft | tostring'")
    expect(cleanupWorkflow).toContain("keep_prerelease=\"$(jq -r '.prerelease | tostring'")
    expect(cleanupWorkflow).not.toContain(".draft // true")
    expect(cleanupWorkflow).not.toContain(".prerelease // true")
  })

  it('分页读取候选并同时删除旧 Release、资产和对应 tag，失败可见', () => {
    expect(cleanupWorkflow).toContain('GH_TOKEN: ${{ github.token }}')
    expect(cleanupWorkflow).toContain('--paginate')
    expect(cleanupWorkflow).toContain('set -euo pipefail')
    expect(cleanupWorkflow).toContain('git/refs/tags/')
    expect(cleanupWorkflow).toContain('select(((.id | tostring) != $keep_id) and (.tag_name != $keep_tag))')
    expect(cleanupWorkflow).toContain('Release 列表未包含当前 Latest')
    expect(cleanupWorkflow).toContain('[[ "$release_id" == "$keep_id" || "$release_tag" == "$keep_tag" ]]')
    expect(cleanupWorkflow).toMatch(/releases\/\$\{?[^}]*id|releases\/\$release_id/)
    expect(cleanupWorkflow).toMatch(/--method DELETE|-X DELETE/)
    expect(cleanupWorkflow).toMatch(/::error::|exit 1|set -euo pipefail/)
    expect(cleanupWorkflow).not.toMatch(/echo\s+.*GH_TOKEN|echo\s+.*github\.token/)
    expect(cleanupWorkflow).not.toContain('git/refs/tags/*')
  })
})
