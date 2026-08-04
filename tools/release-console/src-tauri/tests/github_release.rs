use codex_relay_release_console_lib::infrastructure::gh::{
    GhBackend, GhOperation, GhRequest, GhResponse, SystemGhBackend,
};
use codex_relay_release_console_lib::services::github_release::{
    DraftAuditService, GithubReleaseService, PublishedReleaseEvidence,
};
use codex_relay_release_console_lib::services::release_log::{
    ReleaseLogRecorder, ReleaseLogStore, ReleaseProgressSink,
};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

struct FixtureGhBackend {
    requests: Mutex<Vec<GhRequest>>,
}

struct DispatchFallbackGhBackend {
    requests: Mutex<Vec<GhRequest>>,
}

struct EventuallyVisibleRunGhBackend {
    list_calls: AtomicUsize,
}

struct RunViewGhBackend {
    requests: Mutex<Vec<GhRequest>>,
}

struct FailedRunGhBackend;

struct DraftFixtureGhBackend {
    requests: Mutex<Vec<GhRequest>>,
    downloads: Mutex<Vec<(u64, PathBuf)>>,
    notes: String,
    digest_overrides: BTreeMap<u64, String>,
    scenario: DraftScenario,
}

struct PublishingGhBackend {
    inner: DraftFixtureGhBackend,
    publish_calls: AtomicUsize,
}

struct PublishedFixtureGhBackend {
    inner: DraftFixtureGhBackend,
}

struct CleanupFixtureGhBackend {
    conclusion: &'static str,
    requests: Mutex<Vec<GhRequest>>,
}

struct AlreadyPublishedGhBackend {
    inner: PublishedFixtureGhBackend,
    publish_calls: AtomicUsize,
}

#[derive(Clone, Copy, Debug)]
enum DraftScenario {
    Valid,
    TagUnavailable,
    PlatformNormalizedNotes,
    MissingAsset,
    ExtraAsset,
    ReleaseNotesDrift,
    SizeDrift,
    ManifestNotesDrift,
    ManifestUrlDrift,
    SignatureDrift,
    TagDrift,
    ReleaseIdDrift,
}

fn draft_fixture(notes: &str, scenario: DraftScenario) -> DraftFixtureGhBackend {
    DraftFixtureGhBackend {
        requests: Mutex::new(Vec::new()),
        downloads: Mutex::new(Vec::new()),
        notes: notes.into(),
        digest_overrides: BTreeMap::new(),
        scenario,
    }
}

fn sha256_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let hex = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

fn platform_normalized_notes(notes: &str) -> String {
    notes.replace('\n', "\r\n").trim_end().to_string()
}

fn manifest_bytes(notes: &str, scenario: DraftScenario) -> Vec<u8> {
    let manifest_notes = match scenario {
        DraftScenario::ManifestNotesDrift => "说明发生漂移".to_string(),
        DraftScenario::PlatformNormalizedNotes => platform_normalized_notes(notes),
        _ => notes.to_string(),
    };
    let asset_url = if matches!(scenario, DraftScenario::ManifestUrlDrift) {
        "https://api.github.com/repos/hunxuankai/codex-relay/releases/assets/999"
    } else {
        "https://api.github.com/repos/hunxuankai/codex-relay/releases/assets/501"
    };
    serde_json::to_vec(&serde_json::json!({
        "version": "0.5.0",
        "notes": manifest_notes,
        "pub_date": "2026-07-31T10:02:00Z",
        "platforms": {
            "windows-x86_64": {
                "url": asset_url,
                "signature": "signature-test-not-real"
            },
            "windows-x86_64-nsis": {
                "url": asset_url,
                "signature": "signature-test-not-real"
            }
        }
    }))
    .unwrap()
}

impl GhBackend for RunViewGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        self.requests.lock().unwrap().push(request);
        Box::pin(async {
            Ok(GhResponse {
                stdout: r#"{
  "databaseId": 123,
  "status": "completed",
  "conclusion": "success",
  "headSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "url": "https://github.com/hunxuankai/codex-relay/actions/runs/123",
  "jobs": [
    {
      "name": "发布 Windows 更新",
      "status": "completed",
      "conclusion": "success",
      "startedAt": "2026-07-31T10:00:00Z",
      "completedAt": "2026-07-31T10:01:30Z",
      "steps": [
        {
          "name": "检出源码",
          "number": 1,
          "status": "completed",
          "conclusion": "success",
          "startedAt": "2026-07-31T10:00:05Z",
          "completedAt": "2026-07-31T10:00:15Z"
        }
      ]
    }
  ]
}"#
                .as_bytes()
                .to_vec(),
            })
        })
    }

    fn download_asset<'a>(
        &'a self,
        _asset_id: u64,
        _destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Err("download not expected".into()) })
    }
}

