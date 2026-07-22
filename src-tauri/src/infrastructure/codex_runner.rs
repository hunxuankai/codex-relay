use crate::services::provider_service::ProviderAvailabilityTarget;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use tempfile::{Builder, TempDir};

pub(crate) const SUPPORTED_CODEX_VERSION: &str = "0.144.4";
const TEST_PROVIDER_ID: &str = "codex_relay_test";
const TEST_PROMPT: &str =
    "Reply with exactly CODEX_RELAY_OK. Do not call tools or request permissions.";
const DISABLED_FEATURES: &[&str] = &[
    "shell_tool",
    "unified_exec",
    "apps",
    "browser_use",
    "browser_use_external",
    "browser_use_full_cdp_access",
    "computer_use",
    "image_generation",
    "in_app_browser",
    "plugins",
    "remote_plugin",
    "plugin_sharing",
    "tool_suggest",
    "code_mode_host",
    "auth_elicitation",
    "tool_call_mcp_elicitation",
    "skill_mcp_dependency_install",
    "hooks",
];
const INHERITED_ENV_ALLOWLIST: &[&str] = &[
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "PATH",
    "TEMP",
    "TMP",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "NODE_EXTRA_CA_CERTS",
];

/// Codex 兼容性运行时的安全失败分类。
///
/// 该错误类型只保存稳定的内部分类，不携带命令行、密钥、响应正文或临时路径，
/// 这样即使被转换为日志/调试输出也不会扩大敏感信息的生命周期。
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum CodexRunnerError {
    #[error("临时目录路径不安全")]
    UnsafeTempPath,
    #[error("临时目录操作失败")]
    TempIo,
    #[error("model catalog 无效")]
    CatalogInvalid,
    #[error("检测到系统 managed requirements")]
    ManagedConfig,
    #[error("Codex CLI 版本不受支持")]
    UnsupportedVersion,
    #[error("Codex CLI 不可用")]
    ExecutableUnavailable,
    #[error("Codex 进程启动失败")]
    ProcessStart,
    #[error("Codex 进程超时")]
    Timeout,
    #[error("Codex 测试已取消")]
    Cancelled,
    #[error("Codex 输出超过安全上限")]
    OutputTooLarge,
    #[error("Codex 安全门禁失败")]
    PreflightFailed,
    #[error("Codex 进程树未能安全终止")]
    ProcessTreeTermination,
    #[error("临时目录清理失败")]
    CleanupFailed,
}

/// 每次兼容性测试独占的临时状态布局。
///
/// `TempDir` 持有根目录生命周期；所有子路径都固定在该根目录内，且不会创建
/// `config.toml` 或 `auth.json`。
pub(crate) struct CodexTempLayout {
    root: TempDir,
    home: PathBuf,
    sqlite_home: PathBuf,
    workdir: PathBuf,
    catalog_path: PathBuf,
}

impl CodexTempLayout {
    pub(crate) fn new() -> Result<Self, CodexRunnerError> {
        let root = Builder::new()
            .prefix("codex-relay-provider-test-")
            .tempdir()
            .map_err(|_| CodexRunnerError::TempIo)?;
        if !is_safe_temp_path(root.path()) {
            return Err(CodexRunnerError::UnsafeTempPath);
        }

        let home = root.path().join("home");
        let sqlite_home = root.path().join("sqlite");
        let workdir = root.path().join("work");
        let catalog_path = root.path().join("model-catalog.json");
        for directory in [&home, &sqlite_home, &workdir] {
            fs::create_dir_all(directory).map_err(|_| CodexRunnerError::TempIo)?;
        }

        Ok(Self {
            root,
            home,
            sqlite_home,
            workdir,
            catalog_path,
        })
    }

    #[cfg(test)]
    pub(crate) fn root(&self) -> &Path {
        self.root.path()
    }

    pub(crate) fn home(&self) -> &Path {
        &self.home
    }

    pub(crate) fn sqlite_home(&self) -> &Path {
        &self.sqlite_home
    }

    pub(crate) fn workdir(&self) -> &Path {
        &self.workdir
    }

    pub(crate) fn catalog_path(&self) -> &Path {
        &self.catalog_path
    }

