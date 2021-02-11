use std::mem;

use bindings::windows::{
    win32::{
        display_devices::POINT,
        gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO},
        menus_and_resources::GetCursorPos,
        system_services::{HWND_TOP, MONITOR_DEFAULTTONEAREST, SWP_NOMOVE, SWP_NOSIZE},
        windows_and_messaging::{EnumWindows, HWND, LPARAM},
    },
    BOOL,
};

use crate::{rect::Rect, window::Window, DirectionOperation};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub dimensions:        Rect,
    pub windows:           Vec<Window>,
    pub layout:            Vec<Rect>,
    pub foreground_window: Window,
}

pub const PADDING: i32 = 20;

impl Workspace {
    pub fn get_dimensions(&mut self) {
        let active_monitor = unsafe {
            let mut cursor_pos: POINT = mem::zeroed();
            GetCursorPos(&mut cursor_pos);

            MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST as u32)
        };

        let mut rect: Rect = unsafe {
            let mut info: MONITORINFO = mem::zeroed();
            info.cb_size = mem::size_of::<MONITORINFO>() as u32;

            GetMonitorInfoW(active_monitor, &mut info as *mut MONITORINFO as *mut _);

            info.rc_work.into()
        };

        rect.height = rect.height - (PADDING * 2);
        rect.width = rect.width - (PADDING * 2);
        rect.y = rect.y + PADDING;
        rect.x = rect.x + PADDING;

        self.dimensions = rect;
    }

    pub fn get_visible_windows(&mut self) {
        self.windows.clear();

        unsafe {
            EnumWindows(
                Some(enum_window),
                LPARAM(&mut self.windows as *mut Vec<Window> as isize),
            );
        }
    }

    pub fn get_foreground_window(&mut self) {
        self.foreground_window = Window::foreground();
    }

    pub fn get_foreground_window_index(&mut self) -> usize {
        let mut idx = 0;

        for (i, w) in self.windows.iter().enumerate() {
            if self.foreground_window.0 == w.0 {
                idx = i;
                break;
            }
        }

        idx
    }

    pub fn follow_focus_with_mouse(&mut self, idx: usize) {
        let window = self.windows.get(idx).unwrap();
        window.set_cursor_pos(self.layout[idx]);
    }

    pub fn move_window_up(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let valid_direction = self.windows.len() > 2 && idx != 0 && idx != 1;

        if valid_direction {
            let new_idx = if idx % 2 == 0 { idx - 1 } else { idx - 2 };
            op.handle(self, idx, new_idx);
        }
    }

    pub fn move_window_down(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let len = self.windows.len();
        let valid_direction = len > 2 && idx != len - 1 && idx % 2 != 0;

        if valid_direction {
            let new_idx = idx + 1;
            op.handle(self, idx, new_idx);
        }
    }

    pub fn move_window_left(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = self.windows.len() > 1 && idx != 0;

        if can_move {
            let new_idx = if idx % 2 == 0 { idx - 2 } else { idx - 1 };
            op.handle(self, idx, new_idx);
        }
    }

    pub fn move_window_right(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = self.windows.len() > 1 && idx % 2 == 0;

        if can_move {
            let new_idx = idx + 1;
            op.handle(self, idx, new_idx);
        }
    }

    pub fn swap_window_next(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = self.windows.len() > 1;

        if can_move {
            let new_idx = if idx == self.windows.len() - 1 {
                0
            } else {
                idx + 1
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn swap_window_previous(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = self.windows.len() > 1;

        if can_move {
            let new_idx = if idx == 0 {
                self.windows.len() - 1
            } else {
                idx - 1
            };

            op.handle(self, idx, new_idx);
        }
    }

    // pub fn get_foreground_window_title(&mut self) -> Option<String> {
    //     let idx = self.get_foreground_window_index();
    //     self.windows[idx].get_title()
    // }

    pub fn calculate_layout(&mut self) {
        self.layout = bsp(0, self.windows.len(), self.dimensions, 1);
    }

    pub fn apply_layout(&self, new_focus: Option<usize>) {
        for (i, w) in self.windows.iter().enumerate() {
            if let Some(new_idx) = new_focus {
                // Make sure this is focused
                if i == new_idx {
                    w.set_pos(
                        self.layout[new_idx],
                        Option::from(HWND_TOP),
                        Option::from(SWP_NOMOVE as u32 | SWP_NOSIZE as u32),
                    );
                } else {
                    w.set_pos(self.layout[i], None, None)
                }
            } else {
                w.set_pos(self.layout[i], None, None)
            }
        }
    }
}

impl Default for Workspace {
    fn default() -> Self {
        let mut workspace = Workspace {
            dimensions:        Rect::zero(),
            windows:           vec![],
            layout:            vec![],
            foreground_window: Window::default(),
        };

        workspace.get_dimensions();
        workspace.get_visible_windows();
        workspace.get_foreground_window();
        workspace.calculate_layout();
        workspace.apply_layout(None);

        workspace
    }
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<Window>) };
    let w = Window(hwnd);
    if w.is_visible() && !w.is_minimized() && w.should_listen() {
        windows.push(w)
    }

    true.into()
}

fn bsp(i: usize, window_count: usize, area: Rect, vertical: usize) -> Vec<Rect> {
    if window_count == 0 {
        vec![]
    } else if window_count == 1 {
        vec![Rect {
            x:      area.x,
            y:      area.y,
            width:  area.width,
            height: area.height,
        }]
    } else if i % 2 == vertical {
        let mut res = vec![Rect {
            x:      area.x,
            y:      area.y,
            width:  area.width,
            height: area.height / 2,
        }];
        res.append(&mut bsp(
            i + 1,
            window_count - 1,
            Rect {
                x:      area.x,
                y:      area.y + area.height / 2,
                width:  area.width,
                height: area.height / 2,
            },
            vertical,
        ));
        res
    } else {
        let mut res = vec![Rect {
            x:      area.x,
            y:      area.y,
            width:  area.width / 2,
            height: area.height,
        }];
        res.append(&mut bsp(
            i + 1,
            window_count - 1,
            Rect {
                x:      area.x + area.width / 2,
                y:      area.y,
                width:  area.width / 2,
                height: area.height,
            },
            vertical,
        ));
        res
    }
}