impl GhBackend for FailedRunGhBackend {
    fn execute<'a>(
        &'a self,
        _request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        Box::pin(async {
            Ok(GhResponse {
                stdout: br#"{
  "databaseId": 124,
  "status": "completed",
  "conclusion": "failure",
  "headSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "url": "https://github.com/hunxuankai/codex-relay/actions/runs/124",
  "jobs": []
}"#
                .to_vec(),
            })
        })
    }

    fn download_asset<'a>(
        &'a self,
        _asset_id: u64,
        _destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Err("download not expected".into()) })
    }
}

impl GhBackend for DraftFixtureGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        self.requests.lock().unwrap().push(request.clone());
        let notes = self.notes.clone();
        let digest_overrides = self.digest_overrides.clone();
        let scenario = self.scenario;
        Box::pin(async move {
            let installer = b"installer".to_vec();
            let signature = if matches!(scenario, DraftScenario::SignatureDrift) {
                b"different-signature-not-real\n".to_vec()
            } else {
                b"signature-test-not-real\n".to_vec()
            };
            let manifest = manifest_bytes(&notes, scenario);
            let digest = |asset_id, bytes: &[u8]| {
                digest_overrides
                    .get(&asset_id)
                    .cloned()
                    .unwrap_or_else(|| sha256_digest(bytes))
            };
            let stdout = match request.operation {
                GhOperation::ListDraftReleases => {
                    let release_notes = if matches!(scenario, DraftScenario::PlatformNormalizedNotes)
                    {
                        platform_normalized_notes(&notes)
                    } else {
                        notes.clone()
                    };
                    let mut assets = vec![
                        serde_json::json!({"id": 501, "name": "Codex.Relay_0.5.0_x64-setup.exe", "size": if matches!(scenario, DraftScenario::SizeDrift) { installer.len() + 1 } else { installer.len() }, "digest": digest(501, &installer)}),
                        serde_json::json!({"id": 502, "name": "Codex.Relay_0.5.0_x64-setup.exe.sig", "size": signature.len(), "digest": digest(502, &signature)}),
                        serde_json::json!({"id": 503, "name": "latest.json", "size": manifest.len(), "digest": digest(503, &manifest)}),
                    ];
                    if matches!(scenario, DraftScenario::MissingAsset) {
                        assets.pop();
                    }
                    if matches!(scenario, DraftScenario::ExtraAsset) {
                        assets.push(serde_json::json!({
                            "id": 504,
                            "name": "unexpected.zip",
                            "size": 1,
                            "digest": format!("sha256:{}", "0".repeat(64))
                        }));
                    }
                    serde_json::to_vec(&serde_json::json!([{
                    "id": if matches!(scenario, DraftScenario::ReleaseIdDrift) { 43 } else { 42 },
                    "tag_name": "v0.5.0",
                    "name": "Codex Relay v0.5.0",
                    "target_commitish": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "draft": true,
                    "prerelease": false,
                    "body": if matches!(scenario, DraftScenario::ReleaseNotesDrift) { "说明发生漂移" } else { release_notes.as_str() },
                    "assets": assets
                }]))
                    .unwrap()
                }
                GhOperation::GetTag if matches!(scenario, DraftScenario::TagUnavailable) => {
                    return Err("draft tag is unavailable before publication".into());
                }
                GhOperation::GetTag => serde_json::to_vec(&serde_json::json!({
                    "object": {
                        "type": "commit",
                        "sha": if matches!(scenario, DraftScenario::TagDrift) { "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" } else { "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
                    }
                }))
                .unwrap(),
                _ => return Err("unexpected gh request".into()),
            };
            Ok(GhResponse { stdout })
        })
    }

    fn download_asset<'a>(
        &'a self,
        asset_id: u64,
        destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        self.downloads
            .lock()
            .unwrap()
            .push((asset_id, destination.to_path_buf()));
        let notes = self.notes.clone();
        let scenario = self.scenario;
        Box::pin(async move {
            let bytes = match asset_id {
                501 => b"installer".to_vec(),
                502 if matches!(scenario, DraftScenario::SignatureDrift) => {
                    b"different-signature-not-real\n".to_vec()
                }
                502 => b"signature-test-not-real\n".to_vec(),
                503 => manifest_bytes(&notes, scenario),
                _ => return Err("unknown asset".into()),
            };
            fs::write(destination, bytes).map_err(|_| "fixture write failed".to_string())
        })
    }
}

impl GhBackend for FixtureGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        self.requests.lock().unwrap().push(request);
        Box::pin(async {
            Ok(GhResponse {
                stdout: b"https://github.com/hunxuankai/codex-relay/actions/runs/123\n".to_vec(),
            })
        })
    }

    fn download_asset<'a>(
        &'a self,
        _asset_id: u64,
        _destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Err("download not expected".into()) })
    }
}

