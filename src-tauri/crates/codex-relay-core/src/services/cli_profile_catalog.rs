use crate::error::AppError;
use crate::infrastructure::path_service::AppPaths;
use crate::infrastructure::safe_log::redact;
use crate::models::cli_profile::ProfileProblem;
use sha2::{Digest, Sha256};
use std::fs::{self, Metadata, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, TableLike};
use url::Url;

const PROFILE_SUFFIX: &str = ".config.toml";
const MAX_CONFIG_BYTES: u64 = 2 * 1024 * 1024;

pub(super) struct ProfileConfig {
    pub name: String,
    pub command: Option<String>,
    pub provider_id: Option<String>,
    pub host: Option<String>,
    pub variable: Option<String>,
    pub inherits_default: bool,
    pub problem: Option<ProfileProblem>,
}

pub(super) struct CliProfileCatalog {
    pub profiles: Vec<ProfileConfig>,
    pub default_variable: Option<String>,
    pub fingerprint: [u8; 32],
}

impl CliProfileCatalog {
    pub fn read(paths: &AppPaths) -> Result<Self, AppError> {
        let files = profile_files(&paths.codex_home).map_err(|error| {
            AppError::new(
                "CLI_PROFILES_READ_FAILED",
                "无法读取 Codex 配置目录中的 CLI Profiles。",
                format!("profile directory read failed: {:?}", error.kind()),
            )
        })?;
        let mut fingerprint = Sha256::new();
        let base = read_config(&paths.config_file, true, &mut fingerprint);
        let default_variable = base
            .as_ref()
            .and_then(|document| resolve_profile(String::new(), None, document, None).variable);
        let mut profiles = Vec::with_capacity(files.len());
        for (position, (name, file)) in files.iter().enumerate() {
            let document = read_config(file, false, &mut fingerprint);
            let duplicate = files
                .get(position + 1)
                .is_some_and(|(next, _)| next.eq_ignore_ascii_case(name))
                || position
                    .checked_sub(1)
                    .is_some_and(|previous| files[previous].0.eq_ignore_ascii_case(name));
            let command = (!duplicate && valid_profile_name(name)).then(|| {
                if name.starts_with('-') {
                    format!("codex --profile={name}")
                } else {
                    format!("codex -p {name}")
                }
            });
            let display_name = redact(name);
            let profile = match (&base, &document, &command) {
                (Some(base), Some(document), Some(_)) => {
                    resolve_profile(display_name, command, base, Some(document))
                }
                _ => {
                    ProfileConfig::new(display_name, command, ProfileProblem::InvalidConfiguration)
                }
            };
            profiles.push(profile);
        }
        Ok(Self {
            profiles,
            default_variable,
            fingerprint: fingerprint.finalize().into(),
        })
    }
}

impl ProfileConfig {
    fn new(name: String, command: Option<String>, problem: ProfileProblem) -> Self {
        Self {
            name,
            command,
            provider_id: None,
            host: None,
            variable: None,
            inherits_default: false,
            problem: Some(problem),
        }
    }
}

pub(super) fn valid_profile_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 240
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        && redact(name) == name
}

pub(super) fn valid_variable_name(name: &str) -> bool {
    name.len() <= 128
        && name
            .bytes()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && redact(name) == name
}

fn profile_files(root: &Path) -> io::Result<Vec<(String, PathBuf)>> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() && !is_link(&metadata) => {}
        Ok(_) => return Err(io::ErrorKind::InvalidInput.into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(name) = file_name
            .to_str()
            .and_then(|name| name.strip_suffix(PROFILE_SUFFIX))
        else {
            continue;
        };
        let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if metadata.is_file() && !is_link(&metadata) {
            files.push((name.to_owned(), entry.path()));
        }
    }
    files.sort_by_cached_key(|(name, _)| (name.to_ascii_lowercase(), name.clone()));
    Ok(files)
}

