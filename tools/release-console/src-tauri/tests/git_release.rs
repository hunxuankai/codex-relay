use codex_relay_release_console_lib::infrastructure::git::GitBackend;
use codex_relay_release_console_lib::infrastructure::process::filter_release_environment;
use codex_relay_release_console_lib::services::git_release::{
    ExternalPreflightSnapshot, GitReleaseError, GitReleaseService, ReleasePreflightProbe,
    ReleasePreflightService, RepositoryInspectionService, RepositorySyncStatus,
    ToolchainInspection,
};
use codex_relay_release_console_lib::services::release_candidate::ReleaseCandidateTransaction;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);
const VALID_RELEASE_NOTES: &str = "## 更新内容\n\n- 修复：修复发布流程\n\n## 更新方式\n\n已安装 `v0.4.0` 的用户可从 `v0.4.0` 更新到 `v0.5.0`。\n\n## 注意事项\n\n本版本未使用 Windows Authenticode，Windows 可能显示“未知发布者”。安装和升级不会删除 Codex 配置、Codex Relay 应用数据、日志或备份。\n";

struct FixedPreflightProbe {
    snapshot: ExternalPreflightSnapshot,
}

impl ReleasePreflightProbe for FixedPreflightProbe {
    fn inspect(&self) -> Result<ExternalPreflightSnapshot, String> {
        Ok(self.snapshot.clone())
    }
}

struct TempGitWorkspace {
    root: PathBuf,
}

