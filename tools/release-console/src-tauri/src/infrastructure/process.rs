use std::ffi::OsString;

pub use codex_relay_core::infrastructure::codex_process::{
    ProcessError, ProcessEventSink, ProcessInvocation, ProcessOutput, ProcessStream,
    SafeProcessRunner,
};

const RELEASE_ENV_ALLOWLIST: &[&str] = &[
    "SYSTEMROOT",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    "PATH",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "NPM_CONFIG_CACHE",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "NODE_EXTRA_CA_CERTS",
];

pub fn filter_release_environment<I>(entries: I) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    let mut filtered = Vec::new();
    for (name, value) in entries {
        let normalized = name.to_string_lossy().to_ascii_uppercase();
        if RELEASE_ENV_ALLOWLIST.contains(&normalized.as_str())
            && !filtered.iter().any(|(existing, _): &(OsString, OsString)| {
                existing.to_string_lossy().eq_ignore_ascii_case(&normalized)
            })
        {
            filtered.push((OsString::from(normalized), value));
        }
    }
    filtered
}
