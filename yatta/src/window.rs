use std::mem;

use crate::rect::Rect;
use bindings::windows::win32::{
    dwm::{DwmGetWindowAttribute, DWMWINDOWATTRIBUTE},
    keyboard_and_mouse_input::SetFocus,
    menus_and_resources::SetCursorPos,
    system_services::{
        GWL_EXSTYLE,
        GWL_STYLE,
        HWND_BOTTOM,
        SWP_NOACTIVATE,
        SW_RESTORE,
        WS_EX_CLIENTEDGE,
        WS_EX_OVERLAPPEDWINDOW,
        WS_EX_WINDOWEDGE,
    },
    windows_and_messaging::{
        GetForegroundWindow,
        GetWindowInfo,
        GetWindowLongW,
        GetWindowRect,
        GetWindowTextW,
        IsIconic,
        IsWindowVisible,
        SetForegroundWindow,
        SetWindowPos,
        ShowWindow,
        HWND,
        WINDOWINFO,
    },
};
use log::debug;

use strum::{Display, EnumString};

#[derive(Clone, Copy, FromPrimitive, ToPrimitive, PartialEq, Display, Debug)]
#[repr(u32)]
#[allow(dead_code)]
pub enum WindowStyles {
    OverlappedTiled             = 0x00000000,
    Popup                       = 0x80000000,
    ChildChildWindow            = 0x40000000,
    MinimizeIconic              = 0x20000000,
    Visible                     = 0x10000000,
    Disabled                    = 0x08000000,
    ClipSiblings                = 0x04000000,
    ClipChildren                = 0x02000000,
    Maximize                    = 0x01000000,
    Caption                     = 0x00C00000,
    Border                      = 0x00800000,
    DlgFrame                    = 0x00400000,
    VScroll                     = 0x00200000,
    HScroll                     = 0x00100000,
    SysMenu                     = 0x00080000,
    ThickFrameSizeBox           = 0x00040000,
    GroupMinimizeBox            = 0x00020000,
    TabStopMaximizeBox          = 0x00010000,
    OverlappedWindowTiledWindow =
        (0x00000000 | 0x00C00000 | 0x00080000 | 0x00040000 | 0x00020000 | 0x00010000),
    PopupWindow                 = (0x80000000 | 0x00800000 | 0x00080000),
}

#[derive(Clone, Copy, FromPrimitive, ToPrimitive, PartialEq, Display, EnumString, Debug)]
#[repr(i32)]
#[allow(dead_code)]
pub enum ExtendedWindowStyles {
    AcceptFiles                  = 0x00000010,
    AppWindow                    = 0x00040000,
    ClientEdge                   = 0x00000200,
    Composited                   = 0x02000000,
    ContextHelp                  = 0x00000400,
    ControlParent                = 0x00010000,
    DlgModalFrame                = 0x00000001,
    Layered                      = 0x00080000,
    LayoutRtl                    = 0x00400000,
    LeftLtrReadingRightScrollBar = 0x00000000,
    LeftScrollBar                = 0x00004000,
    MdiChild                     = 0x00000040,
    NoActivate                   = 0x08000000,
    NoInheritLayout              = 0x00100000,
    NoParentNotify               = 0x00000004,
    OverlappedWindow             = (0x00000100 | 0x00000200),
    PaletteWindow                = (0x00000100 | 0x00000080 | 0x00000008),
    Right                        = 0x00001000,
    RtlReading                   = 0x00002000,
    StaticEdge                   = 0x00020000,
    ToolWindow                   = 0x00000080,
    TopMost                      = 0x00000008,
    Transparent                  = 0x00000020,
    WindowEdge                   = 0x00000100,
}

#[derive(Clone, Copy, Debug)]
pub struct Window(pub HWND);

unsafe impl Send for Window {}

impl Window {
    pub fn foreground() -> Window {
        let hwnd = unsafe { GetForegroundWindow() };
        Window(hwnd)
    }

    pub fn rect(self) -> Rect {
        unsafe {
            let mut rect = mem::zeroed();

            GetWindowRect(self.0, &mut rect);

            rect.into()
        }
    }

    pub fn is_visible(self) -> bool {
        unsafe { IsWindowVisible(self.0).into() }
    }

    pub fn is_minimized(self) -> bool {
        unsafe { IsIconic(self.0).into() }
    }

    pub fn is_active(self) -> bool {
        self.info().window_status == 1
    }

    pub fn get_window_styles(self) -> WindowStyles {
        let styles = unsafe { GetWindowLongW(self.0, GWL_STYLE) };
        unsafe { ::std::mem::transmute(styles) }
    }

