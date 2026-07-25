use crate::error::AppError;
use crate::infrastructure::codex_gateway::{
    CodexCompatibilityGateway, GATEWAY_API_KEY, GatewayError, GatewayOutcome,
};
use crate::infrastructure::codex_jsonl::{
    CodexJsonlFailure, parse_codex_jsonl, validate_codex_exit, validate_codex_stderr,
};
use crate::infrastructure::codex_preflight::{
    CodexPreflightServer, PREFLIGHT_API_KEY, PreflightExpectation,
};
use crate::infrastructure::codex_process::{
    CodexProcessBackend, CodexProcessError, SystemCodexProcessBackend,
};
use crate::infrastructure::codex_runner::{
    CodexInvocation, CodexInvocationOptions, CodexRunnerError, CodexTempLayout,
    build_invocation_with_key_and_executable, check_managed_requirements,
    default_managed_requirements_path, parse_codex_version, resolve_codex_executable,
    write_model_catalog,
};
use crate::infrastructure::provider_http::{self, ApiProbeError};
use crate::models::provider_availability::{
    ProviderAvailabilityResult, ProviderAvailabilityTarget, ProviderTestKind, ProviderTestStatus,
};
use crate::services::provider_service::ProviderService;
use crate::services::settings_service::SettingsService;
use chrono::Utc;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::watch;
use uuid::Uuid;

const CODEX_VERSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
const CODEX_PREFLIGHT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const CODEX_RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Clone)]
pub(crate) struct CodexRuntimeVersion {
    pub(crate) executable: PathBuf,
    pub(crate) version: String,
}

pub(crate) trait CodexRuntime: Send + Sync {
    fn probe_version(
        &self,
        cancel: watch::Receiver<bool>,
    ) -> Pin<Box<dyn Future<Output = Result<CodexRuntimeVersion, CodexRunnerError>> + Send>>;
    fn process_backend(&self) -> Arc<dyn CodexProcessBackend>;
    fn managed_requirements_path(&self) -> Option<PathBuf>;
}

#[derive(Clone)]
pub(crate) struct SystemCodexRuntime {
    backend: Arc<dyn CodexProcessBackend>,
}

impl Default for SystemCodexRuntime {
    fn default() -> Self {
        Self {
            backend: Arc::new(SystemCodexProcessBackend::default()),
        }
    }
}

impl CodexRuntime for SystemCodexRuntime {
    fn probe_version(
        &self,
        cancel: watch::Receiver<bool>,
    ) -> Pin<Box<dyn Future<Output = Result<CodexRuntimeVersion, CodexRunnerError>> + Send>> {
        let backend = Arc::clone(&self.backend);
        Box::pin(async move {
            let executable = resolve_codex_executable()?;
            let invocation = CodexInvocation {
                executable: executable.clone(),
                args: vec!["--version".into()],
                env: Vec::new(),
                workdir: std::env::temp_dir(),
            };
            let output = backend
                .run(invocation, CODEX_VERSION_TIMEOUT, cancel)
                .await
                .map_err(map_process_error)?;
            validate_codex_exit(output.exit_code)
                .map_err(|_| CodexRunnerError::UnsupportedVersion)?;
            let version = parse_codex_version(&output.stdout)?;
            Ok(CodexRuntimeVersion {
                executable,
                version,
            })
        })
    }

    fn process_backend(&self) -> Arc<dyn CodexProcessBackend> {
        Arc::clone(&self.backend)
    }

    fn managed_requirements_path(&self) -> Option<PathBuf> {
        default_managed_requirements_path()
    }
}

#[derive(Clone, Default)]
pub(crate) struct ActiveTestRegistry {
    active: Arc<Mutex<Option<ActiveTest>>>,
}

#[derive(Debug)]
struct ActiveTest {
    request_id: Uuid,
    cancel: watch::Sender<bool>,
}

#[derive(Debug)]
pub(crate) struct ActiveTestHandle {
    request_id: Uuid,
    pub(crate) cancel: watch::Receiver<bool>,
    active: Arc<Mutex<Option<ActiveTest>>>,
}

impl ActiveTestRegistry {
    pub(crate) fn begin(&self, request_id: Uuid) -> Result<ActiveTestHandle, AppError> {
        let mut active = self.active.lock().map_err(|_| {
            AppError::new(
                "PROVIDER_TEST_STATE_FAILED",
                "无法保存 Provider 测试状态。",
                "provider availability registry lock poisoned",
            )
        })?;
        if active.is_some() {
            return Err(AppError::new(
                "PROVIDER_TEST_IN_PROGRESS",
                "已有 Provider 测试正在进行，请稍候。",
                "duplicate provider availability test rejected",
            ));
        }
        let (cancel, receiver) = watch::channel(false);
        *active = Some(ActiveTest { request_id, cancel });
        Ok(ActiveTestHandle {
            request_id,
            cancel: receiver,
            active: Arc::clone(&self.active),
        })
    }

    pub(crate) fn cancel(&self, request_id: Uuid) -> Result<bool, AppError> {
        let active = self.active.lock().map_err(|_| {
            AppError::new(
                "PROVIDER_TEST_STATE_FAILED",
                "无法读取 Provider 测试状态。",
                "provider availability registry lock poisoned",
            )
        })?;
        let Some(active) = active.as_ref() else {
            return Ok(false);
        };
        if active.request_id != request_id {
            return Ok(false);
        }
        active.cancel.send(true).map_err(|_| {
            AppError::new(
                "PROVIDER_TEST_STATE_FAILED",
                "无法取消 Provider 测试。",
                "provider availability cancellation receiver dropped",
            )
        })?;
        Ok(true)
    }
}

impl Drop for ActiveTestHandle {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active.lock()
            && active
                .as_ref()
                .is_some_and(|current| current.request_id == self.request_id)
        {
            active.take();
        }
    }
}

#[derive(Clone)]
pub struct ProviderAvailabilityService {
    provider_service: ProviderService,
    settings_service: SettingsService,
    app_version: String,
    registry: ActiveTestRegistry,
    codex_runtime: Arc<dyn CodexRuntime>,
}

