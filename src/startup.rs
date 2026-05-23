use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StartupError {
    #[cfg(target_os = "windows")]
    #[error("failed to access Windows startup setting: error {0}")]
    Registry(u32),
    #[cfg(target_os = "windows")]
    #[error("failed to determine current executable: {0}")]
    CurrentExe(std::io::Error),
    #[cfg(not(target_os = "windows"))]
    #[error("startup toggle is only supported on Windows")]
    Unsupported,
}

#[cfg(target_os = "windows")]
pub fn is_enabled() -> Result<bool, StartupError> {
    windows_startup::is_enabled()
}

#[cfg(not(target_os = "windows"))]
pub fn is_enabled() -> Result<bool, StartupError> {
    Err(StartupError::Unsupported)
}

#[cfg(target_os = "windows")]
pub fn set_enabled(enabled: bool) -> Result<(), StartupError> {
    windows_startup::set_enabled(enabled)
}

#[cfg(not(target_os = "windows"))]
pub fn set_enabled(_enabled: bool) -> Result<(), StartupError> {
    Err(StartupError::Unsupported)
}

pub fn startup_command(exe: &Path) -> String {
    format!("\"{}\" tray", exe.display())
}

#[cfg(target_os = "windows")]
mod windows_startup {
    use std::{env, ptr::null_mut};

    use super::{StartupError, startup_command};
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ, RegCloseKey, RegCreateKeyW,
            RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
        },
    };

    const APP_NAME: &str = "shotlite";
    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    pub fn is_enabled() -> Result<bool, StartupError> {
        let key = open_key(KEY_READ)?;
        let value_name = wide(APP_NAME);
        let result = unsafe {
            RegQueryValueExW(
                key.0,
                value_name.as_ptr(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };

        match result {
            ERROR_SUCCESS => Ok(true),
            ERROR_FILE_NOT_FOUND => Ok(false),
            error => Err(StartupError::Registry(error)),
        }
    }

    pub fn set_enabled(enabled: bool) -> Result<(), StartupError> {
        if enabled {
            let key = create_key()?;
            let value_name = wide(APP_NAME);
            let exe = env::current_exe().map_err(StartupError::CurrentExe)?;
            let command = wide(&startup_command(&exe));
            let bytes = command.len() * std::mem::size_of::<u16>();
            let result = unsafe {
                RegSetValueExW(
                    key.0,
                    value_name.as_ptr(),
                    0,
                    REG_SZ,
                    command.as_ptr().cast(),
                    bytes as u32,
                )
            };
            if result != ERROR_SUCCESS {
                return Err(StartupError::Registry(result));
            }
        } else {
            let key = open_key(KEY_WRITE)?;
            let value_name = wide(APP_NAME);
            let result = unsafe { RegDeleteValueW(key.0, value_name.as_ptr()) };
            if result != ERROR_SUCCESS && result != ERROR_FILE_NOT_FOUND {
                return Err(StartupError::Registry(result));
            }
        }

        Ok(())
    }

    fn open_key(access: u32) -> Result<RegKey, StartupError> {
        let mut key = null_mut();
        let run_key = wide(RUN_KEY);
        let result =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, run_key.as_ptr(), 0, access, &mut key) };
        if result != ERROR_SUCCESS {
            return Err(StartupError::Registry(result));
        }
        Ok(RegKey(key))
    }

    fn create_key() -> Result<RegKey, StartupError> {
        let mut key = null_mut();
        let run_key = wide(RUN_KEY);
        let result = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, run_key.as_ptr(), &mut key) };
        if result != ERROR_SUCCESS {
            return Err(StartupError::Registry(result));
        }
        Ok(RegKey(key))
    }

    struct RegKey(HKEY);

    impl Drop for RegKey {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_command_runs_tray_mode() {
        assert_eq!(
            startup_command(Path::new(r"C:\tools\shotlite.exe")),
            r#""C:\tools\shotlite.exe" tray"#
        );
    }
}
