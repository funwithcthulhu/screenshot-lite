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

#[cfg(any(test, target_os = "windows"))]
const HOTKEY_FULL: i32 = 1;
#[cfg(any(test, target_os = "windows"))]
const HOTKEY_REGION: i32 = 2;
#[cfg(any(test, target_os = "windows"))]
const HOTKEY_QUIT: i32 = 3;
#[cfg(any(test, target_os = "windows"))]
const MENU_FULL: usize = 10;
#[cfg(any(test, target_os = "windows"))]
const MENU_REGION: usize = 11;
#[cfg(any(test, target_os = "windows"))]
const MENU_OPEN_LAST: usize = 12;
#[cfg(any(test, target_os = "windows"))]
const MENU_COPY_LAST: usize = 13;
#[cfg(any(test, target_os = "windows"))]
const MENU_COPY_LAST_PATH: usize = 14;
#[cfg(any(test, target_os = "windows"))]
const MENU_REVEAL_LAST: usize = 15;
#[cfg(any(test, target_os = "windows"))]
const MENU_OPEN_FOLDER: usize = 16;
#[cfg(any(test, target_os = "windows"))]
const MENU_REVEAL_CONFIG: usize = 17;
#[cfg(any(test, target_os = "windows"))]
const MENU_STARTUP: usize = 18;
#[cfg(any(test, target_os = "windows"))]
const MENU_QUIT: usize = 19;

#[cfg(any(test, target_os = "windows"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrayAction {
    FullScreenshot,
    RegionScreenshot,
    OpenLastScreenshot,
    CopyLastScreenshot,
    CopyLastScreenshotPath,
    RevealLastScreenshot,
    OpenScreenshotsFolder,
    ShowConfigFile,
    ToggleStartup,
    Quit,
}

#[cfg(any(test, target_os = "windows"))]
fn action_for_hotkey_id(id: i32) -> Option<TrayAction> {
    match id {
        HOTKEY_FULL => Some(TrayAction::FullScreenshot),
        HOTKEY_REGION => Some(TrayAction::RegionScreenshot),
        HOTKEY_QUIT => Some(TrayAction::Quit),
        _ => None,
    }
}