impl GhBackend for PublishingGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        if request.operation == GhOperation::PublishRelease {
            self.publish_calls.fetch_add(1, Ordering::SeqCst);
            return Box::pin(async move {
                Ok(GhResponse {
                    stdout: serde_json::to_vec(&serde_json::json!({
                        "id": request.resource_id,
                        "tag_name": "v0.5.0",
                        "draft": false,
                        "prerelease": false,
                        "published_at": "2026-07-31T11:00:00Z"
                    }))
                    .unwrap(),
                })
            });
        }
        self.inner.execute(request)
    }

    fn download_asset<'a>(
        &'a self,
        asset_id: u64,
        destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        self.inner.download_asset(asset_id, destination)
    }
}

impl GhBackend for PublishedFixtureGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        if request.operation == GhOperation::LatestRelease {
            let notes = self.inner.notes.clone();
            return Box::pin(async move {
                let installer = b"installer".to_vec();
                let signature = b"signature-test-not-real\n".to_vec();
                let manifest = manifest_bytes(&notes, DraftScenario::Valid);
                Ok(GhResponse {
                    stdout: serde_json::to_vec(&serde_json::json!({
                        "id": 42,
                        "tag_name": "v0.5.0",
                        "name": "Codex Relay v0.5.0",
                        "target_commitish": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "draft": false,
                        "prerelease": false,
                        "published_at": "2026-07-31T11:00:00Z",
                        "body": notes,
                        "assets": [
                            {"id": 501, "name": "Codex.Relay_0.5.0_x64-setup.exe", "size": installer.len(), "digest": sha256_digest(&installer)},
                            {"id": 502, "name": "Codex.Relay_0.5.0_x64-setup.exe.sig", "size": signature.len(), "digest": sha256_digest(&signature)},
                            {"id": 503, "name": "latest.json", "size": manifest.len(), "digest": sha256_digest(&manifest)}
                        ]
                    }))
                    .unwrap(),
                })
            });
        }
        self.inner.execute(request)
    }

    fn download_asset<'a>(
        &'a self,
        asset_id: u64,
        destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        self.inner.download_asset(asset_id, destination)
    }
}

impl GhBackend for CleanupFixtureGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        self.requests.lock().unwrap().push(request.clone());
        Box::pin(async move {
            let stdout = match request.operation {
                GhOperation::CleanupRuns => serde_json::to_vec(&serde_json::json!([{
                    "databaseId": 900,
                    "status": "completed",
                    "conclusion": self.conclusion,
                    "createdAt": "2026-07-31T11:00:02Z",
                    "url": "https://github.com/hunxuankai/codex-relay/actions/runs/900"
                }]))
                .unwrap(),
                GhOperation::ViewReleaseRun => serde_json::to_vec(&serde_json::json!({
                    "databaseId": 900,
                    "status": "completed",
                    "conclusion": self.conclusion,
                    "headSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "url": "https://github.com/hunxuankai/codex-relay/actions/runs/900",
                    "jobs": []
                }))
                .unwrap(),
                _ => return Err("unexpected gh request".into()),
            };
            Ok(GhResponse { stdout })
        })
    }

    fn download_asset<'a>(
        &'a self,
        _asset_id: u64,
        _destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Err("download not expected".into()) })
    }
}

impl GhBackend for AlreadyPublishedGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        match request.operation {
            GhOperation::ListDraftReleases => Box::pin(async {
                Ok(GhResponse {
                    stdout: b"[]".to_vec(),
                })
            }),
            GhOperation::PublishRelease => {
                self.publish_calls.fetch_add(1, Ordering::SeqCst);
                Box::pin(async { Err("publish must not repeat".into()) })
            }
            _ => self.inner.execute(request),
        }
    }

    fn download_asset<'a>(
        &'a self,
        asset_id: u64,
        destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        self.inner.download_asset(asset_id, destination)
    }
}

impl GhBackend for DispatchFallbackGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        self.requests.lock().unwrap().push(request.clone());
        Box::pin(async move {
            let stdout = match request.operation {
                GhOperation::DispatchReleaseWorkflow => Vec::new(),
                GhOperation::ListReleaseRuns => serde_json::to_vec(&serde_json::json!([
                    {
                        "databaseId": 320,
                        "headSha": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                        "createdAt": "2099-07-31T10:00:00Z",
                        "url": "https://github.com/hunxuankai/codex-relay/actions/runs/320"
                    },
                    {
                        "databaseId": 321,
                        "headSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                        "createdAt": "2099-07-31T10:00:01Z",
                        "url": "https://github.com/hunxuankai/codex-relay/actions/runs/321"
                    }
                ]))
                .unwrap(),
                _ => return Err("unexpected gh request".into()),
            };
            Ok(GhResponse { stdout })
        })
    }

    fn download_asset<'a>(
        &'a self,
        _asset_id: u64,
        _destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Err("download not expected".into()) })
    }
}

