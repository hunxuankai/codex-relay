use crate::error::AppError;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

trait PostReplaceHook {
    fn after_replace(&self, path: &Path) -> Result<(), AppError>;
}

struct NoopPostReplaceHook;

impl PostReplaceHook for NoopPostReplaceHook {
    fn after_replace(&self, _path: &Path) -> Result<(), AppError> {
        Ok(())
    }
}

pub fn atomic_write<F>(path: &Path, bytes: &[u8], validator: F) -> Result<(), AppError>
where
    F: Fn(&[u8]) -> Result<(), AppError>,
{
    atomic_write_with_hook(path, bytes, validator, &NoopPostReplaceHook)
}

fn atomic_write_with_hook<F, H>(
    path: &Path,
    bytes: &[u8],
    validator: F,
    hook: &H,
) -> Result<(), AppError>
where
    F: Fn(&[u8]) -> Result<(), AppError>,
    H: PostReplaceHook,
{
    validator(bytes)?;

    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "INVALID_TARGET_PATH",
            "配置文件路径无效。",
            format!("target has no parent: {}", path.display()),
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AppError::new(
                "INVALID_TARGET_PATH",
                "配置文件路径无效。",
                format!("target has no UTF-8 file name: {}", path.display()),
            )
        })?;
    let operation_id = Uuid::new_v4();
    let temporary_path = parent.join(format!(".{file_name}.{operation_id}.tmp"));
    let backup_path = parent.join(format!(".{file_name}.{operation_id}.previous"));

    write_and_sync(&temporary_path, bytes)?;
    let temporary_bytes = fs::read(&temporary_path).map_err(AppError::from)?;
    if let Err(error) = validator(&temporary_bytes) {
        remove_if_exists(&temporary_path);
        return Err(error);
    }

    let original_existed = path.exists();
    if original_existed && let Err(error) = fs::rename(path, &backup_path) {
        remove_if_exists(&temporary_path);
        return Err(AppError::new(
            "TARGET_BACKUP_FAILED",
            "无法准备配置文件替换。",
            error.to_string(),
        ));
    }

    if let Err(error) = fs::rename(&temporary_path, path) {
        let restore_result = restore_previous(path, &backup_path, original_existed);
        remove_if_exists(&temporary_path);
        return match restore_result {
            Ok(()) => Err(AppError::new(
                "TARGET_REPLACE_FAILED",
                "无法替换配置文件。",
                error.to_string(),
            )),
            Err(restore_error) => Err(restore_error),
        };
    }

    let verification = hook
        .after_replace(path)
        .and_then(|()| fs::read(path).map_err(AppError::from))
        .and_then(|written| validator(&written));

    if let Err(error) = verification {
        return match restore_previous(path, &backup_path, original_existed) {
            Ok(()) => Err(error),
            Err(restore_error) => Err(restore_error),
        };
    }

    remove_if_exists(&backup_path);
    Ok(())
}

fn write_and_sync(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(AppError::from)?;
    file.write_all(bytes).map_err(AppError::from)?;
    file.flush().map_err(AppError::from)?;
    file.sync_all().map_err(AppError::from)?;
    Ok(())
}

fn restore_previous(
    path: &Path,
    backup_path: &Path,
    original_existed: bool,
) -> Result<(), AppError> {
    remove_if_exists(path);
    if original_existed {
        fs::rename(backup_path, path).map_err(|error| {
            AppError::new(
                "ATOMIC_RESTORE_FAILED",
                "配置写入失败，并且无法恢复原文件。",
                error.to_string(),
            )
        })?;
    } else {
        remove_if_exists(backup_path);
    }
    Ok(())
}

fn remove_if_exists(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use std::fs;

    fn utf8_validator(bytes: &[u8]) -> Result<(), AppError> {
        std::str::from_utf8(bytes).map(|_| ()).map_err(|error| {
            AppError::new("INVALID_UTF8", "文件不是有效的 UTF-8。", error.to_string())
        })
    }

    #[test]
    fn atomic_write_preserves_exact_bytes_and_newline() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("auth.json");

        atomic_write(
            &path,
            b"{\n  \"OPENAI_API_KEY\": \"test-key-a-not-real\"\n}\n",
            utf8_validator,
        )
        .unwrap();

        assert_eq!(
            fs::read(&path).unwrap(),
            b"{\n  \"OPENAI_API_KEY\": \"test-key-a-not-real\"\n}\n"
        );
    }

    #[test]
    fn validation_failure_leaves_existing_file_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, b"original\n").unwrap();

        let result = atomic_write(&path, b"replacement\n", |_| {
            Err(AppError::new(
                "VALIDATION_FAILED",
                "临时文件验证失败。",
                "injected validation failure",
            ))
        });

        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original\n");
    }

    struct CorruptAfterReplace;

    impl PostReplaceHook for CorruptAfterReplace {
        fn after_replace(&self, path: &std::path::Path) -> Result<(), AppError> {
            fs::write(path, [0xff, 0xfe]).map_err(AppError::from)
        }
    }

    #[test]
    fn post_write_verification_failure_restores_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        fs::write(&path, b"original\n").unwrap();

        let result = atomic_write_with_hook(
            &path,
            b"replacement\n",
            utf8_validator,
            &CorruptAfterReplace,
        );

        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original\n");
    }

    #[test]
    fn failed_new_file_write_does_not_leave_target() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("auth.json");

        let result = atomic_write_with_hook(
            &path,
            b"replacement\n",
            utf8_validator,
            &CorruptAfterReplace,
        );

        assert!(result.is_err());
        assert!(!path.exists());
    }
}