impl TempGitWorkspace {
    fn new() -> Self {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "codex-relay-release-git-{}-{id}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }
}

impl Drop for TempGitWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git_executable() -> PathBuf {
    let output = Command::new("where.exe").arg("git.exe").output().unwrap();
    assert!(output.status.success());
    let first = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .trim()
        .to_string();
    PathBuf::from(first)
}

fn run_git(git: &Path, workdir: &Path, args: &[&str]) -> String {
    let output = Command::new(git)
        .args(args)
        .current_dir(workdir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn create_synced_repository(workspace: &TempGitWorkspace, git: &Path) -> (PathBuf, PathBuf) {
    let remote = workspace.root.join("remote.git");
    fs::create_dir_all(&remote).unwrap();
    run_git(git, &remote, &["init", "--bare", "--initial-branch=main"]);

    let seed = workspace.root.join("seed");
    fs::create_dir_all(&seed).unwrap();
    run_git(git, &seed, &["init", "--initial-branch=main"]);
    run_git(git, &seed, &["config", "user.name", "Release Test"]);
    run_git(
        git,
        &seed,
        &["config", "user.email", "release-test@example.invalid"],
    );
    fs::write(seed.join("README.md"), "initial\n").unwrap();
    write_release_fixture(&seed);
    run_git(git, &seed, &["add", "."]);
    run_git(git, &seed, &["commit", "-m", "feat: initial"]);
    run_git(
        git,
        &seed,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    run_git(git, &seed, &["push", "-u", "origin", "main"]);

    let repository = workspace.root.join("repository");
    run_git(
        git,
        &workspace.root,
        &[
            "clone",
            remote.to_str().unwrap(),
            repository.to_str().unwrap(),
        ],
    );
    run_git(git, &repository, &["branch", "-m", "master"]);
    run_git(
        git,
        &repository,
        &["branch", "--set-upstream-to=origin/main", "master"],
    );
    run_git(git, &repository, &["config", "user.name", "Release Test"]);
    run_git(
        git,
        &repository,
        &["config", "user.email", "release-test@example.invalid"],
    );
    (repository, remote)
}

fn write_release_fixture(repository: &Path) {
    fs::create_dir_all(repository.join("src-tauri/crates/codex-relay-core")).unwrap();
    fs::create_dir_all(repository.join(".github")).unwrap();
    fs::write(
        repository.join("package.json"),
        "{\n  \"name\": \"codex-relay\",\n  \"version\": \"0.4.0\",\n  \"private\": true\n}\n",
    )
    .unwrap();
    fs::write(
        repository.join("package-lock.json"),
        "{\n  \"name\": \"codex-relay\",\n  \"version\": \"0.4.0\",\n  \"lockfileVersion\": 3,\n  \"packages\": {\n    \"\": {\n      \"name\": \"codex-relay\",\n      \"version\": \"0.4.0\"\n    }\n  }\n}\n",
    )
    .unwrap();
    fs::write(
        repository.join("src-tauri/Cargo.toml"),
        "[package]\nname = \"codex-relay\"\nversion = \"0.4.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        repository.join("src-tauri/crates/codex-relay-core/Cargo.toml"),
        "[package]\nname = \"codex-relay-core\"\nversion = \"0.4.0\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        repository.join("src-tauri/Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"codex-relay\"\nversion = \"0.4.0\"\n\n[[package]]\nname = \"codex-relay-core\"\nversion = \"0.4.0\"\n",
    )
    .unwrap();
    fs::write(repository.join(".github/release-notes.md"), "旧发布说明\n").unwrap();
}

#[test]
fn inspection_accepts_clean_synced_repository_even_when_local_branch_is_master() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = RepositoryInspectionService::new("main", remote.canonicalize().unwrap());

    let inspection =
        tauri::async_runtime::block_on(service.inspect(&backend, &repository)).unwrap();

    assert_eq!(inspection.local_branch, "master");
    assert_eq!(inspection.default_branch, "main");
    assert_eq!(inspection.head_sha, inspection.remote_main_sha);
    assert!(inspection.clean);
    assert_eq!(inspection.sync.status, RepositorySyncStatus::Synced);
    assert_eq!(inspection.sync.ahead_count, 0);
    assert_eq!(inspection.sync.behind_count, 0);
    assert!(inspection.sync.ahead_commits.is_empty());
    assert_eq!(
        PathBuf::from(inspection.remote_url).canonicalize().unwrap(),
        remote.canonicalize().unwrap()
    );
}

#[test]
fn recovery_inspection_keeps_repository_identity_checks_but_allows_dirty_or_unsynced_state() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    fs::write(repository.join("unexpected.txt"), "recovery marker\n").unwrap();
    fs::write(repository.join("README.md"), "local recovery state\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(
        &git,
        &repository,
        &["commit", "-m", "chore: local recovery"],
    );
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = RepositoryInspectionService::new("main", remote);

    let inspection =
        tauri::async_runtime::block_on(service.inspect_for_recovery(&backend, &repository))
            .unwrap();

    assert!(!inspection.clean);
    assert_ne!(inspection.head_sha, inspection.remote_main_sha);
}

#[test]
fn inspection_rejects_remote_identity_mismatch() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, _) = create_synced_repository(&workspace, &git);
    let unexpected_remote = workspace.root.join("unexpected.git");
    fs::create_dir_all(&unexpected_remote).unwrap();
    run_git(
        &git,
        &unexpected_remote,
        &["init", "--bare", "--initial-branch=main"],
    );
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = RepositoryInspectionService::new("main", unexpected_remote);

    let error = tauri::async_runtime::block_on(service.inspect(&backend, &repository)).unwrap_err();

    assert!(matches!(error, GitReleaseError::RemoteMismatch));
}

#[test]
fn inspection_reports_dirty_worktree_as_repository_fact() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    fs::write(repository.join("unexpected.txt"), "unexpected\n").unwrap();
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = RepositoryInspectionService::new("main", remote);

    let inspection =
        tauri::async_runtime::block_on(service.inspect(&backend, &repository)).unwrap();

    assert!(!inspection.clean);
    assert_eq!(inspection.sync.status, RepositorySyncStatus::Synced);
}