    pub(crate) fn cleanup(self) -> Result<(), CodexRunnerError> {
        let root_path = self.root.path().to_path_buf();
        if !is_safe_temp_path(&root_path) {
            return Err(CodexRunnerError::UnsafeTempPath);
        }
        let Self { root, .. } = self;
        root.close().map_err(|_| CodexRunnerError::CleanupFailed)?;
        if root_path.exists() {
            return Err(CodexRunnerError::CleanupFailed);
        }
        Ok(())
    }
}

/// 判断候选路径是否位于系统临时目录内，并排除真实 Codex/Relay 数据根。
pub(crate) fn is_safe_temp_path(candidate: &Path) -> bool {
    let Some(candidate) = canonical_or_absolute(candidate) else {
        return false;
    };
    let Some(temp_root) = canonical_or_absolute(&std::env::temp_dir()) else {
        return false;
    };
    if !is_same_or_descendant(&candidate, &temp_root) {
        return false;
    }

    let protected = [
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|path| path.join(".codex")),
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("CodexRelay")),
    ];
    !protected.into_iter().flatten().any(|path| {
        canonical_or_absolute(&path)
            .is_some_and(|protected| is_same_or_descendant(&candidate, &protected))
    })
}

/// 写入并回读验证纯文本 model catalog。
pub(crate) fn write_model_catalog(path: &Path, model: &str) -> Result<(), CodexRunnerError> {
    if !is_safe_temp_path(path)
        || path.file_name().and_then(|name| name.to_str()) != Some("model-catalog.json")
    {
        return Err(CodexRunnerError::UnsafeTempPath);
    }
    let document = model_catalog_json(model).map_err(|_| CodexRunnerError::CatalogInvalid)?;
    let bytes = serde_json::to_vec(&document).map_err(|_| CodexRunnerError::CatalogInvalid)?;
    fs::write(path, bytes).map_err(|_| CodexRunnerError::TempIo)?;
    let read_back = fs::read(path).map_err(|_| CodexRunnerError::TempIo)?;
    let parsed: Value =
        serde_json::from_slice(&read_back).map_err(|_| CodexRunnerError::CatalogInvalid)?;
    let entry = parsed
        .get("models")
        .and_then(Value::as_array)
        .and_then(|models| models.first())
        .ok_or(CodexRunnerError::CatalogInvalid)?;
    let valid = entry.get("slug").and_then(Value::as_str) == Some(model)
        && entry.get("input_modalities") == Some(&json!(["text"]))
        && entry.get("shell_type").and_then(Value::as_str) == Some("disabled")
        && entry
            .get("apply_patch_tool_type")
            .is_some_and(Value::is_null);
    if valid {
        Ok(())
    } else {
        Err(CodexRunnerError::CatalogInvalid)
    }
}

/// managed requirements 是管理员控制面；存在即无法证明安全配置未被强制覆盖。
pub(crate) fn check_managed_requirements(path: &Path) -> Result<(), CodexRunnerError> {
    match fs::metadata(path) {
        Ok(_) => Err(CodexRunnerError::ManagedConfig),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(CodexRunnerError::TempIo),
    }
}

fn canonical_or_absolute(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Some(canonical);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    let mut existing = absolute.as_path();
    let mut missing = Vec::new();
    while !existing.exists() {
        missing.push(existing.file_name()?.to_owned());
        existing = existing.parent()?;
    }
    let mut canonical = fs::canonicalize(existing).ok()?;
    for component in missing.into_iter().rev() {
        canonical.push(component);
    }
    Some(canonical)
}

fn is_same_or_descendant(candidate: &Path, parent: &Path) -> bool {
    let candidate = normalize_for_compare(candidate);
    let parent = normalize_for_compare(parent);
    candidate == parent || candidate.starts_with(&(parent + "\\"))
}

fn normalize_for_compare(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

#[derive(Clone)]
pub(crate) struct CodexInvocation {
    pub(crate) executable: PathBuf,
    pub(crate) args: Vec<OsString>,
    pub(crate) env: Vec<(OsString, OsString)>,
    pub(crate) workdir: PathBuf,
}

pub(crate) struct CodexInvocationOptions<'a> {
    pub(crate) codex_home: &'a Path,
    pub(crate) sqlite_home: &'a Path,
    pub(crate) workdir: &'a Path,
    pub(crate) catalog_path: &'a Path,
    pub(crate) key_env: &'a str,
    pub(crate) key_value: &'a str,
    pub(crate) executable: &'a Path,
}

