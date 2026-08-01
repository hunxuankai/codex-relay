use codex_relay_release_console_lib::infrastructure::git::{GitBackend, GitProxyMode};
use codex_relay_release_console_lib::models::{ReleaseProxySettings, ReleaseProxyType};
use codex_relay_release_console_lib::services::release_network::ReleaseNetworkProfile;
use codex_relay_release_console_lib::services::release_network::{
    ConnectionProbeFailure, ConnectionProbeTarget, ReleaseConnectionProbeBackend,
    ReleaseConnectionService, SystemReleaseConnectionProbeBackend,
};
use std::ffi::OsString;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

fn environment_value(profile: &ReleaseNetworkProfile, name: &str) -> Option<String> {
    profile
        .environment()
        .iter()
        .find(|(candidate, _)| candidate.to_string_lossy().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.to_string_lossy().into_owned())
}

#[test]
fn http_and_socks5_settings_create_one_git_and_gh_proxy_profile() {
    let http = ReleaseNetworkProfile::new(
        &ReleaseProxySettings {
            enabled: true,
            proxy_type: ReleaseProxyType::Http,
            host: "proxy.example.test".into(),
            port: Some(7890),
        },
        [(OsString::from("PATH"), OsString::from(r"D:\tools"))],
    )
    .unwrap();
    assert_eq!(
        environment_value(&http, "HTTPS_PROXY").as_deref(),
        Some("http://proxy.example.test:7890")
    );
    assert_eq!(
        http.git_proxy_mode(),
        &GitProxyMode::Custom("http://proxy.example.test:7890".into())
    );

    let socks = ReleaseNetworkProfile::new(
        &ReleaseProxySettings {
            enabled: true,
            proxy_type: ReleaseProxyType::Socks5,
            host: "::1".into(),
            port: Some(1080),
        },
        [(OsString::from("PATH"), OsString::from(r"D:\tools"))],
    )
    .unwrap();
    assert_eq!(
        environment_value(&socks, "HTTP_PROXY").as_deref(),
        Some("socks5://[::1]:1080")
    );
    assert_eq!(
        socks.git_proxy_mode(),
        &GitProxyMode::Custom("socks5://[::1]:1080".into())
    );
}

#[test]
fn enabled_proxy_rejects_unsafe_hosts_and_missing_or_zero_ports() {
    for host in [
        "",
        "http://127.0.0.1",
        "user@proxy.example.test",
        "proxy.example.test/path",
        "proxy.example.test?token=value",
    ] {
        let error = ReleaseNetworkProfile::new(
            &ReleaseProxySettings {
                enabled: true,
                proxy_type: ReleaseProxyType::Http,
                host: host.into(),
                port: Some(7890),
            },
            [],
        )
        .unwrap_err();
        assert_eq!(error.code(), "RELEASE_PROXY_HOST_INVALID", "{host}");
    }

    for port in [None, Some(0)] {
        let error = ReleaseNetworkProfile::new(
            &ReleaseProxySettings {
                enabled: true,
                proxy_type: ReleaseProxyType::Http,
                host: "127.0.0.1".into(),
                port,
            },
            [],
        )
        .unwrap_err();
        assert_eq!(error.code(), "RELEASE_PROXY_PORT_INVALID");
    }
}

