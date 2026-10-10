//! Windows helpers for a transparent, click-through desktop Buddy overlay.
//!
//! Native helpers for the Windows desktop overlay. Bevy renders the desktop
//! scene to an image; `UpdateLayeredWindow` presents premultiplied per-pixel
//! alpha to the actual HWND, avoiding GPU swapchain alpha inconsistencies.

use bevy::prelude::IVec2;

/// Physical-pixel work area of the primary monitor. The taskbar is excluded;
/// its bottom edge is the appropriate floor for a desktop Buddy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Global desktop pointer and modifier/button state. Uses the physical screen
/// coordinate system rather than window-local coordinates, so it works while
/// Bevy's `CursorOptions.hit_test` is false.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pointer {
    pub screen: IVec2,
    pub left_down: bool,
    pub shift_down: bool,
    pub ctrl_down: bool,
    pub alt_down: bool,
    pub quit_down: bool,
    pub escape_down: bool,
}

impl WorkArea {
    pub fn width(self) -> u32 {
        self.right.saturating_sub(self.left).max(0) as u32
    }

    pub fn height(self) -> u32 {
        self.bottom.saturating_sub(self.top).max(0) as u32
    }

    pub fn origin(self) -> IVec2 {
        IVec2::new(self.left, self.top)
    }
}

/// Get the primary display work area in screen/physical coordinates.
#[cfg(windows)]
pub fn primary_work_area() -> Option<WorkArea> {
    let mut rect = imp::Rect::default();
    // SPI_GETWORKAREA writes the primary monitor's usable desktop rectangle.
    let ok = unsafe { imp::SystemParametersInfoW(0x0030, 0, &mut rect as *mut _ as *mut _, 0) };
    (ok != 0).then_some(WorkArea {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

/// Short alias used by the desktop-mode integration.
pub fn work_area() -> Option<WorkArea> {
    primary_work_area()
}

#[cfg(not(windows))]
pub fn primary_work_area() -> Option<WorkArea> {
    None
}

/// Read the cursor in global screen coordinates, including while this overlay
/// is in click-through mode.
#[cfg(windows)]
pub fn global_cursor_position() -> Option<IVec2> {
    let mut point = imp::Point::default();
    (unsafe { imp::GetCursorPos(&mut point) } != 0).then_some(IVec2::new(point.x, point.y))
}

/// Read global pointer position plus left-button, Shift and Ctrl state.
#[cfg(windows)]
pub fn pointer() -> Option<Pointer> {
    let screen = global_cursor_position()?;
    Some(Pointer {
        screen,
        left_down: imp::mouse_down(),
        shift_down: imp::key_down(0x10),
        ctrl_down: imp::key_down(0x11),
        alt_down: imp::key_down(0x12),
        quit_down: imp::key_down(0x51),
        escape_down: imp::key_down(0x1b),
    })
}

#[cfg(not(windows))]
pub fn global_cursor_position() -> Option<IVec2> {
    None
}

#[cfg(not(windows))]
pub fn pointer() -> Option<Pointer> {
    None
}

/// Convert a global screen cursor position to coordinates local to a window
/// whose top-left corner is `window_origin` in screen coordinates.
pub fn screen_to_window(cursor: IVec2, window_origin: IVec2) -> IVec2 {
    cursor - window_origin
}

/// Native window handle type used by Win32 FFI.
pub type Hwnd = *mut core::ffi::c_void;

/// Extract the primary Bevy window's HWND after its `RawHandleWrapper` appears.
#[cfg(windows)]
pub fn hwnd_from_primary_wrapper(
    wrapper: &bevy::window::RawHandleWrapper,
    _window_title: &str,
) -> Option<Hwnd> {
    match wrapper.get_window_handle() {
        raw_window_handle::RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as Hwnd),
        _ => None,
    }
}

#[cfg(not(windows))]
pub fn hwnd_from_primary_wrapper(
    _wrapper: &bevy::window::RawHandleWrapper,
    _window_title: &str,
) -> Option<Hwnd> {
    None
}

/// Apply overlay styles once the primary HWND exists and expose the native
/// HWND without activation. Bevy's own window should remain hidden and render
/// to an image once this path is active.
#[cfg(windows)]
pub fn configure_overlay(hwnd: Hwnd) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { imp::SetLastError(0) };
    let old = unsafe { imp::GetWindowLongPtrW(hwnd, imp::GWL_EXSTYLE) };
    let new = old | imp::WS_EX_TOOLWINDOW | imp::WS_EX_LAYERED | imp::WS_EX_NOACTIVATE;
    // Reset the layered bit first. This clears any prior SetLayeredWindowAttributes
    // mode so UpdateLayeredWindow can supply per-pixel alpha.
    unsafe {
        if old & imp::WS_EX_LAYERED != 0 {
            imp::SetLastError(0);
            if imp::SetWindowLongPtrW(hwnd, imp::GWL_EXSTYLE, old & !imp::WS_EX_LAYERED) == 0
                && imp::GetLastError() != 0
            {
                return false;
            }
        }
        imp::SetLastError(0);
        if imp::SetWindowLongPtrW(hwnd, imp::GWL_EXSTYLE, new) == 0 && imp::GetLastError() != 0 {
            return false;
        }
    }
    unsafe {
        let positioned = imp::SetWindowPos(
            hwnd,
            imp::HWND_TOPMOST,
            0,
            0,
            0,
            0,
            imp::SWP_NOMOVE
                | imp::SWP_NOSIZE
                | imp::SWP_NOACTIVATE
                | imp::SWP_NOOWNERZORDER
                | imp::SWP_FRAMECHANGED,
        ) != 0;
        imp::ShowWindow(hwnd, imp::SW_SHOWNOACTIVATE);
        positioned
    }
}

/// Preserve the per-pixel-alpha layered style as Winit updates cursor
/// hit-testing, while toggling native click-through for idle desktop space.
#[cfg(windows)]
pub fn update_overlay_input_style(hwnd: Hwnd, click_through: bool) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { imp::update_overlay_input_style(hwnd, click_through) }
}

