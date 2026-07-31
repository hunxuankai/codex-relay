use codex_relay_release_console_lib::infrastructure::process::filter_release_environment;
use std::ffi::OsString;

#[test]
fn release_environment_keeps_toolchain_paths_but_drops_credentials_and_codex_paths() {
    let filtered = filter_release_environment([
        (OsString::from("Path"), OsString::from(r"C:\safe-bin")),
        (OsString::from("SystemRoot"), OsString::from(r"C:\Windows")),
        (
            OsString::from("USERPROFILE"),
            OsString::from(r"C:\Users\maintainer"),
        ),
        (
            OsString::from("APPDATA"),
            OsString::from(r"C:\Users\maintainer\AppData\Roaming"),
        ),
        (
            OsString::from("LOCALAPPDATA"),
            OsString::from(r"C:\Users\maintainer\AppData\Local"),
        ),
        (OsString::from("CARGO_HOME"), OsString::from(r"D:\Cargo")),
        (OsString::from("RUSTUP_HOME"), OsString::from(r"D:\Rustup")),
        (
            OsString::from("HTTPS_PROXY"),
            OsString::from("http://127.0.0.1:7897"),
        ),
        (
            OsString::from("GH_TOKEN"),
            OsString::from("github_pat_test-release-token-not-real"),
        ),
        (
            OsString::from("GITHUB_TOKEN"),
            OsString::from("ghp_test-release-token-not-real"),
        ),
        (
            OsString::from("TAURI_SIGNING_PRIVATE_KEY"),
            OsString::from("untrusted-comment-test-not-real"),
        ),
        (
            OsString::from("TAURI_SIGNING_PRIVATE_KEY_PASSWORD"),
            OsString::from("test-password-not-real"),
        ),
        (
            OsString::from("OPENAI_API_KEY"),
            OsString::from("test-key-release-not-real"),
        ),
        (
            OsString::from("CODEX_HOME"),
            OsString::from(r"C:\Users\maintainer\.codex"),
        ),
        (
            OsString::from("CODEX_RELAY_APP_DATA_DIR"),
            OsString::from(r"C:\Users\maintainer\AppData\Local\CodexRelay"),
        ),
    ]);
    let names = filtered
        .iter()
        .map(|(name, _)| name.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let debug = format!("{filtered:?}");

    assert_eq!(
        names,
        vec![
            "PATH",
            "SYSTEMROOT",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "CARGO_HOME",
            "RUSTUP_HOME",
            "HTTPS_PROXY",
        ]
    );
    assert!(!debug.contains("test-release-token-not-real"));
    assert!(!debug.contains("untrusted-comment-test-not-real"));
    assert!(!debug.contains("test-password-not-real"));
    assert!(!debug.contains("test-key-release-not-real"));
    assert!(!debug.contains(r"\.codex"));
    assert!(!debug.contains("CodexRelay"));
}
