use crate::app_state::{
    ApplicationRequest, ApplicationResponse, ReleaseApplicationBackend, ReleaseApplicationError,
    ReleaseEventSink,
};
use crate::infrastructure::gh::{GhBackend, GhOperation, GhRequest, SystemGhBackend};
use crate::infrastructure::git::GitBackend;
use crate::infrastructure::local_verification::ProcessLocalVerificationBackend;
use crate::infrastructure::process::{ProcessInvocation, SafeProcessRunner};
use crate::models::{
    ExternalPreflightSnapshot, ReleaseConnectionTestResult, ReleaseEvent, ReleaseLogPage,
    ReleasePhase, ReleasePlanFileSummary, ReleasePlanSummary, ReleasePreflightResult,
    ReleaseProxySettings, ReleaseSession, ReleaseSessionSnapshot, SafeRepositoryPushRequest,
    ToolchainInspection,
};
use crate::services::git_release::{
    GitReleaseError, GitReleaseService, RepositoryInspectionService, project_release_preflight,
};
use crate::services::release_candidate::{ReleaseCandidatePlan, ReleaseCandidateTransaction};
use crate::services::release_log::{ReleaseLogRecorder, ReleaseLogStore, ReleaseProgressSink};
use crate::services::release_network::{
    ReleaseConnectionService, ReleaseNetworkProfile, SystemReleaseConnectionProbeBackend,
};
use crate::services::release_notes::{CommitSummary, ReleaseNotesService};
use crate::services::release_orchestrator::{
    GitReleasePushBackend, GithubRemoteBackend, ReleaseOrchestrator, ReleaseOrchestratorError,
};
use crate::services::release_state::{ReleaseStateError, ReleaseStateStore, RepositorySessionLock};
use chrono::Utc;
use codex_relay_core::infrastructure::atomic_file::atomic_write;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const TARGET_REPOSITORY: &str = "hunxuankai/codex-relay";
const RELEASE_WORKFLOW: &str = "release.yml";
const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
const RELEASE_LOG_READ_WARNING: &str =
    "发布诊断日志暂时无法读取；发布会话仍可继续，但部分日志可能不可恢复。";

#[derive(Clone, Copy)]
enum PreflightPurpose {
    NewRelease,
    Recovery,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResumeAction {
    RequiresLocalCancel,
    ReturnSnapshot,
    PushCommitted,
    RemoteToDraft,
    PublishAndFinalize,
}

#[derive(Clone)]
pub struct SystemReleaseApplication {
    inner: Arc<SystemReleaseApplicationInner>,
}

struct SystemReleaseApplicationInner {
    plans: Mutex<HashMap<String, StoredPlan>>,
    sessions: Mutex<HashMap<String, SessionContext>>,
    cancellations: Mutex<HashMap<String, tokio::sync::watch::Sender<bool>>>,
    next_id: AtomicU64,
}

#[derive(Clone)]
struct StoredPlan {
    repository_path: PathBuf,
    git_dir: PathBuf,
    expected_remote_sha: String,
    plan: ReleaseCandidatePlan,
    summary: ReleasePlanSummary,
    tools: ResolvedTools,
}

#[derive(Clone)]
struct SessionContext {
    repository_path: PathBuf,
    git_dir: PathBuf,
    expected_notes: String,
    tools: ResolvedTools,
}

struct ReleaseFailureDetails<'a> {
    step_id: &'a str,
    code: &'a str,
    message: &'a str,
}

#[derive(Clone)]
struct ResolvedTools {
    git: PathBuf,
    node: PathBuf,
    npm: PathBuf,
    cargo: PathBuf,
    gh: PathBuf,
}

impl Default for SystemReleaseApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemReleaseApplication {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SystemReleaseApplicationInner {
                plans: Mutex::new(HashMap::new()),
                sessions: Mutex::new(HashMap::new()),
                cancellations: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    async fn handle(
        &self,
        request: ApplicationRequest,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Result<ApplicationResponse, ReleaseApplicationError> {
        match request {
            ApplicationRequest::TestConnection { proxy } => self
                .test_connection(proxy)
                .await
                .map(ApplicationResponse::ConnectionTest),
            ApplicationRequest::Inspect {
                repository_path,
                proxy,
            } => {
                let profile = network_profile(&proxy)?;
                self.inspect_repository_with_profile(
                    PathBuf::from(repository_path),
                    PreflightPurpose::NewRelease,
                    &profile,
                )
                .await
                .map(|(inspection, _, _)| ApplicationResponse::Inspection(inspection))
            }
            ApplicationRequest::PushRepository { request } => self
                .push_repository(request)
                .await
                .map(ApplicationResponse::Inspection),
            ApplicationRequest::PreparePlan {
                repository_path,
                target_version,
                notes,
                proxy,
            } => self
                .prepare_plan(
                    PathBuf::from(repository_path),
                    &target_version,
                    notes,
                    proxy,
                )
                .await
                .map(ApplicationResponse::Plan),
            ApplicationRequest::Start { plan_id, proxy } => self
                .start_release(&plan_id, proxy, events)
                .await
                .map(ApplicationResponse::Session),
            ApplicationRequest::GetSession { repository_path } => self
                .get_session(PathBuf::from(repository_path))
                .await
                .map(ApplicationResponse::OptionalSnapshot),
            ApplicationRequest::GetLogs {
                session_id,
                before_sequence,
            } => self
                .get_logs(&session_id, before_sequence)
                .await
                .map(ApplicationResponse::Logs),
            ApplicationRequest::Resume { session_id, proxy } => self
                .resume_release(&session_id, proxy, events)
                .await
                .map(ApplicationResponse::Session),
            ApplicationRequest::Cancel { session_id } => self
                .cancel_release(&session_id)
                .await
                .map(ApplicationResponse::Session),
            ApplicationRequest::Publish {
                session_id,
                expected_draft_identity,
                proxy,
            } => self
                .publish_release(&session_id, expected_draft_identity, proxy, events)
                .await
                .map(ApplicationResponse::Session),
            ApplicationRequest::ExportSummary {
                session_id,
                destination_path,
            } => self
                .export_summary(&session_id, PathBuf::from(destination_path))
                .await
                .map(ApplicationResponse::SummaryPath),
        }
    }

    async fn test_connection(
        &self,
        proxy: ReleaseProxySettings,
    ) -> Result<ReleaseConnectionTestResult, ReleaseApplicationError> {
        let profile = network_profile(&proxy)?;
        let directory = tempfile::tempdir().map_err(|_| {
            app_error(
                "RELEASE_CONNECTION_TEST_FAILED",
                "无法创建连接测试临时目录。",
            )
        })?;
        let backend = SystemReleaseConnectionProbeBackend::new(
            resolve_executable(&["git.exe", "git"]).ok(),
            resolve_executable(&["gh.exe", "gh"]).ok(),
            &profile,
            directory.path().to_path_buf(),
        );
        Ok(ReleaseConnectionService::new().test(&backend).await)
    }

    async fn inspect_repository(
        &self,
        repository_path: PathBuf,
        purpose: PreflightPurpose,
    ) -> Result<(ReleasePreflightResult, ResolvedTools, PathBuf), ReleaseApplicationError> {
        let direct = ReleaseProxySettings {
            enabled: false,
            proxy_type: crate::models::ReleaseProxyType::Http,
            host: String::new(),
            port: None,
        };
        let profile = network_profile(&direct)?;
        self.inspect_repository_with_profile(repository_path, purpose, &profile)
            .await
    }

    async fn inspect_repository_with_profile(
        &self,
        repository_path: PathBuf,
        purpose: PreflightPurpose,
        profile: &ReleaseNetworkProfile,
    ) -> Result<(ReleasePreflightResult, ResolvedTools, PathBuf), ReleaseApplicationError> {
        let repository_path = repository_path
            .canonicalize()
            .map_err(|_| app_error("GIT_REPOSITORY_INVALID", "无法读取 Git 仓库。"))?;
        if !repository_path.is_dir() {
            return Err(app_error("GIT_REPOSITORY_INVALID", "无法读取 Git 仓库。"));
        }
        let tools = resolve_tools()?;
        let environment = profile.environment().to_vec();
        let git = GitBackend::new_with_proxy(
            tools.git.clone(),
            environment.clone(),
            profile.git_proxy_mode().clone(),
        );
        let inspection_service = RepositoryInspectionService::for_codex_relay();
        let repository = match purpose {
            PreflightPurpose::NewRelease => {
                inspection_service.inspect(&git, &repository_path).await
            }
            PreflightPurpose::Recovery => {
                inspection_service
                    .inspect_for_recovery(&git, &repository_path)
                    .await
            }
        }
        .map_err(git_error)?;
        let git_dir = git
            .run(&repository_path, &["rev-parse", "--absolute-git-dir"])
            .await
            .map_err(|_| app_error("GIT_COMMAND_FAILED", "无法读取 Git 元数据目录。"))?
            .stdout
            .trim()
            .to_string();
        let git_dir = PathBuf::from(git_dir)
            .canonicalize()
            .map_err(|_| app_error("GIT_REPOSITORY_INVALID", "无法读取 Git 元数据目录。"))?;

        if matches!(purpose, PreflightPurpose::Recovery) {
            return Ok((
                project_release_preflight(
                    repository_path.to_string_lossy().into_owned(),
                    repository,
                    ExternalPreflightSnapshot {
                        tools: ToolchainInspection {
                            git: None,
                            node: None,
                            npm: None,
                            cargo: None,
                            gh: None,
                        },
                        active_release_runs: 0,
                        conflicting_drafts: 0,
                        latest_release_tag: None,
                    },
                ),
                tools,
                git_dir,
            ));
        }

        let versions = ToolchainInspection {
            git: Some(
                tool_version(&tools.git, &["--version"], &repository_path, &environment).await?,
            ),
            node: Some(
                tool_version(&tools.node, &["--version"], &repository_path, &environment).await?,
            ),
            npm: Some(
                tool_version(&tools.npm, &["--version"], &repository_path, &environment).await?,
            ),
            cargo: Some(
                tool_version(&tools.cargo, &["--version"], &repository_path, &environment).await?,
            ),
            gh: Some(
                tool_version(&tools.gh, &["--version"], &repository_path, &environment).await?,
            ),
        };

        let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
        let gh = SystemGhBackend::new(
            tools.gh.clone(),
            environment,
            repository_path.clone(),
            cancel,
        );
        let runs = gh
            .execute(GhRequest {
                operation: GhOperation::PreflightReleaseRuns,
                repository: TARGET_REPOSITORY.into(),
                workflow: Some(RELEASE_WORKFLOW.into()),
                git_ref: None,
                tag_name: None,
                head_sha: None,
                created_after: None,
                resource_id: None,
                stdin: None,
            })
            .await
            .map_err(github_preflight_error)?;
        let runs: Vec<Value> = serde_json::from_slice(&runs.stdout)
            .map_err(|_| app_error("GITHUB_RESPONSE_INVALID", "GitHub CLI 返回无效 JSON。"))?;
        let active_release_runs = runs
            .iter()
            .filter(|run| run.get("status").and_then(Value::as_str) != Some("completed"))
            .count();

        let releases = gh
            .execute(GhRequest {
                operation: GhOperation::ListDraftReleases,
                repository: TARGET_REPOSITORY.into(),
                workflow: None,
                git_ref: None,
                tag_name: None,
                head_sha: None,
                created_after: None,
                resource_id: None,
                stdin: None,
            })
            .await
            .map_err(github_preflight_error)?;
        let releases: Vec<Value> = serde_json::from_slice(&releases.stdout)
            .map_err(|_| app_error("GITHUB_RESPONSE_INVALID", "GitHub CLI 返回无效 JSON。"))?;
        let latest_release_tag = latest_published_release_tag(&releases);
        let conflicting_drafts = releases
            .iter()
            .filter(|release| release.get("draft").and_then(Value::as_bool) == Some(true))
            .count();

        let mut result = project_release_preflight(
            repository_path.to_string_lossy().into_owned(),
            repository,
            ExternalPreflightSnapshot {
                tools: versions,
                active_release_runs,
                conflicting_drafts,
                latest_release_tag,
            },
        );
        if ReleaseStateStore::new(git_dir.clone())
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?
            .is_some_and(|session| !release_phase_is_terminal(session.phase))
        {
            result.release_ready = false;
            result.safe_push = None;
            result
                .blocking_reasons
                .push("该仓库已有未完成的发布会话，请先恢复或取消。".to_string());
        }
        Ok((result, tools, git_dir))
    }

    async fn push_repository(
        &self,
        request: SafeRepositoryPushRequest,
    ) -> Result<ReleasePreflightResult, ReleaseApplicationError> {
        let profile = network_profile(&request.proxy)?;
        let repository_path = PathBuf::from(&request.repository_path);
        let (inspection, tools, _) = self
            .inspect_repository_with_profile(
                repository_path.clone(),
                PreflightPurpose::NewRelease,
                &profile,
            )
            .await?;
        let repository_path = PathBuf::from(&inspection.repository_path);
        let preview = inspection
            .safe_push
            .as_ref()
            .ok_or_else(|| safe_push_blocked(&inspection))?;
        if preview.expected_head_sha != request.expected_head_sha {
            return Err(app_error("GIT_HEAD_MOVED", "本地 HEAD 在确认后发生变化。"));
        }
        if preview.expected_remote_main_sha != request.expected_remote_main_sha {
            return Err(app_error(
                "GIT_REMOTE_MOVED",
                "远端 main 在确认后发生变化。",
            ));
        }
        let git = GitBackend::new_with_proxy(
            tools.git,
            profile.environment().to_vec(),
            profile.git_proxy_mode().clone(),
        );
        GitReleaseService::new("main")
            .push_existing_commits(
                &git,
                &repository_path,
                &request.expected_head_sha,
                &request.expected_remote_main_sha,
            )
            .await
            .map_err(git_error)?;
        let (refreshed, _, _) = self
            .inspect_repository_with_profile(
                repository_path,
                PreflightPurpose::NewRelease,
                &profile,
            )
            .await?;
        verify_refreshed_push(&refreshed, &request.expected_head_sha)?;
        Ok(refreshed)
    }

    async fn prepare_plan(
        &self,
        repository_path: PathBuf,
        target_version: &str,
        notes: Option<String>,
        proxy: ReleaseProxySettings,
    ) -> Result<ReleasePlanSummary, ReleaseApplicationError> {
        let profile = network_profile(&proxy)?;
        let (inspection, tools, git_dir) = self
            .inspect_repository_with_profile(
                repository_path.clone(),
                PreflightPurpose::NewRelease,
                &profile,
            )
            .await?;
        if !inspection.release_ready {
            return Err(app_error(
                "RELEASE_PREFLIGHT_BLOCKED",
                inspection
                    .blocking_reasons
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "仓库尚未满足发布条件。".to_string()),
            ));
        }
        let repository_path = repository_path
            .canonicalize()
            .map_err(|_| app_error("GIT_REPOSITORY_INVALID", "无法读取 Git 仓库。"))?;
        let previous_version = read_package_version(&repository_path)?;
        let notes = match notes {
            Some(notes) => notes,
            None => {
                let git = GitBackend::new_with_proxy(
                    tools.git.clone(),
                    profile.environment().to_vec(),
                    profile.git_proxy_mode().clone(),
                );
                let commits = release_commits(&git, &repository_path, &previous_version).await?;
                ReleaseNotesService::generate(&previous_version, target_version, &commits)
                    .map_err(|error| app_error(error.code(), error.to_string()))?
                    .body
            }
        };
        let plan = ReleaseCandidateTransaction::plan(&repository_path, target_version, &notes)
            .map_err(|error| app_error(error.code(), error.to_string()))?;
        let id = format!(
            "plan-{:016x}",
            self.inner.next_id.fetch_add(1, Ordering::Relaxed)
        );
        let summary = ReleasePlanSummary {
            id: id.clone(),
            repository_path: repository_path.to_string_lossy().into_owned(),
            previous_version: plan.previous_version.clone(),
            target_version: plan.target_version.clone(),
            notes,
            files: plan
                .files
                .iter()
                .map(|file| ReleasePlanFileSummary {
                    relative_path: file.relative_path.clone(),
                    before_sha256: file.expected_fingerprint.sha256.clone().unwrap_or_default(),
                    after_sha256: sha256_hex(&file.after),
                })
                .collect(),
        };
        self.inner.plans.lock().unwrap().insert(
            id,
            StoredPlan {
                repository_path,
                git_dir,
                expected_remote_sha: inspection.repository.remote_main_sha,
                plan,
                summary: summary.clone(),
                tools,
            },
        );
        Ok(summary)
    }