impl GhBackend for EventuallyVisibleRunGhBackend {
    fn execute<'a>(
        &'a self,
        request: GhRequest,
    ) -> Pin<Box<dyn Future<Output = Result<GhResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let stdout = match request.operation {
                GhOperation::DispatchReleaseWorkflow => Vec::new(),
                GhOperation::ListReleaseRuns => {
                    if self.list_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                        b"[]".to_vec()
                    } else {
                        serde_json::to_vec(&serde_json::json!([{
                            "databaseId": 322,
                            "headSha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                            "createdAt": "2099-07-31T10:00:01Z",
                            "url": "https://github.com/hunxuankai/codex-relay/actions/runs/322"
                        }]))
                        .unwrap()
                    }
                }
                _ => return Err("unexpected gh request".into()),
            };
            Ok(GhResponse { stdout })
        })
    }

    fn download_asset<'a>(
        &'a self,
        _asset_id: u64,
        _destination: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async { Err("download not expected".into()) })
    }
}

#[test]
fn dispatch_uses_fixed_workflow_and_structured_json_stdin() {
    let backend = FixtureGhBackend {
        requests: Mutex::new(Vec::new()),
    };
    let service = GithubReleaseService::new();
    let candidate_sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    let dispatched =
        tauri::async_runtime::block_on(service.dispatch_release(&backend, "0.5.0", candidate_sha))
            .unwrap();

    assert_eq!(dispatched.run_id, 123);
    assert_eq!(
        dispatched.url,
        "https://github.com/hunxuankai/codex-relay/actions/runs/123"
    );
    let requests = backend.requests.into_inner().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].operation, GhOperation::DispatchReleaseWorkflow);
    assert_eq!(requests[0].repository, "hunxuankai/codex-relay");
    assert_eq!(requests[0].workflow.as_deref(), Some("release.yml"));
    assert_eq!(requests[0].git_ref.as_deref(), Some("main"));
    let stdin: serde_json::Value =
        serde_json::from_slice(requests[0].stdin.as_deref().unwrap()).unwrap();
    assert_eq!(stdin["expected_version"], "0.5.0");
    assert_eq!(stdin["expected_sha"], candidate_sha);
}

#[test]
fn github_release_errors_expose_stable_codes() {
    use codex_relay_release_console_lib::services::github_release::GithubReleaseError;

    let cases = [
        (GithubReleaseError::BackendFailed, "GITHUB_BACKEND_FAILED"),
        (
            GithubReleaseError::InvalidResponse,
            "GITHUB_RESPONSE_INVALID",
        ),
        (
            GithubReleaseError::CandidateMismatch,
            "GITHUB_RUN_SHA_MISMATCH",
        ),
        (GithubReleaseError::WorkflowRunFailed, "GITHUB_RUN_FAILED"),
        (
            GithubReleaseError::WorkflowRunNotUnique,
            "GITHUB_RUN_NOT_UNIQUE",
        ),
        (
            GithubReleaseError::DraftNotUnique,
            "GITHUB_DRAFT_NOT_UNIQUE",
        ),
        (
            GithubReleaseError::DraftAuditFailed,
            "GITHUB_DRAFT_AUDIT_FAILED",
        ),
        (
            GithubReleaseError::DraftIdentityChanged,
            "GITHUB_DRAFT_IDENTITY_CHANGED",
        ),
        (
            GithubReleaseError::PublishedAuditFailed,
            "GITHUB_PUBLISHED_AUDIT_FAILED",
        ),
        (
            GithubReleaseError::CleanupRunNotUnique,
            "GITHUB_CLEANUP_RUN_NOT_UNIQUE",
        ),
        (
            GithubReleaseError::AssetDownloadFailed,
            "GITHUB_ASSET_DOWNLOAD_FAILED",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.code(), expected);
    }
}

#[test]
fn dispatch_finds_the_unique_new_run_when_gh_returns_no_url() {
    let backend = DispatchFallbackGhBackend {
        requests: Mutex::new(Vec::new()),
    };

    let dispatched = tauri::async_runtime::block_on(GithubReleaseService::new().dispatch_release(
        &backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ))
    .unwrap();

    assert_eq!(dispatched.run_id, 321);
    assert_eq!(
        dispatched.url,
        "https://github.com/hunxuankai/codex-relay/actions/runs/321"
    );
    let requests = backend.requests.into_inner().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.operation)
            .collect::<Vec<_>>(),
        [
            GhOperation::DispatchReleaseWorkflow,
            GhOperation::ListReleaseRuns
        ]
    );
}

#[test]
fn dispatch_waits_for_the_new_run_to_become_visible() {
    let backend = EventuallyVisibleRunGhBackend {
        list_calls: AtomicUsize::new(0),
    };

    let dispatched = tauri::async_runtime::block_on(GithubReleaseService::new().dispatch_release(
        &backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ))
    .unwrap();

    assert_eq!(dispatched.run_id, 322);
    assert_eq!(backend.list_calls.load(Ordering::SeqCst), 2);
}