impl fmt::Debug for CodexInvocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let env_names = self
            .env
            .iter()
            .map(|(name, _)| name.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        formatter
            .debug_struct("CodexInvocation")
            .field("executable", &self.executable)
            .field("args", &self.args)
            .field("env_names", &env_names)
            .field("workdir", &self.workdir)
            .finish()
    }
}

pub(crate) fn is_supported_codex_version(version: &str) -> bool {
    matches!(version.trim(), "0.144.4" | "codex-cli 0.144.4")
}

pub(crate) fn parse_codex_version(stdout: &[u8]) -> Result<String, CodexRunnerError> {
    let text = std::str::from_utf8(stdout).map_err(|_| CodexRunnerError::UnsupportedVersion)?;
    let version = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or(CodexRunnerError::UnsupportedVersion)?;
    if is_supported_codex_version(version) {
        Ok(SUPPORTED_CODEX_VERSION.to_owned())
    } else {
        Err(CodexRunnerError::UnsupportedVersion)
    }
}

pub(crate) fn resolve_codex_executable_from_paths(
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<PathBuf, CodexRunnerError> {
    for directory in paths {
        for candidate in ["codex.exe", "codex.cmd", "codex"] {
            let path = directory.join(candidate);
            if path.is_file() {
                return fs::canonicalize(path).map_err(|_| CodexRunnerError::ExecutableUnavailable);
            }
        }
    }
    Err(CodexRunnerError::ExecutableUnavailable)
}

pub(crate) fn resolve_codex_executable() -> Result<PathBuf, CodexRunnerError> {
    let path = std::env::var_os("PATH").ok_or(CodexRunnerError::ExecutableUnavailable)?;
    resolve_codex_executable_from_paths(std::env::split_paths(&path))
}

pub(crate) fn default_managed_requirements_path() -> Option<PathBuf> {
    std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .or_else(|| cfg!(windows).then(|| PathBuf::from(r"C:\ProgramData")))
        .map(|root| root.join("OpenAI").join("Codex").join("requirements.toml"))
}

#[cfg(test)]
pub(crate) fn build_invocation(
    target: &ProviderAvailabilityTarget,
    codex_home: &Path,
    sqlite_home: &Path,
    workdir: &Path,
    catalog_path: &Path,
    key_env: &str,
) -> Result<CodexInvocation, String> {
    build_invocation_with_key_and_executable(
        target,
        CodexInvocationOptions {
            codex_home,
            sqlite_home,
            workdir,
            catalog_path,
            key_env,
            key_value: "test-key-codex-not-real",
            executable: Path::new("codex"),
        },
    )
}

pub(crate) fn build_invocation_with_key_and_executable(
    target: &ProviderAvailabilityTarget,
    options: CodexInvocationOptions<'_>,
) -> Result<CodexInvocation, String> {
    let CodexInvocationOptions {
        codex_home,
        sqlite_home,
        workdir,
        catalog_path,
        key_env,
        key_value,
        executable,
    } = options;
    if key_env.is_empty()
        || !key_env
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err("invalid provider key environment name".into());
    }
    let mut args = vec![
        OsString::from("exec"),
        OsString::from("--json"),
        OsString::from("--strict-config"),
        OsString::from("--ignore-user-config"),
        OsString::from("--ignore-rules"),
        OsString::from("--ephemeral"),
        OsString::from("--skip-git-repo-check"),
        OsString::from("-C"),
        workdir.as_os_str().to_owned(),
        OsString::from("--sandbox"),
        OsString::from("read-only"),
        OsString::from("--model"),
        OsString::from(&target.model),
    ];
    for (key, value) in [
        ("approval_policy", "\"never\"".to_owned()),
        ("sandbox_mode", "\"read-only\"".to_owned()),
        ("model_provider", toml_string(TEST_PROVIDER_ID)),
        (
            "model_catalog_json",
            toml_string(&catalog_path.to_string_lossy().replace('\\', "/")),
        ),
        ("project_root_markers", "[]".to_owned()),
        (
            "model_providers.codex_relay_test.name",
            toml_string("Codex Relay compatibility test"),
        ),
        (
            "model_providers.codex_relay_test.base_url",
            toml_string(&target.base_url),
        ),
        (
            "model_providers.codex_relay_test.wire_api",
            toml_string("responses"),
        ),
        (
            "model_providers.codex_relay_test.env_key",
            toml_string(key_env),
        ),
        (
            "tools.experimental_request_user_input.enabled",
            "false".to_owned(),
        ),
        ("web_search", toml_string("disabled")),
        ("mcp_servers", "{}".to_owned()),
    ] {
        args.push(OsString::from("-c"));
        args.push(OsString::from(format!("{key}={value}")));
    }
    for feature in DISABLED_FEATURES {
        args.push(OsString::from("-c"));
        args.push(OsString::from(format!("features.{feature}=false")));
    }
    args.push(OsString::from(TEST_PROMPT));

    Ok(CodexInvocation {
        executable: executable.to_owned(),
        args,
        env: vec![
            (
                OsString::from("CODEX_HOME"),
                codex_home.as_os_str().to_owned(),
            ),
            (
                OsString::from("CODEX_SQLITE_HOME"),
                sqlite_home.as_os_str().to_owned(),
            ),
            (OsString::from(key_env), OsString::from(key_value)),
        ],
        workdir: workdir.to_owned(),
    })
}

