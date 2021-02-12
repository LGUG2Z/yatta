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
use yatta_core::Orientation;

use crate::{rect::Rect, window::Window, DirectionOperation};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub dimensions:        Rect,
    pub windows:           Vec<Window>,
    pub layout:            Vec<Rect>,
    pub foreground_window: Window,
    pub gaps:              i32,
    pub orientation:       Orientation,
}

pub const PADDING: i32 = 20;

impl Workspace {
    pub fn set_gaps(&mut self, gaps: i32) {
        self.gaps = gaps;
    }

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
            if self.foreground_window.hwnd == w.hwnd {
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

    pub fn window_op_up(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = match self.orientation {
            Orientation::Vertical => self.windows.len() > 2 && idx != 0 && idx != 1,
            Orientation::Horizontal => self.windows.len() > 1 && idx != 0,
        };

        if can_move {
            let new_idx = match self.orientation {
                Orientation::Vertical => {
                    if idx % 2 == 0 {
                        idx - 1
                    } else {
                        idx - 2
                    }
                }
                Orientation::Horizontal => {
                    if idx % 2 == 0 {
                        idx - 2
                    } else {
                        idx - 1
                    }
                }
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_down(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let len = self.windows.len();

        let can_move = match self.orientation {
            Orientation::Vertical => len > 2 && idx != len - 1 && idx % 2 != 0,
            Orientation::Horizontal => self.windows.len() > 1 && idx % 2 == 0,
        };

        if can_move {
            let new_idx = match self.orientation {
                Orientation::Vertical => idx + 1,
                Orientation::Horizontal => idx + 1,
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_left(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = match self.orientation {
            Orientation::Vertical => self.windows.len() > 1 && idx != 0,
            Orientation::Horizontal => self.windows.len() > 2 && idx != 0 && idx != 1,
        };

        if can_move {
            let new_idx = match self.orientation {
                Orientation::Vertical => {
                    if idx % 2 == 0 {
                        idx - 2
                    } else {
                        idx - 1
                    }
                }
                Orientation::Horizontal => {
                    if idx % 2 == 0 {
                        idx - 1
                    } else {
                        idx - 2
                    }
                }
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_right(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();

        let can_move = match self.orientation {
            Orientation::Vertical => self.windows.len() > 1 && idx % 2 == 0,
            Orientation::Horizontal => {
                self.windows.len() > 2 && idx % 2 != 0 && idx != self.windows.len() - 1
            }
        };

        if can_move {
            let new_idx = match self.orientation {
                Orientation::Vertical => idx + 1,
                Orientation::Horizontal => idx + 1,
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_next(&mut self, op: DirectionOperation) {
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

    pub fn window_op_previous(&mut self, op: DirectionOperation) {
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
        let len = self.windows.iter().filter(|x| x.should_tile).count();
        self.layout = bsp(
            0,
            len,
            self.dimensions,
            self.orientation as usize,
            self.gaps,
        );
    }

    pub fn apply_layout(&self, new_focus: Option<usize>) {
        let mut skipped = 0;
        for (i, w) in self.windows.iter().enumerate() {
            if w.should_tile {
                if let Some(new_idx) = new_focus {
                    // Make sure this is focused
                    if i == new_idx {
                        w.set_pos(
                            self.layout[new_idx],
                            Option::from(HWND_TOP),
                            Option::from(SWP_NOMOVE as u32 | SWP_NOSIZE as u32),
                        );
                    } else {
                        w.set_pos(self.layout[i - skipped], None, None)
                    }
                } else {
                    w.set_pos(self.layout[i - skipped], None, None)
                }
            } else {
                skipped += 1
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
            gaps:              5,
            orientation:       Orientation::Vertical,
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
    let w = Window {
        hwnd,
        should_tile: true,
    };
    if w.is_visible() && !w.is_minimized() && w.should_manage() {
        windows.push(w)
    }

    true.into()
}

fn bsp(i: usize, window_count: usize, area: Rect, vertical: usize, gaps: i32) -> Vec<Rect> {
    if window_count == 0 {
        vec![]
    } else if window_count == 1 {
        vec![Rect {
            x:      area.x + gaps,
            y:      area.y + gaps,
            width:  area.width - gaps * 2,
            height: area.height - gaps * 2,
        }]
    } else if i % 2 == vertical {
        let mut res = vec![Rect {
            x:      area.x + gaps,
            y:      area.y + gaps,
            width:  area.width - gaps * 2,
            height: area.height / 2 - gaps * 2,
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
            gaps,
        ));
        res
    } else {
        let mut res = vec![Rect {
            x:      area.x + gaps,
            y:      area.y + gaps,
            width:  area.width / 2 - gaps * 2,
            height: area.height - gaps * 2,
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
            gaps,
        ));
        res
    }
}