fn proxy_disabled_error() -> AppError {
    AppError::new(
        "PROVIDER_TEST_PROXY_DISABLED",
        "设置中的“网络代理”尚未启用，无法使用代理测试。",
        "provider test requested a disabled network proxy",
    )
}

impl ProviderAvailabilityService {
    pub fn new(
        provider_service: ProviderService,
        settings_service: SettingsService,
        app_version: impl Into<String>,
    ) -> Self {
        Self::with_codex_runtime(
            provider_service,
            settings_service,
            app_version,
            Arc::new(SystemCodexRuntime::default()),
        )
    }

    pub(crate) fn with_codex_runtime(
        provider_service: ProviderService,
        settings_service: SettingsService,
        app_version: impl Into<String>,
        codex_runtime: Arc<dyn CodexRuntime>,
    ) -> Self {
        Self {
            provider_service,
            settings_service,
            app_version: app_version.into(),
            registry: ActiveTestRegistry::default(),
            codex_runtime,
        }
    }

    pub async fn test_api(
        &self,
        provider_id: &str,
        request_id: Uuid,
        use_proxy: bool,
    ) -> Result<ProviderAvailabilityResult, AppError> {
        let mut active = self.registry.begin(request_id)?;
        let started = Instant::now();
        let target = match self
            .provider_service
            .resolve_availability_target(provider_id)
        {
            Ok(target) => target,
            Err(error) => {
                return Ok(result_from_target_error(
                    provider_id,
                    ProviderTestKind::Api,
                    started,
                    error.code(),
                    error.public_message(),
                ));
            }
        };
        let proxy = self.resolve_test_proxy(use_proxy)?;
        let probe = provider_http::probe_api(
            &target,
            proxy.as_deref(),
            &self.app_version,
            &mut active.cancel,
        )
        .await;
        let result = match probe {
            Ok(report) => ProviderAvailabilityResult {
                provider_id: target.provider_id,
                kind: ProviderTestKind::Api,
                status: ProviderTestStatus::Passed,
                code: "API_TEST_PASSED".into(),
                message: "API 可用性测试通过，已收到完成的 Responses 响应。".into(),
                model: target.model,
                duration_ms: started.elapsed().as_millis() as u64,
                tested_at: Utc::now().to_rfc3339(),
                http_status: Some(report.http_status),
                codex_version: None,
            },
            Err(error) => result_from_api_error(&target, started, error),
        };
        drop(active);
        Ok(result)
    }

    pub fn cancel(&self, request_id: Uuid) -> Result<bool, AppError> {
        self.registry.cancel(request_id)
    }

    pub async fn test_codex(
        &self,
        provider_id: &str,
        request_id: Uuid,
        use_proxy: bool,
    ) -> Result<ProviderAvailabilityResult, AppError> {
        let active = self.registry.begin(request_id)?;
        let started = Instant::now();
        let target = match self
            .provider_service
            .resolve_availability_target(provider_id)
        {
            Ok(target) => target,
            Err(error) => {
                return Ok(result_from_target_error(
                    provider_id,
                    ProviderTestKind::Codex,
                    started,
                    error.code(),
                    error.public_message(),
                ));
            }
        };
        let proxy = self.resolve_test_proxy(use_proxy)?;

        let version = match self
            .codex_runtime
            .probe_version(active.cancel.clone())
            .await
        {
            Ok(version) => version,
            Err(error) => {
                return Ok(result_from_codex_runner_error(
                    &target, started, error, None,
                ));
            }
        };
        if let Some(requirements) = self.codex_runtime.managed_requirements_path()
            && let Err(error) = check_managed_requirements(&requirements)
        {
            return Ok(result_from_codex_runner_error(
                &target,
                started,
                error,
                Some(version.version),
            ));
        }

        let layout = match CodexTempLayout::new() {
            Ok(layout) => layout,
            Err(error) => {
                return Ok(result_from_codex_runner_error(
                    &target,
                    started,
                    error,
                    Some(version.version),
                ));
            }
        };
        let execution = self
            .execute_codex(
                &target,
                &version,
                &layout,
                proxy.as_deref(),
                active.cancel.clone(),
            )
            .await;
        let cleanup = layout.cleanup();
        let execution = match cleanup {
            Ok(()) => execution,
            Err(error) => CodexExecutionOutcome::from_runner_error(error),
        };

        Ok(result_from_codex_execution(
            &target,
            started,
            Some(version.version),
            execution,
        ))
    }

    fn resolve_test_proxy(&self, use_proxy: bool) -> Result<Option<String>, AppError> {
        if !use_proxy {
            return Ok(None);
        }
        let settings = self.settings_service.load_read_only()?;
        if !settings.network_proxy.enabled || settings.network_proxy.url.is_empty() {
            return Err(proxy_disabled_error());
        }
        Ok(Some(settings.network_proxy.url))
    }

