use crate::app_state::{
    ApplicationRequest, ApplicationResponse, ReleaseApplicationBackend, ReleaseApplicationError,
    ReleaseEventSink,
};
use crate::infrastructure::gh::{GhBackend, GhOperation, GhRequest, SystemGhBackend};
use crate::infrastructure::git::GitBackend;
use crate::infrastructure::local_verification::ProcessLocalVerificationBackend;
use crate::infrastructure::process::{ProcessInvocation, SafeProcessRunner};
use crate::models::{
    ExternalPreflightSnapshot, ReleaseConnectionTestResult, ReleaseEvent, ReleasePhase,
    ReleasePlanFileSummary, ReleasePlanSummary, ReleasePreflightResult, ReleaseProxySettings,
    ReleaseSession, SafeRepositoryPushRequest, ToolchainInspection,
};
use crate::services::git_release::{
    GitReleaseError, GitReleaseService, RepositoryInspectionService, project_release_preflight,
};
use crate::services::release_candidate::{ReleaseCandidatePlan, ReleaseCandidateTransaction};
use crate::services::release_network::{
    ReleaseConnectionService, ReleaseNetworkProfile, SystemReleaseConnectionProbeBackend,
};
use crate::services::release_notes::{CommitSummary, ReleaseNotesService};
use crate::services::release_orchestrator::{
    GitReleasePushBackend, GithubRemoteBackend, ReleaseOrchestrator,
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
                .map(ApplicationResponse::OptionalSession),
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
        let application = self.clone();
        let initial = session.clone();
        tauri::async_runtime::spawn(async move {
            application
                .run_initial_pipeline(initial, stored, profile, cancel, events)
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
            let verification = ProcessLocalVerificationBackend::new(
                stored.tools.npm.clone(),
                stored.tools.cargo.clone(),
                environment.clone(),
                cancel.clone(),
            );
            let push = GitReleasePushBackend::new(git, stored.expected_remote_sha.clone());
            let store = ReleaseStateStore::new(stored.git_dir.clone());
            let result = ReleaseOrchestrator::new()
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
                self.finish_with_error(&session.id, &store, events.as_deref(), error.code());
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
            let remote = GithubRemoteBackend::new(&gh);
            match ReleaseOrchestrator::new()
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
                    self.finish_with_error(&session.id, &store, events.as_deref(), error.code());
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
    ) -> Result<Option<ReleaseSession>, ReleaseApplicationError> {
        let (_, tools, git_dir) = self
            .inspect_repository(repository_path.clone(), PreflightPurpose::Recovery)
            .await?;
        let store = ReleaseStateStore::new(git_dir.clone());
        let session = store
            .load()
            .map_err(|_| app_error("RELEASE_STATE_INVALID", "发布会话状态无效。"))?;
        if let Some(session) = &session {
            let notes = fs::read_to_string(repository_path.join(".github/release-notes.md"))
                .map_err(|_| app_error("RELEASE_NOTES_READ_FAILED", "无法读取发布说明。"))?;
            self.inner.sessions.lock().unwrap().insert(
                session.id.clone(),
                SessionContext {
                    repository_path: repository_path
                        .canonicalize()
                        .map_err(|_| app_error("GIT_REPOSITORY_INVALID", "无法读取 Git 仓库。"))?,
                    git_dir,
                    expected_notes: notes,
                    tools,
                },
            );
        }
        Ok(session)
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
                .run_resume_pipeline(initial, context, profile, cancel, events)
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
    ) {
        let watch_store = ReleaseStateStore::new(context.git_dir.clone());
        run_with_session_updates(watch_store, events.clone(), async {
            let environment = profile.environment().to_vec();
            let store = ReleaseStateStore::new(context.git_dir.clone());
            if resume_action(session.phase) == ResumeAction::PushCommitted {
                let git = GitBackend::new_cancellable_with_proxy(
                    context.tools.git.clone(),
                    environment.clone(),
                    profile.git_proxy_mode().clone(),
                    cancel.clone(),
                );
                let push = GitReleasePushBackend::for_committed(git);
                if let Err(error) = ReleaseOrchestrator::new()
                    .push_committed(
                        &mut session,
                        &store,
                        &context.repository_path,
                        &context.git_dir,
                        &push,
                    )
                    .await
                {
                    self.finish_with_error(&session.id, &store, events.as_deref(), error.code());
                    return;
                }
            }
            let gh = SystemGhBackend::new(
                context.tools.gh.clone(),
                environment,
                context.repository_path.clone(),
                cancel,
            );
            let remote = GithubRemoteBackend::new(&gh);
            let result = match resume_action(session.phase) {
                ResumeAction::RemoteToDraft => ReleaseOrchestrator::new()
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
                                "RELEASE_REMOTE_STATE_INVALID",
                            );
                            return;
                        }
                    };
                    ReleaseOrchestrator::new()
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
                Err(error) => {
                    self.finish_with_error(&session.id, &store, events.as_deref(), error.code())
                }
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
                let remote = GithubRemoteBackend::new(&gh);
                let store = ReleaseStateStore::new(context.git_dir.clone());
                let mut current = initial;
                match ReleaseOrchestrator::new()
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
                    Err(error) => application.finish_with_error(
                        &current.id,
                        &store,
                        events.as_deref(),
                        error.code(),
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
        code: &str,
    ) {
        if let Ok(Some(mut current)) = store.load()
            && !matches!(
                current.phase,
                ReleasePhase::Committed | ReleasePhase::Failed | ReleasePhase::Cancelled
            )
        {
            let _ = store.advance(&mut current, ReleasePhase::Failed);
            emit(
                events,
                ReleaseEvent::SessionUpdated {
                    session: Box::new(current),
                },
            );
        }
        emit(
            events,
            ReleaseEvent::StepFailed {
                step_id: "releasePipeline".into(),
                code: code.into(),
                message: "发布流程失败，请查看对应阶段证据。".into(),
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
    fn duplicate_pipeline_registration_for_the_same_session_is_rejected() {
        let application = SystemReleaseApplication::new();

        application.register_pipeline("session-duplicate").unwrap();
        let error = application
            .register_pipeline("session-duplicate")
            .unwrap_err();

        assert_eq!(error.code, "RELEASE_SESSION_ALREADY_RUNNING");
    }
}