#[test]
fn explicit_direct_or_custom_modes_override_inherited_proxy_variables() {
    let inherited = [
        (OsString::from("PATH"), OsString::from(r"D:\tools")),
        (
            OsString::from("HTTPS_PROXY"),
            OsString::from("http://inherited.test:9000"),
        ),
        (OsString::from("NO_PROXY"), OsString::from("github.com")),
    ];
    let direct = ReleaseNetworkProfile::new(
        &ReleaseProxySettings {
            enabled: false,
            proxy_type: ReleaseProxyType::Http,
            host: "proxy.example.test".into(),
            port: Some(7890),
        },
        inherited.clone(),
    )
    .unwrap();
    assert_eq!(direct.git_proxy_mode(), &GitProxyMode::Direct);
    assert_eq!(
        environment_value(&direct, "PATH").as_deref(),
        Some(r"D:\tools")
    );
    assert_eq!(environment_value(&direct, "HTTPS_PROXY"), None);
    assert_eq!(environment_value(&direct, "NO_PROXY"), None);
    assert_eq!(
        environment_value(&direct, "GIT_TERMINAL_PROMPT").as_deref(),
        Some("0")
    );

    let custom = ReleaseNetworkProfile::new(
        &ReleaseProxySettings {
            enabled: true,
            proxy_type: ReleaseProxyType::Http,
            host: "proxy.example.test".into(),
            port: Some(7890),
        },
        inherited,
    )
    .unwrap();
    assert_eq!(
        environment_value(&custom, "HTTPS_PROXY").as_deref(),
        Some("http://proxy.example.test:7890")
    );
    assert_eq!(environment_value(&custom, "NO_PROXY"), None);
}

#[test]
fn git_invocations_override_global_proxy_without_mutating_git_configuration() {
    let direct =
        GitBackend::new_with_proxy(PathBuf::from("git.exe"), Vec::new(), GitProxyMode::Direct);
    let invocation = direct.invocation_for(
        PathBuf::from(r"D:\safe-temp\repository").as_path(),
        &["fetch", "origin", "main"],
    );
    assert_eq!(
        invocation.args,
        [
            "-c",
            "http.proxy=",
            "-c",
            "https.proxy=",
            "fetch",
            "origin",
            "main",
        ]
        .into_iter()
        .map(OsString::from)
        .collect::<Vec<_>>()
    );

    let custom = GitBackend::new_with_proxy(
        PathBuf::from("git.exe"),
        Vec::new(),
        GitProxyMode::Custom("socks5://127.0.0.1:1080".into()),
    );
    let invocation = custom.invocation_for(
        PathBuf::from(r"D:\safe-temp\repository").as_path(),
        &["push", "origin", "HEAD:refs/heads/main"],
    );
    assert_eq!(
        invocation.args[..4],
        [
            OsString::from("-c"),
            OsString::from("http.proxy=socks5://127.0.0.1:1080"),
            OsString::from("-c"),
            OsString::from("https.proxy=socks5://127.0.0.1:1080"),
        ]
    );
}

struct FixtureConnectionBackend;

impl ReleaseConnectionProbeBackend for FixtureConnectionBackend {
    fn probe<'a>(
        &'a self,
        target: ConnectionProbeTarget,
    ) -> Pin<Box<dyn Future<Output = Result<(), ConnectionProbeFailure>> + Send + 'a>> {
        Box::pin(async move {
            match target {
                ConnectionProbeTarget::Git => Ok(()),
                ConnectionProbeTarget::Github => Err(ConnectionProbeFailure::Timeout),
            }
        })
    }
}

#[test]
fn connection_service_preserves_independent_git_and_github_results() {
    let result = tauri::async_runtime::block_on(
        ReleaseConnectionService::new().test(&FixtureConnectionBackend),
    );

    assert!(result.git.success);
    assert_eq!(result.git.code, None);
    assert_eq!(result.git.message, "Git 远端连接正常。");
    assert!(!result.github.success);
    assert_eq!(
        result.github.code.as_deref(),
        Some("GITHUB_PROCESS_TIMEOUT")
    );
    assert_eq!(result.github.message, "GitHub API 连接超时。");
}

#[test]
fn system_connection_backend_reports_missing_git_and_gh_independently() {
    let profile = ReleaseNetworkProfile::new(
        &ReleaseProxySettings {
            enabled: false,
            proxy_type: ReleaseProxyType::Http,
            host: String::new(),
            port: None,
        },
        [],
    )
    .unwrap();
    let backend =
        SystemReleaseConnectionProbeBackend::new(None, None, &profile, std::env::temp_dir());

    let result = tauri::async_runtime::block_on(ReleaseConnectionService::new().test(&backend));

    assert_eq!(result.git.code.as_deref(), Some("GIT_TOOL_MISSING"));
    assert_eq!(result.github.code.as_deref(), Some("GITHUB_TOOL_MISSING"));
}