fn is_link(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

fn read_config(path: &Path, allow_missing: bool, fingerprint: &mut Sha256) -> Option<DocumentMut> {
    let name = path.file_name()?.to_str()?;
    fingerprint.update(name.len().to_le_bytes());
    fingerprint.update(name.as_bytes());
    let bytes = match read_regular_file(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound && allow_missing => {
            fingerprint.update([0]);
            return Some(DocumentMut::new());
        }
        Err(_) => {
            fingerprint.update([1]);
            return None;
        }
    };
    fingerprint.update([2]);
    fingerprint.update(Sha256::digest(&bytes));
    std::str::from_utf8(&bytes).ok()?.parse().ok()
}

fn read_regular_file(path: &Path) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || is_link(&metadata) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || is_link(&metadata) || metadata.len() > MAX_CONFIG_BYTES {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CONFIG_BYTES {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(bytes)
}

struct Layers<'document> {
    profile: Option<&'document dyn TableLike>,
    base: Option<&'document dyn TableLike>,
}

impl<'document> Layers<'document> {
    fn get(&self, key: &str) -> Option<(&'document Item, bool)> {
        self.profile
            .and_then(|table| table.get(key))
            .map(|item| (item, false))
            .or_else(|| {
                self.base
                    .and_then(|table| table.get(key))
                    .map(|item| (item, true))
            })
    }

    fn table(&self, key: &str) -> Option<Self> {
        let (item, inherited) = self.get(key)?;
        let selected = item.as_table_like()?;
        let base = if inherited {
            Some(selected)
        } else {
            self.base
                .and_then(|table| table.get(key))
                .and_then(Item::as_table_like)
        };
        Some(Self {
            profile: (!inherited).then_some(selected),
            base,
        })
    }

    fn value(&self, key: &str) -> Option<(&'document str, bool)> {
        let (item, inherited) = self.get(key)?;
        Some((item.as_str()?, inherited))
    }
}

fn resolve_profile(
    name: String,
    command: Option<String>,
    base: &DocumentMut,
    profile: Option<&DocumentMut>,
) -> ProfileConfig {
    let mut output = ProfileConfig::new(name, command, ProfileProblem::MissingProvider);
    let settings = Layers {
        profile: profile.map(|document| document.as_table() as &dyn TableLike),
        base: Some(base.as_table()),
    };
    let Some((provider_id, provider_inherited)) = settings.value("model_provider") else {
        return output;
    };
    if !valid_profile_name(provider_id) {
        return output;
    }
    output.provider_id = Some(provider_id.to_owned());
    output.inherits_default = provider_inherited
        || settings
            .get("model")
            .is_some_and(|(_, inherited)| inherited);
    if matches!(
        provider_id,
        "openai" | "ollama" | "lmstudio" | "gcp" | "amazon-bedrock" | "amazon-bedrock-runtime"
    ) {
        output.problem = Some(ProfileProblem::UnsupportedAuthentication);
        return output;
    }
    let Some(provider) = settings
        .table("model_providers")
        .and_then(|providers| providers.table(provider_id))
    else {
        return output;
    };
    if let Some((variable, inherited)) = provider.value("env_key")
        && valid_variable_name(variable)
    {
        output.inherits_default |= inherited;
        output.variable = Some(variable.to_owned());
    }
    if let Some((base_url, inherited)) = provider.value("base_url")
        && let Ok(url) = Url::parse(base_url)
        && matches!(url.scheme(), "https" | "http")
        && url.username().is_empty()
        && url.password().is_none()
        && let Some(host) = url.host_str()
        && redact(host) == host
    {
        output.inherits_default |= inherited;
        output.host = Some(host.to_owned());
    }
    output.problem = if has_alternative_auth(&provider) {
        Some(ProfileProblem::UnsupportedAuthentication)
    } else if output.host.is_none() {
        Some(ProfileProblem::InvalidConfiguration)
    } else if output.variable.is_none() {
        Some(ProfileProblem::MissingEnvironmentVariable)
    } else {
        None
    };
    output
}

fn has_alternative_auth(provider: &Layers<'_>) -> bool {
    [
        "auth",
        "gateway_oauth",
        "experimental_bearer_token",
        "aws",
        "gcp",
    ]
    .iter()
    .any(|key| provider.get(key).is_some())
        || provider
            .get("requires_openai_auth")
            .is_some_and(|(item, _)| item.as_bool() != Some(false))
        || provider
            .get("wire_api")
            .is_some_and(|(item, _)| item.as_str() != Some("responses"))
        || ["http_headers", "env_http_headers"].iter().any(|key| {
            let Some(headers) = provider.table(key) else {
                return provider.get(key).is_some();
            };
            headers
                .profile
                .into_iter()
                .chain(headers.base)
                .any(|table| {
                    table.iter().any(|(header, _)| {
                        header.eq_ignore_ascii_case("authorization")
                            || header.eq_ignore_ascii_case("proxy-authorization")
                    })
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discovers_profiles_using_layered_provider_and_env_key_without_writing_configuration() {
        let temporary = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(temporary.path().join("home"), temporary.path().join("app"))
            .unwrap();
        fs::create_dir(&paths.codex_home).unwrap();
        let base = br#"
model = "shared-model"
model_provider = "default-route"
[model_providers.default-route]
base_url = "https://default.test/v1"
env_key = "DEFAULT_ROUTE_API_KEY"
[model_providers.gateway]
base_url = "https://gateway.test/v1"
env_key = "GATEWAY_API_KEY"
"#;
        let custom = br#"
model_provider = "gateway"
[model_providers.gateway]
env_key = "SPECIFIC_TOKEN"
"#;
        let standalone = br#"
model = "other-model"
model_provider = "other"
[model_providers.other]
base_url = "https://other.test/api?filter=hidden"
env_key = "INDEPENDENT_API_KEY"
"#;
        fs::write(&paths.config_file, base).unwrap();
        let profile_file = paths.codex_home.join("agentrouter.config.toml");
        fs::write(&profile_file, custom).unwrap();
        fs::write(paths.codex_home.join("qiandao.config.toml"), standalone).unwrap();
        let catalog = CliProfileCatalog::read(&paths).unwrap();
        assert_eq!(catalog.profiles.len(), 2);
        assert_eq!(
            catalog.default_variable.as_deref(),
            Some("DEFAULT_ROUTE_API_KEY")
        );
        let inherited = &catalog.profiles[0];
        assert_eq!(inherited.name, "agentrouter");
        assert_eq!(inherited.provider_id.as_deref(), Some("gateway"));
        assert_eq!(inherited.variable.as_deref(), Some("SPECIFIC_TOKEN"));
        assert_eq!(inherited.host.as_deref(), Some("gateway.test"));
        assert_eq!(inherited.command.as_deref(), Some("codex -p agentrouter"));
        assert!(inherited.inherits_default);
        assert_eq!(inherited.problem, None);
        let independent = &catalog.profiles[1];
        assert_eq!(independent.name, "qiandao");
        assert_eq!(independent.host.as_deref(), Some("other.test"));
        assert!(!independent.inherits_default);
        assert_eq!(fs::read(profile_file).unwrap(), custom);
        assert_eq!(fs::read(paths.config_file).unwrap(), base);
        assert!(!paths.app_data_dir.exists());
    }

    #[test]
    fn isolates_invalid_and_unsupported_profiles_and_never_copies_an_untrusted_name() {
        let temporary = tempfile::tempdir().unwrap();
        let paths = AppPaths::for_test(temporary.path().join("home"), temporary.path().join("app"))
            .unwrap();
        fs::create_dir(&paths.codex_home).unwrap();
        for (name, config) in [
            ("bad", "[model_providers"),
            ("invalid name", "model_provider = 'openai'"),
            ("not-env", "model_provider = 'openai'"),
        ] {
            fs::write(paths.codex_home.join(format!("{name}.config.toml")), config).unwrap();
        }
        fs::create_dir(paths.codex_home.join("nested.config.toml")).unwrap();
        let catalog = CliProfileCatalog::read(&paths).unwrap();
        assert_eq!(catalog.profiles.len(), 3);
        assert_eq!(
            catalog.profiles[0].problem,
            Some(ProfileProblem::InvalidConfiguration)
        );
        assert!(catalog.profiles[1].command.is_none());
        assert_eq!(
            catalog.profiles[2].problem,
            Some(ProfileProblem::UnsupportedAuthentication)
        );
        assert!(catalog.profiles[2].variable.is_none());
    }
}
