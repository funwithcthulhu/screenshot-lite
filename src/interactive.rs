use crate::redact::Rect;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InteractiveError {
    #[cfg(not(target_os = "windows"))]
    #[error("interactive region selection is not supported on this platform")]
    Unsupported,
    #[error("region selection was canceled")]
    Canceled,
    #[cfg(target_os = "windows")]
    #[error("window error")]
    Window,
}

#[cfg(target_os = "windows")]
pub fn select_region() -> Result<Rect, InteractiveError> {
    windows_overlay::select_region()
}

#[cfg(not(target_os = "windows"))]
pub fn select_region() -> Result<Rect, InteractiveError> {
    Err(InteractiveError::Unsupported)
}

#[cfg(target_os = "windows")]
mod windows_overlay {
    use std::{
        mem,
        ptr::{null, null_mut},
        sync::Mutex,
    };

    use super::InteractiveError;
    use crate::redact::Rect;
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::{
            BeginPaint, CreatePen, DeleteObject, EndPaint, GetStockObject, NULL_BRUSH, PAINTSTRUCT,
            PS_SOLID, RDW_INVALIDATE, Rectangle, RedrawWindow, SelectObject,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, VK_ESCAPE},
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
                DispatchMessageW, GetMessageW, GetSystemMetrics, IDC_CROSS, LWA_ALPHA, LoadCursorW,
                MSG, PostQuitMessage, RegisterClassW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
                SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOW, SetForegroundWindow,
                SetLayeredWindowAttributes, ShowWindow, TranslateMessage, WM_DESTROY, WM_KEYDOWN,
                WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WM_RBUTTONDOWN, WNDCLASSW,
                WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
            },
        },
    };

    static STATE: Mutex<Option<State>> = Mutex::new(None);

    #[derive(Clone, Copy, Debug)]
    struct State {
        origin_x: i32,
        origin_y: i32,
        start: Option<(i32, i32)>,
        current: Option<(i32, i32)>,
        result: Option<Rect>,
        canceled: bool,
    }

    pub fn select_region() -> Result<Rect, InteractiveError> {
        unsafe {
            let hinstance = GetModuleHandleW(null());
            let class_name = wide("shotlite-region-overlay");
            let cursor = LoadCursorW(null_mut(), IDC_CROSS);
            let wnd_class = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                hInstance: hinstance,
                hCursor: cursor,
                lpszClassName: class_name.as_ptr(),
                ..Default::default()
            };
            RegisterClassW(&wnd_class);

            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            *STATE.lock().unwrap() = Some(State {
                origin_x: x,
                origin_y: y,
                start: None,
                current: None,
                result: None,
                canceled: false,
            });

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
                class_name.as_ptr(),
                wide("shotlite region").as_ptr(),
                WS_POPUP,
                x,
                y,
                width,
                height,
                null_mut(),
                null_mut(),
                hinstance,
                null(),
            );
            if hwnd.is_null() {
                return Err(InteractiveError::Window);
            }

            SetLayeredWindowAttributes(hwnd, 0, 70, LWA_ALPHA);
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let state = STATE
                .lock()
                .unwrap()
                .take()
                .ok_or(InteractiveError::Window)?;
            if state.canceled {
                return Err(InteractiveError::Canceled);
            }
            state.result.ok_or(InteractiveError::Canceled)
        }
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_LBUTTONDOWN => {
                let point = lparam_point(lparam);
                if let Some(state) = STATE.lock().unwrap().as_mut() {
                    state.start = Some(point);
                    state.current = Some(point);
                }
                unsafe { SetCapture(hwnd) };
                0
            }
            WM_MOUSEMOVE => {
                if wparam & 0x0001 != 0 {
                    if let Some(state) = STATE.lock().unwrap().as_mut() {
                        state.current = Some(lparam_point(lparam));
                    }
                    unsafe { RedrawWindow(hwnd, null(), null_mut(), RDW_INVALIDATE) };
                }
                0
            }
            WM_LBUTTONUP => {
                unsafe { ReleaseCapture() };
                if let Some(state) = STATE.lock().unwrap().as_mut() {
                    state.current = Some(lparam_point(lparam));
                    if let (Some(start), Some(current)) = (state.start, state.current) {
                        state.result =
                            rect_from_points(start, current, state.origin_x, state.origin_y);
                    }
                }
                unsafe { DestroyWindow(hwnd) };
                0
            }
            WM_RBUTTONDOWN => {
                if let Some(state) = STATE.lock().unwrap().as_mut() {
                    state.canceled = true;
                }
                unsafe { DestroyWindow(hwnd) };
                0
            }
            WM_KEYDOWN => {
                if wparam as u32 == u32::from(VK_ESCAPE) {
                    if let Some(state) = STATE.lock().unwrap().as_mut() {
                        state.canceled = true;
                    }
                    unsafe { DestroyWindow(hwnd) };
                }
                0
            }
            WM_PAINT => {
                let mut ps: PAINTSTRUCT = unsafe { mem::zeroed() };
                let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
                if let Some(state) = *STATE.lock().unwrap()
                    && let (Some(start), Some(current)) = (state.start, state.current)
                {
                    let left = start.0.min(current.0);
                    let top = start.1.min(current.1);
                    let right = start.0.max(current.0);
                    let bottom = start.1.max(current.1);
                    let shadow_pen = unsafe { CreatePen(PS_SOLID, 5, rgb(0, 0, 0)) };
                    let old_pen = unsafe { SelectObject(hdc, shadow_pen) };
                    let old_brush = unsafe { SelectObject(hdc, GetStockObject(NULL_BRUSH)) };
                    unsafe {
                        Rectangle(hdc, left, top, right, bottom);
                        SelectObject(hdc, old_pen);
                        DeleteObject(shadow_pen);
                    }

                    let pen = unsafe { CreatePen(PS_SOLID, 2, rgb(255, 220, 0)) };
                    let old_pen = unsafe { SelectObject(hdc, pen) };
                    unsafe {
                        Rectangle(hdc, left, top, right, bottom);
                        SelectObject(hdc, old_brush);
                        SelectObject(hdc, old_pen);
                        DeleteObject(pen);
                    }
                }
                unsafe { EndPaint(hwnd, &ps) };
                0
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    fn lparam_point(lparam: LPARAM) -> (i32, i32) {
        let x = (lparam & 0xffff) as u16 as i16 as i32;
        let y = ((lparam >> 16) & 0xffff) as u16 as i16 as i32;
        (x, y)
    }

    fn rect_from_points(
        start: (i32, i32),
        end: (i32, i32),
        origin_x: i32,
        origin_y: i32,
    ) -> Option<Rect> {
        let left = start.0.min(end.0);
        let top = start.1.min(end.1);
        let width = start.0.abs_diff(end.0);
        let height = start.1.abs_diff(end.1);
        if width == 0 || height == 0 {
            return None;
        }
        Some(Rect {
            x: origin_x + left,
            y: origin_y + top,
            width,
            height,
        })
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain([0]).collect()
    }

    fn rgb(r: u8, g: u8, b: u8) -> u32 {
        u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16)
    }
}