#[cfg(not(windows))]
pub fn update_overlay_input_style(_hwnd: Hwnd, _click_through: bool) -> bool {
    false
}

/// Present an RGBA frame through the native per-pixel-alpha layered-window
/// API. Pixels are converted to premultiplied BGRA as required by ULW_ALPHA.
#[cfg(windows)]
pub fn present_rgba(hwnd: Hwnd, width: u32, height: u32, rgba: &[u8]) -> bool {
    unsafe { imp::present_rgba(hwnd, width, height, rgba) }
}

#[cfg(not(windows))]
pub fn present_rgba(_hwnd: Hwnd, _width: u32, _height: u32, _rgba: &[u8]) -> bool {
    false
}

/// Toggle activation for an editor that needs keyboard input. Call with true
/// only when explicitly opening a script/text editor; call false when it closes.
/// Enabling removes `WS_EX_NOACTIVATE` and asks Windows to foreground the Buddy.
#[cfg(windows)]
pub fn set_keyboard_input(hwnd: Hwnd, enabled: bool) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { imp::SetLastError(0) };
    let old = unsafe { imp::GetWindowLongPtrW(hwnd, imp::GWL_EXSTYLE) };
    let new = if enabled {
        old & !imp::WS_EX_NOACTIVATE
    } else {
        old | imp::WS_EX_NOACTIVATE
    };
    if old != new {
        unsafe { imp::SetLastError(0) };
        if unsafe { imp::SetWindowLongPtrW(hwnd, imp::GWL_EXSTYLE, new) } == 0
            && unsafe { imp::GetLastError() } != 0
        {
            return false;
        }
        if unsafe {
            imp::SetWindowPos(
                hwnd,
                imp::HWND_TOPMOST,
                0,
                0,
                0,
                0,
                imp::SWP_NOMOVE
                    | imp::SWP_NOSIZE
                    | imp::SWP_NOOWNERZORDER
                    | imp::SWP_FRAMECHANGED
                    | if enabled { 0 } else { imp::SWP_NOACTIVATE },
            )
        } == 0
        {
            return false;
        }
    }
    !enabled || unsafe { imp::SetForegroundWindow(hwnd) != 0 }
}

/// Consume tool clicks while desktop play is armed, so shooting a window
/// does not also activate the button or link underneath the cursor.
#[cfg(windows)]
pub fn set_tool_capture(enabled: bool) {
    unsafe {
        imp::set_tool_capture(enabled);
    }
}
#[cfg(not(windows))]
pub fn set_tool_capture(_enabled: bool) {}
#[cfg(windows)]
pub fn release_tool_capture() {
    unsafe {
        imp::release_tool_capture();
    }
}
#[cfg(not(windows))]
pub fn release_tool_capture() {}