    pub fn get_extended_window_styles(self) -> ExtendedWindowStyles {
        let extended_styles = unsafe { GetWindowLongW(self.0, GWL_EXSTYLE) };
        unsafe { ::std::mem::transmute(extended_styles) }
    }

    pub fn should_listen(self) -> bool {
        let styles = self.get_window_styles();
        let extended_styles = self.get_extended_window_styles();

        let is_cloaked = self.is_cloaked();
        let has_title = self.get_title().is_some();

        if has_title && !is_cloaked {
            match extended_styles {
                ExtendedWindowStyles::ClientEdge
                | ExtendedWindowStyles::OverlappedWindow
                | ExtendedWindowStyles::WindowEdge => true,
                _ => {
                    debug!(
                        "should ignore event from {:?} with styles {} and extended styles {}",
                        self.get_title(),
                        styles,
                        extended_styles
                    );
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn is_regular(self) -> bool {
        unsafe {
            let extended_styles = GetWindowLongW(self.0, GWL_EXSTYLE);
            extended_styles == WS_EX_WINDOWEDGE || extended_styles == WS_EX_CLIENTEDGE
        }
    }

    pub fn get_title(self) -> Option<String> {
        let mut text: [u16; 512] = [0; 512];
        let len = unsafe { GetWindowTextW(self.0, text.as_mut_ptr(), text.len() as i32) };
        let text = String::from_utf16_lossy(&text[..len as usize]);

        if text.is_empty() {
            None
        } else {
            Option::from(text)
        }
    }

    pub fn get_index(self, windows: &Vec<Window>) -> Option<usize> {
        for (i, window) in windows.iter().enumerate() {
            if window.0 == self.0 {
                return Some(i);
            }
        }

        None
    }

    pub fn is_cloaked(self) -> bool {
        unsafe {
            let mut cloaked: DWMWINDOWATTRIBUTE = mem::zeroed();
            // TODO: Handle an error code here, ? won't be threadsafe
            let _ = DwmGetWindowAttribute(
                self.0,
                DWMWINDOWATTRIBUTE::DWMWA_CLOAKED.0 as u32,
                &mut cloaked as *mut DWMWINDOWATTRIBUTE as *mut _,
                mem::size_of::<DWMWINDOWATTRIBUTE>() as u32,
            );

            cloaked.0 == 1
        }
    }

    pub fn set_pos(&self, rect: Rect, insert_after: Option<i32>, flags: Option<u32>) {
        unsafe {
            SetWindowPos(
                self.0,
                HWND(insert_after.unwrap_or(HWND_BOTTOM) as isize),
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                flags.unwrap_or(SWP_NOACTIVATE as u32),
            );
        }
    }

    pub fn set_cursor_pos(&self, rect: Rect) {
        unsafe {
            SetCursorPos(rect.x + (rect.width / 2), rect.y + (rect.height / 2));
        }
    }

    pub fn set_foreground(self) {
        unsafe {
            SetForegroundWindow(self.0);
            // This isn't really needed when the above command works as expected via AHK
            SetFocus(self.0);
        }
    }

    pub fn info(self) -> WindowInfo {
        unsafe {
            let mut info: WINDOWINFO = mem::zeroed();
            info.cb_size = mem::size_of::<WINDOWINFO>() as u32;

            GetWindowInfo(self.0, &mut info);

            info.into()
        }
    }

    pub fn transparent_border(self) -> (i32, i32) {
        let info = self.info();

        let x = {
            (info.window_rect.x - info.client_rect.x)
                + (info.window_rect.width - info.client_rect.width)
        };

        let y = {
            (info.window_rect.y - info.client_rect.y)
                + (info.window_rect.height - info.client_rect.height)
        };

        (x, y)
    }

    pub fn restore(&mut self) {
        unsafe {
            ShowWindow(self.0, SW_RESTORE);
        };
    }
}

impl Default for Window {
    fn default() -> Self {
        Window(HWND(0))
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Window) -> bool {
        self.0 == other.0
    }
}

#[derive(Debug)]
pub struct WindowInfo {
    pub window_rect:     Rect,
    pub client_rect:     Rect,
    pub styles:          u32,
    pub extended_styles: u32,
    pub window_status:   u32,
    pub x_borders:       u32,
    pub y_borders:       u32,
}

impl From<WINDOWINFO> for WindowInfo {
    fn from(info: WINDOWINFO) -> Self {
        WindowInfo {
            window_rect:     info.rc_window.into(),
            client_rect:     info.rc_client.into(),
            styles:          info.dw_style,
            extended_styles: info.dw_ex_style,
            window_status:   info.dw_window_status,
            x_borders:       info.cx_window_borders,
            y_borders:       info.cy_window_borders,
        }
    }
}
