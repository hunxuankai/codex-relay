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

export interface RepositoryInspection {
  localBranch: string
  defaultBranch: string
  headSha: string
  remoteMainSha: string
  remoteUrl: string
  clean: boolean
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
}

export type ReleaseEvent =
  | { kind: 'sessionUpdated'; session: ReleaseSession }
  | { kind: 'stepStarted'; stepId: string; startedAt: string }
  | { kind: 'stepLog'; stepId: string; message: string }
  | { kind: 'stepCompleted'; stepId: string; completedAt: string; durationMillis: number }
  | { kind: 'stepFailed'; stepId: string; code: string; message: string }
  | { kind: 'draftReady'; draft: DraftAuditEvidence }
  | { kind: 'releasePublished'; published: PublishedReleaseEvidence }