    async fn start_release(
        &self,
        plan_id: &str,
        proxy: ReleaseProxySettings,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Result<ReleaseSession, ReleaseApplicationError> {
        let profile = network_profile(&proxy)?;
        let stored = self
            .inner
            .plans
            .lock()
            .unwrap()
            .get(plan_id)
            .cloned()
            .ok_or_else(|| app_error("RELEASE_PLAN_NOT_FOUND", "发布计划不存在或已过期。"))?;
        let session_id = format!(
            "release-{}-{:016x}",
            Utc::now().format("%Y%m%d%H%M%S"),
            self.inner.next_id.fetch_add(1, Ordering::Relaxed)
        );
        let session = ReleaseSession::new(
            session_id.clone(),
            stored.repository_path.to_string_lossy(),
            stored.plan.target_version.clone(),
        );
        let store = ReleaseStateStore::new(stored.git_dir.clone());
        let start_lock = RepositorySessionLock::acquire(&stored.git_dir).map_err(|error| {
            if matches!(error, ReleaseStateError::SessionLocked) {
                app_error("RELEASE_SESSION_ALREADY_ACTIVE", "该仓库已有活动发布会话。")
            } else {
                app_error("RELEASE_STATE_FAILED", "无法保存发布会话。")
            }
        })?;
        store.initialize(&session).map_err(|error| {
            if matches!(error, ReleaseStateError::ActiveSessionExists) {
                app_error("RELEASE_SESSION_ALREADY_ACTIVE", "该仓库已有活动发布会话。")
            } else {
                app_error("RELEASE_STATE_FAILED", "无法保存发布会话。")
            }
        })?;
        self.inner.sessions.lock().unwrap().insert(
            session_id.clone(),
            SessionContext {
                repository_path: stored.repository_path.clone(),
                git_dir: stored.git_dir.clone(),
                expected_notes: stored.summary.notes.clone(),
                tools: stored.tools.clone(),
            },
        );
        let cancel = self.register_pipeline(&session_id)?;
        drop(start_lock);
        emit(
            events.as_deref(),
            ReleaseEvent::SessionUpdated {
                session: Box::new(session.clone()),
            },
        );
        let recorder = Arc::new(initialize_release_log_recorder(
            stored.git_dir.clone(),
            &session_id,
            events.clone(),
        ));
        let application = self.clone();
        let initial = session.clone();
        tauri::async_runtime::spawn(async move {
            application
                .run_initial_pipeline(initial, stored, profile, cancel, events, recorder)
                .await;
        });
        Ok(session)
    }

    async fn run_initial_pipeline(
        &self,
        mut session: ReleaseSession,
        stored: StoredPlan,
        profile: ReleaseNetworkProfile,
        cancel: tokio::sync::watch::Receiver<bool>,
        events: Option<Arc<dyn ReleaseEventSink>>,
        recorder: Arc<ReleaseLogRecorder>,
    ) {
        let watch_store = ReleaseStateStore::new(stored.git_dir.clone());
        run_with_session_updates(watch_store, events.clone(), async {
            let environment = profile.environment().to_vec();
            let git = GitBackend::new_cancellable_with_proxy(
                stored.tools.git.clone(),
                environment.clone(),
                profile.git_proxy_mode().clone(),
                cancel.clone(),
            );
            let verification = ProcessLocalVerificationBackend::with_recorder(
                stored.tools.npm.clone(),
                stored.tools.cargo.clone(),
                environment.clone(),
                cancel.clone(),
                Arc::clone(&recorder),
            );
            let push = GitReleasePushBackend::new(git, stored.expected_remote_sha.clone());
            let store = ReleaseStateStore::new(stored.git_dir.clone());
            let progress: Arc<dyn ReleaseProgressSink> = recorder.clone();
            let result = ReleaseOrchestrator::new()
                .with_progress(progress.clone())
                .run_to_pushed(
                    &mut session,
                    &store,
                    &stored.repository_path,
                    &stored.git_dir,
                    &stored.plan,
                    &verification,
                    &push,
                )
                .await;
            if let Err(error) = result {
                self.finish_with_orchestrator_error(
                    &session.id,
                    &store,
                    events.as_deref(),
                    Some(recorder.as_ref()),
                    &error,
                );
                return;
            }
            emit(
                events.as_deref(),
                ReleaseEvent::SessionUpdated {
                    session: Box::new(session.clone()),
                },
            );

            let gh = SystemGhBackend::new(
                stored.tools.gh.clone(),
                environment,
                stored.repository_path.clone(),
                cancel,
            );
            let remote = GithubRemoteBackend::new(&gh).with_progress(progress.clone());
            match ReleaseOrchestrator::new()
                .with_progress(progress)
                .run_remote_to_draft(
                    &mut session,
                    &store,
                    &stored.git_dir,
                    &stored.summary.notes,
                    &remote,
                )
                .await
            {
                Ok(draft) => {
                    emit(events.as_deref(), ReleaseEvent::DraftReady { draft });
                    emit(
                        events.as_deref(),
                        ReleaseEvent::SessionUpdated {
                            session: Box::new(session.clone()),
                        },
                    );
                }
                Err(error) => {
                    self.finish_with_orchestrator_error(
                        &session.id,
                        &store,
                        events.as_deref(),
                        Some(recorder.as_ref()),
                        &error,
                    );
                    return;
                }
            }
            self.inner.cancellations.lock().unwrap().remove(&session.id);
        })
        .await;
    }

    async fn get_session(
        &self,
        repository_path: PathBuf,
    ) -> Result<Option<ReleaseSessionSnapshot>, ReleaseApplicationError> {
        let (_, tools, git_dir) = self
            .inspect_repository(repository_path.clone(), PreflightPurpose::Recovery)
            .await?;
        let store = ReleaseStateStore::new(git_dir.clone());
        let session = store
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?;
        if let Some(session) = session {
            let notes = fs::read_to_string(repository_path.join(".github/release-notes.md"))
                .map_err(|_| app_error("RELEASE_NOTES_READ_FAILED", "无法读取发布说明。"))?;
            self.inner.sessions.lock().unwrap().insert(
                session.id.clone(),
                SessionContext {
                    repository_path: repository_path
                        .canonicalize()
                        .map_err(|_| app_error("GIT_REPOSITORY_INVALID", "无法读取 Git 仓库。"))?,
                    git_dir: git_dir.clone(),
                    expected_notes: notes,
                    tools,
                },
            );
            let logs = load_release_log_page(git_dir, session.id.clone(), None).await;
            return Ok(Some(ReleaseSessionSnapshot { session, logs }));
        }
        Ok(None)
    }

    async fn get_logs(
        &self,
        session_id: &str,
        before_sequence: Option<u64>,
    ) -> Result<ReleaseLogPage, ReleaseApplicationError> {
        let context = self.context(session_id)?;
        let session_matches = ReleaseStateStore::new(context.git_dir.clone())
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?
            .is_some_and(|session| session.id == session_id);
        if !session_matches {
            return Err(app_error("RELEASE_SESSION_NOT_FOUND", "未找到发布会话。"));
        }
        Ok(load_release_log_page(context.git_dir, session_id.to_string(), before_sequence).await)
    }

    async fn resume_release(
        &self,
        session_id: &str,
        proxy: ReleaseProxySettings,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Result<ReleaseSession, ReleaseApplicationError> {
        let profile = network_profile(&proxy)?;
        let context = self.context(session_id)?;
        let store = ReleaseStateStore::new(context.git_dir.clone());
        let session = store
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?
            .filter(|session| session.id == session_id)
            .ok_or_else(|| app_error("RELEASE_SESSION_NOT_FOUND", "未找到发布会话。"))?;
        let recorder = Arc::new(open_release_log_recorder(
            context.git_dir.clone(),
            session_id,
            events.clone(),
        ));
        if resume_action(session.phase) == ResumeAction::RequiresLocalCancel {
            return Err(app_error(
                "RELEASE_LOCAL_RESUME_REQUIRES_CANCEL",
                "本地发布阶段中断，请先取消并验证回滚后重新开始。",
            ));
        }
        if resume_action(session.phase) == ResumeAction::ReturnSnapshot {
            return Ok(session);
        }
        let cancel = self.register_pipeline(session_id)?;
        let application = self.clone();
        let initial = session.clone();
        tauri::async_runtime::spawn(async move {
            application
                .run_resume_pipeline(initial, context, profile, cancel, events, recorder)
                .await;
        });
        Ok(session)
    }

    async fn run_resume_pipeline(
        &self,
        mut session: ReleaseSession,
        context: SessionContext,
        profile: ReleaseNetworkProfile,
        cancel: tokio::sync::watch::Receiver<bool>,
        events: Option<Arc<dyn ReleaseEventSink>>,
        recorder: Arc<ReleaseLogRecorder>,
    ) {
        let watch_store = ReleaseStateStore::new(context.git_dir.clone());
        run_with_session_updates(watch_store, events.clone(), async {
            let environment = profile.environment().to_vec();
            let store = ReleaseStateStore::new(context.git_dir.clone());
            let progress: Arc<dyn ReleaseProgressSink> = recorder.clone();
            if resume_action(session.phase) == ResumeAction::PushCommitted {
                let git = GitBackend::new_cancellable_with_proxy(
                    context.tools.git.clone(),
                    environment.clone(),
                    profile.git_proxy_mode().clone(),
                    cancel.clone(),
                );
                let push = GitReleasePushBackend::for_committed(git);
                if let Err(error) = ReleaseOrchestrator::new()
                    .with_progress(progress.clone())
                    .push_committed(
                        &mut session,
                        &store,
                        &context.repository_path,
                        &context.git_dir,
                        &push,
                    )
                    .await
                {
                    self.finish_with_orchestrator_error(
                        &session.id,
                        &store,
                        events.as_deref(),
                        Some(recorder.as_ref()),
                        &error,
                    );
                    return;
                }
            }
            let gh = SystemGhBackend::new(
                context.tools.gh.clone(),
                environment,
                context.repository_path.clone(),
                cancel,
            );
            let remote = GithubRemoteBackend::new(&gh).with_progress(progress.clone());
            let result = match resume_action(session.phase) {
                ResumeAction::RemoteToDraft => ReleaseOrchestrator::new()
                    .with_progress(progress.clone())
                    .run_remote_to_draft(
                        &mut session,
                        &store,
                        &context.git_dir,
                        &context.expected_notes,
                        &remote,
                    )
                    .await
                    .map(|_| ()),
                ResumeAction::PublishAndFinalize => {
                    let identity = match session.draft.as_ref() {
                        Some(draft) => draft.identity(),
                        None => {
                            self.finish_with_error(
                                &session.id,
                                &store,
                                events.as_deref(),
                                Some(recorder.as_ref()),
                                "RELEASE_REMOTE_STATE_INVALID",
                            );
                            return;
                        }
                    };
                    ReleaseOrchestrator::new()
                        .with_progress(progress)
                        .publish_and_finalize(
                            &mut session,
                            &store,
                            &context.git_dir,
                            &identity,
                            &context.expected_notes,
                            &remote,
                        )
                        .await
                }
                ResumeAction::RequiresLocalCancel
                | ResumeAction::ReturnSnapshot
                | ResumeAction::PushCommitted => return,
            };
            match result {
                Ok(()) => emit(
                    events.as_deref(),
                    ReleaseEvent::SessionUpdated {
                        session: Box::new(session.clone()),
                    },
                ),
                Err(error) => self.finish_with_orchestrator_error(
                    &session.id,
                    &store,
                    events.as_deref(),
                    Some(recorder.as_ref()),
                    &error,
                ),
            }
            self.inner.cancellations.lock().unwrap().remove(&session.id);
        })
        .await;
    }

    async fn publish_release(
        &self,
        session_id: &str,
        expected_identity: crate::models::DraftIdentity,
        proxy: ReleaseProxySettings,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> Result<ReleaseSession, ReleaseApplicationError> {
        let profile = network_profile(&proxy)?;
        let context = self.context(session_id)?;
        let store = ReleaseStateStore::new(context.git_dir.clone());
        let session = store
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?
            .filter(|session| session.id == session_id)
            .ok_or_else(|| app_error("RELEASE_SESSION_NOT_FOUND", "未找到发布会话。"))?;
        if session.phase != ReleasePhase::AwaitingPublishApproval {
            return Err(app_error(
                "RELEASE_PUBLISH_NOT_READY",
                "当前发布会话尚未通过 Draft 审计。",
            ));
        }
        let recorder = Arc::new(open_release_log_recorder(
            context.git_dir.clone(),
            session_id,
            events.clone(),
        ));
        let cancel = self.register_pipeline(session_id)?;
        let application = self.clone();
        let initial = session.clone();
        tauri::async_runtime::spawn(async move {
            let watch_store = ReleaseStateStore::new(context.git_dir.clone());
            run_with_session_updates(watch_store, events.clone(), async {
                let environment = profile.environment().to_vec();
                let gh = SystemGhBackend::new(
                    context.tools.gh.clone(),
                    environment,
                    context.repository_path.clone(),
                    cancel,
                );
                let progress: Arc<dyn ReleaseProgressSink> = recorder.clone();
                let remote = GithubRemoteBackend::new(&gh).with_progress(progress.clone());
                let store = ReleaseStateStore::new(context.git_dir.clone());
                let mut current = initial;
                match ReleaseOrchestrator::new()
                    .with_progress(progress)
                    .publish_and_finalize(
                        &mut current,
                        &store,
                        &context.git_dir,
                        &expected_identity,
                        &context.expected_notes,
                        &remote,
                    )
                    .await
                {
                    Ok(()) => {
                        if let Some(published) = current.published.clone() {
                            emit(
                                events.as_deref(),
                                ReleaseEvent::ReleasePublished { published },
                            );
                        }
                        emit(
                            events.as_deref(),
                            ReleaseEvent::SessionUpdated {
                                session: Box::new(current.clone()),
                            },
                        );
                    }
                    Err(error) => application.finish_with_orchestrator_error(
                        &current.id,
                        &store,
                        events.as_deref(),
                        Some(recorder.as_ref()),
                        &error,
                    ),
                }
                application
                    .inner
                    .cancellations
                    .lock()
                    .unwrap()
                    .remove(&current.id);
            })
            .await;
        });
        Ok(session)
    }

    async fn cancel_release(
        &self,
        session_id: &str,
    ) -> Result<ReleaseSession, ReleaseApplicationError> {
        let context = self.context(session_id)?;
        let store = ReleaseStateStore::new(context.git_dir.clone());
        let mut session = store
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?
            .filter(|session| session.id == session_id)
            .ok_or_else(|| app_error("RELEASE_SESSION_NOT_FOUND", "未找到发布会话。"))?;
        if !should_signal_process_cancel(session.phase) {
            ReleaseOrchestrator::new()
                .cancel_active(
                    &mut session,
                    &store,
                    &context.repository_path,
                    &context.git_dir,
                )
                .map_err(|error| app_error(error.code(), error.to_string()))?;
            return Ok(session);
        }
        let active_sender = {
            self.inner
                .cancellations
                .lock()
                .unwrap()
                .get(session_id)
                .cloned()
        };
        if let Some(sender) = active_sender {
            let _ = sender.send(true);
            for _ in 0..150 {
                if let Some(session) = store.load().ok().flatten()
                    && matches!(
                        session.phase,
                        ReleasePhase::Cancelled | ReleasePhase::Failed
                    )
                {
                    return Ok(session);
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
        session = store
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?
            .filter(|session| session.id == session_id)
            .ok_or_else(|| app_error("RELEASE_SESSION_NOT_FOUND", "未找到发布会话。"))?;
        ReleaseOrchestrator::new()
            .cancel_active(
                &mut session,
                &store,
                &context.repository_path,
                &context.git_dir,
            )
            .map_err(|error| app_error(error.code(), error.to_string()))?;
        Ok(session)
    }

    async fn export_summary(
        &self,
        session_id: &str,
        destination_path: PathBuf,
    ) -> Result<String, ReleaseApplicationError> {
        let context = self.context(session_id)?;
        if path_is_protected(&destination_path) {
            return Err(app_error(
                "RELEASE_EXPORT_PATH_UNSAFE",
                "摘要不能写入 Codex 或 Codex Relay 用户数据目录。",
            ));
        }
        let session = ReleaseStateStore::new(context.git_dir)
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?
            .filter(|session| session.id == session_id)
            .ok_or_else(|| app_error("RELEASE_SESSION_NOT_FOUND", "未找到发布会话。"))?;
        let mut bytes = serde_json::to_vec_pretty(&session)
            .map_err(|_| app_error("RELEASE_EXPORT_FAILED", "无法生成发布摘要。"))?;
        bytes.push(b'\n');
        atomic_write(&destination_path, &bytes, |candidate| {
            if candidate == bytes {
                Ok(())
            } else {
                Err(codex_relay_core::error::AppError::new(
                    "RELEASE_EXPORT_VERIFY_FAILED",
                    "发布摘要写入验证失败。",
                    "summary bytes differ",
                ))
            }
        })
        .map_err(|_| app_error("RELEASE_EXPORT_FAILED", "无法写入发布摘要。"))?;
        Ok(destination_path.to_string_lossy().into_owned())
    }

    fn context(&self, session_id: &str) -> Result<SessionContext, ReleaseApplicationError> {
        self.inner
            .sessions
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .ok_or_else(|| {
                app_error(
                    "RELEASE_SESSION_CONTEXT_MISSING",
                    "请先选择仓库并加载活动发布会话。",
                )
            })
    }

    fn register_pipeline(
        &self,
        session_id: &str,
    ) -> Result<tokio::sync::watch::Receiver<bool>, ReleaseApplicationError> {
        let (sender, receiver) = tokio::sync::watch::channel(false);
        let mut cancellations = self.inner.cancellations.lock().unwrap();
        if cancellations.contains_key(session_id) {
            return Err(app_error(
                "RELEASE_SESSION_ALREADY_RUNNING",
                "该发布会话已有后台任务正在运行。",
            ));
        }
        cancellations.insert(session_id.to_string(), sender);
        Ok(receiver)
    }

    fn finish_with_error(
        &self,
        session_id: &str,
        store: &ReleaseStateStore,
        events: Option<&dyn ReleaseEventSink>,
        recorder: Option<&ReleaseLogRecorder>,
        code: &str,
    ) {
        self.finish_with_error_details(
            session_id,
            store,
            events,
            recorder,
            ReleaseFailureDetails {
                step_id: "releasePipeline",
                code,
                message: "发布流程失败，请查看对应阶段证据。",
            },
        );
    }

    fn finish_with_orchestrator_error(
        &self,
        session_id: &str,
        store: &ReleaseStateStore,
        events: Option<&dyn ReleaseEventSink>,
        recorder: Option<&ReleaseLogRecorder>,
        error: &ReleaseOrchestratorError,
    ) {
        let message = error.failure_message();
        self.finish_with_error_details(
            session_id,
            store,
            events,
            recorder,
            ReleaseFailureDetails {
                step_id: error.failure_step_id(),
                code: error.code(),
                message: &message,
            },
        );
    }

    fn finish_with_error_details(
        &self,
        session_id: &str,
        store: &ReleaseStateStore,
        events: Option<&dyn ReleaseEventSink>,
        recorder: Option<&ReleaseLogRecorder>,
        failure: ReleaseFailureDetails<'_>,
    ) {
        if let Some(recorder) = recorder {
            recorder.record(
                failure.step_id,
                crate::models::ReleaseLogSource::Lifecycle,
                crate::models::ReleaseLogLevel::Error,
                format!("{}：{}", failure.code, failure.message),
            );
        }
        if let Ok(Some(mut current)) = store.load() {
            let should_emit = if matches!(
                current.phase,
                ReleasePhase::Committed | ReleasePhase::Failed | ReleasePhase::Cancelled
            ) {
                current.phase == ReleasePhase::Failed
            } else {
                store
                    .fail(&mut current, failure.step_id, failure.code)
                    .is_ok()
            };
            if should_emit {
                emit(
                    events,
                    ReleaseEvent::SessionUpdated {
                        session: Box::new(current),
                    },
                );
            }
        }
        emit(
            events,
            ReleaseEvent::StepFailed {
                step_id: failure.step_id.into(),
                code: failure.code.into(),
                message: failure.message.into(),
            },
        );
        self.inner.cancellations.lock().unwrap().remove(session_id);
    }
}

impl ReleaseApplicationBackend for SystemReleaseApplication {
    fn execute<'a>(
        &'a self,
        request: ApplicationRequest,
        events: Option<Arc<dyn ReleaseEventSink>>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<ApplicationResponse, ReleaseApplicationError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move { self.handle(request, events).await })
    }
}

async fn tool_version(
    executable: &Path,
    args: &[&str],
    workdir: &Path,
    environment: &[(OsString, OsString)],
) -> Result<String, ReleaseApplicationError> {
    let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
    let output = SafeProcessRunner::default()
        .run(
            ProcessInvocation {
                executable: executable.to_path_buf(),
                args: args.iter().map(OsString::from).collect(),
                env: environment.to_vec(),
                workdir: workdir.to_path_buf(),
                stdin: None,
                stdout_file: None,
            },
            PROCESS_TIMEOUT,
            cancel,
            None,
        )
        .await
        .map_err(|_| app_error("RELEASE_TOOL_MISSING", "发布所需工具不可用。"))?;
    if output.exit_code != Some(0) {
        return Err(app_error("RELEASE_TOOL_MISSING", "发布所需工具不可用。"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout
        .lines()
        .chain(stderr.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
        .ok_or_else(|| app_error("RELEASE_TOOL_MISSING", "发布所需工具不可用。"))
}

async fn release_commits(
    git: &GitBackend,
    repository_path: &Path,
    previous_version: &str,
) -> Result<Vec<CommitSummary>, ReleaseApplicationError> {
    let range = format!("v{previous_version}..HEAD");
    let output = match git
        .run(repository_path, &["log", "--format=%H%x09%s", &range])
        .await
    {
        Ok(output) => output,
        Err(_) => git
            .run(repository_path, &["log", "-20", "--format=%H%x09%s"])
            .await
            .map_err(|_| app_error("GIT_LOG_FAILED", "无法读取发布提交历史。"))?,
    };
    Ok(output
        .stdout
        .lines()
        .filter_map(|line| {
            let (sha, subject) = line.split_once('\t')?;
            Some(CommitSummary {
                sha: sha.to_string(),
                subject: subject.to_string(),
            })
        })
        .collect())
}

fn resolve_tools() -> Result<ResolvedTools, ReleaseApplicationError> {
    Ok(ResolvedTools {
        git: resolve_executable(&["git.exe", "git"])?,
        node: resolve_executable(&["node.exe", "node"])?,
        npm: resolve_executable(&["npm.cmd", "npm.exe", "npm"])?,
        cargo: resolve_executable(&["cargo.exe", "cargo"])?,
        gh: resolve_executable(&["gh.exe", "gh"])?,
    })
}

fn resolve_executable(candidates: &[&str]) -> Result<PathBuf, ReleaseApplicationError> {
    let path = std::env::var_os("PATH")
        .ok_or_else(|| app_error("RELEASE_TOOL_MISSING", "发布所需工具不可用。"))?;
    for directory in std::env::split_paths(&path) {
        for candidate in candidates {
            let path = directory.join(candidate);
            if path.is_file() {
                return path
                    .canonicalize()
                    .map_err(|_| app_error("RELEASE_TOOL_MISSING", "发布所需工具不可用。"));
            }
        }
    }
    Err(app_error("RELEASE_TOOL_MISSING", "发布所需工具不可用。"))
}

fn read_package_version(repository_path: &Path) -> Result<String, ReleaseApplicationError> {
    let bytes = fs::read(repository_path.join("package.json"))
        .map_err(|_| app_error("RELEASE_FILE_READ_FAILED", "无法读取 package.json。"))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| app_error("RELEASE_FILE_JSON_INVALID", "package.json 无效。"))?;
    value
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| app_error("RELEASE_VERSION_INVALID", "当前版本号无效。"))
}

fn git_error(error: GitReleaseError) -> ReleaseApplicationError {
    app_error(error.code(), error.to_string())
}

fn network_profile(
    proxy: &ReleaseProxySettings,
) -> Result<ReleaseNetworkProfile, ReleaseApplicationError> {
    ReleaseNetworkProfile::new(proxy, std::env::vars_os())
        .map_err(|error| app_error(error.code(), error.to_string()))
}

fn release_phase_is_terminal(phase: ReleasePhase) -> bool {
    matches!(
        phase,
        ReleasePhase::Completed
            | ReleasePhase::CompletedWithWarnings
            | ReleasePhase::Failed
            | ReleasePhase::Cancelled
    )
}

fn github_preflight_error(error: String) -> ReleaseApplicationError {
    match error.as_str() {
        "GH_PROCESS_START_FAILED" => {
            app_error("GITHUB_PROCESS_START_FAILED", "GitHub CLI 进程启动失败。")
        }
        "GH_PROCESS_TIMEOUT" => app_error("GITHUB_PROCESS_TIMEOUT", "GitHub API 请求超时。"),
        "GH_PROCESS_CANCELLED" => app_error("GITHUB_PROCESS_CANCELLED", "GitHub API 请求已取消。"),
        "GH_PROCESS_TREE_TERMINATION_FAILED" => app_error(
            "GITHUB_PROCESS_TREE_TERMINATION_FAILED",
            "GitHub CLI 进程树未能安全结束。",
        ),
        _ => app_error("GITHUB_COMMAND_FAILED", "GitHub API 请求失败。"),
    }
}

fn safe_push_blocked(inspection: &ReleasePreflightResult) -> ReleaseApplicationError {
    if !inspection.repository.clean {
        return app_error("GIT_WORKTREE_DIRTY", "Git 工作区存在未提交改动。");
    }
    match inspection.repository.sync.status {
        crate::models::RepositorySyncStatus::Behind => {
            return app_error("GIT_REPOSITORY_BEHIND", "本地分支落后远端 main。");
        }
        crate::models::RepositorySyncStatus::Diverged => {
            return app_error("GIT_REPOSITORY_DIVERGED", "本地分支与远端 main 已分叉。");
        }
        _ => {}
    }
    if inspection.external.active_release_runs > 0 {
        return app_error(
            "GITHUB_ACTIVE_RELEASE_RUN",
            "已有活动发布工作流，请等待其结束后再继续。",
        );
    }
    if inspection.external.conflicting_drafts > 0 {
        return app_error(
            "GITHUB_CONFLICTING_DRAFT",
            "已有 Draft Release，请先在 GitHub 明确处理。",
        );
    }
    app_error("GIT_SAFE_PUSH_FORBIDDEN", "当前仓库状态不允许安全推送。")
}

fn verify_refreshed_push(
    refreshed: &ReleasePreflightResult,
    expected_head_sha: &str,
) -> Result<(), ReleaseApplicationError> {
    if refreshed.repository.remote_main_sha == expected_head_sha {
        return Ok(());
    }
    Err(app_error(
        "GIT_REMOTE_VERIFICATION_FAILED",
        "推送后远端 main 未通过精确验证。",
    ))
}

fn app_error(code: impl Into<String>, message: impl Into<String>) -> ReleaseApplicationError {
    ReleaseApplicationError::new(code, message)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn emit(events: Option<&dyn ReleaseEventSink>, event: ReleaseEvent) {
    if let Some(events) = events {
        let _ = events.send(event);
    }
}

async fn load_release_log_page(
    git_dir: PathBuf,
    session_id: String,
    before_sequence: Option<u64>,
) -> ReleaseLogPage {
    tokio::task::spawn_blocking(move || {
        let store = ReleaseLogStore::new(git_dir);
        store
            .load_page(&session_id, before_sequence)
            .unwrap_or_else(|_| unavailable_log_page())
    })
    .await
    .unwrap_or_else(|_| unavailable_log_page())
}

fn initialize_release_log_recorder(
    git_dir: PathBuf,
    session_id: &str,
    events: Option<Arc<dyn ReleaseEventSink>>,
) -> ReleaseLogRecorder {
    let store = ReleaseLogStore::new(git_dir);
    if store.initialize(session_id).is_ok() {
        ReleaseLogRecorder::new(session_id, store, 0, events)
    } else {
        ReleaseLogRecorder::volatile(session_id, 0, events)
    }
}

fn open_release_log_recorder(
    git_dir: PathBuf,
    session_id: &str,
    events: Option<Arc<dyn ReleaseEventSink>>,
) -> ReleaseLogRecorder {
    let store = ReleaseLogStore::new(git_dir);
    match store.open(session_id) {
        Ok(opened) => {
            let recorder = ReleaseLogRecorder::new(session_id, store, opened.last_sequence, events);
            if let Some(warning) = opened.warning {
                recorder.record(
                    "releasePipeline",
                    crate::models::ReleaseLogSource::Lifecycle,
                    crate::models::ReleaseLogLevel::Warning,
                    warning,
                );
            }
            recorder
        }
        Err(_) => ReleaseLogRecorder::volatile(session_id, 0, events),
    }
}

fn unavailable_log_page() -> ReleaseLogPage {
    ReleaseLogPage {
        entries: Vec::new(),
        next_before_sequence: None,
        has_earlier: false,
        total_entries: 0,
        total_bytes: 0,
        truncated: false,
        warning: Some(RELEASE_LOG_READ_WARNING.to_string()),
    }
}

async fn watch_session_state(
    store: ReleaseStateStore,
    events: Arc<dyn ReleaseEventSink>,
    mut stop: tokio::sync::watch::Receiver<bool>,
) {
    let mut previous = None;

    loop {
        if let Ok(Some(session)) = store.load()
            && previous.as_ref() != Some(&session)
        {
            previous = Some(session.clone());
            if events
                .send(ReleaseEvent::SessionUpdated {
                    session: Box::new(session),
                })
                .is_err()
            {
                break;
            }
        }

        if *stop.borrow() {
            break;
        }

        tokio::select! {
            changed = stop.changed() => {
                if changed.is_err() {
                    break;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(125)) => {}
        }
    }
}

async fn run_with_session_updates<T>(
    store: ReleaseStateStore,
    events: Option<Arc<dyn ReleaseEventSink>>,
    operation: impl Future<Output = T>,
) -> T {
    let Some(events) = events else {
        return operation.await;
    };
    let (stop_sender, stop) = tokio::sync::watch::channel(false);
    let watcher = tokio::spawn(watch_session_state(store, events, stop));
    let result = operation.await;
    let _ = stop_sender.send(true);
    let _ = watcher.await;
    result
}

fn path_is_protected(path: &Path) -> bool {
    let normalized = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    normalized.contains("\\.codex\\")
        || normalized.ends_with("\\.codex")
        || normalized.contains("\\appdata\\local\\codexrelay\\")
        || normalized.ends_with("\\appdata\\local\\codexrelay")
}

fn should_signal_process_cancel(phase: ReleasePhase) -> bool {
    matches!(
        phase,
        ReleasePhase::ApplyingCandidate
            | ReleasePhase::LocalChecks
            | ReleasePhase::LocalBuild
            | ReleasePhase::SourceAudit
    )
}

fn resume_action(phase: ReleasePhase) -> ResumeAction {
    match phase {
        ReleasePhase::ApplyingCandidate
        | ReleasePhase::LocalChecks
        | ReleasePhase::LocalBuild
        | ReleasePhase::SourceAudit => ResumeAction::RequiresLocalCancel,
        ReleasePhase::Committed => ResumeAction::PushCommitted,
        ReleasePhase::Pushed
        | ReleasePhase::WorkflowQueued
        | ReleasePhase::WorkflowRunning
        | ReleasePhase::AuditingDraft => ResumeAction::RemoteToDraft,
        ReleasePhase::Publishing
        | ReleasePhase::VerifyingPublishedRelease
        | ReleasePhase::MonitoringCleanup => ResumeAction::PublishAndFinalize,
        ReleasePhase::Idle
        | ReleasePhase::Inspected
        | ReleasePhase::Planned
        | ReleasePhase::AwaitingPublishApproval
        | ReleasePhase::Completed
        | ReleasePhase::CompletedWithWarnings
        | ReleasePhase::Failed
        | ReleasePhase::Cancelled => ResumeAction::ReturnSnapshot,
    }
}

fn latest_published_release_tag(releases: &[Value]) -> Option<String> {
    releases
        .iter()
        .find(|release| {
            release.get("draft").and_then(Value::as_bool) == Some(false)
                && release.get("prerelease").and_then(Value::as_bool) == Some(false)
        })
        .and_then(|release| release.get("tag_name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ReleaseLogLevel, ReleaseLogSource};
    use crate::services::release_log::ReleaseLogRecorder;

    #[derive(Default)]
    struct TestEventSink {
        events: Mutex<Vec<ReleaseEvent>>,
    }

    impl ReleaseEventSink for TestEventSink {
        fn send(&self, event: ReleaseEvent) -> Result<(), String> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }
    }

    struct ClosedEventSink;

    impl ReleaseEventSink for ClosedEventSink {
        fn send(&self, _event: ReleaseEvent) -> Result<(), String> {
            Err("release event channel closed".into())
        }
    }

    fn missing_tools(directory: &Path) -> ResolvedTools {
        ResolvedTools {
            git: directory.join("missing-git.exe"),
            node: directory.join("missing-node.exe"),
            npm: directory.join("missing-npm.cmd"),
            cargo: directory.join("missing-cargo.exe"),
            gh: directory.join("missing-gh.exe"),
        }
    }

    fn direct_proxy() -> ReleaseProxySettings {
        ReleaseProxySettings {
            enabled: false,
            proxy_type: crate::models::ReleaseProxyType::Http,
            host: String::new(),
            port: None,
        }
    }

    #[tokio::test]
    async fn starting_release_rotates_old_logs_and_persists_after_the_channel_closes() {
        let directory = tempfile::tempdir().unwrap();
        let repository_path = directory.path().join("repository");
        let git_dir = directory.path().join("git-dir");
        fs::create_dir_all(&repository_path).unwrap();
        fs::create_dir_all(&git_dir).unwrap();

        let old_store = ReleaseLogStore::new(git_dir.clone());
        old_store.initialize("old-session").unwrap();
        ReleaseLogRecorder::new("old-session", old_store, 0, None).record(
            "candidate",
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Info,
            "旧会话日志",
        );

        let application = SystemReleaseApplication::new();
        let plan_id = "plan-log-rotation".to_string();
        application.inner.plans.lock().unwrap().insert(
            plan_id.clone(),
            StoredPlan {
                repository_path: repository_path.clone(),
                git_dir: git_dir.clone(),
                expected_remote_sha: "a".repeat(40),
                plan: ReleaseCandidatePlan {
                    previous_version: "0.4.0".into(),
                    target_version: "0.5.0".into(),
                    files: Vec::new(),
                },
                summary: ReleasePlanSummary {
                    id: plan_id.clone(),
                    repository_path: repository_path.to_string_lossy().into_owned(),
                    previous_version: "0.4.0".into(),
                    target_version: "0.5.0".into(),
                    notes: "测试发布说明".into(),
                    files: Vec::new(),
                },
                tools: missing_tools(directory.path()),
            },
        );

        let session = application
            .start_release(&plan_id, direct_proxy(), Some(Arc::new(ClosedEventSink)))
            .await
            .unwrap();

        let mut page = None;
        for _ in 0..100 {
            if let Ok(candidate) =
                ReleaseLogStore::new(git_dir.clone()).load_page(&session.id, None)
                && !candidate.entries.is_empty()
            {
                page = Some(candidate);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let page = page.expect("新会话应在事件通道断开后继续持久化日志");
        assert!(
            page.entries
                .iter()
                .all(|entry| entry.session_id == session.id)
        );
        assert!(
            page.entries
                .iter()
                .all(|entry| !entry.message.contains("旧会话日志"))
        );

        for _ in 0..100 {
            if !application
                .inner
                .cancellations
                .lock()
                .unwrap()
                .contains_key(&session.id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let final_page = ReleaseLogStore::new(git_dir.clone())
            .load_page(&session.id, None)
            .unwrap();
        let failure_code = ReleaseStateStore::new(git_dir)
            .load()
            .unwrap()
            .unwrap()
            .failure
            .expect("失败管线必须持久化权威失败证据")
            .code;
        assert!(final_page.entries.iter().any(|entry| {
            entry.level == ReleaseLogLevel::Error && entry.message.contains(&failure_code)
        }));
        assert!(final_page.entries.iter().all(|entry| {
            !entry
                .message
                .contains(directory.path().to_string_lossy().as_ref())
        }));
    }

    #[tokio::test]
    async fn resuming_release_opens_existing_logs_and_continues_the_sequence() {
        let directory = tempfile::tempdir().unwrap();
        let repository_path = directory.path().join("repository");
        let git_dir = directory.path().join("git-dir");
        fs::create_dir_all(&repository_path).unwrap();
        fs::create_dir_all(&git_dir).unwrap();
        let mut session = ReleaseSession::new(
            "session-resume-log",
            repository_path.to_string_lossy(),
            "0.5.0",
        );
        session.phase = ReleasePhase::Committed;
        session.candidate_sha = Some("a".repeat(40));
        ReleaseStateStore::new(git_dir.clone())
            .save(&session)
            .unwrap();
        let log_store = ReleaseLogStore::new(git_dir.clone());
        log_store.initialize(&session.id).unwrap();
        ReleaseLogRecorder::new(&session.id, log_store, 0, None).record(
            "candidate",
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Info,
            "此前已完成的发布日志",
        );

        let application = SystemReleaseApplication::new();
        application.inner.sessions.lock().unwrap().insert(
            session.id.clone(),
            SessionContext {
                repository_path,
                git_dir: git_dir.clone(),
                expected_notes: "测试发布说明".into(),
                tools: missing_tools(directory.path()),
            },
        );

        let resumed = application
            .resume_release(&session.id, direct_proxy(), Some(Arc::new(ClosedEventSink)))
            .await
            .unwrap();
        assert_eq!(resumed.phase, ReleasePhase::Committed);

        let mut page = None;
        for _ in 0..100 {
            let candidate = ReleaseLogStore::new(git_dir.clone())
                .load_page(&session.id, None)
                .unwrap();
            if candidate.entries.len() > 1 {
                page = Some(candidate);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let page = page.expect("恢复发布应继续追加持久化日志");
        assert_eq!(page.entries[0].sequence, 1);
        assert_eq!(page.entries[0].message, "此前已完成的发布日志");
        assert!(page.entries.windows(2).all(|pair| {
            pair[1].sequence == pair[0].sequence + 1 && pair[1].session_id == pair[0].session_id
        }));
        assert!(
            page.entries
                .iter()
                .skip(1)
                .any(|entry| entry.step_id == "commitPush")
        );
    }

    #[tokio::test]
    async fn publishing_release_opens_existing_logs_and_continues_the_sequence() {
        let directory = tempfile::tempdir().unwrap();
        let repository_path = directory.path().join("repository");
        let git_dir = directory.path().join("git-dir");
        fs::create_dir_all(&repository_path).unwrap();
        fs::create_dir_all(&git_dir).unwrap();
        let candidate_sha = "b".repeat(40);
        let draft = crate::models::DraftAuditEvidence {
            release_id: 42,
            tag_name: "v0.5.0".into(),
            target_commit_sha: candidate_sha.clone(),
            assets: vec![crate::models::DraftAssetEvidence {
                id: 7,
                name: "CodexRelay_0.5.0_x64-setup.exe".into(),
                size: 1_024,
                sha256: "c".repeat(64),
            }],
            manifest_version: "0.5.0".into(),
            manifest_notes: "测试发布说明".into(),
            signature: "test-signature-not-real".into(),
        };
        let identity = draft.identity();
        let mut session = ReleaseSession::new(
            "session-publish-log",
            repository_path.to_string_lossy(),
            "0.5.0",
        );
        session.phase = ReleasePhase::AwaitingPublishApproval;
        session.candidate_sha = Some(candidate_sha.clone());
        session.remote_main_sha = Some(candidate_sha);
        session.workflow = Some(crate::models::WorkflowDispatch {
            run_id: 41,
            url: "https://github.com/hunxuankai/codex-relay/actions/runs/41".into(),
        });
        session.draft = Some(draft);
        ReleaseStateStore::new(git_dir.clone())
            .save(&session)
            .unwrap();
        let log_store = ReleaseLogStore::new(git_dir.clone());
        log_store.initialize(&session.id).unwrap();
        ReleaseLogRecorder::new(&session.id, log_store, 0, None).record(
            "draftAudit",
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Info,
            "此前已完成的 Draft 审计日志",
        );

        let application = SystemReleaseApplication::new();
        application.inner.sessions.lock().unwrap().insert(
            session.id.clone(),
            SessionContext {
                repository_path,
                git_dir: git_dir.clone(),
                expected_notes: "测试发布说明".into(),
                tools: missing_tools(directory.path()),
            },
        );

        let publishing = application
            .publish_release(
                &session.id,
                identity,
                direct_proxy(),
                Some(Arc::new(ClosedEventSink)),
            )
            .await
            .unwrap();
        assert_eq!(publishing.phase, ReleasePhase::AwaitingPublishApproval);

        let mut page = None;
        for _ in 0..100 {
            let candidate = ReleaseLogStore::new(git_dir.clone())
                .load_page(&session.id, None)
                .unwrap();
            if candidate.entries.len() > 1 {
                page = Some(candidate);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let page = page.expect("公开发布应继续追加持久化日志");
        assert_eq!(page.entries[0].sequence, 1);
        assert!(page.entries.windows(2).all(|pair| {
            pair[1].sequence == pair[0].sequence + 1 && pair[1].session_id == pair[0].session_id
        }));
        assert!(
            page.entries
                .iter()
                .skip(1)
                .any(|entry| entry.step_id == "publishApproval")
        );
    }

    #[tokio::test]
    async fn publish_identity_failure_logs_before_session_and_step_failure_events() {
        let directory = tempfile::tempdir().unwrap();
        let repository_path = directory.path().join("repository");
        let git_dir = directory.path().join("git-dir");
        fs::create_dir_all(&repository_path).unwrap();
        fs::create_dir_all(&git_dir).unwrap();
        let candidate_sha = "d".repeat(40);
        let draft = crate::models::DraftAuditEvidence {
            release_id: 42,
            tag_name: "v0.5.0".into(),
            target_commit_sha: candidate_sha.clone(),
            assets: vec![crate::models::DraftAssetEvidence {
                id: 8,
                name: "CodexRelay_0.5.0_x64-setup.exe".into(),
                size: 2_048,
                sha256: "e".repeat(64),
            }],
            manifest_version: "0.5.0".into(),
            manifest_notes: "测试发布说明".into(),
            signature: "test-signature-not-real".into(),
        };
        let mut mismatched_identity = draft.identity();
        mismatched_identity.release_id += 1;
        let mut session = ReleaseSession::new(
            "session-publish-identity-failure",
            repository_path.to_string_lossy(),
            "0.5.0",
        );
        session.phase = ReleasePhase::AwaitingPublishApproval;
        session.candidate_sha = Some(candidate_sha.clone());
        session.remote_main_sha = Some(candidate_sha);
        session.workflow = Some(crate::models::WorkflowDispatch {
            run_id: 43,
            url: "https://github.com/hunxuankai/codex-relay/actions/runs/43".into(),
        });
        session.draft = Some(draft);
        ReleaseStateStore::new(git_dir.clone())
            .save(&session)
            .unwrap();
        let log_store = ReleaseLogStore::new(git_dir.clone());
        log_store.initialize(&session.id).unwrap();
        ReleaseLogRecorder::new(&session.id, log_store, 0, None).record(
            "draftAudit",
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Info,
            "Draft 审计已完成",
        );

        let application = SystemReleaseApplication::new();
        application.inner.sessions.lock().unwrap().insert(
            session.id.clone(),
            SessionContext {
                repository_path,
                git_dir: git_dir.clone(),
                expected_notes: "测试发布说明".into(),
                tools: missing_tools(directory.path()),
            },
        );
        let events = Arc::new(TestEventSink::default());

        application
            .publish_release(
                &session.id,
                mismatched_identity,
                direct_proxy(),
                Some(events.clone()),
            )
            .await
            .unwrap();

        for _ in 0..100 {
            if events.events.lock().unwrap().iter().any(|event| {
                matches!(
                    event,
                    ReleaseEvent::StepFailed { code, .. }
                        if code == "RELEASE_PUBLISH_IDENTITY_MISMATCH"
                )
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let emitted = events.events.lock().unwrap();
        let log_index = emitted
            .iter()
            .position(|event| {
                matches!(
                    event,
                    ReleaseEvent::StepLog { entry, .. }
                        if entry.level == ReleaseLogLevel::Error
                            && entry.message.contains("RELEASE_PUBLISH_IDENTITY_MISMATCH")
                )
            })
            .expect("身份不匹配应先产生稳定 error 日志");
        let session_index = emitted
            .iter()
            .position(|event| {
                matches!(
                    event,
                    ReleaseEvent::SessionUpdated { session }
                        if session.phase == ReleasePhase::Failed
                            && session.failure.as_ref().is_some_and(|failure| {
                                failure.step_id == "publishApproval"
                                    && failure.code == "RELEASE_PUBLISH_IDENTITY_MISMATCH"
                            })
                )
            })
            .expect("身份不匹配应发送权威失败会话");
        let failed_index = emitted
            .iter()
            .position(|event| {
                matches!(
                    event,
                    ReleaseEvent::StepFailed { step_id, code, .. }
                        if step_id == "publishApproval"
                            && code == "RELEASE_PUBLISH_IDENTITY_MISMATCH"
                )
            })
            .expect("身份不匹配应最后发送步骤失败事件");
        assert!(log_index < session_index && session_index < failed_index);
        drop(emitted);

        let page = ReleaseLogStore::new(git_dir)
            .load_page(&session.id, None)
            .unwrap();
        assert!(page.entries.iter().any(|entry| {
            entry.level == ReleaseLogLevel::Error
                && entry.message.contains("RELEASE_PUBLISH_IDENTITY_MISMATCH")
        }));
    }

    #[tokio::test]
    async fn application_log_pages_are_bounded_and_authorized_by_session_context() {
        let directory = tempfile::tempdir().unwrap();
        let repository_path = directory.path().join("repository");
        let git_dir = directory.path().join("git-dir");
        fs::create_dir_all(&repository_path).unwrap();
        fs::create_dir_all(&git_dir).unwrap();
        let session =
            ReleaseSession::new("session-page", repository_path.to_string_lossy(), "0.5.0");
        ReleaseStateStore::new(git_dir.clone())
            .save(&session)
            .unwrap();
        let log_store = ReleaseLogStore::new(git_dir.clone());
        log_store.initialize(&session.id).unwrap();
        let recorder = ReleaseLogRecorder::new(&session.id, log_store, 0, None);
        for index in 0..2_001 {
            recorder.record(
                "full-project-check",
                ReleaseLogSource::Stdout,
                ReleaseLogLevel::Info,
                format!("诊断记录 {index}"),
            );
        }

        let application = SystemReleaseApplication::new();
        application.inner.sessions.lock().unwrap().insert(
            session.id.clone(),
            SessionContext {
                repository_path,
                git_dir,
                expected_notes: "测试发布说明".into(),
                tools: missing_tools(directory.path()),
            },
        );

        let latest = application.get_logs(&session.id, None).await.unwrap();
        assert_eq!(latest.entries.len(), 2_000);
        assert_eq!(latest.entries.first().unwrap().sequence, 2);
        assert_eq!(latest.entries.last().unwrap().sequence, 2_001);
        assert_eq!(latest.next_before_sequence, Some(2));
        assert!(latest.has_earlier);
        assert_eq!(latest.total_entries, 2_001);

        let earlier = application
            .get_logs(&session.id, latest.next_before_sequence)
            .await
            .unwrap();
        assert_eq!(earlier.entries.len(), 1);
        assert_eq!(earlier.entries[0].sequence, 1);
        assert!(!earlier.has_earlier);

        let error = application
            .get_logs("another-session", None)
            .await
            .unwrap_err();
        assert_eq!(error.code, "RELEASE_SESSION_CONTEXT_MISSING");
    }

    #[tokio::test]
    async fn missing_and_corrupt_logs_return_nonfatal_pages_with_recovery_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let git_dir = directory.path().join("git-dir");
        fs::create_dir_all(&git_dir).unwrap();

        let missing = load_release_log_page(git_dir.clone(), "session-recovery".into(), None).await;
        assert!(missing.entries.is_empty());
        assert_eq!(missing.warning, None);

        let store = ReleaseLogStore::new(git_dir.clone());
        store.initialize("session-recovery").unwrap();
        ReleaseLogRecorder::new("session-recovery", store, 0, None).record(
            "candidate",
            ReleaseLogSource::Lifecycle,
            ReleaseLogLevel::Info,
            "有效诊断记录",
        );
        let log_path = git_dir
            .join("codex-relay-release-console")
            .join("session.log.jsonl");
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&log_path)
            .unwrap();
        std::io::Write::write_all(&mut file, b"{invalid-json}\n").unwrap();
        std::io::Write::flush(&mut file).unwrap();
        drop(file);
        let untrusted_bytes = fs::read(&log_path).unwrap();

        let recovered = load_release_log_page(git_dir, "session-recovery".into(), None).await;
        assert_eq!(recovered.entries.len(), 1);
        assert_eq!(recovered.entries[0].message, "有效诊断记录");
        assert!(
            recovered
                .warning
                .as_deref()
                .is_some_and(|warning| warning.contains("损坏记录"))
        );
        assert_eq!(fs::read(log_path).unwrap(), untrusted_bytes);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn log_initialization_failure_warns_without_changing_start_result() {
        use std::os::windows::fs::OpenOptionsExt;

        let directory = tempfile::tempdir().unwrap();
        let repository_path = directory.path().join("repository");
        let git_dir = directory.path().join("git-dir");
        fs::create_dir_all(&repository_path).unwrap();
        let log_path = git_dir
            .join("codex-relay-release-console")
            .join("session.log.jsonl");
        fs::create_dir_all(log_path.parent().unwrap()).unwrap();
        fs::write(&log_path, b"old-session-log\n").unwrap();
        let _locked_log = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0x0000_0001 | 0x0000_0002)
            .open(&log_path)
            .unwrap();

        let application = SystemReleaseApplication::new();
        let plan_id = "plan-log-initialization-failure".to_string();
        application.inner.plans.lock().unwrap().insert(
            plan_id.clone(),
            StoredPlan {
                repository_path: repository_path.clone(),
                git_dir: git_dir.clone(),
                expected_remote_sha: "f".repeat(40),
                plan: ReleaseCandidatePlan {
                    previous_version: "0.4.0".into(),
                    target_version: "0.5.0".into(),
                    files: Vec::new(),
                },
                summary: ReleasePlanSummary {
                    id: plan_id.clone(),
                    repository_path: repository_path.to_string_lossy().into_owned(),
                    previous_version: "0.4.0".into(),
                    target_version: "0.5.0".into(),
                    notes: "测试发布说明".into(),
                    files: Vec::new(),
                },
                tools: missing_tools(directory.path()),
            },
        );
        let events = Arc::new(TestEventSink::default());

        let session = application
            .start_release(&plan_id, direct_proxy(), Some(events.clone()))
            .await
            .expect("日志初始化失败不得改变发布启动结果");
        assert_eq!(session.phase, ReleasePhase::Idle);
        assert!(events.events.lock().unwrap().iter().any(|event| {
            matches!(
                event,
                ReleaseEvent::StepLog { entry, .. }
                    if entry.level == ReleaseLogLevel::Warning
                        && entry.message.contains("重启后可能丢失")
            )
        }));
        let (session_index, warning_index) = {
            let emitted = events.events.lock().unwrap();
            let session_index = emitted
                .iter()
                .position(|event| {
                    matches!(
                        event,
                        ReleaseEvent::SessionUpdated { session: updated }
                            if updated.id == session.id && updated.phase == ReleasePhase::Idle
                    )
                })
                .expect("前端必须先收到初始会话事实");
            let warning_index = emitted
                .iter()
                .position(|event| {
                    matches!(
                        event,
                        ReleaseEvent::StepLog { entry, .. }
                            if entry.level == ReleaseLogLevel::Warning
                                && entry.message.contains("重启后可能丢失")
                    )
                })
                .expect("日志初始化失败必须显示易失 warning");
            (session_index, warning_index)
        };
        assert!(session_index < warning_index);

        for _ in 0..100 {
            if !application
                .inner
                .cancellations
                .lock()
                .unwrap()
                .contains_key(&session.id)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    #[test]
    fn latest_published_release_tag_skips_drafts_and_prereleases() {
        let releases = vec![
            serde_json::json!({
                "tag_name": "v0.6.0",
                "draft": true,
                "prerelease": false
            }),
            serde_json::json!({
                "tag_name": "v0.5.0-rc.1",
                "draft": false,
                "prerelease": true
            }),
            serde_json::json!({
                "tag_name": "v0.4.0",
                "draft": false,
                "prerelease": false
            }),
        ];

        assert_eq!(
            latest_published_release_tag(&releases),
            Some("v0.4.0".to_string())
        );
        assert_eq!(latest_published_release_tag(&releases[..2]), None);
        assert_eq!(latest_published_release_tag(&[]), None);
    }

    #[test]
    fn github_preflight_errors_preserve_process_failure_categories() {
        assert_eq!(
            github_preflight_error("GH_PROCESS_START_FAILED".into()).code,
            "GITHUB_PROCESS_START_FAILED"
        );
        assert_eq!(
            github_preflight_error("GH_PROCESS_TIMEOUT".into()).code,
            "GITHUB_PROCESS_TIMEOUT"
        );
        assert_eq!(
            github_preflight_error("GH_PROCESS_CANCELLED".into()).code,
            "GITHUB_PROCESS_CANCELLED"
        );
        assert_eq!(
            github_preflight_error("GH_PROCESS_TREE_TERMINATION_FAILED".into()).code,
            "GITHUB_PROCESS_TREE_TERMINATION_FAILED"
        );
        assert_eq!(
            github_preflight_error("GH_COMMAND_FAILED".into()).code,
            "GITHUB_COMMAND_FAILED"
        );
    }

    #[test]
    fn post_push_verification_uses_the_exact_remote_sha_not_release_readiness() {
        let expected_sha = "b".repeat(40);
        let refreshed = ReleasePreflightResult {
            repository_path: "D:\\safe-temp\\repository".into(),
            repository: crate::models::RepositoryInspection {
                local_branch: "master".into(),
                default_branch: "main".into(),
                head_sha: expected_sha.clone(),
                remote_main_sha: expected_sha.clone(),
                remote_url: "https://github.com/hunxuankai/codex-relay.git".into(),
                clean: true,
                sync: crate::models::RepositorySyncInspection {
                    status: crate::models::RepositorySyncStatus::Synced,
                    ahead_count: 0,
                    behind_count: 0,
                    ahead_commits: Vec::new(),
                },
            },
            external: ExternalPreflightSnapshot {
                tools: ToolchainInspection {
                    git: Some("2.50".into()),
                    node: Some("24".into()),
                    npm: Some("11".into()),
                    cargo: Some("1.90".into()),
                    gh: Some("2.76".into()),
                },
                active_release_runs: 1,
                conflicting_drafts: 0,
                latest_release_tag: Some("v0.4.0".into()),
            },
            release_ready: false,
            blocking_reasons: vec!["已有活动发布工作流，请等待其结束后再继续。".into()],
            safe_push: None,
        };

        assert!(verify_refreshed_push(&refreshed, &expected_sha).is_ok());
    }

    #[test]
    fn process_cancellation_is_only_signalled_before_the_remote_push_boundary() {
        assert!(should_signal_process_cancel(ReleasePhase::LocalChecks));
        assert!(should_signal_process_cancel(ReleasePhase::LocalBuild));
        assert!(!should_signal_process_cancel(ReleasePhase::Pushed));
        assert!(!should_signal_process_cancel(
            ReleasePhase::AwaitingPublishApproval
        ));
    }

    #[test]
    fn committed_sessions_resume_at_push_without_repeating_local_work() {
        assert_eq!(
            resume_action(ReleasePhase::Committed),
            ResumeAction::PushCommitted
        );
        assert_eq!(
            resume_action(ReleasePhase::Pushed),
            ResumeAction::RemoteToDraft
        );
        assert_eq!(
            resume_action(ReleasePhase::Publishing),
            ResumeAction::PublishAndFinalize
        );
    }

    #[tokio::test]
    async fn persisted_phase_changes_are_forwarded_to_the_visual_event_channel() {
        let directory = tempfile::tempdir().unwrap();
        let store = ReleaseStateStore::new(directory.path().to_path_buf());
        let mut session = ReleaseSession::new("session-watch", r"D:\safe-temp\repository", "0.5.0");
        store.save(&session).unwrap();
        let sink = Arc::new(TestEventSink::default());
        let (stop_sender, stop) = tokio::sync::watch::channel(false);
        let watcher = tokio::spawn(watch_session_state(
            ReleaseStateStore::new(directory.path().to_path_buf()),
            sink.clone(),
            stop,
        ));

        store
            .advance(&mut session, ReleasePhase::Inspected)
            .unwrap();
        for _ in 0..50 {
            if sink.events.lock().unwrap().iter().any(|event| {
                matches!(
                    event,
                    ReleaseEvent::SessionUpdated { session }
                        if session.phase == ReleasePhase::Inspected
                )
            }) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        stop_sender.send(true).unwrap();
        watcher.await.unwrap();

        assert!(sink.events.lock().unwrap().iter().any(|event| {
            matches!(
                event,
                ReleaseEvent::SessionUpdated { session }
                    if session.phase == ReleasePhase::Inspected
            )
        }));
    }

    #[tokio::test]
    async fn visual_session_forwarding_stops_when_the_pipeline_finishes() {
        let directory = tempfile::tempdir().unwrap();
        let store = ReleaseStateStore::new(directory.path().to_path_buf());
        let mut session =
            ReleaseSession::new("session-watch-stop", r"D:\safe-temp\repository", "0.5.0");
        store.save(&session).unwrap();
        let sink = Arc::new(TestEventSink::default());

        run_with_session_updates(
            ReleaseStateStore::new(directory.path().to_path_buf()),
            Some(sink.clone()),
            async {
                store
                    .advance(&mut session, ReleasePhase::Inspected)
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(150)).await;
            },
        )
        .await;
        let event_count_after_pipeline = sink.events.lock().unwrap().len();

        store.advance(&mut session, ReleasePhase::Planned).unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert_eq!(
            sink.events.lock().unwrap().len(),
            event_count_after_pipeline
        );
    }

    #[test]
    fn push_failure_keeps_the_committed_session_recoverable() {
        let directory = tempfile::tempdir().unwrap();
        let store = ReleaseStateStore::new(directory.path().to_path_buf());
        let mut session = ReleaseSession::new(
            "session-committed-error",
            r"D:\safe-temp\repository",
            "0.5.0",
        );
        session.phase = ReleasePhase::Committed;
        session.candidate_sha = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        store.save(&session).unwrap();
        let sink = TestEventSink::default();

        SystemReleaseApplication::new().finish_with_error(
            &session.id,
            &store,
            Some(&sink),
            None,
            "RELEASE_PUSH_FAILED",
        );

        assert_eq!(store.load().unwrap().unwrap(), session);
        assert!(sink.events.lock().unwrap().iter().any(|event| {
            matches!(
                event,
                ReleaseEvent::StepFailed { code, .. } if code == "RELEASE_PUSH_FAILED"
            )
        }));
    }

    #[test]
    fn local_verification_nonzero_exit_emits_specific_safe_failure_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let store = ReleaseStateStore::new(directory.path().to_path_buf());
        let session = ReleaseSession::new(
            "session-local-verification-exit",
            r"D:\safe-temp\repository",
            "0.5.0",
        );
        store.save(&session).unwrap();
        let sink = TestEventSink::default();
        let error = crate::services::release_orchestrator::ReleaseOrchestratorError::LocalVerificationFailed {
            command_id: "release-structure-tests".into(),
            failure: crate::services::local_verification::LocalVerificationFailure::ExitCode(1),
        };

        SystemReleaseApplication::new().finish_with_orchestrator_error(
            &session.id,
            &store,
            Some(&sink),
            None,
            &error,
        );

        let persisted = store.load().unwrap().unwrap();
        assert_eq!(persisted.phase, ReleasePhase::Failed);
        assert_eq!(
            persisted.failure,
            Some(crate::models::ReleaseFailureEvidence {
                phase: ReleasePhase::Idle,
                step_id: "release-structure-tests".into(),
                code: "RELEASE_LOCAL_VERIFICATION_FAILED".into(),
            })
        );
        assert!(sink.events.lock().unwrap().iter().any(|event| {
            matches!(
                event,
                ReleaseEvent::StepFailed {
                    step_id,
                    code,
                    message,
                } if step_id == "release-structure-tests"
                    && code == "RELEASE_LOCAL_VERIFICATION_FAILED"
                    && message == "本地发布门禁退出码 1；候选文件已回滚，尚未提交或推送。"
            )
        }));
    }

    #[test]
    fn local_verification_timeout_emits_specific_safe_failure_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let store = ReleaseStateStore::new(directory.path().to_path_buf());
        let session = ReleaseSession::new(
            "session-local-verification-start",
            r"D:\safe-temp\repository",
            "0.5.0",
        );
        store.save(&session).unwrap();
        let sink = TestEventSink::default();
        let error = crate::services::release_orchestrator::ReleaseOrchestratorError::LocalVerificationFailed {
            command_id: "release-structure-tests".into(),
            failure: crate::services::local_verification::LocalVerificationFailure::Process(
                crate::services::local_verification::LocalVerificationProcessError::Timeout,
            ),
        };

        SystemReleaseApplication::new().finish_with_orchestrator_error(
            &session.id,
            &store,
            Some(&sink),
            None,
            &error,
        );

        assert!(sink.events.lock().unwrap().iter().any(|event| {
            matches!(
                event,
                ReleaseEvent::StepFailed {
                    step_id,
                    code,
                    message,
                } if step_id == "release-structure-tests"
                    && code == "RELEASE_LOCAL_VERIFICATION_FAILED"
                    && message == "本地发布门禁超过允许时间；候选文件已回滚，尚未提交或推送。"
            )
        }));
    }

    #[test]
    fn local_verification_process_failures_keep_specific_safe_messages() {
        use crate::services::local_verification::{
            LocalVerificationFailure, LocalVerificationProcessError,
        };

        let cases = [
            (
                LocalVerificationProcessError::JobUnavailable,
                "本地发布门禁进程无法安全启动；候选文件已回滚，尚未提交或推送。",
            ),
            (
                LocalVerificationProcessError::OutputTooLarge,
                "本地发布门禁输出超过安全上限；候选文件已回滚，尚未提交或推送。",
            ),
            (
                LocalVerificationProcessError::Timeout,
                "本地发布门禁超过允许时间；候选文件已回滚，尚未提交或推送。",
            ),
            (
                LocalVerificationProcessError::ProcessTreeTermination,
                "本地发布门禁进程树未能安全结束；候选文件已回滚，尚未提交或推送。",
            ),
            (
                LocalVerificationProcessError::OutputRead,
                "无法完整读取本地发布门禁结果；候选文件已回滚，尚未提交或推送。",
            ),
            (
                LocalVerificationProcessError::InputWrite,
                "本地发布门禁输入边界失败；候选文件已回滚，尚未提交或推送。",
            ),
        ];

        for (failure, expected) in cases {
            let error = ReleaseOrchestratorError::LocalVerificationFailed {
                command_id: "full-project-check".into(),
                failure: LocalVerificationFailure::Process(failure),
            };

            assert_eq!(error.failure_message(), expected);
        }
    }

    #[test]
    fn already_failed_session_reemits_the_persisted_failure_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let store = ReleaseStateStore::new(directory.path().to_path_buf());
        let mut session = ReleaseSession::new(
            "session-persisted-local-failure",
            r"D:\safe-temp\repository",
            "0.5.0",
        );
        session.phase = ReleasePhase::LocalChecks;
        store.save(&session).unwrap();
        store
            .fail(
                &mut session,
                "full-project-check",
                "RELEASE_LOCAL_VERIFICATION_FAILED",
            )
            .unwrap();
        let sink = TestEventSink::default();
        let error = crate::services::release_orchestrator::ReleaseOrchestratorError::LocalVerificationFailed {
            command_id: "full-project-check".into(),
            failure: crate::services::local_verification::LocalVerificationFailure::ExitCode(1),
        };

        SystemReleaseApplication::new().finish_with_orchestrator_error(
            &session.id,
            &store,
            Some(&sink),
            None,
            &error,
        );

        assert!(sink.events.lock().unwrap().iter().any(|event| {
            matches!(
                event,
                ReleaseEvent::SessionUpdated { session: snapshot }
                    if snapshot.phase == ReleasePhase::Failed
                        && snapshot.failure == session.failure
            )
        }));
    }

    #[test]
    fn duplicate_pipeline_registration_for_the_same_session_is_rejected() {
        let application = SystemReleaseApplication::new();

        application.register_pipeline("session-duplicate").unwrap();
        let error = application
            .register_pipeline("session-duplicate")
            .unwrap_err();

        assert_eq!(error.code, "RELEASE_SESSION_ALREADY_RUNNING");
    }
}