#[cfg(any(test, target_os = "windows"))]
fn action_for_menu_command(command: usize) -> Option<TrayAction> {
    match command {
        MENU_FULL => Some(TrayAction::FullScreenshot),
        MENU_REGION => Some(TrayAction::RegionScreenshot),
        MENU_OPEN_LAST => Some(TrayAction::OpenLastScreenshot),
        MENU_COPY_LAST => Some(TrayAction::CopyLastScreenshot),
        MENU_COPY_LAST_PATH => Some(TrayAction::CopyLastScreenshotPath),
        MENU_REVEAL_LAST => Some(TrayAction::RevealLastScreenshot),
        MENU_OPEN_FOLDER => Some(TrayAction::OpenScreenshotsFolder),
        MENU_REVEAL_CONFIG => Some(TrayAction::ShowConfigFile),
        MENU_STARTUP => Some(TrayAction::ToggleStartup),
        MENU_QUIT => Some(TrayAction::Quit),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
mod windows_tray {
    use std::{
        mem,
        path::{Path, PathBuf},
        ptr::{null, null_mut},
        sync::{
            Mutex,
            atomic::{AtomicIsize, Ordering},
        },
    };

    use super::{
        HOTKEY_FULL, HOTKEY_QUIT, HOTKEY_REGION, MENU_COPY_LAST, MENU_COPY_LAST_PATH, MENU_FULL,
        MENU_OPEN_FOLDER, MENU_OPEN_LAST, MENU_QUIT, MENU_REGION, MENU_REVEAL_CONFIG,
        MENU_REVEAL_LAST, MENU_STARTUP, TrayError,
    };
    use crate::{
        capture::{self, CaptureOutput},
        clipboard,
        config::Config,
        file_action, interactive, paths, startup,
    };
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
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
                AppendMenuW, CS_HREDRAW, CS_VREDRAW, CreateIconIndirect, CreatePopupMenu,
                CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu, DispatchMessageW,
                GetCursorPos, GetMessageW, HICON, ICONINFO, MF_CHECKED, MF_GRAYED, MF_SEPARATOR,
                MF_STRING, MF_UNCHECKED, MSG, PostQuitMessage, RegisterClassW, SetForegroundWindow,
                TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, TPM_TOPALIGN, TrackPopupMenu,
                TranslateMessage, WM_COMMAND, WM_DESTROY, WM_HOTKEY, WM_LBUTTONDBLCLK,
                WM_RBUTTONUP, WM_USER, WNDCLASSW,
            },
        },
    };

    const ICON_SIZE: usize = 32;
    const TRAY_ID: u32 = 1;
    const TRAY_MESSAGE: u32 = WM_USER + 1;
    static TRAY_ICON: AtomicIsize = AtomicIsize::new(0);
    static LAST_CAPTURE: Mutex<Option<PathBuf>> = Mutex::new(None);

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
                match super::action_for_hotkey_id(wparam as i32) {
                    Some(super::TrayAction::FullScreenshot) => {
                        notify_capture(hwnd, capture_full());
                    }
                    Some(super::TrayAction::RegionScreenshot) => {
                        notify_capture(hwnd, capture_region());
                    }
                    Some(super::TrayAction::Quit) => {
                        unsafe { PostQuitMessage(0) };
                    }
                    _ => {}
                }
                0
            }
            TRAY_MESSAGE => {
                match lparam as u32 {
                    WM_LBUTTONDBLCLK => notify_capture(hwnd, capture_full()),
                    WM_RBUTTONUP => {
                        if let Some(command) = unsafe { show_tray_menu(hwnd) } {
                            run_menu_command(hwnd, command);
                        }
                    }
                    _ => {}
                }
                0
            }
            WM_COMMAND => {
                let command = wparam & 0xffff;
                if command != 0 {
                    run_menu_command(hwnd, command);
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

    fn run_menu_command(hwnd: HWND, command: usize) {
        match super::action_for_menu_command(command) {
            Some(super::TrayAction::FullScreenshot) => notify_capture(hwnd, capture_full()),
            Some(super::TrayAction::RegionScreenshot) => notify_capture(hwnd, capture_region()),
            Some(super::TrayAction::OpenLastScreenshot) => notify_action(hwnd, open_last_capture()),
            Some(super::TrayAction::CopyLastScreenshot) => notify_action(hwnd, copy_last_capture()),
            Some(super::TrayAction::CopyLastScreenshotPath) => {
                notify_action(hwnd, copy_last_capture_path())
            }
            Some(super::TrayAction::RevealLastScreenshot) => {
                notify_action(hwnd, reveal_last_capture())
            }
            Some(super::TrayAction::OpenScreenshotsFolder) => {
                notify_action(hwnd, open_output_folder())
            }
            Some(super::TrayAction::ShowConfigFile) => notify_action(hwnd, reveal_config_file()),
            Some(super::TrayAction::ToggleStartup) => notify_action(hwnd, toggle_startup()),
            Some(super::TrayAction::Quit) => unsafe { PostQuitMessage(0) },
            _ => {}
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

    unsafe fn show_tray_menu(hwnd: HWND) -> Option<usize> {
        let menu = unsafe { CreatePopupMenu() };
        if menu.is_null() {
            return None;
        }

        let has_last_capture = LAST_CAPTURE.lock().unwrap().is_some();
        let has_recent_capture = has_last_capture || recent_capture().is_some();
        let items = [
            (MENU_FULL, "Full screenshot", MF_STRING),
            (MENU_REGION, "Region screenshot", MF_STRING),
            (
                MENU_OPEN_LAST,
                "Open last screenshot",
                enabled_menu_flag(has_recent_capture),
            ),
            (
                MENU_COPY_LAST,
                "Copy last screenshot",
                enabled_menu_flag(has_recent_capture),
            ),
            (
                MENU_COPY_LAST_PATH,
                "Copy last screenshot path",
                enabled_menu_flag(has_recent_capture),
            ),
            (
                MENU_REVEAL_LAST,
                "Reveal last screenshot",
                enabled_menu_flag(has_recent_capture),
            ),
            (MENU_OPEN_FOLDER, "Open screenshots folder", MF_STRING),
            (MENU_REVEAL_CONFIG, "Show config file", MF_STRING),
        ];
        for (id, label, flags) in items {
            let label = wide(label);
            unsafe {
                AppendMenuW(menu, flags, id, label.as_ptr());
            }
        }
        unsafe {
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
        }
        let startup = wide("Start with Windows");
        let startup_state = match startup::is_enabled() {
            Ok(true) => MF_CHECKED,
            Ok(false) | Err(_) => MF_UNCHECKED,
        };
        unsafe {
            AppendMenuW(
                menu,
                MF_STRING | startup_state,
                MENU_STARTUP,
                startup.as_ptr(),
            );
        }
        unsafe {
            AppendMenuW(menu, MF_SEPARATOR, 0, null());
        }
        let quit = wide("Quit");
        unsafe {
            AppendMenuW(menu, MF_STRING, MENU_QUIT, quit.as_ptr());
        }

        let mut point = POINT::default();
        let command = if unsafe { GetCursorPos(&mut point) } != 0 {
            unsafe { SetForegroundWindow(hwnd) };
            unsafe {
                TrackPopupMenu(
                    menu,
                    TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
                    point.x,
                    point.y,
                    0,
                    hwnd,
                    null(),
                )
            }
        } else {
            0
        };
        unsafe { DestroyMenu(menu) };

        (command > 0).then_some(command as usize)
    }

    fn notify_capture(hwnd: HWND, result: Result<std::path::PathBuf, TrayError>) {
        match result {
            Ok(path) => {
                *LAST_CAPTURE.lock().unwrap() = Some(path.clone());
                show_balloon(hwnd, "Screenshot saved", display_path(&path), NIIF_INFO);
            }
            Err(error) => show_balloon(hwnd, "Screenshot failed", error.to_string(), NIIF_ERROR),
        }
    }

    fn notify_action(hwnd: HWND, result: Result<Option<String>, String>) {
        match result {
            Ok(Some(message)) => show_balloon(hwnd, "shotlite", message, NIIF_INFO),
            Ok(None) => {}
            Err(error) => show_balloon(hwnd, "shotlite", error, NIIF_ERROR),
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

    fn open_output_folder() -> Result<Option<String>, String> {
        let config = Config::load().map_err(|error| error.to_string())?;
        file_action::open(&config.output_dir).map_err(|error| error.to_string())?;
        Ok(None)
    }

    fn open_last_capture() -> Result<Option<String>, String> {
        let path = last_capture_path()?;
        file_action::open(&path).map_err(|error| error.to_string())?;
        Ok(None)
    }

    fn copy_last_capture() -> Result<Option<String>, String> {
        let path = last_capture_path()?;
        clipboard::copy_image_file(&path).map_err(|error| error.to_string())?;
        Ok(Some("Copied last screenshot".to_owned()))
    }

    fn copy_last_capture_path() -> Result<Option<String>, String> {
        let path = last_capture_path()?;
        clipboard::copy_text(&path.display().to_string()).map_err(|error| error.to_string())?;
        Ok(Some("Copied last screenshot path".to_owned()))
    }

    fn reveal_last_capture() -> Result<Option<String>, String> {
        let path = last_capture_path()?;
        file_action::reveal(&path).map_err(|error| error.to_string())?;
        Ok(None)
    }

    fn reveal_config_file() -> Result<Option<String>, String> {
        let config_file =
            paths::config_file().ok_or_else(|| "could not determine config path".to_owned())?;
        file_action::reveal(&config_file).map_err(|error| error.to_string())?;
        Ok(None)
    }

    fn toggle_startup() -> Result<Option<String>, String> {
        let enabled = startup::is_enabled().map_err(|error| error.to_string())?;
        startup::set_enabled(!enabled).map_err(|error| error.to_string())?;
        let state = if enabled { "disabled" } else { "enabled" };
        Ok(Some(format!("Start with Windows {state}")))
    }

    fn last_capture_path() -> Result<PathBuf, String> {
        LAST_CAPTURE
            .lock()
            .unwrap()
            .clone()
            .or_else(recent_capture)
            .ok_or_else(|| "no screenshot has been captured yet".to_owned())
    }

    fn recent_capture() -> Option<PathBuf> {
        let config = Config::load().ok()?;
        newest_png(&config.output_dir)
    }

    fn newest_png(dir: &Path) -> Option<PathBuf> {
        std::fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let is_png = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("png"));
                if !is_png {
                    return None;
                }
                Some((entry.metadata().ok()?.modified().ok()?, path))
            })
            .max_by_key(|(modified, _)| *modified)
            .map(|(_, path)| path)
    }

    fn enabled_menu_flag(enabled: bool) -> u32 {
        if enabled {
            MF_STRING
        } else {
            MF_STRING | MF_GRAYED
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_ids_map_to_documented_actions() {
        assert_eq!(
            action_for_hotkey_id(HOTKEY_FULL),
            Some(TrayAction::FullScreenshot)
        );
        assert_eq!(
            action_for_hotkey_id(HOTKEY_REGION),
            Some(TrayAction::RegionScreenshot)
        );
        assert_eq!(action_for_hotkey_id(HOTKEY_QUIT), Some(TrayAction::Quit));
        assert_eq!(action_for_hotkey_id(99), None);
    }

    #[test]
    fn menu_commands_map_to_tray_actions() {
        assert_eq!(
            action_for_menu_command(MENU_OPEN_LAST),
            Some(TrayAction::OpenLastScreenshot)
        );
        assert_eq!(
            action_for_menu_command(MENU_COPY_LAST),
            Some(TrayAction::CopyLastScreenshot)
        );
        assert_eq!(
            action_for_menu_command(MENU_COPY_LAST_PATH),
            Some(TrayAction::CopyLastScreenshotPath)
        );
        assert_eq!(
            action_for_menu_command(MENU_REVEAL_LAST),
            Some(TrayAction::RevealLastScreenshot)
        );
        assert_eq!(
            action_for_menu_command(MENU_OPEN_FOLDER),
            Some(TrayAction::OpenScreenshotsFolder)
        );
        assert_eq!(
            action_for_menu_command(MENU_REVEAL_CONFIG),
            Some(TrayAction::ShowConfigFile)
        );
        assert_eq!(
            action_for_menu_command(MENU_STARTUP),
            Some(TrayAction::ToggleStartup)
        );
        assert_eq!(action_for_menu_command(MENU_QUIT), Some(TrayAction::Quit));
        assert_eq!(action_for_menu_command(999), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn tray_startup_window_failure_is_clear() {
        let error = TrayError::Window.to_string();

        assert!(error.contains("failed to start tray mode"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn tray_run_reports_unsupported_platform() {
        let error = run().unwrap_err().to_string();

        assert_eq!(error, "tray mode is only supported on Windows");
    }
}