#[test]
fn system_gh_backend_builds_fixed_dispatch_and_direct_asset_download_invocations() {
    let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
    let backend = SystemGhBackend::new(
        PathBuf::from(r"D:\tools\gh.exe"),
        vec![
            (OsString::from("PATH"), OsString::from(r"D:\safe-bin")),
            (
                OsString::from("GH_TOKEN"),
                OsString::from("github_pat_test-token-not-real"),
            ),
        ],
        PathBuf::from(r"D:\safe-temp\repository"),
        cancel,
    );
    let request = GhRequest {
        operation: GhOperation::DispatchReleaseWorkflow,
        repository: "hunxuankai/codex-relay".into(),
        workflow: Some("release.yml".into()),
        git_ref: Some("main".into()),
        tag_name: None,
        head_sha: None,
        created_after: None,
        resource_id: None,
        stdin: Some(br#"{"expected_version":"0.5.0","expected_sha":"aaaa"}"#.to_vec()),
    };

    let dispatch = backend.invocation_for(&request).unwrap();
    assert_eq!(dispatch.executable, PathBuf::from(r"D:\tools\gh.exe"));
    assert_eq!(
        dispatch.args,
        [
            "workflow",
            "run",
            "release.yml",
            "--repo",
            "hunxuankai/codex-relay",
            "--ref",
            "main",
            "--json",
        ]
    );
    assert_eq!(dispatch.stdin, request.stdin);
    assert!(dispatch.stdout_file.is_none());
    assert_eq!(dispatch.env.len(), 1);

    let connection = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::ConnectionTest,
            repository: "hunxuankai/codex-relay".into(),
            workflow: None,
            git_ref: None,
            tag_name: None,
            head_sha: None,
            created_after: None,
            resource_id: None,
            stdin: None,
        })
        .unwrap();
    assert_eq!(
        connection.args,
        ["api", "repos/hunxuankai/codex-relay", "--silent",]
    );

    let destination = PathBuf::from(r"D:\safe-temp\artifacts\installer.exe");
    let download = backend.asset_download_invocation(77, &destination);
    assert_eq!(
        download.args,
        [
            "api",
            "-H",
            "Accept: application/octet-stream",
            "repos/hunxuankai/codex-relay/releases/assets/77",
        ]
    );
    assert_eq!(download.stdout_file.as_deref(), Some(destination.as_path()));
    assert!(download.stdin.is_none());
    let debug = format!("{download:?}");
    assert!(!debug.contains("test-token-not-real"));

    let view = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::ViewReleaseRun,
            repository: "hunxuankai/codex-relay".into(),
            workflow: Some("release.yml".into()),
            git_ref: Some("main".into()),
            tag_name: None,
            head_sha: None,
            created_after: None,
            resource_id: Some(123),
            stdin: None,
        })
        .unwrap();
    assert_eq!(
        view.args,
        [
            "run",
            "view",
            "123",
            "--repo",
            "hunxuankai/codex-relay",
            "--json",
            "databaseId,status,conclusion,headSha,url,jobs",
        ]
    );

    let list_runs = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::ListReleaseRuns,
            repository: "hunxuankai/codex-relay".into(),
            workflow: Some("release.yml".into()),
            git_ref: Some("main".into()),
            tag_name: None,
            head_sha: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
            created_after: Some("2026-07-31T10:00:00Z".into()),
            resource_id: None,
            stdin: None,
        })
        .unwrap();
    assert_eq!(
        list_runs.args,
        [
            "run",
            "list",
            "--repo",
            "hunxuankai/codex-relay",
            "--workflow",
            "release.yml",
            "--branch",
            "main",
            "--event",
            "workflow_dispatch",
            "--commit",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--created",
            ">=2026-07-31T10:00:00Z",
            "--limit",
            "10",
            "--json",
            "databaseId,headSha,createdAt,url",
        ]
    );

    let preflight_runs = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::PreflightReleaseRuns,
            repository: "hunxuankai/codex-relay".into(),
            workflow: Some("release.yml".into()),
            git_ref: None,
            tag_name: None,
            head_sha: None,
            created_after: None,
            resource_id: None,
            stdin: None,
        })
        .unwrap();
    assert_eq!(
        preflight_runs.args,
        [
            "run",
            "list",
            "--repo",
            "hunxuankai/codex-relay",
            "--workflow",
            "release.yml",
            "--limit",
            "20",
            "--json",
            "databaseId,status,conclusion,headSha,url",
        ]
    );

    let releases = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::ListDraftReleases,
            repository: "hunxuankai/codex-relay".into(),
            workflow: None,
            git_ref: None,
            tag_name: Some("v0.5.0".into()),
            head_sha: None,
            created_after: None,
            resource_id: None,
            stdin: None,
        })
        .unwrap();
    assert_eq!(
        releases.args,
        ["api", "repos/hunxuankai/codex-relay/releases?per_page=100",]
    );
    let tag = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::GetTag,
            repository: "hunxuankai/codex-relay".into(),
            workflow: None,
            git_ref: None,
            tag_name: Some("v0.5.0".into()),
            head_sha: None,
            created_after: None,
            resource_id: None,
            stdin: None,
        })
        .unwrap();
    assert_eq!(
        tag.args,
        ["api", "repos/hunxuankai/codex-relay/git/ref/tags/v0.5.0",]
    );

    let publish = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::PublishRelease,
            repository: "hunxuankai/codex-relay".into(),
            workflow: None,
            git_ref: None,
            tag_name: Some("v0.5.0".into()),
            head_sha: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
            created_after: None,
            resource_id: Some(42),
            stdin: Some(br#"{"draft":false}"#.to_vec()),
        })
        .unwrap();
    assert_eq!(
        publish.args,
        [
            "api",
            "--method",
            "PATCH",
            "repos/hunxuankai/codex-relay/releases/42",
            "--input",
            "-",
        ]
    );
    assert_eq!(publish.stdin, Some(br#"{"draft":false}"#.to_vec()));

    let latest = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::LatestRelease,
            repository: "hunxuankai/codex-relay".into(),
            workflow: None,
            git_ref: None,
            tag_name: Some("v0.5.0".into()),
            head_sha: None,
            created_after: None,
            resource_id: Some(42),
            stdin: None,
        })
        .unwrap();
    assert_eq!(
        latest.args,
        ["api", "repos/hunxuankai/codex-relay/releases/latest"]
    );

    let cleanup = backend
        .invocation_for(&GhRequest {
            operation: GhOperation::CleanupRuns,
            repository: "hunxuankai/codex-relay".into(),
            workflow: Some("cleanup-old-releases.yml".into()),
            git_ref: Some("main".into()),
            tag_name: None,
            head_sha: None,
            created_after: Some("2026-07-31T11:00:00Z".into()),
            resource_id: None,
            stdin: None,
        })
        .unwrap();
    assert_eq!(
        cleanup.args,
        [
            "run",
            "list",
            "--repo",
            "hunxuankai/codex-relay",
            "--workflow",
            "cleanup-old-releases.yml",
            "--branch",
            "main",
            "--event",
            "release",
            "--created",
            ">=2026-07-31T11:00:00Z",
            "--limit",
            "10",
            "--json",
            "databaseId,status,conclusion,createdAt,url",
        ]
    );
}