#[cfg(not(windows))]
pub fn set_keyboard_input(_hwnd: Hwnd, _enabled: bool) -> bool {
    false
}

/// Opt into per-monitor-v2 DPI awareness. Call before `work_area()` and before
/// creating the Bevy window; Windows can reject this if awareness was already
/// selected by the process manifest, in which case the existing mode remains.
#[cfg(windows)]
pub fn enable_dpi_awareness() -> bool {
    unsafe { imp::SetProcessDpiAwarenessContext((-4isize) as *mut core::ffi::c_void) != 0 }
}

#[cfg(not(windows))]
pub fn enable_dpi_awareness() -> bool {
    false
}

#[cfg(not(windows))]
pub fn configure_overlay(_hwnd: Hwnd) -> bool {
    false
}

/// Low-level fallback to toggle `WS_EX_TRANSPARENT`. Prefer changing Bevy's
/// `CursorOptions.hit_test` per frame: Bevy 0.19's Winit backend maps it to
/// Windows' ignore-cursor-events style and keeps its window state in sync.
/// `click_through=true` allows mouse input to pass to apps beneath the Buddy.
#[cfg(windows)]
#[allow(dead_code)]
pub fn set_click_through(hwnd: Hwnd, click_through: bool) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe { imp::SetLastError(0) };
    let old = unsafe { imp::GetWindowLongPtrW(hwnd, imp::GWL_EXSTYLE) };
    let new = if click_through {
        old | imp::WS_EX_TRANSPARENT
    } else {
        old & !imp::WS_EX_TRANSPARENT
    };
    if old == new {
        return true;
    }
    unsafe { imp::SetLastError(0) };
    if unsafe { imp::SetWindowLongPtrW(hwnd, imp::GWL_EXSTYLE, new) } == 0
        && unsafe { imp::GetLastError() } != 0
    {
        return false;
    }
    unsafe {
        imp::SetWindowPos(
            hwnd,
            imp::HWND_TOPMOST,
            0,
            0,
            0,
            0,
            imp::SWP_NOMOVE
                | imp::SWP_NOSIZE
                | imp::SWP_NOACTIVATE
                | imp::SWP_NOOWNERZORDER
                | imp::SWP_FRAMECHANGED,
        ) != 0
    }
}

#[cfg(not(windows))]
pub fn set_click_through(_hwnd: Hwnd, _click_through: bool) -> bool {
    false
}

#[cfg(windows)]
mod imp {
    use core::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