#[test]
fn inspection_reports_clean_local_commits_as_ahead_with_ordered_summaries() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    run_git(&git, &repository, &["config", "user.name", "Release Test"]);
    run_git(
        &git,
        &repository,
        &["config", "user.email", "release-test@example.invalid"],
    );
    fs::write(repository.join("README.md"), "local candidate\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(
        &git,
        &repository,
        &["commit", "-m", "feat: first local commit"],
    );
    let first_sha = run_git(&git, &repository, &["rev-parse", "HEAD"]);
    fs::write(repository.join("README.md"), "second local candidate\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(
        &git,
        &repository,
        &["commit", "-m", "fix: second local commit"],
    );
    let second_sha = run_git(&git, &repository, &["rev-parse", "HEAD"]);
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = RepositoryInspectionService::new("main", remote);

    let inspection =
        tauri::async_runtime::block_on(service.inspect(&backend, &repository)).unwrap();

    assert_eq!(inspection.sync.status, RepositorySyncStatus::Ahead);
    assert_eq!(inspection.sync.ahead_count, 2);
    assert_eq!(inspection.sync.behind_count, 0);
    assert_eq!(
        inspection.sync.ahead_commits,
        vec![
            codex_relay_release_console_lib::models::RepositoryCommitSummary {
                sha: second_sha,
                subject: "fix: second local commit".into(),
            },
            codex_relay_release_console_lib::models::RepositoryCommitSummary {
                sha: first_sha,
                subject: "feat: first local commit".into(),
            },
        ]
    );
}

#[test]
fn inspection_reports_remote_only_commits_as_behind() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let seed = workspace.root.join("seed");
    fs::write(seed.join("README.md"), "remote update\n").unwrap();
    run_git(&git, &seed, &["add", "README.md"]);
    run_git(&git, &seed, &["commit", "-m", "fix: remote only"]);
    run_git(&git, &seed, &["push", "origin", "main"]);
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = RepositoryInspectionService::new("main", remote);

    let inspection =
        tauri::async_runtime::block_on(service.inspect(&backend, &repository)).unwrap();

    assert_eq!(inspection.sync.status, RepositorySyncStatus::Behind);
    assert_eq!(inspection.sync.ahead_count, 0);
    assert_eq!(inspection.sync.behind_count, 1);
    assert!(inspection.sync.ahead_commits.is_empty());
}

#[test]
fn inspection_reports_independent_local_and_remote_commits_as_diverged() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    fs::write(repository.join("README.md"), "local update\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(&git, &repository, &["commit", "-m", "feat: local branch"]);
    let local_sha = run_git(&git, &repository, &["rev-parse", "HEAD"]);
    let seed = workspace.root.join("seed");
    fs::write(seed.join("README.md"), "remote update\n").unwrap();
    run_git(&git, &seed, &["add", "README.md"]);
    run_git(&git, &seed, &["commit", "-m", "fix: remote branch"]);
    run_git(&git, &seed, &["push", "origin", "main"]);
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = RepositoryInspectionService::new("main", remote);

    let inspection =
        tauri::async_runtime::block_on(service.inspect(&backend, &repository)).unwrap();

    assert_eq!(inspection.sync.status, RepositorySyncStatus::Diverged);
    assert_eq!(inspection.sync.ahead_count, 1);
    assert_eq!(inspection.sync.behind_count, 1);
    assert_eq!(
        inspection.sync.ahead_commits,
        vec![
            codex_relay_release_console_lib::models::RepositoryCommitSummary {
                sha: local_sha,
                subject: "feat: local branch".into(),
            }
        ]
    );
}

#[test]
fn codex_relay_policy_accepts_only_the_target_github_repository() {
    let service = RepositoryInspectionService::for_codex_relay();

    assert!(service.accepts_remote_url("https://github.com/hunxuankai/codex-relay.git"));
    assert!(service.accepts_remote_url("git@github.com:hunxuankai/codex-relay.git"));
    assert!(service.accepts_remote_url("ssh://git@github.com/hunxuankai/codex-relay"));
    assert!(!service.accepts_remote_url("https://github.com/other/codex-relay.git"));
    assert!(!service.accepts_remote_url("https://example.com/hunxuankai/codex-relay.git"));
}