    async fn execute_codex(
        &self,
        target: &ProviderAvailabilityTarget,
        version: &CodexRuntimeVersion,
        layout: &CodexTempLayout,
        proxy: Option<&str>,
        cancel: watch::Receiver<bool>,
    ) -> CodexExecutionOutcome {
        if let Err(error) = write_model_catalog(layout.catalog_path(), &target.model) {
            return CodexExecutionOutcome::from_runner_error(error);
        }
        let key_env = format!(
            "CODEX_RELAY_PROVIDER_KEY_{}",
            Uuid::new_v4().simple().to_string().to_ascii_uppercase()
        );
        let backend = self.codex_runtime.process_backend();

        let preflight =
            match CodexPreflightServer::start(PreflightExpectation::new(target.model.clone())) {
                Ok(server) => server,
                Err(_) => return CodexExecutionOutcome::preflight_failed(),
            };
        let mut preflight_target = target.clone();
        preflight_target.base_url = preflight.provider_base_url();
        preflight_target.api_key.clear();
        let invocation = match build_invocation_with_key_and_executable(
            &preflight_target,
            CodexInvocationOptions {
                codex_home: layout.home(),
                sqlite_home: layout.sqlite_home(),
                workdir: layout.workdir(),
                catalog_path: layout.catalog_path(),
                key_env: &key_env,
                key_value: PREFLIGHT_API_KEY,
                executable: &version.executable,
            },
        ) {
            Ok(invocation) => invocation,
            Err(_) => return CodexExecutionOutcome::preflight_failed(),
        };
        let preflight_process = backend
            .run(invocation, CODEX_PREFLIGHT_TIMEOUT, cancel.clone())
            .await;
        let preflight_output = match preflight_process {
            Ok(output) => output,
            Err(error) => return CodexExecutionOutcome::from_process_error(error),
        };
        let preflight_report = match tokio::task::spawn_blocking(move || {
            preflight.wait(CODEX_PREFLIGHT_TIMEOUT)
        })
        .await
        {
            Ok(Ok(report)) => report,
            _ => return CodexExecutionOutcome::preflight_failed(),
        };
        if let Some(outcome) = validate_codex_output(&preflight_output, false) {
            return outcome;
        }
        if preflight_report.tool_names != vec!["update_plan".to_owned(), "view_image".to_owned()] {
            return CodexExecutionOutcome::preflight_failed();
        }

        let gateway = match CodexCompatibilityGateway::start(target.clone(), proxy).await {
            Ok(gateway) => gateway,
            Err(_) => return CodexExecutionOutcome::preflight_failed(),
        };
        let gateway_url = format!("{}/v1", gateway.base_url());
        let mut real_target = target.clone();
        real_target.base_url = gateway_url;
        real_target.api_key.clear();
        let invocation = match build_invocation_with_key_and_executable(
            &real_target,
            CodexInvocationOptions {
                codex_home: layout.home(),
                sqlite_home: layout.sqlite_home(),
                workdir: layout.workdir(),
                catalog_path: layout.catalog_path(),
                key_env: &key_env,
                key_value: GATEWAY_API_KEY,
                executable: &version.executable,
            },
        ) {
            Ok(invocation) => invocation,
            Err(_) => return CodexExecutionOutcome::preflight_failed(),
        };
        let process = backend.run(invocation, CODEX_RUN_TIMEOUT, cancel).await;
        let output = match process {
            Ok(output) => output,
            Err(error) => {
                drop(gateway);
                return CodexExecutionOutcome::from_process_error(error);
            }
        };
        let gateway_outcome = match gateway.wait(CODEX_RUN_TIMEOUT).await {
            Ok(outcome) => outcome,
            Err(GatewayError::UpstreamNetwork) => {
                return CodexExecutionOutcome::failed(
                    "CODEX_PROVIDER_NETWORK_FAILED",
                    "无法连接 Provider。",
                );
            }
            Err(GatewayError::Timeout) => {
                return CodexExecutionOutcome::failed(
                    "CODEX_GATEWAY_TIMEOUT",
                    "兼容性转发层超时。",
                );
            }
            Err(GatewayError::Cancelled) => return CodexExecutionOutcome::cancelled(),
            Err(_) => {
                return CodexExecutionOutcome::failed("CODEX_GATEWAY_FAILED", "兼容性转发层失败。");
            }
        };
        match gateway_outcome {
            GatewayOutcome::Passed => {}
            GatewayOutcome::ToolCallBlocked => {
                return CodexExecutionOutcome::failed(
                    "CODEX_TOOL_CALL_BLOCKED",
                    "Codex 兼容性测试检测到工具调用，已安全阻止。",
                );
            }
            GatewayOutcome::RequestRejected => return CodexExecutionOutcome::preflight_failed(),
            GatewayOutcome::UpstreamHttp(status) => {
                return CodexExecutionOutcome::failed_with_http(
                    "CODEX_PROVIDER_HTTP_FAILED",
                    "Provider 返回了错误。",
                    status,
                );
            }
            GatewayOutcome::UpstreamInvalid => {
                return CodexExecutionOutcome::failed(
                    "CODEX_PROVIDER_RESPONSE_INVALID",
                    "Provider Responses 流格式无效。",
                );
            }
            GatewayOutcome::Cancelled => return CodexExecutionOutcome::cancelled(),
        }
        if let Some(outcome) = validate_codex_output(&output, true) {
            return outcome;
        }
        CodexExecutionOutcome::Passed
    }
}

#[derive(Clone, Debug)]
enum CodexExecutionOutcome {
    Passed,
    Failed {
        status: ProviderTestStatus,
        code: &'static str,
        message: &'static str,
        http_status: Option<u16>,
    },
}

impl CodexExecutionOutcome {
    fn failed(code: &'static str, message: &'static str) -> Self {
        Self::Failed {
            status: ProviderTestStatus::Failed,
            code,
            message,
            http_status: None,
        }
    }

    fn failed_with_http(code: &'static str, message: &'static str, status: u16) -> Self {
        Self::Failed {
            status: ProviderTestStatus::Failed,
            code,
            message,
            http_status: Some(status),
        }
    }

    fn preflight_failed() -> Self {
        Self::Failed {
            status: ProviderTestStatus::Unsupported,
            code: "CODEX_PREFLIGHT_FAILED",
            message: "当前 Codex 无法通过安全兼容性预检。",
            http_status: None,
        }
    }

    fn cancelled() -> Self {
        Self::Failed {
            status: ProviderTestStatus::Cancelled,
            code: "PROVIDER_TEST_CANCELLED",
            message: "Provider 测试已取消。",
            http_status: None,
        }
    }

    fn from_process_error(error: CodexProcessError) -> Self {
        match error {
            CodexProcessError::Cancelled => Self::cancelled(),
            CodexProcessError::Timeout => Self::failed("CODEX_TIMEOUT", "Codex 兼容性测试超时。"),
            CodexProcessError::OutputTooLarge => {
                Self::failed("CODEX_OUTPUT_TOO_LARGE", "Codex 输出超过安全大小上限。")
            }
            CodexProcessError::JobUnavailable | CodexProcessError::JobAssignment => {
                Self::preflight_failed()
            }
            CodexProcessError::ProcessTreeTermination => {
                Self::failed("CODEX_PROCESS_TREE_FAILED", "Codex 进程树未能安全终止。")
            }
            CodexProcessError::ProcessStart
            | CodexProcessError::ProcessResume
            | CodexProcessError::OutputRead => {
                Self::failed("CODEX_PROCESS_FAILED", "Codex 进程运行失败。")
            }
        }
    }

