use super::{UserEnvironment, UserEnvironmentTransaction, UserValue, unavailable};
use crate::error::AppError;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::process::Command;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ENVVAR_NOT_FOUND, ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE, SetLastError,
};
use windows_sys::Win32::Storage::FileSystem::{
    CommitTransaction, CreateTransaction, RollbackTransaction,
};
use windows_sys::Win32::System::Environment::GetEnvironmentVariableW;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ,
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegOpenKeyTransactedW, RegQueryValueExW,
    RegSetValueExW,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SMTO_BLOCK, SendMessageTimeoutW, WM_SETTINGCHANGE,
};
use zeroize::Zeroizing;

const USER_SUBKEY: &str = "Environment";
const SYSTEM_SUBKEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
const MAX_VALUE_BYTES: u32 = 65_536;

pub(super) struct WindowsUserEnvironment;

pub(super) fn open_settings() -> bool {
    let mut directory = [0; 260];
    let length =
        unsafe { GetSystemDirectoryW(directory.as_mut_ptr(), directory.len() as u32) } as usize;
    if length == 0 || length >= directory.len() {
        return false;
    }
    let directory = PathBuf::from(OsString::from_wide(&directory[..length]));
    let mut applet = directory.join("sysdm.cpl").into_os_string();
    applet.push(",EditEnvironmentVariables");
    Command::new(directory.join("rundll32.exe"))
        .arg(applet)
        .spawn()
        .is_ok()
}

impl UserEnvironment for WindowsUserEnvironment {
    fn user_value(&self, name: &str) -> Result<UserValue, AppError> {
        open_key(HKEY_CURRENT_USER, &wide(USER_SUBKEY))?
            .ok_or_else(unavailable)?
            .read(name)
    }

    fn system_has_value(&self, name: &str) -> Result<bool, AppError> {
        let key = open_key(HKEY_LOCAL_MACHINE, &wide(SYSTEM_SUBKEY))?.ok_or_else(unavailable)?;
        let mut length = 0;
        let status = unsafe {
            RegQueryValueExW(
                key.0,
                wide(name).as_ptr(),
                null(),
                null_mut(),
                null_mut(),
                &mut length,
            )
        };
        match status {
            ERROR_SUCCESS => Ok(true),
            ERROR_FILE_NOT_FOUND => Ok(false),
            _ => Err(unavailable()),
        }
    }

    fn process_has_value(&self, name: &str) -> Result<bool, AppError> {
        unsafe { SetLastError(ERROR_SUCCESS) };
        let length = unsafe { GetEnvironmentVariableW(wide(name).as_ptr(), null_mut(), 0) };
        if length != 0 {
            return Ok(length > 1);
        }
        match unsafe { GetLastError() } {
            ERROR_ENVVAR_NOT_FOUND | ERROR_SUCCESS => Ok(false),
            _ => Err(unavailable()),
        }
    }

    fn begin(&self) -> Result<Box<dyn UserEnvironmentTransaction + '_>, AppError> {
        open_transaction(HKEY_CURRENT_USER, &wide(USER_SUBKEY))
    }

    fn notify_changed(&self) -> bool {
        let environment = wide(USER_SUBKEY);
        let mut result = 0;
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                environment.as_ptr() as isize,
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                250,
                &mut result,
            ) != 0
        }
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

struct RegistryKey(HKEY);

impl RegistryKey {
    fn read(&self, name: &str) -> Result<UserValue, AppError> {
        let name = wide(name);
        let mut value_type = 0;
        let mut length = 0;
        let status = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                null(),
                &mut value_type,
                null_mut(),
                &mut length,
            )
        };
        match status {
            ERROR_FILE_NOT_FOUND => return Ok(UserValue::Missing),
            ERROR_SUCCESS => {}
            _ => return Err(unavailable()),
        }
        if value_type != REG_SZ || !(2..=MAX_VALUE_BYTES).contains(&length) {
            return Ok(UserValue::Unsupported);
        }
        let mut bytes = Zeroizing::new(vec![0; length as usize]);
        let status = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                null(),
                &mut value_type,
                bytes.as_mut_ptr(),
                &mut length,
            )
        };
        if status != ERROR_SUCCESS || length as usize > bytes.len() {
            return Err(unavailable());
        }
        bytes.truncate(length as usize);
        if value_type != REG_SZ || !valid_wide_string(&bytes) {
            return Ok(UserValue::Unsupported);
        }
        Ok(UserValue::Text(bytes))
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