#[test]
fn exact_release_files_are_committed_and_pushed_to_remote_main() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));
    let inspection_service = RepositoryInspectionService::new("main", remote.clone());
    let inspection =
        tauri::async_runtime::block_on(inspection_service.inspect(&backend, &repository)).unwrap();
    let git_dir = repository.join(".git");
    let plan =
        ReleaseCandidateTransaction::plan(&repository, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    ReleaseCandidateTransaction::apply(&repository, &git_dir, &plan).unwrap();
    let service = GitReleaseService::new("main");

    let candidate_sha = tauri::async_runtime::block_on(service.commit_candidate(
        &backend,
        &repository,
        &plan,
        &inspection.remote_main_sha,
    ))
    .unwrap();
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        inspection.remote_main_sha
    );
    let outcome = tauri::async_runtime::block_on(service.push_candidate(
        &backend,
        &repository,
        &candidate_sha,
    ))
    .unwrap();

    assert_eq!(outcome.remote_main_sha, outcome.candidate_sha);
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        outcome.candidate_sha
    );
    assert_eq!(
        run_git(&git, &repository, &["log", "-1", "--pretty=%s"]),
        "chore(release): 准备 v0.5.0 发布"
    );
    let mut changed = run_git(
        &git,
        &repository,
        &[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            &outcome.candidate_sha,
        ],
    )
    .lines()
    .map(str::to_string)
    .collect::<Vec<_>>();
    changed.sort();
    let mut expected = plan
        .files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(changed, expected);
}

#[test]
fn reviewed_ahead_commits_are_pushed_by_exact_sha_only_to_remote_main() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let expected_remote_sha = run_git(&git, &remote, &["rev-parse", "refs/heads/main"]);
    fs::write(repository.join("README.md"), "reviewed local change\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(
        &git,
        &repository,
        &["commit", "-m", "feat: reviewed local change"],
    );
    let expected_head_sha = run_git(&git, &repository, &["rev-parse", "HEAD"]);
    run_git(&git, &repository, &["tag", "local-only"]);
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));

    let outcome =
        tauri::async_runtime::block_on(GitReleaseService::new("main").push_existing_commits(
            &backend,
            &repository,
            &expected_head_sha,
            &expected_remote_sha,
        ))
        .unwrap();

    assert_eq!(outcome.candidate_sha, expected_head_sha);
    assert_eq!(outcome.remote_main_sha, expected_head_sha);
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        expected_head_sha
    );
    assert!(
        !run_git(
            &git,
            &remote,
            &["for-each-ref", "--format=%(refname)", "refs/tags"]
        )
        .contains("refs/tags/local-only")
    );
}

#[test]
fn safe_push_rechecks_dirty_head_and_remote_expectations_before_writing() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let expected_remote_sha = run_git(&git, &remote, &["rev-parse", "refs/heads/main"]);
    fs::write(repository.join("README.md"), "reviewed local change\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(
        &git,
        &repository,
        &["commit", "-m", "feat: reviewed local change"],
    );
    let expected_head_sha = run_git(&git, &repository, &["rev-parse", "HEAD"]);
    fs::write(repository.join("unexpected.txt"), "dirty\n").unwrap();
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));
    let service = GitReleaseService::new("main");

    let dirty = tauri::async_runtime::block_on(service.push_existing_commits(
        &backend,
        &repository,
        &expected_head_sha,
        &expected_remote_sha,
    ))
    .unwrap_err();
    assert!(matches!(dirty, GitReleaseError::WorktreeDirty));
    fs::remove_file(repository.join("unexpected.txt")).unwrap();

    let moved_head = tauri::async_runtime::block_on(service.push_existing_commits(
        &backend,
        &repository,
        &"f".repeat(40),
        &expected_remote_sha,
    ))
    .unwrap_err();
    assert!(matches!(moved_head, GitReleaseError::HeadMoved));

    let moved_remote = tauri::async_runtime::block_on(service.push_existing_commits(
        &backend,
        &repository,
        &expected_head_sha,
        &"e".repeat(40),
    ))
    .unwrap_err();
    assert!(matches!(moved_remote, GitReleaseError::RemoteMoved));
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        expected_remote_sha
    );
}