    static TOOL_CAPTURE: AtomicBool = AtomicBool::new(false);
    static LEFT_DOWN: AtomicBool = AtomicBool::new(false);
    static BLOCKED_DOWN: AtomicBool = AtomicBool::new(false);
    static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);

    pub fn mouse_down() -> bool {
        if MOUSE_HOOK.load(Ordering::Relaxed) != 0 {
            LEFT_DOWN.load(Ordering::Relaxed)
        } else {
            key_down(0x01)
        }
    }

    pub unsafe fn set_tool_capture(enabled: bool) {
        unsafe {
            if enabled && MOUSE_HOOK.load(Ordering::Relaxed) == 0 {
                LEFT_DOWN.store(key_down(0x01), Ordering::Relaxed);
                let hook =
                    SetWindowsHookExW(14, Some(mouse_hook), GetModuleHandleW(core::ptr::null()), 0);
                MOUSE_HOOK.store(hook as isize, Ordering::Relaxed);
            }
            TOOL_CAPTURE.store(enabled, Ordering::Relaxed);
        }
    }

    pub unsafe fn release_tool_capture() {
        TOOL_CAPTURE.store(false, Ordering::Relaxed);
        let hook = MOUSE_HOOK.swap(0, Ordering::Relaxed);
        if hook != 0 {
            unsafe {
                UnhookWindowsHookEx(hook as *mut c_void);
            }
        }
    }

    unsafe extern "system" fn mouse_hook(code: i32, message: usize, data: isize) -> isize {
        if code >= 0 && matches!(message, 0x0201 | 0x0202) {
            LEFT_DOWN.store(message == 0x0201, Ordering::Relaxed);
            let blocked = if message == 0x0201 {
                let armed = TOOL_CAPTURE.load(Ordering::Relaxed);
                BLOCKED_DOWN.store(armed, Ordering::Relaxed);
                armed
            } else {
                BLOCKED_DOWN.swap(false, Ordering::Relaxed)
            };
            if blocked {
                return 1;
            }
        }
        unsafe { CallNextHookEx(core::ptr::null_mut(), code, message, data) }
    }

    pub const GWL_EXSTYLE: i32 = -20;
    pub const WS_EX_TRANSPARENT: isize = 0x0000_0020;
    pub const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
    pub const WS_EX_LAYERED: isize = 0x0008_0000;
    pub const WS_EX_NOACTIVATE: isize = 0x0800_0000;
    pub const HWND_TOPMOST: *mut c_void = (-1isize) as *mut c_void;
    pub const SW_SHOWNOACTIVATE: i32 = 4;
    pub const SWP_NOSIZE: u32 = 0x0001;
    pub const SWP_NOMOVE: u32 = 0x0002;
    pub const SWP_NOACTIVATE: u32 = 0x0010;
    pub const SWP_FRAMECHANGED: u32 = 0x0020;
    pub const SWP_NOOWNERZORDER: u32 = 0x0200;
    const ULW_ALPHA: u32 = 0x0000_0002;
    const AC_SRC_OVER: u8 = 0;
    const AC_SRC_ALPHA: u8 = 1;
    const DIB_RGB_COLORS: u32 = 0;
    const BI_RGB: u32 = 0;

    pub fn key_down(vkey: i32) -> bool {
        (unsafe { GetAsyncKeyState(vkey) }) < 0
    }

    pub unsafe fn update_overlay_input_style(hwnd: *mut c_void, click_through: bool) -> bool {
        unsafe {
            SetLastError(0);
            let old = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let mut new = old | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE;
            if click_through {
                new |= WS_EX_TRANSPARENT;
            } else {
                new &= !WS_EX_TRANSPARENT;
            }
            if new != old {
                SetLastError(0);
                if SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new) == 0 && GetLastError() != 0 {
                    return false;
                }
                return SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_FRAMECHANGED,
                ) != 0;
            }
            true
        }
    }

    pub unsafe fn present_rgba(hwnd: *mut c_void, width: u32, height: u32, rgba: &[u8]) -> bool {
        if hwnd.is_null() || width == 0 || height == 0 {
            return false;
        }
        let Some(byte_len) = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
        else {
            return false;
        };
        if rgba.len() != byte_len || width > i32::MAX as u32 || height > i32::MAX as u32 {
            return false;
        }
        unsafe {
            let mut rect = Rect::default();
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return false;
            }
            if IsWindowVisible(hwnd) == 0 {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
            let screen_dc = GetDC(core::ptr::null_mut());
            if screen_dc.is_null() {
                return false;
            }
            let memory_dc = CreateCompatibleDC(screen_dc);
            if memory_dc.is_null() {
                ReleaseDC(core::ptr::null_mut(), screen_dc);
                return false;
            }
            let mut info = BitmapInfo::top_down(width as i32, height as i32);
            let mut bits = core::ptr::null_mut();
            let bitmap = CreateDIBSection(
                screen_dc,
                &mut info,
                DIB_RGB_COLORS,
                &mut bits,
                core::ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                DeleteDC(memory_dc);
                ReleaseDC(core::ptr::null_mut(), screen_dc);
                return false;
            }
            let old_bitmap = SelectObject(memory_dc, bitmap);
            if old_bitmap.is_null() {
                DeleteObject(bitmap);
                DeleteDC(memory_dc);
                ReleaseDC(core::ptr::null_mut(), screen_dc);
                return false;
            }
            let dst = core::slice::from_raw_parts_mut(bits as *mut u8, byte_len);
            for (src, out) in rgba.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
                let a = src[3] as u16;
                out[0] = ((src[2] as u16 * a + 127) / 255) as u8;
                out[1] = ((src[1] as u16 * a + 127) / 255) as u8;
                out[2] = ((src[0] as u16 * a + 127) / 255) as u8;
                out[3] = src[3];
            }
            let size = Size {
                cx: width as i32,
                cy: height as i32,
            };
            let destination = Point {
                x: rect.left,
                y: rect.top,
            };
            let source = Point { x: 0, y: 0 };
            let blend = BlendFunction {
                operation: AC_SRC_OVER,
                flags: 0,
                alpha: 255,
                format: AC_SRC_ALPHA,
            };
            let ok = UpdateLayeredWindow(
                hwnd,
                screen_dc,
                &destination,
                &size,
                memory_dc,
                &source,
                0,
                &blend,
                ULW_ALPHA,
            ) != 0;
            SelectObject(memory_dc, old_bitmap);
            DeleteObject(bitmap);
            DeleteDC(memory_dc);
            ReleaseDC(core::ptr::null_mut(), screen_dc);
            ok
        }
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct Rect {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

    #[repr(C)]
    struct Size {
        cx: i32,
        cy: i32,
    }

    #[repr(C)]
    struct BlendFunction {
        operation: u8,
        flags: u8,
        alpha: u8,
        format: u8,
    }

    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_pels_per_meter: i32,
        y_pels_per_meter: i32,
        clr_used: u32,
        clr_important: u32,
    }

    #[repr(C)]
    struct BitmapInfo {
        header: BitmapInfoHeader,
        colors: [u32; 1],
    }

    impl BitmapInfo {
        fn top_down(width: i32, height: i32) -> Self {
            Self {
                header: BitmapInfoHeader {
                    size: core::mem::size_of::<BitmapInfoHeader>() as u32,
                    width,
                    height: -height,
                    planes: 1,
                    bit_count: 32,
                    compression: BI_RGB,
                    size_image: (width as u32)
                        .saturating_mul(height as u32)
                        .saturating_mul(4),
                    x_pels_per_meter: 0,
                    y_pels_per_meter: 0,
                    clr_used: 0,
                    clr_important: 0,
                },
                colors: [0],
            }
        }
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetWindowsHookExW(
            kind: i32,
            callback: Option<unsafe extern "system" fn(i32, usize, isize) -> isize>,
            module: *mut c_void,
            thread: u32,
        ) -> *mut c_void;
        fn UnhookWindowsHookEx(hook: *mut c_void) -> i32;
        fn CallNextHookEx(hook: *mut c_void, code: i32, message: usize, data: isize) -> isize;
        pub fn SystemParametersInfoW(
            action: u32,
            param: u32,
            output: *mut c_void,
            win_ini: u32,
        ) -> i32;
        pub fn GetCursorPos(point: *mut Point) -> i32;
        pub fn GetAsyncKeyState(vkey: i32) -> i16;
        pub fn GetWindowLongPtrW(hwnd: *mut c_void, index: i32) -> isize;
        pub fn SetWindowLongPtrW(hwnd: *mut c_void, index: i32, value: isize) -> isize;
        pub fn SetForegroundWindow(hwnd: *mut c_void) -> i32;
        pub fn SetProcessDpiAwarenessContext(context: *mut c_void) -> i32;
        pub fn SetWindowPos(
            hwnd: *mut c_void,
            insert_after: *mut c_void,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        fn GetWindowRect(hwnd: *mut c_void, rect: *mut Rect) -> i32;
        fn GetDC(hwnd: *mut c_void) -> *mut c_void;
        fn ReleaseDC(hwnd: *mut c_void, dc: *mut c_void) -> i32;
        pub fn ShowWindow(hwnd: *mut c_void, command: i32) -> i32;
        fn IsWindowVisible(hwnd: *mut c_void) -> i32;
        fn UpdateLayeredWindow(
            hwnd: *mut c_void,
            destination_dc: *mut c_void,
            destination: *const Point,
            size: *const Size,
            source_dc: *mut c_void,
            source: *const Point,
            color_key: u32,
            blend: *const BlendFunction,
            flags: u32,
        ) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateCompatibleDC(dc: *mut c_void) -> *mut c_void;
        fn CreateDIBSection(
            dc: *mut c_void,
            info: *const BitmapInfo,
            usage: u32,
            bits: *mut *mut c_void,
            section: *mut c_void,
            offset: u32,
        ) -> *mut c_void;
        fn SelectObject(dc: *mut c_void, object: *mut c_void) -> *mut c_void;
        fn DeleteObject(object: *mut c_void) -> i32;
        fn DeleteDC(dc: *mut c_void) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
        pub fn SetLastError(error: u32);
        pub fn GetLastError() -> u32;
    }
}
