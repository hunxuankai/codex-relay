import type { ReleaseProxySettings } from './network'

export type ReleasePhase =
  | 'idle'
  | 'inspected'
  | 'planned'
  | 'applyingCandidate'
  | 'localChecks'
  | 'localBuild'
  | 'sourceAudit'
  | 'committed'
  | 'pushed'
  | 'workflowQueued'
  | 'workflowRunning'
  | 'auditingDraft'
  | 'awaitingPublishApproval'
  | 'publishing'
  | 'verifyingPublishedRelease'
  | 'monitoringCleanup'
  | 'completed'
  | 'completedWithWarnings'
  | 'failed'
  | 'cancelled'

export interface CommandError {
  code: string
  message: string
}

export interface CommandResult<T> {
  success: boolean
  data?: T
  error?: CommandError
}

export interface ToolchainInspection {
  git: string | null
  node: string | null
  npm: string | null
  cargo: string | null
  gh: string | null
}

export type RepositorySyncStatus = 'synced' | 'ahead' | 'behind' | 'diverged'

export interface RepositoryCommitSummary {
  sha: string
  subject: string
}

export interface RepositorySyncInspection {
  status: RepositorySyncStatus
  aheadCount: number
  behindCount: number
  aheadCommits: readonly RepositoryCommitSummary[]
}

export interface RepositoryInspection {
  localBranch: string
  defaultBranch: string
  headSha: string
  remoteMainSha: string
  remoteUrl: string
  clean: boolean
  sync: RepositorySyncInspection
}

export interface SafeRepositoryPushPreview {
  expectedHeadSha: string
  expectedRemoteMainSha: string
  commitCount: number
  commits: readonly RepositoryCommitSummary[]
}

export interface SafeRepositoryPushRequest {
  repositoryPath: string
  expectedHeadSha: string
  expectedRemoteMainSha: string
  proxy: ReleaseProxySettings
}

export interface ReleasePreflightResult {
  repositoryPath: string
  repository: RepositoryInspection
  external: {
    tools: ToolchainInspection
    activeReleaseRuns: number
    conflictingDrafts: number
    latestReleaseTag: string | null
  }
  releaseReady: boolean
  blockingReasons: readonly string[]
  safePush: SafeRepositoryPushPreview | null
}

export interface ReleasePlanFileSummary {
  relativePath: string
  beforeSha256: string
  afterSha256: string
}

export interface ReleasePlanSummary {
  id: string
  repositoryPath: string
  previousVersion: string
  targetVersion: string
  notes: string
  files: readonly ReleasePlanFileSummary[]
}

export interface WorkflowDispatch {
  runId: number
  url: string
}

export interface WorkflowStepStatus {
  name: string
  number: number
  status: string
  conclusion: string | null
  startedAt: string | null
  completedAt: string | null
  durationMillis: number | null
}

export interface WorkflowJobStatus {
  name: string
  status: string
  conclusion: string | null
  startedAt: string | null
  completedAt: string | null
  durationMillis: number | null
  steps: readonly WorkflowStepStatus[]
}

export interface DraftAssetEvidence {
  id: number
  name: string
  size: number
  sha256: string
}

export interface DraftIdentity {
  releaseId: number
  tagName: string
  targetCommitSha: string
}

export interface DraftAuditEvidence extends DraftIdentity {
  assets: readonly DraftAssetEvidence[]
  manifestVersion: string
  manifestNotes: string
  signature: string
}

export interface PublishedReleaseEvidence {
  releaseId: number
  tagName: string
  publishedAt: string
}

export interface CleanupRunEvidence {
  runId: number
  url: string
  status: string
  conclusion: string | null
  succeeded: boolean
  jobs: readonly WorkflowJobStatus[]
}

export interface ReleaseFailureEvidence {
  phase: ReleasePhase
  stepId: string
  code: string
}

export type ReleaseLogSource = 'lifecycle' | 'stdout' | 'stderr'

export type ReleaseLogLevel = 'info' | 'warning' | 'error'

export type ReleaseLogViewMode = 'latest' | 'history'

export interface ReleaseLogEntry {
  sessionId: string
  sequence: number
  timestamp: string
  stepId: string
  source: ReleaseLogSource
  level: ReleaseLogLevel
  message: string
}

export interface ReleaseLogPage {
  entries: readonly ReleaseLogEntry[]
  nextBeforeSequence: number | null
  hasEarlier: boolean
  totalEntries: number
  totalBytes: number
  truncated: boolean
  warning: string | null
}

export interface ReleaseSession {
  id: string
  repositoryPath: string
  targetVersion: string
  phase: ReleasePhase
  candidateSha: string | null
  remoteMainSha: string | null
  workflow: WorkflowDispatch | null
  draft: DraftAuditEvidence | null
  published: PublishedReleaseEvidence | null
  cleanup: CleanupRunEvidence | null
  cleanupWarning: string | null
  failure: ReleaseFailureEvidence | null
}

export interface ReleaseSessionSnapshot {
  session: ReleaseSession
  logs: ReleaseLogPage
}

export type ReleaseEvent =
  | { kind: 'sessionUpdated'; session: ReleaseSession }
  | { kind: 'stepStarted'; stepId: string; startedAt: string }
  | { kind: 'stepLog'; entry: ReleaseLogEntry; page?: ReleaseLogPage }
  | { kind: 'stepCompleted'; stepId: string; completedAt: string; durationMillis: number }
  | { kind: 'stepFailed'; stepId: string; code: string; message: string }
  | { kind: 'draftReady'; draft: DraftAuditEvidence }
  | { kind: 'releasePublished'; published: PublishedReleaseEvidence }