pub(crate) fn filter_inherited_environment<I>(entries: I) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    let mut filtered = Vec::new();
    for (name, value) in entries {
        let normalized = name.to_string_lossy().to_ascii_uppercase();
        if INHERITED_ENV_ALLOWLIST.contains(&normalized.as_str())
            && !filtered.iter().any(|(existing, _): &(OsString, OsString)| {
                existing.to_string_lossy().eq_ignore_ascii_case(&normalized)
            })
        {
            filtered.push((OsString::from(normalized), value));
        }
    }
    filtered
}

pub(crate) fn model_catalog_json(model: &str) -> Result<Value, String> {
    if model.trim().is_empty() {
        return Err("model cannot be empty".into());
    }
    Ok(json!({
        "models": [{
            "slug": model,
            "display_name": model,
            "description": "Codex Relay compatibility probe",
            "default_reasoning_level": "none",
            "supported_reasoning_levels": [{
                "effort": "none",
                "description": "No additional reasoning"
            }],
            "input_modalities": ["text"],
            "shell_type": "disabled",
            "apply_patch_tool_type": null,
            "visibility": "list",
            "supported_in_api": true,
            "priority": 1
        }]
    }))
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("JSON strings are valid TOML basic string literals")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::provider_service::ProviderAvailabilityTarget;
    use std::path::Path;

    #[test]
    fn only_the_experimentally_verified_cli_version_is_supported() {
        assert!(is_supported_codex_version("codex-cli 0.144.4"));
        assert!(!is_supported_codex_version("codex-cli 0.145.0"));
        assert!(!is_supported_codex_version("codex-cli 0.144.3"));
        assert!(!is_supported_codex_version("unexpected"));
    }

    #[test]
    fn version_probe_is_exact_and_executable_resolution_returns_absolute_path() {
        assert_eq!(
            parse_codex_version(b"codex-cli 0.144.4\n").unwrap(),
            SUPPORTED_CODEX_VERSION
        );
        assert_eq!(
            parse_codex_version(b"codex-cli 0.145.0\n").unwrap_err(),
            CodexRunnerError::UnsupportedVersion
        );
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("codex.exe");
        std::fs::write(&executable, b"fake").unwrap();
        let resolved =
            resolve_codex_executable_from_paths([directory.path().to_path_buf()]).unwrap();
        assert!(resolved.is_absolute());
        assert_eq!(resolved, std::fs::canonicalize(executable).unwrap());
    }

    #[test]
    fn invocation_contains_strict_flags_but_never_the_api_key() {
        let target = ProviderAvailabilityTarget {
            provider_id: "provider-a".into(),
            base_url: "https://provider.example.test/v1".into(),
            model: "gpt-5.6-sol".into(),
            api_key: "test-key-target-not-real".into(),
        };
        let invocation = build_invocation(
            &target,
            Path::new(r"C:\Temp\codex-home"),
            Path::new(r"C:\Temp\codex-sqlite"),
            Path::new(r"C:\Temp\codex-work"),
            Path::new(r"C:\Temp\catalog.json"),
            "CODEX_RELAY_PROVIDER_KEY_TEST",
        )
        .unwrap();
        let debug = format!("{invocation:?}");
        let args = invocation
            .args
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert!(!debug.contains("test-key-target-not-real"));
        assert!(args.iter().any(|arg| arg == "--ignore-user-config"));
        assert!(args.iter().any(|arg| arg == "--ignore-rules"));
        assert!(args.iter().any(|arg| arg == "--ephemeral"));
        assert!(args.iter().any(|arg| arg == "--strict-config"));
        assert!(args.iter().any(|arg| arg == "--json"));
        assert!(args.iter().any(|arg| arg == "mcp_servers={}"));
        assert!(args.iter().any(|arg| arg == "web_search=\"disabled\""));
        assert!(
            invocation
                .env
                .iter()
                .any(|(name, value)| name == "CODEX_RELAY_PROVIDER_KEY_TEST"
                    && value == "test-key-codex-not-real")
        );
        assert!(invocation.env.iter().any(|(name, value)| {
            name == "CODEX_SQLITE_HOME" && value == Path::new(r"C:\Temp\codex-sqlite")
        }));
    }

    #[test]
    fn inherited_environment_is_reduced_to_the_explicit_allowlist() {
        let filtered = filter_inherited_environment([
            (OsString::from("Path"), OsString::from(r"C:\safe-bin")),
            (
                OsString::from("HTTPS_PROXY"),
                OsString::from("http://127.0.0.1:8080"),
            ),
            (
                OsString::from("OPENAI_API_KEY"),
                OsString::from("test-key-must-not-pass-not-real"),
            ),
            (
                OsString::from("USERPROFILE"),
                OsString::from(r"C:\Users\real"),
            ),
            (
                OsString::from("CODEX_HOME"),
                OsString::from(r"C:\Users\real\.codex"),
            ),
        ]);
        let names = filtered
            .iter()
            .map(|(name, _)| name.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["PATH", "HTTPS_PROXY"]);
        assert!(!format!("{filtered:?}").contains("test-key-must-not-pass-not-real"));
    }

    #[test]
    fn model_catalog_disables_non_text_input_and_shell_capabilities() {
        let catalog = model_catalog_json("gpt-5.6-sol").unwrap();
        let model = &catalog["models"][0];

        assert_eq!(model["slug"], "gpt-5.6-sol");
        assert_eq!(model["input_modalities"], serde_json::json!(["text"]));
        assert_eq!(model["shell_type"], "disabled");
        assert!(model["apply_patch_tool_type"].is_null());
    }

    #[test]
    fn temp_layout_is_bounded_and_catalog_is_written_without_auth_files() {
        let layout = CodexTempLayout::new().unwrap();
        let root = layout.root().to_path_buf();
        write_model_catalog(layout.catalog_path(), "gpt-5.6-sol").unwrap();

        assert!(is_safe_temp_path(&root));
        assert!(layout.home().is_dir());
        assert!(layout.workdir().is_dir());
        assert!(layout.catalog_path().is_file());
        assert!(!layout.home().join("config.toml").exists());
        assert!(!layout.home().join("auth.json").exists());
        drop(layout);
        assert!(!root.exists());
    }

    #[test]
    fn managed_requirements_gate_fails_closed_when_file_exists() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("OpenAI").join("Codex");
        std::fs::create_dir_all(&path).unwrap();
        let requirements = path.join("requirements.toml");

        assert!(check_managed_requirements(&requirements).is_ok());
        std::fs::write(&requirements, "[features]\nhooks = true\n").unwrap();
        assert_eq!(
            check_managed_requirements(&requirements).unwrap_err(),
            CodexRunnerError::ManagedConfig
        );
    }

    #[cfg(windows)]
    #[test]
    fn explicit_cleanup_reports_locked_directory_failure() {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;

        let layout = CodexTempLayout::new().unwrap();
        let root = layout.root().to_path_buf();
        let locked_path = root.join("locked.txt");
        std::fs::write(&locked_path, b"locked").unwrap();
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&locked_path)
            .unwrap();

        assert_eq!(
            layout.cleanup().unwrap_err(),
            CodexRunnerError::CleanupFailed
        );
        assert!(root.exists());
        drop(locked);
        std::fs::remove_dir_all(root).unwrap();
    }
}