fn valid_wide_string(bytes: &[u8]) -> bool {
    let Some(text) = bytes.strip_suffix(&[0, 0]) else {
        return false;
    };
    let (units, remainder) = text.as_chunks::<2>();
    remainder.is_empty()
        && char::decode_utf16(units.iter().copied().map(u16::from_le_bytes))
            .all(|character| character.is_ok_and(|character| character != '\0'))
}

fn open_key(root: HKEY, subkey: &[u16]) -> Result<Option<RegistryKey>, AppError> {
    let mut key = null_mut();
    let status = unsafe { RegOpenKeyExW(root, subkey.as_ptr(), 0, KEY_QUERY_VALUE, &mut key) };
    match status {
        ERROR_SUCCESS => Ok(Some(RegistryKey(key))),
        ERROR_FILE_NOT_FOUND => Ok(None),
        _ => Err(unavailable()),
    }
}

struct TransactionHandle(HANDLE);

impl Drop for TransactionHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

struct RegistryTransaction {
    key: RegistryKey,
    transaction: TransactionHandle,
}

fn open_transaction(
    root: HKEY,
    subkey: &[u16],
) -> Result<Box<dyn UserEnvironmentTransaction>, AppError> {
    let transaction = unsafe { CreateTransaction(null_mut(), null_mut(), 0, 0, 0, 30_000, null()) };
    if transaction == INVALID_HANDLE_VALUE {
        return Err(unavailable());
    }
    let transaction = TransactionHandle(transaction);
    let mut key = null_mut();
    let status = unsafe {
        RegOpenKeyTransactedW(
            root,
            subkey.as_ptr(),
            0,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            &mut key,
            transaction.0,
            null(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(unavailable());
    }
    Ok(Box::new(RegistryTransaction {
        key: RegistryKey(key),
        transaction,
    }))
}

impl UserEnvironmentTransaction for RegistryTransaction {
    fn read(&self, name: &str) -> Result<UserValue, AppError> {
        self.key.read(name)
    }

    fn stage(&mut self, name: &str, value: &UserValue) -> Result<(), AppError> {
        let name = wide(name);
        let status = match value {
            UserValue::Text(bytes) => unsafe {
                RegSetValueExW(
                    self.key.0,
                    name.as_ptr(),
                    0,
                    REG_SZ,
                    bytes.as_ptr(),
                    bytes.len() as u32,
                )
            },
            UserValue::Missing => {
                let status = unsafe { RegDeleteValueW(self.key.0, name.as_ptr()) };
                if status == ERROR_FILE_NOT_FOUND {
                    return Ok(());
                }
                status
            }
            UserValue::Unsupported => return Err(unavailable()),
        };
        if status != ERROR_SUCCESS {
            return Err(unavailable());
        }
        Ok(())
    }

    fn commit(self: Box<Self>) -> Result<(), AppError> {
        if unsafe { CommitTransaction(self.transaction.0) } == 0 {
            return Err(unavailable());
        }
        Ok(())
    }

    fn roll_back(self: Box<Self>) -> Result<(), AppError> {
        if unsafe { RollbackTransaction(self.transaction.0) } == 0 {
            return Err(unavailable());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use windows_sys::Win32::System::Registry::{
        REG_OPTION_NON_VOLATILE, RegCreateKeyExW, RegDeleteKeyW,
    };

    struct DisposableKey(Vec<u16>);

    impl DisposableKey {
        fn create() -> Self {
            let subkey = wide(&format!(
                r"Software\CodexRelay.ProfileEnvTest.{}",
                Uuid::new_v4()
            ));
            let mut handle = null_mut();
            let mut disposition = 0;
            let status = unsafe {
                RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    subkey.as_ptr(),
                    0,
                    null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_QUERY_VALUE | KEY_SET_VALUE,
                    null(),
                    &mut handle,
                    &mut disposition,
                )
            };
            assert_eq!(status, ERROR_SUCCESS);
            unsafe { RegCloseKey(handle) };
            assert_eq!(
                disposition,
                windows_sys::Win32::System::Registry::REG_CREATED_NEW_KEY
            );
            Self(subkey)
        }

        fn assert_exists(&self, name: &str, expected: bool) {
            let mut handle = null_mut();
            let status = unsafe {
                RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    self.0.as_ptr(),
                    0,
                    KEY_QUERY_VALUE,
                    &mut handle,
                )
            };
            assert_eq!(status, ERROR_SUCCESS);
            let mut length = 0;
            let status = unsafe {
                RegQueryValueExW(
                    handle,
                    wide(name).as_ptr(),
                    null(),
                    null_mut(),
                    null_mut(),
                    &mut length,
                )
            };
            unsafe { RegCloseKey(handle) };
            assert_eq!(
                status,
                if expected {
                    ERROR_SUCCESS
                } else {
                    ERROR_FILE_NOT_FOUND
                }
            );
        }
    }

    impl Drop for DisposableKey {
        fn drop(&mut self) {
            let status = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, self.0.as_ptr()) };
            if !std::thread::panicking() {
                assert_eq!(status, ERROR_SUCCESS);
            }
        }
    }

    #[test]
    fn native_registry_transaction_commits_or_rolls_back_only_an_isolated_test_key() {
        let key = DisposableKey::create();
        let variable = "VENDOR_TEST_API_KEY";
        let original = UserValue::text("test-key-native-original-not-real");
        let replacement = UserValue::text("test-key-native-replacement-not-real");

        let mut create = open_transaction(HKEY_CURRENT_USER, &key.0).unwrap();
        assert!(create.read(variable).unwrap() == UserValue::Missing);
        create.stage(variable, &original).unwrap();
        assert!(create.read(variable).unwrap() == original);
        create.roll_back().unwrap();
        key.assert_exists(variable, false);

        let mut create = open_transaction(HKEY_CURRENT_USER, &key.0).unwrap();
        create.stage(variable, &original).unwrap();
        let committed = open_key(HKEY_CURRENT_USER, &key.0).unwrap().unwrap();
        assert!(committed.read(variable).unwrap() == UserValue::Missing);
        create.commit().unwrap();
        key.assert_exists(variable, true);
        assert!(committed.read(variable).unwrap() == original);
        drop(committed);

        let mut replace = open_transaction(HKEY_CURRENT_USER, &key.0).unwrap();
        assert!(replace.read("vendor_test_api_key").unwrap() == original);
        replace.stage(variable, &replacement).unwrap();
        assert!(
            open_key(HKEY_CURRENT_USER, &key.0)
                .unwrap()
                .unwrap()
                .read(variable)
                .unwrap()
                == original
        );
        drop(replace);
        let verify = open_transaction(HKEY_CURRENT_USER, &key.0).unwrap();
        assert!(verify.read(variable).unwrap() == original);
        verify.roll_back().unwrap();

        let mut remove = open_transaction(HKEY_CURRENT_USER, &key.0).unwrap();
        remove.stage(variable, &UserValue::Missing).unwrap();
        assert!(remove.read(variable).unwrap() == UserValue::Missing);
        remove.commit().unwrap();
        key.assert_exists(variable, false);
    }

    #[test]
    fn native_registry_transaction_does_not_override_nontransacted_write() {
        let key = DisposableKey::create();
        let variable = "VENDOR_TEST_API_KEY";
        let mut replace = open_transaction(HKEY_CURRENT_USER, &key.0).unwrap();
        replace
            .stage(
                variable,
                &UserValue::text("test-key-native-staged-not-real"),
            )
            .unwrap();
        let mut outside = null_mut();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                key.0.as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut outside,
            )
        };
        assert_eq!(status, ERROR_SUCCESS);
        let outside = RegistryKey(outside);
        let UserValue::Text(competing_value) = UserValue::text("test-key-native-other-not-real")
        else {
            unreachable!()
        };
        let status = unsafe {
            RegSetValueExW(
                outside.0,
                wide(variable).as_ptr(),
                0,
                REG_SZ,
                competing_value.as_ptr(),
                competing_value.len() as u32,
            )
        };
        assert_eq!(status, ERROR_SUCCESS);
        drop(outside);
        assert!(replace.commit().is_err());
        let committed = open_key(HKEY_CURRENT_USER, &key.0).unwrap().unwrap();
        assert!(committed.read(variable).unwrap() == UserValue::Text(competing_value));
    }
}
