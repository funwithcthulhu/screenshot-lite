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
        path::Path,
        ptr::{null, null_mut},
        sync::atomic::{AtomicIsize, Ordering},
    };

    use super::TrayError;
    use crate::{
        capture::{self, CaptureOutput},
        config::Config,
        interactive,
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{CreateBitmap, DeleteObject},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{
                MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey, UnregisterHotKey, VK_1, VK_2,
                VK_Q,
            },
            Shell::{
                NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO, NIM_ADD,
                NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CreateIconIndirect, CreateWindowExW, DefWindowProcW,
                DestroyIcon, DispatchMessageW, GetMessageW, HICON, ICONINFO, MSG, PostQuitMessage,
                RegisterClassW, TranslateMessage, WM_DESTROY, WM_HOTKEY, WM_USER, WNDCLASSW,
            },
        },
    };

    const ICON_SIZE: usize = 32;
    const HOTKEY_FULL: i32 = 1;
    const HOTKEY_REGION: i32 = 2;
    const HOTKEY_QUIT: i32 = 3;
    const TRAY_ID: u32 = 1;
    const TRAY_MESSAGE: u32 = WM_USER + 1;
    static TRAY_ICON: AtomicIsize = AtomicIsize::new(0);

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
                        notify_capture(hwnd, capture_full());
                    }
                    HOTKEY_REGION => {
                        notify_capture(hwnd, capture_region());
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
                    notify_capture(hwnd, capture_full());
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

    fn capture_full() -> Result<std::path::PathBuf, TrayError> {
        let config = Config::load()?;
        let result = capture::capture_full_to(CaptureOutput::Directory(config.output_dir))?;
        Ok(result.path)
    }

    fn capture_region() -> Result<std::path::PathBuf, TrayError> {
        let config = Config::load()?;
        let rect = interactive::select_region()?;
        let result =
            capture::capture_region_to(CaptureOutput::Directory(config.output_dir), Some(rect))?;
        Ok(result.path)
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
        let icon = unsafe { create_eye_icon() };
        if icon.is_null() {
            return Err(TrayError::Window);
        }

        let mut data = NOTIFYICONDATAW {
            cbSize: mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            hIcon: icon,
            ..Default::default()
        };
        write_wide_array(
            &mut data.szTip,
            "shotlite: Ctrl+Shift+1 full, Ctrl+Shift+2 region, Ctrl+Shift+Q quit",
        );
        if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } == 0 {
            unsafe { DestroyIcon(icon) };
            return Err(TrayError::Window);
        }
        TRAY_ICON.store(icon as isize, Ordering::Relaxed);
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
        let icon = TRAY_ICON.swap(0, Ordering::Relaxed) as HICON;
        if !icon.is_null() {
            unsafe { DestroyIcon(icon) };
        }
    }

    fn notify_capture(hwnd: HWND, result: Result<std::path::PathBuf, TrayError>) {
        match result {
            Ok(path) => show_balloon(hwnd, "Screenshot saved", display_path(&path), NIIF_INFO),
            Err(error) => show_balloon(hwnd, "Screenshot failed", error.to_string(), NIIF_ERROR),
        }
    }

    fn show_balloon(hwnd: HWND, title: &str, message: impl AsRef<str>, flags: u32) {
        let mut data = NOTIFYICONDATAW {
            cbSize: mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ID,
            uFlags: NIF_INFO,
            dwInfoFlags: flags,
            ..Default::default()
        };
        write_wide_array(&mut data.szInfoTitle, title);
        write_wide_array(&mut data.szInfo, message.as_ref());
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    fn display_path(path: &Path) -> String {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("screenshot saved")
            .to_owned()
    }

    unsafe fn create_eye_icon() -> HICON {
        let (pixels, mask) = eye_icon_bitmaps();
        let color = unsafe {
            CreateBitmap(
                ICON_SIZE as i32,
                ICON_SIZE as i32,
                1,
                32,
                pixels.as_ptr().cast(),
            )
        };
        let alpha_mask = unsafe {
            CreateBitmap(
                ICON_SIZE as i32,
                ICON_SIZE as i32,
                1,
                1,
                mask.as_ptr().cast(),
            )
        };
        if color.is_null() || alpha_mask.is_null() {
            if !color.is_null() {
                unsafe { DeleteObject(color) };
            }
            if !alpha_mask.is_null() {
                unsafe { DeleteObject(alpha_mask) };
            }
            return null_mut();
        }

        let info = ICONINFO {
            fIcon: 1,
            hbmColor: color,
            hbmMask: alpha_mask,
            ..Default::default()
        };
        let icon = unsafe { CreateIconIndirect(&info) };
        unsafe {
            DeleteObject(color);
            DeleteObject(alpha_mask);
        }
        icon
    }

    fn eye_icon_bitmaps() -> (Vec<u32>, Vec<u8>) {
        let mut pixels = vec![0; ICON_SIZE * ICON_SIZE];
        let mut mask = vec![0xff; ICON_SIZE * ICON_SIZE / 8];

        for y in 0..ICON_SIZE {
            for x in 0..ICON_SIZE {
                let dx = (x as f32 - 15.5) / 13.5;
                let dy = (y as f32 - 15.5) / 7.0;
                let eye = dx * dx + dy * dy;
                if eye > 1.0 {
                    continue;
                }

                clear_mask_bit(&mut mask, x, y);
                let iris_dx = x as f32 - 15.5;
                let iris_dy = y as f32 - 15.5;
                let iris = iris_dx * iris_dx + iris_dy * iris_dy;
                let color = if eye > 0.78 {
                    bgra(42, 47, 55)
                } else if iris <= 5.2 * 5.2 {
                    bgra(45, 134, 166)
                } else {
                    bgra(250, 250, 244)
                };
                pixels[y * ICON_SIZE + x] = color;

                if iris <= 2.5 * 2.5 {
                    pixels[y * ICON_SIZE + x] = bgra(16, 19, 24);
                }
                let highlight_dx = x as f32 - 13.0;
                let highlight_dy = y as f32 - 13.0;
                if highlight_dx * highlight_dx + highlight_dy * highlight_dy <= 1.8 * 1.8 {
                    pixels[y * ICON_SIZE + x] = bgra(255, 255, 255);
                }
            }
        }

        (pixels, mask)
    }

    fn clear_mask_bit(mask: &mut [u8], x: usize, y: usize) {
        let byte = y * (ICON_SIZE / 8) + (x / 8);
        let bit = 7 - (x % 8);
        mask[byte] &= !(1 << bit);
    }

    fn bgra(red: u8, green: u8, blue: u8) -> u32 {
        u32::from(blue) | (u32::from(green) << 8) | (u32::from(red) << 16)
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