#[test]
fn safe_push_distinguishes_behind_and_diverged_repositories() {
    let behind_workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (behind_repository, behind_remote) = create_synced_repository(&behind_workspace, &git);
    let behind_seed = behind_workspace.root.join("seed");
    fs::write(behind_seed.join("README.md"), "remote only\n").unwrap();
    run_git(&git, &behind_seed, &["add", "README.md"]);
    run_git(&git, &behind_seed, &["commit", "-m", "fix: remote only"]);
    run_git(&git, &behind_seed, &["push", "origin", "main"]);
    let behind_head = run_git(&git, &behind_repository, &["rev-parse", "HEAD"]);
    let behind_remote_sha = run_git(&git, &behind_remote, &["rev-parse", "refs/heads/main"]);
    let behind_backend =
        GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));

    let behind =
        tauri::async_runtime::block_on(GitReleaseService::new("main").push_existing_commits(
            &behind_backend,
            &behind_repository,
            &behind_head,
            &behind_remote_sha,
        ))
        .unwrap_err();
    assert!(matches!(behind, GitReleaseError::RepositoryBehind));

    let diverged_workspace = TempGitWorkspace::new();
    let (diverged_repository, diverged_remote) =
        create_synced_repository(&diverged_workspace, &git);
    fs::write(diverged_repository.join("README.md"), "local only\n").unwrap();
    run_git(&git, &diverged_repository, &["add", "README.md"]);
    run_git(
        &git,
        &diverged_repository,
        &["commit", "-m", "feat: local only"],
    );
    let diverged_seed = diverged_workspace.root.join("seed");
    fs::write(diverged_seed.join("README.md"), "remote only\n").unwrap();
    run_git(&git, &diverged_seed, &["add", "README.md"]);
    run_git(&git, &diverged_seed, &["commit", "-m", "fix: remote only"]);
    run_git(&git, &diverged_seed, &["push", "origin", "main"]);
    let diverged_head = run_git(&git, &diverged_repository, &["rev-parse", "HEAD"]);
    let diverged_remote_sha = run_git(&git, &diverged_remote, &["rev-parse", "refs/heads/main"]);
    let diverged_backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));

    let diverged =
        tauri::async_runtime::block_on(GitReleaseService::new("main").push_existing_commits(
            &diverged_backend,
            &diverged_repository,
            &diverged_head,
            &diverged_remote_sha,
        ))
        .unwrap_err();
    assert!(matches!(diverged, GitReleaseError::RepositoryDiverged));
}

#[test]
fn safe_push_reports_remote_movement_during_the_confirmed_push() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let expected_remote_sha = run_git(&git, &remote, &["rev-parse", "refs/heads/main"]);
    fs::write(repository.join("README.md"), "local reviewed change\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(
        &git,
        &repository,
        &["commit", "-m", "feat: local reviewed change"],
    );
    let expected_head_sha = run_git(&git, &repository, &["rev-parse", "HEAD"]);
    let seed = workspace.root.join("seed");
    fs::write(seed.join("README.md"), "remote race\n").unwrap();
    run_git(&git, &seed, &["add", "README.md"]);
    run_git(&git, &seed, &["commit", "-m", "fix: remote race"]);
    let race_sha = run_git(&git, &seed, &["rev-parse", "HEAD"]);
    run_git(&git, &seed, &["push", "origin", "HEAD:refs/heads/race"]);
    let remote_for_shell = remote.to_string_lossy().replace('\\', "/");
    fs::write(
        repository.join(".git/hooks/pre-push"),
        format!(
            "#!/bin/sh\ngit --git-dir=\"{remote_for_shell}\" update-ref refs/heads/main {race_sha}\n"
        ),
    )
    .unwrap();
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));

    let error =
        tauri::async_runtime::block_on(GitReleaseService::new("main").push_existing_commits(
            &backend,
            &repository,
            &expected_head_sha,
            &expected_remote_sha,
        ))
        .unwrap_err();

    assert!(matches!(error, GitReleaseError::RemoteMoved));
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        race_sha
    );
}