#[test]
fn system_gh_backend_preserves_process_start_failures_for_safe_public_mapping() {
    let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
    let backend = SystemGhBackend::new(
        PathBuf::from(r"D:\missing\gh.exe"),
        Vec::new(),
        std::env::temp_dir(),
        cancel,
    );

    let error = tauri::async_runtime::block_on(backend.execute(GhRequest {
        operation: GhOperation::ConnectionTest,
        repository: "hunxuankai/codex-relay".into(),
        workflow: None,
        git_ref: None,
        tag_name: None,
        head_sha: None,
        created_after: None,
        resource_id: None,
        stdin: None,
    }))
    .unwrap_err();

    assert_eq!(error, "GH_PROCESS_START_FAILED");
}

#[test]
fn run_view_preserves_job_and_step_status_with_real_durations() {
    let backend = RunViewGhBackend {
        requests: Mutex::new(Vec::new()),
    };
    let service = GithubReleaseService::new();

    let run = tauri::async_runtime::block_on(service.get_release_run(
        &backend,
        123,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ))
    .unwrap();

    assert_eq!(run.id, 123);
    assert_eq!(run.status, "completed");
    assert_eq!(run.conclusion.as_deref(), Some("success"));
    assert_eq!(run.jobs.len(), 1);
    assert_eq!(run.jobs[0].duration_millis, Some(90_000));
    assert_eq!(run.jobs[0].steps[0].name, "检出源码");
    assert_eq!(run.jobs[0].steps[0].duration_millis, Some(10_000));
    let requests = backend.requests.into_inner().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].operation, GhOperation::ViewReleaseRun);
    assert_eq!(requests[0].resource_id, Some(123));
}

#[test]
fn completed_failed_run_stops_before_draft_audit() {
    let error = tauri::async_runtime::block_on(GithubReleaseService::new().get_release_run(
        &FailedRunGhBackend,
        124,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        codex_relay_release_console_lib::services::github_release::GithubReleaseError::WorkflowRunFailed
    ));
}