    fn from_runner_error(error: CodexRunnerError) -> Self {
        match error {
            CodexRunnerError::ExecutableUnavailable => Self::Failed {
                status: ProviderTestStatus::Unsupported,
                code: "CODEX_CLI_MISSING",
                message: "未检测到 Codex CLI，无法运行兼容性测试。",
                http_status: None,
            },
            CodexRunnerError::UnsupportedVersion => Self::Failed {
                status: ProviderTestStatus::Unsupported,
                code: "CODEX_VERSION_UNSUPPORTED",
                message: "当前 Codex CLI 版本不支持安全兼容性测试。",
                http_status: None,
            },
            CodexRunnerError::ManagedConfig => Self::Failed {
                status: ProviderTestStatus::Unsupported,
                code: "CODEX_MANAGED_CONFIG_UNSUPPORTED",
                message: "检测到系统 managed requirements，无法安全运行兼容性测试。",
                http_status: None,
            },
            CodexRunnerError::UnsafeTempPath
            | CodexRunnerError::CatalogInvalid
            | CodexRunnerError::PreflightFailed => Self::preflight_failed(),
            CodexRunnerError::CleanupFailed => {
                Self::failed("CODEX_CLEANUP_FAILED", "兼容性测试临时目录清理失败。")
            }
            CodexRunnerError::Timeout => Self::failed("CODEX_TIMEOUT", "Codex 兼容性测试超时。"),
            CodexRunnerError::Cancelled => Self::cancelled(),
            _ => Self::failed("CODEX_RUNNER_FAILED", "Codex 兼容性测试未完成。"),
        }
    }
}

fn map_process_error(error: CodexProcessError) -> CodexRunnerError {
    match error {
        CodexProcessError::JobUnavailable | CodexProcessError::JobAssignment => {
            CodexRunnerError::PreflightFailed
        }
        CodexProcessError::ProcessStart
        | CodexProcessError::ProcessResume
        | CodexProcessError::OutputRead => CodexRunnerError::ProcessStart,
        CodexProcessError::OutputTooLarge => CodexRunnerError::OutputTooLarge,
        CodexProcessError::Timeout => CodexRunnerError::Timeout,
        CodexProcessError::Cancelled => CodexRunnerError::Cancelled,
        CodexProcessError::ProcessTreeTermination => CodexRunnerError::ProcessTreeTermination,
    }
}

fn validate_codex_output(
    output: &crate::infrastructure::codex_process::CodexProcessOutput,
    require_agent_message: bool,
) -> Option<CodexExecutionOutcome> {
    if let Err(error) = validate_codex_exit(output.exit_code) {
        return Some(codex_jsonl_outcome(error));
    }
    if let Err(error) = validate_codex_stderr(&output.stderr) {
        return Some(codex_jsonl_outcome(error));
    }
    match parse_codex_jsonl(&output.stdout) {
        Ok(summary)
            if summary.turn_completed
                && (!require_agent_message || summary.agent_message_count > 0) =>
        {
            None
        }
        Ok(_) => Some(CodexExecutionOutcome::failed(
            "CODEX_JSONL_INVALID",
            "Codex JSONL 未包含完整的正常回合。",
        )),
        Err(error) => Some(codex_jsonl_outcome(error)),
    }
}

fn codex_jsonl_outcome(error: CodexJsonlFailure) -> CodexExecutionOutcome {
    match error {
        CodexJsonlFailure::TurnFailed | CodexJsonlFailure::RemoteError => {
            CodexExecutionOutcome::failed("CODEX_REMOTE_FAILED", "Codex Provider 返回了失败结果。")
        }
        CodexJsonlFailure::ToolCall => CodexExecutionOutcome::failed(
            "CODEX_TOOL_CALL_BLOCKED",
            "Codex 兼容性测试检测到工具调用，已安全阻止。",
        ),
        CodexJsonlFailure::StderrWarning => {
            CodexExecutionOutcome::failed("CODEX_STDERR_WARNING", "Codex 输出了安全警告。")
        }
        CodexJsonlFailure::ExitFailure => {
            CodexExecutionOutcome::failed("CODEX_EXIT_FAILED", "Codex 进程退出码异常。")
        }
        CodexJsonlFailure::SecurityEvent | CodexJsonlFailure::SecurityWarning => {
            CodexExecutionOutcome::failed("CODEX_SECURITY_EVENT", "Codex 运行触发了安全事件。")
        }
        CodexJsonlFailure::UnknownEvent
        | CodexJsonlFailure::InvalidJson
        | CodexJsonlFailure::InvalidUtf8
        | CodexJsonlFailure::Truncated
        | CodexJsonlFailure::ProtocolOrder => {
            CodexExecutionOutcome::failed("CODEX_JSONL_INVALID", "Codex JSONL 协议无效。")
        }
    }
}

fn result_from_codex_runner_error(
    target: &ProviderAvailabilityTarget,
    started: Instant,
    error: CodexRunnerError,
    codex_version: Option<String>,
) -> ProviderAvailabilityResult {
    result_from_codex_execution(
        target,
        started,
        codex_version,
        CodexExecutionOutcome::from_runner_error(error),
    )
}

fn result_from_codex_execution(
    target: &ProviderAvailabilityTarget,
    started: Instant,
    codex_version: Option<String>,
    execution: CodexExecutionOutcome,
) -> ProviderAvailabilityResult {
    let (status, code, message, http_status) = match execution {
        CodexExecutionOutcome::Passed => (
            ProviderTestStatus::Passed,
            "CODEX_COMPATIBILITY_PASSED",
            "Codex 兼容性测试通过，已完成一次无工具正常回合。",
            None,
        ),
        CodexExecutionOutcome::Failed {
            status,
            code,
            message,
            http_status,
        } => (status, code, message, http_status),
    };
    ProviderAvailabilityResult {
        provider_id: target.provider_id.clone(),
        kind: ProviderTestKind::Codex,
        status,
        code: code.into(),
        message: message.into(),
        model: target.model.clone(),
        duration_ms: started.elapsed().as_millis() as u64,
        tested_at: Utc::now().to_rfc3339(),
        http_status,
        codex_version,
    }
}

