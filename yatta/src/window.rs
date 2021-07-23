use std::mem;

use anyhow::Result;
use bitflags::bitflags;
use log::debug;

use bindings::Windows::Win32::{
    Foundation::{HWND, PWSTR},
    Graphics::{
        Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
        Gdi::{MonitorFromWindow, HMONITOR, MONITOR_DEFAULTTOPRIMARY},
    },
    System::Threading::{
        OpenProcess,
        QueryFullProcessImageNameW,
        PROCESS_NAME_FORMAT,
        PROCESS_QUERY_INFORMATION,
    },
    UI::{
        KeyboardAndMouseInput::SetFocus,
        WindowsAndMessaging::{
            GetForegroundWindow,
            GetWindowInfo,
            GetWindowLongW,
            GetWindowRect,
            GetWindowTextW,
            GetWindowThreadProcessId,
            IsIconic,
            IsWindow,
            IsWindowVisible,
            RealGetWindowClassW,
            SetCursorPos,
            SetForegroundWindow,
            SetWindowPos,
            ShowWindow,
            GWL_EXSTYLE,
            GWL_STYLE,
            HWND_BOTTOM,
            SET_WINDOW_POS_FLAGS,
            SWP_NOACTIVATE,
            SW_RESTORE,
            WINDOWINFO,
            WS_BORDER,
            WS_CAPTION,
            WS_CHILD,
            WS_CHILDWINDOW,
            WS_CLIPCHILDREN,
            WS_CLIPSIBLINGS,
            WS_DISABLED,
            WS_DLGFRAME,
            WS_EX_ACCEPTFILES,
            WS_EX_APPWINDOW,
            WS_EX_CLIENTEDGE,
            WS_EX_COMPOSITED,
            WS_EX_CONTEXTHELP,
            WS_EX_CONTROLPARENT,
            WS_EX_DLGMODALFRAME,
            WS_EX_LAYERED,
            WS_EX_LAYOUTRTL,
            WS_EX_LEFT,
            WS_EX_LEFTSCROLLBAR,
            WS_EX_LTRREADING,
            WS_EX_MDICHILD,
            WS_EX_NOACTIVATE,
            WS_EX_NOINHERITLAYOUT,
            WS_EX_NOPARENTNOTIFY,
            WS_EX_NOREDIRECTIONBITMAP,
            WS_EX_OVERLAPPEDWINDOW,
            WS_EX_PALETTEWINDOW,
            WS_EX_RIGHT,
            WS_EX_RIGHTSCROLLBAR,
            WS_EX_RTLREADING,
            WS_EX_STATICEDGE,
            WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST,
            WS_EX_TRANSPARENT,
            WS_EX_WINDOWEDGE,
            WS_GROUP,
            WS_HSCROLL,
            WS_ICONIC,
            WS_MAXIMIZE,
            WS_MAXIMIZEBOX,
            WS_MINIMIZE,
            WS_MINIMIZEBOX,
            WS_OVERLAPPED,
            WS_OVERLAPPEDWINDOW,
            WS_POPUP,
            WS_POPUPWINDOW,
            WS_SIZEBOX,
            WS_SYSMENU,
            WS_TABSTOP,
            WS_THICKFRAME,
            WS_TILED,
            WS_TILEDWINDOW,
            WS_VISIBLE,
            WS_VSCROLL,
        },
    },
};

use crate::{
    rect::Rect,
    windows_event::WindowsEventType,
    FLOAT_CLASSES,
    FLOAT_EXES,
    FLOAT_TITLES,
    LAYERED_EXE_WHITELIST,
};