#[test]
fn draft_audit_verifies_identity_assets_manifest_hashes_and_signature_relationship() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let backend = DraftFixtureGhBackend {
        requests: Mutex::new(Vec::new()),
        downloads: Mutex::new(Vec::new()),
        notes: notes.into(),
        digest_overrides: BTreeMap::new(),
        scenario: DraftScenario::Valid,
    };

    let audit = tauri::async_runtime::block_on(DraftAuditService::new().audit(
        &backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();

    assert_eq!(audit.release_id, 42);
    assert_eq!(audit.tag_name, "v0.5.0");
    assert_eq!(
        audit.target_commit_sha,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(audit.assets.len(), 3);
    assert_eq!(
        audit
            .assets
            .iter()
            .map(|asset| asset.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Codex.Relay_0.5.0_x64-setup.exe",
            "Codex.Relay_0.5.0_x64-setup.exe.sig",
            "latest.json",
        ]
    );
    assert!(audit.assets.iter().all(|asset| asset.sha256.len() == 64));
    assert_eq!(audit.manifest_version, "0.5.0");
    assert_eq!(audit.manifest_notes, notes);
    assert_eq!(audit.signature, "signature-test-not-real");
    let requests = backend.requests.into_inner().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.operation)
            .collect::<Vec<_>>(),
        [GhOperation::ListDraftReleases]
    );
    let downloads = backend.downloads.into_inner().unwrap();
    assert_eq!(
        downloads.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [501, 502, 503]
    );
    assert!(downloads.iter().all(|(_, path)| !path.exists()));
}