fn result_from_api_error(
    target: &ProviderAvailabilityTarget,
    started: Instant,
    error: ApiProbeError,
) -> ProviderAvailabilityResult {
    let (status, code, message, http_status) = match error {
        ApiProbeError::InvalidEndpoint => (
            ProviderTestStatus::Failed,
            "API_INVALID_ENDPOINT",
            "Provider Base URL 无法用于 API 测试。",
            None,
        ),
        ApiProbeError::RequestBuild => (
            ProviderTestStatus::Failed,
            "API_REQUEST_FAILED",
            "无法构造 API 测试请求。",
            None,
        ),
        ApiProbeError::Network => (
            ProviderTestStatus::Failed,
            "API_NETWORK_FAILED",
            "无法连接 Provider API。",
            None,
        ),
        ApiProbeError::Tls => (
            ProviderTestStatus::Failed,
            "API_TLS_FAILED",
            "Provider TLS 连接失败。",
            None,
        ),
        ApiProbeError::Timeout => (
            ProviderTestStatus::Failed,
            "API_TIMEOUT",
            "API 测试超时。",
            None,
        ),
        ApiProbeError::Auth => (
            ProviderTestStatus::Failed,
            "API_AUTH_FAILED",
            "Provider 拒绝了 API Key。",
            None,
        ),
        ApiProbeError::EndpointOrModelNotFound => (
            ProviderTestStatus::Failed,
            "API_ENDPOINT_OR_MODEL_NOT_FOUND",
            "Provider 找不到 Responses 端点或当前模型。",
            None,
        ),
        ApiProbeError::RateLimited => (
            ProviderTestStatus::Failed,
            "API_RATE_LIMITED",
            "Provider 暂时限制了请求频率。",
            None,
        ),
        ApiProbeError::Provider => (
            ProviderTestStatus::Failed,
            "API_PROVIDER_ERROR",
            "Provider 返回了服务器错误。",
            None,
        ),
        ApiProbeError::Http(status) => (
            ProviderTestStatus::Failed,
            "API_HTTP_FAILED",
            "Provider 返回了未预期的 HTTP 状态。",
            Some(status),
        ),
        ApiProbeError::ResponseTooLarge => (
            ProviderTestStatus::Failed,
            "API_RESPONSE_TOO_LARGE",
            "Provider 响应超过安全大小上限。",
            None,
        ),
        ApiProbeError::ResponseInvalid => (
            ProviderTestStatus::Failed,
            "API_RESPONSE_INVALID",
            "Provider 返回的 Responses 响应格式无效。",
            None,
        ),
        ApiProbeError::Cancelled => (
            ProviderTestStatus::Cancelled,
            "PROVIDER_TEST_CANCELLED",
            "Provider 测试已取消。",
            None,
        ),
    };
    ProviderAvailabilityResult {
        provider_id: target.provider_id.clone(),
        kind: ProviderTestKind::Api,
        status,
        code: code.into(),
        message: message.into(),
        model: target.model.clone(),
        duration_ms: started.elapsed().as_millis() as u64,
        tested_at: Utc::now().to_rfc3339(),
        http_status,
        codex_version: None,
    }
}