#[test]
fn safe_push_fails_when_remote_main_does_not_stay_on_the_confirmed_sha() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let expected_remote_sha = run_git(&git, &remote, &["rev-parse", "refs/heads/main"]);
    fs::write(repository.join("README.md"), "local reviewed change\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(
        &git,
        &repository,
        &["commit", "-m", "feat: local reviewed change"],
    );
    let expected_head_sha = run_git(&git, &repository, &["rev-parse", "HEAD"]);
    let seed = workspace.root.join("seed");
    fs::write(seed.join("README.md"), "verification race\n").unwrap();
    run_git(&git, &seed, &["add", "README.md"]);
    run_git(&git, &seed, &["commit", "-m", "fix: verification race"]);
    let race_sha = run_git(&git, &seed, &["rev-parse", "HEAD"]);
    run_git(&git, &seed, &["push", "origin", "HEAD:refs/heads/race"]);
    fs::write(
        remote.join("hooks/post-receive"),
        format!("#!/bin/sh\ngit update-ref refs/heads/main {race_sha}\n"),
    )
    .unwrap();
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));

    let error =
        tauri::async_runtime::block_on(GitReleaseService::new("main").push_existing_commits(
            &backend,
            &repository,
            &expected_head_sha,
            &expected_remote_sha,
        ))
        .unwrap_err();

    assert!(matches!(error, GitReleaseError::RemoteVerificationFailed));
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        race_sha
    );
}

#[test]
fn unstage_candidate_clears_the_planned_index_without_reverting_candidate_bytes() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, _) = create_synced_repository(&workspace, &git);
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));
    let plan =
        ReleaseCandidateTransaction::plan(&repository, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    ReleaseCandidateTransaction::apply(&repository, &repository.join(".git"), &plan).unwrap();
    let mut add_arguments = vec!["add".to_string(), "--".to_string()];
    add_arguments.extend(plan.files.iter().map(|file| file.relative_path.clone()));
    let add_arguments = add_arguments.iter().map(String::as_str).collect::<Vec<_>>();
    run_git(&git, &repository, &add_arguments);
    assert!(!run_git(&git, &repository, &["diff", "--cached", "--name-only"]).is_empty());

    tauri::async_runtime::block_on(GitReleaseService::new("main").unstage_candidate(
        &backend,
        &repository,
        &plan,
    ))
    .unwrap();

    assert!(run_git(&git, &repository, &["diff", "--cached", "--name-only"]).is_empty());
    for file in &plan.files {
        assert_eq!(
            fs::read(repository.join(&file.relative_path)).unwrap(),
            file.after
        );
    }
}

#[test]
fn commit_refuses_planned_file_content_drift_before_staging() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));
    let inspection_service = RepositoryInspectionService::new("main", remote.clone());
    let inspection =
        tauri::async_runtime::block_on(inspection_service.inspect(&backend, &repository)).unwrap();
    let plan =
        ReleaseCandidateTransaction::plan(&repository, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    ReleaseCandidateTransaction::apply(&repository, &repository.join(".git"), &plan).unwrap();
    fs::write(
        repository.join("package.json"),
        b"{\n  \"name\": \"codex-relay\",\n  \"version\": \"0.5.0\",\n  \"drift\": true\n}\n",
    )
    .unwrap();
    let service = GitReleaseService::new("main");

    let error = tauri::async_runtime::block_on(service.commit_candidate(
        &backend,
        &repository,
        &plan,
        &inspection.remote_main_sha,
    ))
    .unwrap_err();

    assert!(matches!(error, GitReleaseError::PlannedFilesMismatch));
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        inspection.remote_main_sha
    );
    assert_eq!(
        run_git(&git, &repository, &["rev-parse", "HEAD"]),
        inspection.head_sha
    );
}

#[test]
fn commit_refuses_unplanned_untracked_file() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));
    let inspection_service = RepositoryInspectionService::new("main", remote.clone());
    let inspection =
        tauri::async_runtime::block_on(inspection_service.inspect(&backend, &repository)).unwrap();
    let plan =
        ReleaseCandidateTransaction::plan(&repository, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    ReleaseCandidateTransaction::apply(&repository, &repository.join(".git"), &plan).unwrap();
    fs::write(repository.join("unexpected.txt"), "do not commit\n").unwrap();
    let service = GitReleaseService::new("main");

    let error = tauri::async_runtime::block_on(service.commit_candidate(
        &backend,
        &repository,
        &plan,
        &inspection.remote_main_sha,
    ))
    .unwrap_err();

    assert!(matches!(error, GitReleaseError::PlannedFilesMismatch));
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        inspection.remote_main_sha
    );
}

