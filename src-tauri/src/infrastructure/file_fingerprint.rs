use crate::error::AppError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFingerprint {
    pub exists: bool,
    pub len: u64,
    pub modified_unix_millis: Option<u64>,
    pub sha256: Option<String>,
}

impl FileFingerprint {
    pub fn missing() -> Self {
        Self {
            exists: false,
            len: 0,
            modified_unix_millis: None,
            sha256: None,
        }
    }

    pub fn from_path(path: &Path) -> Result<Self, AppError> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::missing()),
            Err(error) => return Err(AppError::from(error)),
        };

        let bytes = fs::read(path).map_err(AppError::from)?;
        let modified_unix_millis = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64);
        let digest = Sha256::digest(&bytes);
        let sha256 = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        Ok(Self {
            exists: true,
            len: metadata.len(),
            modified_unix_millis,
            sha256: Some(sha256),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSetFingerprint {
    pub config: FileFingerprint,
    pub auth: FileFingerprint,
    pub providers: FileFingerprint,
}

impl FileSetFingerprint {
    pub fn from_paths(config: &Path, auth: &Path, providers: &Path) -> Result<Self, AppError> {
        Ok(Self {
            config: FileFingerprint::from_path(config)?,
            auth: FileFingerprint::from_path(auth)?,
            providers: FileFingerprint::from_path(providers)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn missing_file_has_stable_missing_fingerprint() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("missing.toml");

        let fingerprint = FileFingerprint::from_path(&path).unwrap();

        assert!(!fingerprint.exists);
        assert_eq!(fingerprint.len, 0);
        assert!(fingerprint.sha256.is_none());
    }

    #[test]
    fn content_change_changes_sha256() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, "model = \"a\"\n").unwrap();
        let before = FileFingerprint::from_path(&path).unwrap();

        fs::write(&path, "model = \"b\"\n").unwrap();
        let after = FileFingerprint::from_path(&path).unwrap();

        assert!(before.exists);
        assert_ne!(before.sha256, after.sha256);
    }

    #[test]
    fn file_set_captures_all_configuration_files() {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("config.toml");
        let auth = directory.path().join("auth.json");
        let providers = directory.path().join("providers.json");
        fs::write(&config, "model_provider = \"provider-a\"\n").unwrap();

        let fingerprints = FileSetFingerprint::from_paths(&config, &auth, &providers).unwrap();

        assert!(fingerprints.config.exists);
        assert!(!fingerprints.auth.exists);
        assert!(!fingerprints.providers.exists);
    }
}
