use super::TransactionService;
use crate::error::AppError;
use crate::infrastructure::atomic_file::atomic_write;
use crate::infrastructure::user_environment::{UserEnvironment, UserValue};
use crate::services::cli_profile_catalog::valid_variable_name;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingWrite {
    operation_id: Uuid,
}

impl TransactionService {
    pub(crate) async fn write_profile_environment(
        &self,
        environment: &dyn UserEnvironment,
        name: &str,
        expected: [u8; 32],
        replacement: &UserValue,
        confirm_target: impl Fn() -> Result<(), AppError> + Send + Sync,
    ) -> Result<(), AppError> {
        if !valid_variable_name(name) || !matches!(replacement, UserValue::Text(_)) {
            return Err(profile_key_conflict());
        }
        let _guard = self.write_lock.lock().await;
        confirm_target()?;
        let mut transaction = environment.begin()?;
        let original = transaction.read(name)?;
        if original == UserValue::Unsupported || original.fingerprint() != expected {
            return Err(profile_key_conflict());
        }
        let marker = self.prepare_profile_key_marker(name)?;

        let staged = transaction.stage(name, replacement).and_then(|()| {
            if transaction.read(name)? != *replacement {
                return Err(write_failed());
            }
            confirm_target()?;
            if environment.user_value(name)? != original {
                return Err(profile_key_conflict());
            }
            Ok(())
        });
        if let Err(error) = staged {
            if transaction.roll_back().is_err() {
                return Err(rollback_incomplete());
            }
            return Err(recover_after_failure(
                &marker,
                environment,
                name,
                &original,
                replacement,
                false,
                error,
            ));
        }

        let commit = transaction.commit();
        let committed = commit.is_ok();
        let result = commit.and_then(|()| {
            confirm_target()?;
            if environment.user_value(name)? != *replacement {
                return Err(write_failed());
            }
            fs::remove_file(&marker).map_err(AppError::from)
        });
        result.map_err(|error| {
            recover_after_failure(
                &marker,
                environment,
                name,
                &original,
                replacement,
                committed,
                error,
            )
        })
    }

    pub(crate) fn profile_key_recovery_pending(&self) -> bool {
        match fs::read_dir(self.profile_key_marker_dir()) {
            Ok(mut entries) => entries.next().is_some(),
            Err(error) => error.kind() != ErrorKind::NotFound,
        }
    }

    fn profile_key_marker_dir(&self) -> PathBuf {
        self.paths.app_data_dir.join("profile-env-pending")
    }

    fn prepare_profile_key_marker(&self, name: &str) -> Result<PathBuf, AppError> {
        let directory = self.profile_key_marker_dir();
        fs::create_dir_all(&directory)?;
        let marker = directory.join(format!("{}.json", name.to_ascii_uppercase()));
        let record = PendingWrite {
            operation_id: Uuid::new_v4(),
        };
        let mut bytes = serde_json::to_vec_pretty(&record)?;
        bytes.push(b'\n');
        atomic_write(&marker, &bytes, |candidate| {
            if serde_json::from_slice::<PendingWrite>(candidate)
                .ok()
                .as_ref()
                != Some(&record)
            {
                return Err(AppError::new(
                    "PROFILE_KEY_JOURNAL_FAILED",
                    "无法核对凭据事务记录，尚未修改变量。",
                    "profile environment marker verification failed",
                ));
            }
            Ok(())
        })?;
        Ok(marker)
    }
}

fn recover_after_failure(
    marker: &Path,
    environment: &dyn UserEnvironment,
    name: &str,
    original: &UserValue,
    replacement: &UserValue,
    committed: bool,
    error: AppError,
) -> AppError {
    let original_verified = environment
        .user_value(name)
        .is_ok_and(|actual| actual == *original);
    if !original_verified
        && !(committed && restore_if_unchanged(environment, name, original, replacement))
    {
        return rollback_incomplete();
    }
    if fs::remove_file(marker).is_err() {
        return rollback_incomplete();
    }
    if matches!(
        error.code(),
        "PROFILE_KEY_CONFLICT" | "PROFILE_PROVIDER_KEY_UNAVAILABLE"
    ) {
        error
    } else {
        write_failed()
    }
}

fn restore_if_unchanged(
    environment: &dyn UserEnvironment,
    name: &str,
    original: &UserValue,
    replacement: &UserValue,
) -> bool {
    let attempt = || -> Result<bool, AppError> {
        let mut rollback = environment.begin()?;
        if rollback.read(name)? != *replacement {
            return Ok(false);
        }
        rollback.stage(name, original)?;
        if rollback.read(name)? != *original {
            return Ok(false);
        }
        if environment.user_value(name)? != *replacement {
            return Ok(false);
        }
        rollback.commit()?;
        Ok(environment.user_value(name)? == *original)
    };
    attempt().unwrap_or(false)
}

pub(in crate::services) fn profile_key_conflict() -> AppError {
    AppError::new(
        "PROFILE_KEY_CONFLICT",
        "Profile 配置或环境变量已发生变化，请刷新后重新打开设置。",
        "profile configuration or user environment baseline changed",
    )
}

fn write_failed() -> AppError {
    AppError::new(
        "PROFILE_KEY_WRITE_FAILED",
        "保存未完成，已验证原用户变量状态。",
        "profile environment write failed with verified original state",
    )
}

fn rollback_incomplete() -> AppError {
    AppError::new(
        "ROLLBACK_INCOMPLETE",
        "无法确认原用户变量已恢复。请刷新并核对 Windows 用户环境变量。",
        "profile environment rollback or journal cleanup could not be verified",
    )
}