#[test]
fn non_fast_forward_race_is_reported_as_push_failure() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let backend = GitBackend::new(git.clone(), filter_release_environment(std::env::vars_os()));
    let inspection_service = RepositoryInspectionService::new("main", remote.clone());
    let inspection =
        tauri::async_runtime::block_on(inspection_service.inspect(&backend, &repository)).unwrap();
    let plan =
        ReleaseCandidateTransaction::plan(&repository, "0.5.0", VALID_RELEASE_NOTES).unwrap();
    ReleaseCandidateTransaction::apply(&repository, &repository.join(".git"), &plan).unwrap();

    let seed = workspace.root.join("seed");
    fs::write(seed.join("README.md"), "remote race\n").unwrap();
    run_git(&git, &seed, &["add", "README.md"]);
    run_git(&git, &seed, &["commit", "-m", "fix: remote race"]);
    let race_sha = run_git(&git, &seed, &["rev-parse", "HEAD"]);
    run_git(&git, &seed, &["push", "origin", "HEAD:refs/heads/race"]);
    let remote_for_shell = remote.to_string_lossy().replace('\\', "/");
    fs::write(
        repository.join(".git/hooks/pre-push"),
        format!(
            "#!/bin/sh\ngit --git-dir=\"{remote_for_shell}\" update-ref refs/heads/main {race_sha}\n"
        ),
    )
    .unwrap();
    let service = GitReleaseService::new("main");
    let candidate_sha = tauri::async_runtime::block_on(service.commit_candidate(
        &backend,
        &repository,
        &plan,
        &inspection.remote_main_sha,
    ))
    .unwrap();

    let error = tauri::async_runtime::block_on(service.push_candidate(
        &backend,
        &repository,
        &candidate_sha,
    ))
    .unwrap_err();

    assert!(matches!(error, GitReleaseError::PushFailed));
    assert_eq!(
        run_git(&git, &remote, &["rev-parse", "refs/heads/main"]),
        race_sha
    );
}

#[test]
fn git_release_errors_expose_stable_codes() {
    assert_eq!(
        GitReleaseError::RemoteMismatch.code(),
        "GIT_REMOTE_MISMATCH"
    );
    assert_eq!(GitReleaseError::WorktreeDirty.code(), "GIT_WORKTREE_DIRTY");
    assert_eq!(
        GitReleaseError::HeadRemoteMismatch.code(),
        "GIT_HEAD_REMOTE_MISMATCH"
    );
    assert_eq!(GitReleaseError::HeadMoved.code(), "GIT_HEAD_MOVED");
    assert_eq!(
        GitReleaseError::RepositoryBehind.code(),
        "GIT_REPOSITORY_BEHIND"
    );
    assert_eq!(
        GitReleaseError::RepositoryDiverged.code(),
        "GIT_REPOSITORY_DIVERGED"
    );
    assert_eq!(
        GitReleaseError::PlannedFilesMismatch.code(),
        "GIT_PLANNED_FILES_MISMATCH"
    );
    assert_eq!(GitReleaseError::RemoteMoved.code(), "GIT_REMOTE_MOVED");
    assert_eq!(GitReleaseError::FetchTimeout.code(), "GIT_FETCH_TIMEOUT");
    assert_eq!(GitReleaseError::FetchFailed.code(), "GIT_FETCH_FAILED");
    assert_eq!(GitReleaseError::PushFailed.code(), "GIT_PUSH_FAILED");
}

#[test]
fn preflight_combines_real_git_inspection_with_mocked_tool_and_github_state() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = ReleasePreflightService::new(RepositoryInspectionService::new("main", remote));
    let probe = FixedPreflightProbe {
        snapshot: ExternalPreflightSnapshot {
            tools: ToolchainInspection {
                git: Some("git version test".into()),
                node: Some("v22.test".into()),
                npm: Some("11.test".into()),
                cargo: Some("cargo test".into()),
                gh: Some("gh version test".into()),
            },
            active_release_runs: 0,
            conflicting_drafts: 0,
            latest_release_tag: Some("v0.4.0".into()),
        },
    };

    let result =
        tauri::async_runtime::block_on(service.inspect(&backend, &repository, &probe)).unwrap();

    assert_eq!(result.repository.local_branch, "master");
    assert!(result.release_ready);
    assert!(result.blocking_reasons.is_empty());
    assert!(result.safe_push.is_none());
    assert_eq!(
        PathBuf::from(&result.repository_path)
            .canonicalize()
            .unwrap(),
        repository.canonicalize().unwrap()
    );
    assert_eq!(result.external, probe.snapshot);
}