bitflags! {
    #[derive(Default)]
    pub struct GwlStyle: u32 {
        const BORDER = WS_BORDER.0;
        const CAPTION = WS_CAPTION.0;
        const CHILD = WS_CHILD.0;
        const CHILDWINDOW = WS_CHILDWINDOW.0;
        const CLIPCHILDREN = WS_CLIPCHILDREN.0;
        const CLIPSIBLINGS = WS_CLIPSIBLINGS.0;
        const DISABLED = WS_DISABLED.0;
        const DLGFRAME = WS_DLGFRAME.0;
        const GROUP = WS_GROUP.0;
        const HSCROLL = WS_HSCROLL.0;
        const ICONIC = WS_ICONIC.0;
        const MAXIMIZE = WS_MAXIMIZE.0;
        const MAXIMIZEBOX = WS_MAXIMIZEBOX.0;
        const MINIMIZE = WS_MINIMIZE.0;
        const MINIMIZEBOX = WS_MINIMIZEBOX.0;
        const OVERLAPPED = WS_OVERLAPPED.0;
        const OVERLAPPEDWINDOW = WS_OVERLAPPEDWINDOW.0;
        const POPUP = WS_POPUP.0;
        const POPUPWINDOW = WS_POPUPWINDOW.0;
        const SIZEBOX = WS_SIZEBOX.0;
        const SYSMENU = WS_SYSMENU.0;
        const TABSTOP = WS_TABSTOP.0;
        const THICKFRAME = WS_THICKFRAME.0;
        const TILED = WS_TILED.0;
        const TILEDWINDOW = WS_TILEDWINDOW.0;
        const VISIBLE = WS_VISIBLE.0;
        const VSCROLL = WS_VSCROLL.0;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct GwlExStyle: u32 {
        const ACCEPTFILES = WS_EX_ACCEPTFILES.0;
        const APPWINDOW = WS_EX_APPWINDOW.0;
        const CLIENTEDGE = WS_EX_CLIENTEDGE.0;
        const COMPOSITED = WS_EX_COMPOSITED.0;
        const CONTEXTHELP = WS_EX_CONTEXTHELP.0;
        const CONTROLPARENT = WS_EX_CONTROLPARENT.0;
        const DLGMODALFRAME = WS_EX_DLGMODALFRAME.0;
        const LAYERED = WS_EX_LAYERED.0;
        const LAYOUTRTL = WS_EX_LAYOUTRTL.0;
        const LEFT = WS_EX_LEFT.0;
        const LEFTSCROLLBAR = WS_EX_LEFTSCROLLBAR.0;
        const LTRREADING = WS_EX_LTRREADING.0;
        const MDICHILD = WS_EX_MDICHILD.0;
        const NOACTIVATE = WS_EX_NOACTIVATE.0;
        const NOINHERITLAYOUT = WS_EX_NOINHERITLAYOUT.0;
        const NOPARENTNOTIFY = WS_EX_NOPARENTNOTIFY.0;
        const NOREDIRECTIONBITMAP = WS_EX_NOREDIRECTIONBITMAP.0;
        const OVERLAPPEDWINDOW = WS_EX_OVERLAPPEDWINDOW.0;
        const PALETTEWINDOW = WS_EX_PALETTEWINDOW.0;
        const RIGHT = WS_EX_RIGHT.0;
        const RIGHTSCROLLBAR = WS_EX_RIGHTSCROLLBAR.0;
        const RTLREADING = WS_EX_RTLREADING.0;
        const STATICEDGE = WS_EX_STATICEDGE.0;
        const TOOLWINDOW = WS_EX_TOOLWINDOW.0;
        const TOPMOST = WS_EX_TOPMOST.0;
        const TRANSPARENT = WS_EX_TRANSPARENT.0;
        const WINDOWEDGE = WS_EX_WINDOWEDGE.0;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub hwnd:     HWND,
    pub hmonitor: HMONITOR,
    pub tile:     bool,
    pub resize:   Option<Rect>,
}

unsafe impl Send for Window {}

fn nullable_to_result<T: PartialEq<i32>>(v: T) -> Result<T> {
    if v != 0 {
        Ok(v)
    } else {
        Err(anyhow::anyhow!("WinAPI return value is null"))
    }
}

pub fn exe_name_from_path(path: &str) -> String {
    path.split('\\').last().unwrap().to_string()
}

impl Window {
    pub fn foreground() -> Window {
        let hwnd = unsafe { GetForegroundWindow() };
        let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) };

        Window {
            hwnd,
            hmonitor,
            tile: true,
            resize: None,
        }
    }

    pub fn should_tile(&self) -> bool {
        let classes = FLOAT_CLASSES.lock().unwrap();
        let exes = FLOAT_EXES.lock().unwrap();
        let titles = FLOAT_TITLES.lock().unwrap();

        let mut should = true;

        if !self.tile {
            should = false
        }

        if let Ok(class) = self.class() {
            if classes.contains(&class) {
                should = false
            }
        }

        if let Ok(exe_path) = self.exe_path() {
            let exe = exe_name_from_path(&exe_path);
            if exes.contains(&exe) {
                should = false
            }
        }

        if let Some(title) = self.title() {
            for t in titles.iter() {
                if title.contains(t) {
                    should = false
                }
            }
        }

        should
    }

    pub fn class(&self) -> Result<String> {
        const BUF_SIZE: usize = 512;
        let mut buff: [u16; BUF_SIZE] = [0; BUF_SIZE];

        let writ_chars =
            unsafe { RealGetWindowClassW(self.hwnd, PWSTR(buff.as_mut_ptr()), BUF_SIZE as u32) };

        if writ_chars == 0 {
            return Err(std::io::Error::last_os_error().into());
        }

        Ok(String::from_utf16_lossy(&buff[0..writ_chars as usize]))
    }

    pub fn thread_process_id(&self) -> (u32, u32) {
        let mut process_pid: u32 = 0;
        let thread_pid = unsafe { GetWindowThreadProcessId(self.hwnd, &mut process_pid) };

        (process_pid, thread_pid)
    }

    pub fn exe_path(&self) -> Result<String> {
        let (pid, _) = self.thread_process_id();
        // PROCESS_QUERY_INFORMATION (0x0400)
        // https://docs.microsoft.com/en-us/windows/win32/procthread/process-security-and-access-rights
        let handle = unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, false, pid) };

        let mut buf_len = 260_u32;
        let mut result: Vec<u16> = vec![0; buf_len as usize];
        let text_ptr = result.as_mut_ptr();

        unsafe {
            let success: bool = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(text_ptr),
                &mut buf_len as *mut u32,
            )
            .into();
            if !success {
                return Err(std::io::Error::last_os_error().into());
            }
        }

        Ok(String::from_utf16_lossy(&result[..buf_len as usize]))
    }

    pub fn rect(self) -> Rect {
        unsafe {
            let mut rect = mem::zeroed();

            GetWindowRect(self.hwnd, &mut rect);

            rect.into()
        }
    }

    pub fn is_visible(self) -> bool {
        unsafe { IsWindowVisible(self.hwnd).into() }
    }

    pub fn is_minimized(self) -> bool {
        unsafe { IsIconic(self.hwnd).into() }
    }

    pub fn is_window(self) -> bool {
        unsafe { IsWindow(self.hwnd).into() }
    }

    pub fn is_active(self) -> bool {
        self.info().window_status == 1
    }

    pub fn get_style(&self) -> Result<GwlStyle> {
        unsafe {
            nullable_to_result(GetWindowLongW(self.hwnd, GWL_STYLE))
                .map(|x| GwlStyle::from_bits_unchecked(x as u32))
        }
    }

    pub fn get_ex_style(&self) -> Result<GwlExStyle> {
        unsafe {
            nullable_to_result(GetWindowLongW(self.hwnd, GWL_EXSTYLE))
                .map(|x| GwlExStyle::from_bits_unchecked(x as u32))
        }
    }

    pub fn toggle_float(&mut self) {
        self.tile = !self.tile;
    }

    pub fn should_manage(&self, event: Option<WindowsEventType>) -> bool {
        match self.title() {
            None => return false,
            Some(_) => {}
        }

        let is_cloaked = self.is_cloaked();
        let styles = self.get_style();
        let extended_styles = self.get_ex_style();

        let mut allow_cloaked = false;
        if let Some(event) = event {
            if WindowsEventType::Hide == event {
                allow_cloaked = true
            }
        }

        match (allow_cloaked, is_cloaked) {
            // if allowing cloaked windows, we don't need to check the cloaked status
            (true, _) |
            // if not allowing cloaked windows, we need to ensure the window is not cloaked
            (false, false) => {
                match (styles, extended_styles) {
                    (Ok(style), Ok(ex_style)) => {
                        if let (Some(title), Ok(path)) = (self.title(), self.exe_path()) {
                            let exe_name = exe_name_from_path(&path);
                            let allow_layered = LAYERED_EXE_WHITELIST.contains(&exe_name);

                            if style.contains(GwlStyle::CAPTION)
                                && ex_style.contains(GwlExStyle::WINDOWEDGE)
                                && !ex_style.contains(GwlExStyle::DLGMODALFRAME)
                                // Get a lot of dupe events coming through that make the redrawing go crazy
                                // on FocusChange events if I don't filter out this one. But, if we are
                                // allowing a specific layered window on the whitelist (like Steam), it should
                                // pass this check
                                && (allow_layered || !ex_style.contains(GwlExStyle::LAYERED))
                            {
                                debug!(
                                    "managing {} - {} (styles: {:?}) (extended styles: {:?})",
                                    exe_name_from_path(&path),
                                    title,
                                    style,
                                    ex_style
                                );

                                true
                            } else {
                                if let Some(event) = event {
                                    debug!(
                                        "ignoring {} - {} (event: {}) (cloaked: {}) (styles: {:?}) (extended styles: {:?})",
                                        exe_name_from_path(&path),
                                        title,
                                        event,
                                        self.is_cloaked(),
                                        style,
                                        ex_style
                                    );
                                }
                                false
                            }
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn title(self) -> Option<String> {
        let mut text: [u16; 512] = [0; 512];
        let len = unsafe { GetWindowTextW(self.hwnd, PWSTR(text.as_mut_ptr()), text.len() as i32) };
        let text = String::from_utf16_lossy(&text[..len as usize]);

        if text.is_empty() {
            None
        } else {
            Option::from(text)
        }
    }

    pub fn index(self, windows: &[Window]) -> Option<usize> {
        for (i, window) in windows.iter().enumerate() {
            if window.hwnd == self.hwnd {
                return Some(i);
            }
        }

        None
    }

    // Shamelessly lifted from https://github.com/robmikh/screenshot-rs/blob/ac1e21f70720e85bed5772194721e5f1cea29d88/src/capture.rs
    pub fn is_cloaked(self) -> bool {
        unsafe {
            let mut cloaked: u32 = 0;
            // TODO: Handle an error code here, ? won't be threadsafe
            let _ = DwmGetWindowAttribute(
                self.hwnd,
                std::mem::transmute::<_, u32>(DWMWA_CLOAKED),
                &mut cloaked as *mut _ as *mut _,
                std::mem::size_of::<u32>() as u32,
            );

            // DWM_CLOAKED_APP (value 0x0000001). The window was cloaked by its owner
            // application.
            //
            // DWM_CLOAKED_SHELL (value 0x0000002). The window was cloaked by the Shell.
            //
            // DWM_CLOAKED_INHERITED (value 0x0000004). The cloak value was inherited from
            // its owner window.
            cloaked == 0x0000001 || cloaked == 0x0000002 || cloaked == 0x0000004
        }
    }

    pub fn set_pos(
        &self,
        rect: Rect,
        insert_after: Option<HWND>,
        flags: Option<SET_WINDOW_POS_FLAGS>,
    ) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                insert_after.unwrap_or(HWND_BOTTOM),
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                flags.unwrap_or(SWP_NOACTIVATE),
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
            SetForegroundWindow(self.hwnd);
            // This isn't really needed when the above command works as expected via AHK
            SetFocus(self.hwnd);
        }
    }

    pub fn info(self) -> WindowInfo {
        unsafe {
            let mut info: WINDOWINFO = mem::zeroed();
            info.cbSize = mem::size_of::<WINDOWINFO>() as u32;

            GetWindowInfo(self.hwnd, &mut info);

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
            ShowWindow(self.hwnd, SW_RESTORE);
        };
    }
}

impl Default for Window {
    fn default() -> Self {
        Window {
            hwnd:     HWND(0),
            hmonitor: HMONITOR(0),
            tile:     true,
            resize:   None,
        }
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Window) -> bool {
        self.hwnd == other.hwnd
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
            window_rect:     info.rcWindow.into(),
            client_rect:     info.rcClient.into(),
            styles:          info.dwStyle,
            extended_styles: info.dwExStyle,
            window_status:   info.dwWindowStatus,
            x_borders:       info.cxWindowBorders,
            y_borders:       info.cyWindowBorders,
        }
    }
}
