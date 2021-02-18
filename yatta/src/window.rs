use std::mem;

use anyhow::Result;
use bitflags::bitflags;
use log::debug;

use bindings::windows::win32::{
    dwm::{DwmGetWindowAttribute, DWMWINDOWATTRIBUTE},
    keyboard_and_mouse_input::SetFocus,
    menus_and_resources::SetCursorPos,
    system_services::*,
    windows_and_messaging::{
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
        SetForegroundWindow,
        SetWindowPos,
        ShowWindow,
        HWND,
        WINDOWINFO,
    },
};

use crate::{rect::Rect, windows_event::WindowsEventType, FLOAT_CLASSES, FLOAT_EXES, FLOAT_TITLES};

bitflags! {
    #[derive(Default)]
    pub struct GwlStyle: i32 {
        const BORDER = WS_BORDER as i32;
        const CAPTION = WS_CAPTION as i32;
        const CHILD = WS_CHILD as i32;
        const CHILDWINDOW = WS_CHILDWINDOW as i32;
        const CLIPCHILDREN = WS_CLIPCHILDREN as i32;
        const CLIPSIBLINGS = WS_CLIPSIBLINGS as i32;
        const DISABLED = WS_DISABLED as i32;
        const DLGFRAME = WS_DLGFRAME as i32;
        const GROUP = WS_GROUP as i32;
        const HSCROLL = WS_HSCROLL as i32;
        const ICONIC = WS_ICONIC as i32;
        const MAXIMIZE = WS_MAXIMIZE as i32;
        const MAXIMIZEBOX = WS_MAXIMIZEBOX as i32;
        const MINIMIZE = WS_MINIMIZE as i32;
        const MINIMIZEBOX = WS_MINIMIZEBOX as i32;
        const OVERLAPPED = WS_OVERLAPPED as i32;
        const OVERLAPPEDWINDOW = WS_OVERLAPPEDWINDOW as i32;
        const POPUP = WS_POPUP as i32;
        const POPUPWINDOW = WS_POPUPWINDOW as i32;
        const SIZEBOX = WS_SIZEBOX as i32;
        const SYSMENU = WS_SYSMENU as i32;
        const TABSTOP = WS_TABSTOP as i32;
        const THICKFRAME = WS_THICKFRAME as i32;
        const TILED = WS_TILED as i32;
        const TILEDWINDOW = WS_TILEDWINDOW as i32;
        const VISIBLE = WS_VISIBLE as i32;
        const VSCROLL = WS_VSCROLL as i32;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct GwlExStyle: i32 {
        const ACCEPTFILES = WS_EX_ACCEPTFILES as i32;
        const APPWINDOW = WS_EX_APPWINDOW as i32;
        const CLIENTEDGE = WS_EX_CLIENTEDGE as i32;
        const COMPOSITED = WS_EX_COMPOSITED as i32;
        const CONTEXTHELP = WS_EX_CONTEXTHELP as i32;
        const CONTROLPARENT = WS_EX_CONTROLPARENT as i32;
        const DLGMODALFRAME = WS_EX_DLGMODALFRAME as i32;
        // This isn't available in windows-rs
        const LAYERED = 0x00080000_i32;
        const LAYOUTRTL = WS_EX_LAYOUTRTL as i32;
        const LEFT = WS_EX_LEFT as i32;
        const LEFTSCROLLBAR = WS_EX_LEFTSCROLLBAR as i32;
        const LTRREADING = WS_EX_LTRREADING as i32;
        const MDICHILD = WS_EX_MDICHILD as i32;
        const NOACTIVATE = WS_EX_NOACTIVATE as i32;
        const NOINHERITLAYOUT = WS_EX_NOINHERITLAYOUT as i32;
        const NOPARENTNOTIFY = WS_EX_NOPARENTNOTIFY as i32;
        const NOREDIRECTIONBITMAP = WS_EX_NOREDIRECTIONBITMAP as i32;
        const OVERLAPPEDWINDOW = WS_EX_OVERLAPPEDWINDOW as i32;
        const PALETTEWINDOW = WS_EX_PALETTEWINDOW as i32;
        const RIGHT = WS_EX_RIGHT as i32;
        const RIGHTSCROLLBAR = WS_EX_RIGHTSCROLLBAR as i32;
        const RTLREADING = WS_EX_RTLREADING as i32;
        const STATICEDGE = WS_EX_STATICEDGE as i32;
        const TOOLWINDOW = WS_EX_TOOLWINDOW as i32;
        const TOPMOST = WS_EX_TOPMOST as i32;
        const TRANSPARENT = WS_EX_TRANSPARENT as i32;
        const WINDOWEDGE = WS_EX_WINDOWEDGE as i32;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Window {
    pub hwnd: HWND,
    pub tile: bool,
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
        Window { hwnd, tile: true }
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
            unsafe { RealGetWindowClassW(self.hwnd, buff.as_mut_ptr(), BUF_SIZE as u32) };

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
        let handle = unsafe { OpenProcess(0x0400, false.into(), pid) };

        let mut buf_len = 260_u32;
        let mut result: Vec<u16> = vec![0; buf_len as usize];
        let text_ptr = result.as_mut_ptr();

        unsafe {
            let success: bool =
                QueryFullProcessImageNameW(handle, 0, text_ptr, &mut buf_len as *mut u32).into();
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
                .map(|x| GwlStyle::from_bits_unchecked(x as u32 as i32))
        }
    }

    pub fn get_ex_style(&self) -> Result<GwlExStyle> {
        unsafe {
            nullable_to_result(GetWindowLongW(self.hwnd, GWL_EXSTYLE))
                .map(|x| GwlExStyle::from_bits_unchecked(x as u32 as i32))
        }
    }

    pub fn toggle_float(&mut self) {
        self.tile = !self.tile;
    }

    pub fn should_manage(&self, event: Option<WindowsEventType>) -> bool {
        let is_cloaked = self.is_cloaked();
        let has_title = self.title().is_some();
        let styles = self.get_style();
        let extended_styles = self.get_ex_style();

        let mut allow_cloaked = false;
        if let Some(event) = event {
            if WindowsEventType::Hide == event {
                allow_cloaked = true
            }
        }

        if has_title && if allow_cloaked { true } else { !is_cloaked } {
            match (styles, extended_styles) {
                (Ok(style), Ok(ex_style)) => {
                    if style.contains(GwlStyle::CAPTION)
                        && ex_style.contains(GwlExStyle::WINDOWEDGE)
                        && !ex_style.contains(GwlExStyle::DLGMODALFRAME)
                        // Get a lot of dupe events coming through that make the redrawing go crazy 
                        // on FocusChange events if I don't filter out this one 
                        && !ex_style.contains(GwlExStyle::LAYERED)
                    {
                        if let Some(title) = self.title() {
                            if let Ok(path) = self.exe_path() {
                                debug!(
                                    "managing {} - {} (styles: {:?}) (extended styles: {:?})",
                                    exe_name_from_path(&path),
                                    title,
                                    style,
                                    ex_style
                                );
                            }
                        }

                        true
                    } else {
                        if let Some(event) = event {
                            if let Some(title) = self.title() {
                                if let Ok(path) = self.exe_path() {
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
                            }
                        }
                        false
                    }
                }
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn title(self) -> Option<String> {
        let mut text: [u16; 512] = [0; 512];
        let len = unsafe { GetWindowTextW(self.hwnd, text.as_mut_ptr(), text.len() as i32) };
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
                std::mem::transmute::<_, u32>(DWMWINDOWATTRIBUTE::DWMWA_CLOAKED),
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

    pub fn set_pos(&self, rect: Rect, insert_after: Option<i32>, flags: Option<u32>) {
        unsafe {
            SetWindowPos(
                self.hwnd,
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
            SetForegroundWindow(self.hwnd);
            // This isn't really needed when the above command works as expected via AHK
            SetFocus(self.hwnd);
        }
    }

    pub fn info(self) -> WindowInfo {
        unsafe {
            let mut info: WINDOWINFO = mem::zeroed();
            info.cb_size = mem::size_of::<WINDOWINFO>() as u32;

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
            hwnd: HWND(0),
            tile: true,
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