#[test]
fn draft_audit_uses_target_commitish_before_github_creates_the_tag_ref() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let backend = draft_fixture(notes, DraftScenario::TagUnavailable);

    let audit = tauri::async_runtime::block_on(DraftAuditService::new().audit(
        &backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();

    assert_eq!(
        audit.target_commit_sha,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    let requests = backend.requests.into_inner().unwrap();
    assert_eq!(
        requests
            .iter()
            .map(|request| request.operation)
            .collect::<Vec<_>>(),
        [GhOperation::ListDraftReleases]
    );
}

#[test]
fn draft_audit_accepts_github_line_endings_without_accepting_note_drift() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let backend = draft_fixture(notes, DraftScenario::PlatformNormalizedNotes);

    let audit = tauri::async_runtime::block_on(DraftAuditService::new().audit(
        &backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();

    assert!(audit.manifest_notes.contains("\r\n"));
    assert!(!audit.manifest_notes.ends_with(['\r', '\n']));
}

#[test]
fn draft_audit_rejects_an_asset_whose_remote_digest_does_not_match_downloaded_bytes() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let backend = DraftFixtureGhBackend {
        requests: Mutex::new(Vec::new()),
        downloads: Mutex::new(Vec::new()),
        notes: notes.into(),
        digest_overrides: BTreeMap::from([(501, format!("sha256:{}", "0".repeat(64)))]),
        scenario: DraftScenario::Valid,
    };

    let error = tauri::async_runtime::block_on(DraftAuditService::new().audit(
        &backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        codex_relay_release_console_lib::services::github_release::GithubReleaseError::DraftAuditFailed
    ));
}

#[test]
fn draft_audit_rejects_identity_asset_manifest_and_signature_drift() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";

    for scenario in [
        DraftScenario::MissingAsset,
        DraftScenario::ExtraAsset,
        DraftScenario::ReleaseNotesDrift,
        DraftScenario::SizeDrift,
        DraftScenario::ManifestNotesDrift,
        DraftScenario::ManifestUrlDrift,
        DraftScenario::SignatureDrift,
    ] {
        let backend = DraftFixtureGhBackend {
            requests: Mutex::new(Vec::new()),
            downloads: Mutex::new(Vec::new()),
            notes: notes.into(),
            digest_overrides: BTreeMap::new(),
            scenario,
        };

        let error = tauri::async_runtime::block_on(DraftAuditService::new().audit(
            &backend,
            "0.5.0",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            notes,
        ))
        .unwrap_err();

        assert!(
            matches!(
                error,
                codex_relay_release_console_lib::services::github_release::GithubReleaseError::DraftAuditFailed
            ),
            "scenario {scenario:?} should fail"
        );
    }
}

#[test]
fn publish_reaudits_the_same_draft_identity_before_the_patch_request() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let expected_backend = draft_fixture(notes, DraftScenario::Valid);
    let expected = tauri::async_runtime::block_on(DraftAuditService::new().audit(
        &expected_backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();
    let backend = PublishingGhBackend {
        inner: draft_fixture(notes, DraftScenario::ReleaseIdDrift),
        publish_calls: AtomicUsize::new(0),
    };

    let error = tauri::async_runtime::block_on(GithubReleaseService::new().publish_release(
        &backend,
        &expected,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        codex_relay_release_console_lib::services::github_release::GithubReleaseError::DraftIdentityChanged
    ));
    assert_eq!(backend.publish_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn publish_uses_the_verified_release_id_and_returns_published_identity() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let expected_backend = draft_fixture(notes, DraftScenario::Valid);
    let expected = tauri::async_runtime::block_on(DraftAuditService::new().audit(
        &expected_backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();
    let backend = PublishingGhBackend {
        inner: draft_fixture(notes, DraftScenario::Valid),
        publish_calls: AtomicUsize::new(0),
    };

    let published = tauri::async_runtime::block_on(GithubReleaseService::new().publish_release(
        &backend,
        &expected,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();

    assert_eq!(published.release_id, 42);
    assert_eq!(published.tag_name, "v0.5.0");
    assert_eq!(published.published_at, "2026-07-31T11:00:00Z");
    assert_eq!(backend.publish_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn publish_resume_recognizes_the_same_release_already_public_without_repatching() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let draft_backend = draft_fixture(notes, DraftScenario::Valid);
    let expected = tauri::async_runtime::block_on(DraftAuditService::new().audit(
        &draft_backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();
    let backend = AlreadyPublishedGhBackend {
        inner: PublishedFixtureGhBackend {
            inner: draft_fixture(notes, DraftScenario::Valid),
        },
        publish_calls: AtomicUsize::new(0),
    };

    let published = tauri::async_runtime::block_on(GithubReleaseService::new().publish_release(
        &backend,
        &expected,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();

    assert_eq!(published.release_id, 42);
    assert_eq!(published.published_at, "2026-07-31T11:00:00Z");
    assert_eq!(backend.publish_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn published_release_rechecks_latest_tag_manifest_and_asset_evidence() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let draft_backend = draft_fixture(notes, DraftScenario::Valid);
    let draft = tauri::async_runtime::block_on(DraftAuditService::new().audit(
        &draft_backend,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap();
    let published = PublishedReleaseEvidence {
        release_id: 42,
        tag_name: "v0.5.0".into(),
        published_at: "2026-07-31T11:00:00Z".into(),
    };
    let backend = PublishedFixtureGhBackend {
        inner: draft_fixture(notes, DraftScenario::Valid),
    };

    let verified =
        tauri::async_runtime::block_on(GithubReleaseService::new().verify_published_release(
            &backend,
            &draft,
            &published,
            "0.5.0",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            notes,
        ))
        .unwrap();

    assert_eq!(verified, draft);
}

#[test]
fn published_release_audit_rejects_a_tag_ref_that_drifted_from_the_candidate() {
    let notes = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";
    let backend = PublishedFixtureGhBackend {
        inner: draft_fixture(notes, DraftScenario::TagDrift),
    };

    let error = tauri::async_runtime::block_on(DraftAuditService::new().audit_published(
        &backend,
        42,
        "0.5.0",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        notes,
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        codex_relay_release_console_lib::services::github_release::GithubReleaseError::DraftAuditFailed
    ));
}

#[test]
fn cleanup_success_and_failure_remain_separate_from_published_release_success() {
    for (conclusion, expected_success) in [("success", true), ("failure", false)] {
        let backend = CleanupFixtureGhBackend {
            conclusion,
            requests: Mutex::new(Vec::new()),
        };

        let cleanup = tauri::async_runtime::block_on(
            GithubReleaseService::new().monitor_cleanup(&backend, "2026-07-31T11:00:00Z"),
        )
        .unwrap();

        assert_eq!(cleanup.run_id, 900);
        assert_eq!(cleanup.succeeded, expected_success);
        assert_eq!(cleanup.conclusion.as_deref(), Some(conclusion));
    }
}

#[test]
fn cleanup_monitor_logs_the_first_completed_run_projection() {
    let backend = CleanupFixtureGhBackend {
        conclusion: "success",
        requests: Mutex::new(Vec::new()),
    };
    let git_dir = tempfile::tempdir().unwrap();
    let store = ReleaseLogStore::new(git_dir.path().to_path_buf());
    store.initialize("session-a").unwrap();
    let recorder = Arc::new(ReleaseLogRecorder::new("session-a", store, 0, None));
    let service =
        GithubReleaseService::new().with_progress(recorder.clone() as Arc<dyn ReleaseProgressSink>);

    let cleanup =
        tauri::async_runtime::block_on(service.monitor_cleanup(&backend, "2026-07-31T11:00:00Z"))
            .unwrap();

    assert_eq!(cleanup.run_id, 900);
    let page = ReleaseLogStore::new(git_dir.path().to_path_buf())
        .load_page("session-a", None)
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].step_id, "cleanup");
    assert!(page.entries[0].message.contains("Run 900"));
    assert!(page.entries[0].message.contains("status=completed"));
    assert!(page.entries[0].message.contains("conclusion=success"));
    assert!(!page.entries[0].message.contains("https://"));
    assert!(!page.entries[0].message.contains("aaaaaaaaaaaaaaaa"));
}
