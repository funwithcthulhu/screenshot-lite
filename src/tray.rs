use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrayError {
    #[cfg(not(target_os = "windows"))]
    #[error("tray mode is only supported on Windows")]
    Unsupported,
    #[cfg(target_os = "windows")]
    #[error("failed to start tray mode")]
    Window,
    #[cfg(target_os = "windows")]
    #[error(transparent)]
    Capture(#[from] crate::capture::CaptureError),
    #[cfg(target_os = "windows")]
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    #[cfg(target_os = "windows")]
    #[error(transparent)]
    Interactive(#[from] crate::interactive::InteractiveError),
}

#[cfg(target_os = "windows")]
pub fn run() -> Result<(), TrayError> {
    windows_tray::run()
}

#[cfg(not(target_os = "windows"))]
pub fn run() -> Result<(), TrayError> {
    Err(TrayError::Unsupported)
}

#[cfg(target_os = "windows")]
mod windows_tray {
    use std::{
        mem,
        ptr::{null, null_mut},
    };

    use super::TrayError;
    use crate::{
        capture::{self, CaptureOutput},
        config::Config,
        interactive,
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{
                MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey, UnregisterHotKey, VK_1, VK_2,
                VK_Q,
            },
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
                Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DispatchMessageW,
                GetMessageW, IDI_APPLICATION, LoadIconW, MSG, PostQuitMessage, RegisterClassW,
                TranslateMessage, WM_DESTROY, WM_HOTKEY, WM_USER, WNDCLASSW,
            },
        },
    };

    const HOTKEY_FULL: i32 = 1;
    const HOTKEY_REGION: i32 = 2;
    const HOTKEY_QUIT: i32 = 3;
    const TRAY_ID: u32 = 1;
    const TRAY_MESSAGE: u32 = WM_USER + 1;

    pub fn run() -> Result<(), TrayError> {
        unsafe {
            let hinstance = GetModuleHandleW(null());
            let class_name = wide("shotlite-tray");
            let wnd_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                hInstance: hinstance,
                lpszClassName: class_name.as_ptr(),
                ..Default::default()
            };
            RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide("shotlite tray").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                null_mut(),
                null_mut(),
                hinstance,
                null(),
            );
            if hwnd.is_null() {
                return Err(TrayError::Window);
            }

            add_tray_icon(hwnd)?;
            register_hotkeys(hwnd)?;

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            delete_tray_icon(hwnd);
            UnregisterHotKey(hwnd, HOTKEY_FULL);
            UnregisterHotKey(hwnd, HOTKEY_REGION);
            UnregisterHotKey(hwnd, HOTKEY_QUIT);
        }

        Ok(())
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_HOTKEY => {
                match wparam as i32 {
                    HOTKEY_FULL => {
                        let _ = capture_full();
                    }
                    HOTKEY_REGION => {
                        let _ = capture_region();
                    }
                    HOTKEY_QUIT => {
                        unsafe { PostQuitMessage(0) };
                    }
                    _ => {}
                }
                0
            }
            TRAY_MESSAGE => {
                if lparam as u32 == 0x0203 {
                    let _ = capture_full();
                }
                0
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    fn capture_full() -> Result<(), TrayError> {
        let config = Config::load()?;
        capture::capture_full_to(CaptureOutput::Directory(config.output_dir))?;
        Ok(())
    }

    fn capture_region() -> Result<(), TrayError> {
        let config = Config::load()?;
        let rect = interactive::select_region()?;
        capture::capture_region_to(CaptureOutput::Directory(config.output_dir), Some(rect))?;
        Ok(())
    }

    unsafe fn register_hotkeys(hwnd: HWND) -> Result<(), TrayError> {
        let modifiers = MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT;
        if unsafe { RegisterHotKey(hwnd, HOTKEY_FULL, modifiers, u32::from(VK_1)) } == 0
            || unsafe { RegisterHotKey(hwnd, HOTKEY_REGION, modifiers, u32::from(VK_2)) } == 0
            || unsafe { RegisterHotKey(hwnd, HOTKEY_QUIT, modifiers, u32::from(VK_Q)) } == 0
        {
            return Err(TrayError::Window);
        }
        Ok(())
    }

    unsafe fn add_tray_icon(hwnd: HWND) -> Result<(), TrayError> {
        let mut data = NOTIFYICONDATAW {
            cbSize: mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            hIcon: unsafe { LoadIconW(null_mut(), IDI_APPLICATION) },
            ..Default::default()
        };
        write_wide_array(
            &mut data.szTip,
            "shotlite: Ctrl+Shift+1 full, Ctrl+Shift+2 region, Ctrl+Shift+Q quit",
        );
        if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
            return Err(TrayError::Window);
        }
        Ok(())
    }

    unsafe fn delete_tray_icon(hwnd: HWND) {
        let data = NOTIFYICONDATAW {
            cbSize: mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            ..Default::default()
        };
        unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    fn write_wide_array<const N: usize>(target: &mut [u16; N], value: &str) {
        for (slot, code) in target.iter_mut().zip(value.encode_utf16().chain([0])) {
            *slot = code;
        }
    }
}