#[test]
fn preflight_exposes_safe_push_only_for_clean_ahead_repository_without_conflicts() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    fs::write(repository.join("README.md"), "ready to push\n").unwrap();
    run_git(&git, &repository, &["add", "README.md"]);
    run_git(
        &git,
        &repository,
        &["commit", "-m", "feat: reviewed local commit"],
    );
    let head_sha = run_git(&git, &repository, &["rev-parse", "HEAD"]);
    let remote_main_sha = run_git(&git, &remote, &["rev-parse", "refs/heads/main"]);
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = ReleasePreflightService::new(RepositoryInspectionService::new("main", remote));
    let probe = FixedPreflightProbe {
        snapshot: ExternalPreflightSnapshot {
            tools: ToolchainInspection {
                git: Some("git version test".into()),
                node: Some("v22.test".into()),
                npm: Some("11.test".into()),
                cargo: Some("cargo test".into()),
                gh: Some("gh version test".into()),
            },
            active_release_runs: 0,
            conflicting_drafts: 0,
            latest_release_tag: Some("v0.4.0".into()),
        },
    };

    let result =
        tauri::async_runtime::block_on(service.inspect(&backend, &repository, &probe)).unwrap();

    assert!(!result.release_ready);
    assert_eq!(result.blocking_reasons.len(), 1);
    let preview = result
        .safe_push
        .expect("clean ahead repository should be pushable");
    assert_eq!(preview.expected_head_sha, head_sha);
    assert_eq!(preview.expected_remote_main_sha, remote_main_sha);
    assert_eq!(preview.commit_count, 1);
    assert_eq!(preview.commits.len(), 1);
    assert_eq!(preview.commits[0].subject, "feat: reviewed local commit");
}

#[test]
fn preflight_projects_missing_tools_active_runs_and_conflicting_drafts_as_facts() {
    let workspace = TempGitWorkspace::new();
    let git = git_executable();
    let (repository, remote) = create_synced_repository(&workspace, &git);
    let backend = GitBackend::new(git, filter_release_environment(std::env::vars_os()));
    let service = ReleasePreflightService::new(RepositoryInspectionService::new("main", remote));
    let available_tools = ToolchainInspection {
        git: Some("git version test".into()),
        node: Some("v22.test".into()),
        npm: Some("11.test".into()),
        cargo: Some("cargo test".into()),
        gh: Some("gh version test".into()),
    };
    let cases = [
        (
            ExternalPreflightSnapshot {
                tools: ToolchainInspection {
                    gh: None,
                    ..available_tools.clone()
                },
                active_release_runs: 0,
                conflicting_drafts: 0,
                latest_release_tag: None,
            },
            "工具",
        ),
        (
            ExternalPreflightSnapshot {
                tools: available_tools.clone(),
                active_release_runs: 1,
                conflicting_drafts: 0,
                latest_release_tag: None,
            },
            "活动发布工作流",
        ),
        (
            ExternalPreflightSnapshot {
                tools: available_tools,
                active_release_runs: 0,
                conflicting_drafts: 1,
                latest_release_tag: None,
            },
            "Draft Release",
        ),
    ];

    for (snapshot, expected) in cases {
        let probe = FixedPreflightProbe { snapshot };
        let result =
            tauri::async_runtime::block_on(service.inspect(&backend, &repository, &probe)).unwrap();
        assert!(!result.release_ready);
        assert!(result.safe_push.is_none());
        assert!(
            result
                .blocking_reasons
                .iter()
                .any(|reason| reason.contains(expected)),
            "missing blocker containing {expected:?}: {:?}",
            result.blocking_reasons
        );
    }
}