fn result_from_target_error(
    provider_id: &str,
    kind: ProviderTestKind,
    started: Instant,
    code: &str,
    message: &str,
) -> ProviderAvailabilityResult {
    ProviderAvailabilityResult {
        provider_id: provider_id.into(),
        kind,
        status: ProviderTestStatus::Failed,
        code: code.into(),
        message: message.into(),
        model: String::new(),
        duration_ms: started.elapsed().as_millis() as u64,
        tested_at: Utc::now().to_rfc3339(),
        http_status: None,
        codex_version: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::path_service::AppPaths;
    use crate::models::provider_availability::ProviderTestStatus;
    use crate::services::provider_service::ProviderService;
    use crate::services::settings_service::SettingsService;
    use std::fs;
    use std::future::Future;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;
    use uuid::Uuid;

    const CODEX_JSONL_SUCCESS: &str = concat!(
        "{\"type\":\"thread.started\",\"thread_id\":\"thread\"}\n",
        "{\"type\":\"turn.started\"}\n",
        "{\"type\":\"item.completed\",\"item\":{\"id\":\"item\",\"type\":\"agent_message\",\"text\":\"CODEX_RELAY_OK\"}}\n",
        "{\"type\":\"turn.completed\",\"usage\":{\"input_tokens\":1,\"cached_input_tokens\":0,\"output_tokens\":1,\"reasoning_output_tokens\":0}}\n"
    );

    #[derive(Clone)]
    struct FakeCodexRuntime {
        backend: Arc<ScriptedCodexBackend>,
        supported: bool,
        managed_requirements: Option<PathBuf>,
    }

    impl CodexRuntime for FakeCodexRuntime {
        fn probe_version(
            &self,
            _cancel: watch::Receiver<bool>,
        ) -> Pin<Box<dyn Future<Output = Result<CodexRuntimeVersion, CodexRunnerError>> + Send>>
        {
            let supported = self.supported;
            Box::pin(async move {
                if !supported {
                    return Err(CodexRunnerError::UnsupportedVersion);
                }
                Ok(CodexRuntimeVersion {
                    executable: PathBuf::from(r"C:\fake\codex.exe"),
                    version: "0.144.4".into(),
                })
            })
        }

        fn process_backend(&self) -> Arc<dyn CodexProcessBackend> {
            Arc::clone(&self.backend) as Arc<dyn CodexProcessBackend>
        }

        fn managed_requirements_path(&self) -> Option<PathBuf> {
            self.managed_requirements.clone()
        }
    }

    struct MissingCodexRuntime {
        backend: Arc<ScriptedCodexBackend>,
    }

    impl CodexRuntime for MissingCodexRuntime {
        fn probe_version(
            &self,
            _cancel: watch::Receiver<bool>,
        ) -> Pin<Box<dyn Future<Output = Result<CodexRuntimeVersion, CodexRunnerError>> + Send>>
        {
            Box::pin(async { Err(CodexRunnerError::ExecutableUnavailable) })
        }

        fn process_backend(&self) -> Arc<dyn CodexProcessBackend> {
            Arc::clone(&self.backend) as Arc<dyn CodexProcessBackend>
        }

        fn managed_requirements_path(&self) -> Option<PathBuf> {
            None
        }
    }

    #[derive(Default)]
    struct ScriptedCodexBackend {
        requests: AtomicUsize,
    }

    impl CodexProcessBackend for ScriptedCodexBackend {
        fn run(
            &self,
            invocation: CodexInvocation,
            _timeout: Duration,
            cancel: watch::Receiver<bool>,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<
                            crate::infrastructure::codex_process::CodexProcessOutput,
                            CodexProcessError,
                        >,
                    > + Send,
            >,
        > {
            let is_version =
                invocation.args.len() == 1 && invocation.args[0].to_string_lossy() == "--version";
            let base_url = invocation
                .args
                .iter()
                .map(|arg| arg.to_string_lossy().to_string())
                .find_map(|arg| {
                    arg.strip_prefix("model_providers.codex_relay_test.base_url=")
                        .map(|value| value.trim_matches('"').to_owned())
                });
            let key = invocation
                .env
                .iter()
                .find(|(name, _)| {
                    name.to_string_lossy()
                        .starts_with("CODEX_RELAY_PROVIDER_KEY_")
                })
                .map(|(_, value)| value.to_string_lossy().to_string());
            self.requests.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if *cancel.borrow() {
                    return Err(CodexProcessError::Cancelled);
                }
                if is_version {
                    return Ok(crate::infrastructure::codex_process::CodexProcessOutput {
                        exit_code: Some(0),
                        stdout: b"codex-cli 0.144.4\n".to_vec(),
                        stderr: Vec::new(),
                    });
                }
                let base_url = base_url.ok_or(CodexProcessError::ProcessStart)?;
                let key = key.ok_or(CodexProcessError::ProcessStart)?;
                crate::infrastructure::rustls_provider::ensure_ring_crypto_provider()
                    .map_err(|_| CodexProcessError::ProcessStart)?;
                let body = serde_json::to_vec(&serde_json::json!({
                    "model": "gpt-5.6-sol",
                    "input": [],
                    "tools": [
                        {"type":"function","name":"update_plan"},
                        {"type":"function","name":"view_image"}
                    ],
                    "stream": true
                }))
                .map_err(|_| CodexProcessError::ProcessStart)?;
                let client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .no_proxy()
                    .build()
                    .map_err(|_| CodexProcessError::ProcessStart)?;
                let mut attempt = 0;
                let status = loop {
                    match send_scripted_codex_request(&client, &base_url, &key, &body).await {
                        Ok(status) => break status,
                        Err(_error) if attempt < 2 => {
                            attempt += 1;
                            tokio::time::sleep(Duration::from_millis(5)).await;
                        }
                        Err(error) => return Err(error),
                    }
                };
                if status == 200 {
                    Ok(crate::infrastructure::codex_process::CodexProcessOutput {
                        exit_code: Some(0),
                        stdout: CODEX_JSONL_SUCCESS.as_bytes().to_vec(),
                        stderr: Vec::new(),
                    })
                } else {
                    Ok(crate::infrastructure::codex_process::CodexProcessOutput {
                        exit_code: Some(1),
                        stdout: b"{\"type\":\"turn.failed\",\"error\":{\"message\":\"upstream failed\"}}\n".to_vec(),
                        stderr: Vec::new(),
                    })
                }
            })
        }
    }

    async fn send_scripted_codex_request(
        client: &reqwest::Client,
        base_url: &str,
        key: &str,
        body: &[u8],
    ) -> Result<u16, CodexProcessError> {
        let response = client
            .post(format!("{}/responses", base_url.trim_end_matches('/')))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::CONNECTION, "close")
            .bearer_auth(key)
            .body(body.to_vec())
            .send()
            .await
            .map_err(|_| CodexProcessError::ProcessStart)?;
        let status = response.status().as_u16();
        response
            .bytes()
            .await
            .map_err(|_| CodexProcessError::OutputRead)?;
        Ok(status)
    }

    #[test]
    fn active_test_registry_rejects_duplicates_and_releases_on_drop() {
        let registry = ActiveTestRegistry::default();
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();

        let first = registry.begin(first_id).unwrap();
        assert_eq!(
            registry.begin(second_id).unwrap_err().code(),
            "PROVIDER_TEST_IN_PROGRESS"
        );
        assert!(!registry.cancel(second_id).unwrap());
        assert!(registry.cancel(first_id).unwrap());
        assert!(*first.cancel.borrow());

        drop(first);

        assert!(registry.begin(second_id).is_ok());
    }

    #[tokio::test]
    async fn api_test_returns_safe_failed_result_for_missing_key() {
        let directory = tempfile::tempdir().unwrap();
        let codex = directory.path().join("codex");
        let app_data = directory.path().join("app-data");
        fs::create_dir_all(&codex).unwrap();
        fs::create_dir_all(&app_data).unwrap();
        let paths = AppPaths::for_test(codex, app_data).unwrap();
        fs::write(
            &paths.config_file,
            "model_provider = \"provider-a\"\n\n[model_providers.provider-a]\nname = \"Provider A\"\nbase_url = \"https://provider-a.example.test/v1\"\nwire_api = \"responses\"\n",
        )
        .unwrap();
        fs::write(&paths.providers_file, "{\"version\":1,\"providers\":{}}\n").unwrap();
        fs::write(
            &paths.provider_preferences_file,
            "{\"version\":1,\"providers\":{\"provider-a\":{\"models\":[\"gpt-5.6-sol\"],\"selectedModel\":\"gpt-5.6-sol\",\"reasoningEfforts\":{\"gpt-5.6-sol\":\"medium\"}}}}\n",
        )
        .unwrap();
        let settings = SettingsService::new(paths.clone());
        settings.bootstrap().unwrap();
        let service = ProviderAvailabilityService::new(
            ProviderService::new(paths, "0.1.0"),
            settings,
            "0.1.0",
        );

        let result = service
            .test_api("provider-a", Uuid::new_v4(), false)
            .await
            .unwrap();

        assert_eq!(result.status, ProviderTestStatus::Failed);
        assert_eq!(result.code, "PROVIDER_TEST_KEY_MISSING");
        assert!(!serde_json::to_string(&result).unwrap().contains("test-key"));
    }

    #[tokio::test]
    async fn api_test_returns_passed_result_without_exposing_key() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).unwrap();
            let body = r#"{"id":"resp_test","status":"completed","output":[]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let directory = tempfile::tempdir().unwrap();
        let codex = directory.path().join("codex");
        let app_data = directory.path().join("app-data");
        fs::create_dir_all(&codex).unwrap();
        fs::create_dir_all(&app_data).unwrap();
        let paths = AppPaths::for_test(codex, app_data).unwrap();
        fs::write(
            &paths.config_file,
            format!(
                "model_provider = \"provider-a\"\n\n[model_providers.provider-a]\nname = \"Provider A\"\nbase_url = \"http://{address}/v1\"\nwire_api = \"responses\"\n"
            ),
        )
        .unwrap();
        fs::write(
            &paths.providers_file,
            "{\"version\":1,\"providers\":{\"provider-a\":{\"apiKey\":\"test-key-api-not-real\"}}}\n",
        )
        .unwrap();
        fs::write(
            &paths.auth_file,
            "{\"OPENAI_API_KEY\":\"test-key-api-not-real\"}\n",
        )
        .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            "{\"version\":1,\"providers\":{\"provider-a\":{\"models\":[\"gpt-5.6-sol\"],\"selectedModel\":\"gpt-5.6-sol\",\"reasoningEfforts\":{\"gpt-5.6-sol\":\"medium\"}}}}\n",
        )
        .unwrap();
        let settings = SettingsService::new(paths.clone());
        settings.bootstrap().unwrap();
        let service = ProviderAvailabilityService::new(
            ProviderService::new(paths, "0.1.0"),
            settings,
            "0.1.0",
        );

        let result = service
            .test_api("provider-a", Uuid::new_v4(), false)
            .await
            .unwrap();

        assert_eq!(result.status, ProviderTestStatus::Passed);
        assert_eq!(result.code, "API_TEST_PASSED");
        assert_eq!(result.http_status, Some(200));
        assert_eq!(result.model, "gpt-5.6-sol");
        assert!(
            !serde_json::to_string(&result)
                .unwrap()
                .contains("test-key-api-not-real")
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn api_test_does_not_create_settings_file() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request).unwrap();
            let body = r#"{"id":"resp_test","status":"completed","output":[]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let directory = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap();
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        write_provider_state(&paths, &format!("http://{address}/v1"));
        assert!(!paths.settings_file.exists());
        let settings = SettingsService::new(paths.clone());
        let service = ProviderAvailabilityService::new(
            ProviderService::new(paths.clone(), "0.1.0"),
            settings,
            "0.1.0",
        );

        let result = service
            .test_api("provider-a", Uuid::new_v4(), false)
            .await
            .unwrap();

        assert_eq!(result.status, ProviderTestStatus::Passed);
        assert!(!paths.settings_file.exists());
        server.join().unwrap();
    }

    fn write_provider_state(paths: &AppPaths, base_url: &str) {
        fs::write(
            &paths.config_file,
            format!(
                "model_provider = \"provider-a\"\n\n[model_providers.provider-a]\nname = \"Provider A\"\nbase_url = \"{base_url}\"\nwire_api = \"responses\"\n"
            ),
        )
        .unwrap();
        fs::write(
            &paths.providers_file,
            "{\"version\":1,\"providers\":{\"provider-a\":{\"apiKey\":\"test-key-target-not-real\"}}}\n",
        )
        .unwrap();
        fs::write(
            &paths.auth_file,
            "{\"OPENAI_API_KEY\":\"test-key-target-not-real\"}\n",
        )
        .unwrap();
        fs::write(
            &paths.provider_preferences_file,
            "{\"version\":1,\"providers\":{\"provider-a\":{\"models\":[\"gpt-5.6-sol\"],\"selectedModel\":\"gpt-5.6-sol\",\"reasoningEfforts\":{\"gpt-5.6-sol\":\"medium\"}}}}\n",
        )
        .unwrap();
    }

    #[tokio::test]
    async fn proxy_mode_requires_an_enabled_network_proxy_without_writing_settings() {
        let directory = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap();
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        write_provider_state(&paths, "http://127.0.0.1:9/v1");
        assert!(!paths.settings_file.exists());
        let service = ProviderAvailabilityService::new(
            ProviderService::new(paths.clone(), "0.1.0"),
            SettingsService::new(paths.clone()),
            "0.1.0",
        );

        let api_error = service
            .test_api("provider-a", Uuid::new_v4(), true)
            .await
            .unwrap_err();
        let codex_error = service
            .test_codex("provider-a", Uuid::new_v4(), true)
            .await
            .unwrap_err();

        assert_eq!(api_error.code(), "PROVIDER_TEST_PROXY_DISABLED");
        assert_eq!(codex_error.code(), "PROVIDER_TEST_PROXY_DISABLED");
        assert!(!paths.settings_file.exists());
        assert!(!api_error.public_message().contains("127.0.0.1"));
        assert!(!codex_error.public_message().contains("127.0.0.1"));
    }

    #[tokio::test]
    async fn codex_test_uses_fake_child_key_and_monitored_gateway() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = stream.read(&mut buffer).unwrap();
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let request_text = String::from_utf8(request).unwrap();
            let lower = request_text.to_ascii_lowercase();
            assert!(lower.contains("authorization: bearer test-key-target-not-real"));
            assert!(!lower.contains("test-key-codex-preflight-not-real"));
            assert!(!lower.contains("test-key-codex-gateway-not-real"));
            let body = concat!(
                "event: response.created\n",
                "data: {\"type\":\"response.created\",\"response\":{\"id\":\"r1\"}}\n\n",
                "event: response.completed\n",
                "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"r1\"}}\n\n"
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let directory = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap();
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        write_provider_state(&paths, &format!("http://{address}/v1"));
        let settings = SettingsService::new(paths.clone());
        settings.bootstrap().unwrap();
        let backend = Arc::new(ScriptedCodexBackend::default());
        let runtime = Arc::new(FakeCodexRuntime {
            backend: Arc::clone(&backend),
            supported: true,
            managed_requirements: None,
        });
        let service = ProviderAvailabilityService::with_codex_runtime(
            ProviderService::new(paths.clone(), "0.1.0"),
            settings,
            "0.1.0",
            runtime,
        );
        let before_config = fs::read(&paths.config_file).unwrap();
        let result = service
            .test_codex("provider-a", Uuid::new_v4(), false)
            .await
            .unwrap();

        assert_eq!(
            result.status,
            ProviderTestStatus::Passed,
            "unexpected Codex result code {}; backend requests {}",
            result.code,
            backend.requests.load(Ordering::SeqCst)
        );
        assert_eq!(result.code, "CODEX_COMPATIBILITY_PASSED");
        assert_eq!(result.codex_version.as_deref(), Some("0.144.4"));
        assert_eq!(fs::read(&paths.config_file).unwrap(), before_config);
        assert_eq!(backend.requests.load(Ordering::SeqCst), 2);
        upstream.join().unwrap();
    }

    #[tokio::test]
    async fn codex_version_and_managed_gates_fail_before_provider_contact() {
        let directory = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap();
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        write_provider_state(&paths, "http://127.0.0.1:9/v1");
        let settings = SettingsService::new(paths.clone());
        settings.bootstrap().unwrap();
        let unsupported_backend = Arc::new(ScriptedCodexBackend::default());
        let unsupported_runtime = Arc::new(FakeCodexRuntime {
            backend: Arc::clone(&unsupported_backend),
            supported: false,
            managed_requirements: None,
        });
        let unsupported_service = ProviderAvailabilityService::with_codex_runtime(
            ProviderService::new(paths.clone(), "0.1.0"),
            settings.clone(),
            "0.1.0",
            unsupported_runtime,
        );
        let unsupported = unsupported_service
            .test_codex("provider-a", Uuid::new_v4(), false)
            .await
            .unwrap();
        assert_eq!(unsupported.status, ProviderTestStatus::Unsupported);
        assert_eq!(unsupported.code, "CODEX_VERSION_UNSUPPORTED");
        assert_eq!(unsupported_backend.requests.load(Ordering::SeqCst), 0);

        let requirements = directory.path().join("requirements.toml");
        fs::write(&requirements, "[features]\nhooks = true\n").unwrap();
        let managed_backend = Arc::new(ScriptedCodexBackend::default());
        let managed_runtime = Arc::new(FakeCodexRuntime {
            backend: Arc::clone(&managed_backend),
            supported: true,
            managed_requirements: Some(requirements),
        });
        let managed_service = ProviderAvailabilityService::with_codex_runtime(
            ProviderService::new(paths, "0.1.0"),
            settings,
            "0.1.0",
            managed_runtime,
        );
        let managed = managed_service
            .test_codex("provider-a", Uuid::new_v4(), false)
            .await
            .unwrap();
        assert_eq!(managed.status, ProviderTestStatus::Unsupported);
        assert_eq!(managed.code, "CODEX_MANAGED_CONFIG_UNSUPPORTED");
        assert_eq!(managed_backend.requests.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn codex_cli_missing_is_distinguished_from_unsupported_version() {
        let directory = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap();
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        write_provider_state(&paths, "http://127.0.0.1:9/v1");
        let settings = SettingsService::new(paths.clone());
        settings.bootstrap().unwrap();
        let backend = Arc::new(ScriptedCodexBackend::default());
        let runtime = Arc::new(MissingCodexRuntime {
            backend: Arc::clone(&backend),
        });
        let service = ProviderAvailabilityService::with_codex_runtime(
            ProviderService::new(paths, "0.1.0"),
            settings,
            "0.1.0",
            runtime,
        );

        let result = service
            .test_codex("provider-a", Uuid::new_v4(), false)
            .await
            .unwrap();

        assert_eq!(result.status, ProviderTestStatus::Unsupported);
        assert_eq!(result.code, "CODEX_CLI_MISSING");
        assert_eq!(backend.requests.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn monitored_gateway_turns_upstream_tool_calls_into_safe_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let upstream = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let body = "event: response.output_item.added\ndata: {\"type\":\"response.output_item.added\",\"item\":{\"type\":\"function_call\"}}\n\n";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let directory = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap();
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        write_provider_state(&paths, &format!("http://{address}/v1"));
        let settings = SettingsService::new(paths.clone());
        settings.bootstrap().unwrap();
        let backend = Arc::new(ScriptedCodexBackend::default());
        let runtime = Arc::new(FakeCodexRuntime {
            backend: Arc::clone(&backend),
            supported: true,
            managed_requirements: None,
        });
        let service = ProviderAvailabilityService::with_codex_runtime(
            ProviderService::new(paths, "0.1.0"),
            settings,
            "0.1.0",
            runtime,
        );

        let result = service
            .test_codex("provider-a", Uuid::new_v4(), false)
            .await
            .unwrap();

        assert_eq!(
            result.code,
            "CODEX_TOOL_CALL_BLOCKED",
            "unexpected status {:?}; backend requests {}",
            result.status,
            backend.requests.load(Ordering::SeqCst)
        );
        assert_eq!(result.status, ProviderTestStatus::Failed);
        upstream.join().unwrap();
    }
}
