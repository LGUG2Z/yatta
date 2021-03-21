use std::{cmp::Ordering, mem};

use bindings::windows::{
    win32::{
        display_devices::{POINT, RECT},
        gdi::{
            EnumDisplayMonitors,
            GetMonitorInfoW,
            MonitorFromPoint,
            MonitorFromWindow,
            HDC,
            MONITORINFO,
        },
        menus_and_resources::{GetCursorPos, SetCursorPos},
        system_services::{
            HWND_NOTOPMOST,
            MONITOR_DEFAULTTONEAREST,
            MONITOR_DEFAULTTOPRIMARY,
            SWP_NOMOVE,
            SWP_NOSIZE,
        },
        windows_and_messaging::{EnumWindows, HWND, LPARAM},
    },
    BOOL,
};
use yatta_core::{CycleDirection, Layout};

use crate::{rect::Rect, window::Window, DirectionOperation};
use enigo::{Enigo, MouseButton, MouseControllable};
use std::borrow::BorrowMut;

#[derive(Debug, Clone)]
pub struct Desktop {
    pub displays: Vec<Display>,
    pub paused:   bool,
}

#[derive(Debug, Clone)]
pub struct Display {
    pub windows:           Vec<Window>,
    pub hmonitor:          isize,
    pub dimensions:        Rect,
    pub layout:            Layout,
    pub layout_dimensions: Vec<Rect>,
    pub foreground_window: Window,
    pub gaps:              i32,
}

impl Display {
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

    pub fn set_cursor_pos_to_centre(&self) {
        unsafe {
            SetCursorPos(
                self.dimensions.x + (self.dimensions.width / 2),
                self.dimensions.y + (self.dimensions.height / 2),
            );
        }
    }

    pub fn follow_focus_with_mouse(&mut self, idx: usize) {
        if let Some(window) = self.windows.get(idx) {
            window.set_cursor_pos(self.layout_dimensions[idx]);
        };
    }

    pub fn window_op_up(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = match self.layout {
            Layout::BSPV => self.windows.len() > 2 && idx != 0 && idx != 1,
            Layout::BSPH => self.windows.len() > 1 && idx != 0,
            Layout::Columns | Layout::Monocle => false,
            Layout::Rows => idx != 0,
        };

        if can_move {
            let new_idx = match self.layout {
                Layout::BSPV => {
                    if idx % 2 == 0 {
                        idx - 1
                    } else {
                        idx - 2
                    }
                }

                Layout::BSPH => {
                    if idx % 2 == 0 {
                        idx - 2
                    } else {
                        idx - 1
                    }
                }
                Layout::Columns | Layout::Monocle => unreachable!(),
                Layout::Rows => idx - 1,
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_down(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let len = self.windows.len();

        let can_move = match self.layout {
            Layout::BSPV => len > 2 && idx != len - 1 && idx % 2 != 0,
            Layout::BSPH => self.windows.len() > 1 && idx % 2 == 0,
            Layout::Columns | Layout::Monocle => false,
            Layout::Rows => idx != self.windows.len() - 1,
        };

        if can_move {
            let new_idx = match self.layout {
                Layout::BSPV | Layout::BSPH | Layout::Rows => idx + 1,
                Layout::Columns | Layout::Monocle => unreachable!(),
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_left(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();
        let can_move = match self.layout {
            Layout::BSPV => self.windows.len() > 1 && idx != 0,
            Layout::BSPH => self.windows.len() > 2 && idx != 0 && idx != 1,
            Layout::Columns => idx != 0,
            Layout::Rows | Layout::Monocle => false,
        };

        if can_move {
            let new_idx = match self.layout {
                Layout::BSPV => {
                    if idx % 2 == 0 {
                        idx - 2
                    } else {
                        idx - 1
                    }
                }

                Layout::BSPH => {
                    if idx % 2 == 0 {
                        idx - 1
                    } else {
                        idx - 2
                    }
                }

                Layout::Columns => idx - 1,
                Layout::Rows | Layout::Monocle => unreachable!(),
            };

            op.handle(self, idx, new_idx);
        }
    }

    pub fn window_op_right(&mut self, op: DirectionOperation) {
        let idx = self.get_foreground_window_index();

        let can_move = match self.layout {
            Layout::BSPV => self.windows.len() > 1 && idx % 2 == 0,
            Layout::BSPH => self.windows.len() > 2 && idx % 2 != 0 && idx != self.windows.len() - 1,
            Layout::Columns => idx != self.windows.len() - 1,
            Layout::Rows | Layout::Monocle => false,
        };

        if can_move {
            let new_idx = match self.layout {
                Layout::BSPV | Layout::BSPH | Layout::Columns => idx + 1,
                Layout::Rows | Layout::Monocle => unreachable!(),
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

    pub fn calculate_layout(&mut self) {
        let len = self.windows.iter().filter(|x| x.should_tile()).count();

        match self.layout {
            Layout::Monocle => self.layout_dimensions = bsp(0, 1, self.dimensions, 1, self.gaps),
            Layout::BSPV => {
                self.layout_dimensions = bsp(0, len, self.dimensions, 1, self.gaps);
            }
            Layout::BSPH => {
                self.layout_dimensions = bsp(0, len, self.dimensions, 0, self.gaps);
            }
            Layout::Columns => {
                let width_f = self.dimensions.width as f32 / len as f32;
                let width = width_f.floor() as i32;

                let mut x = 0;
                let mut layouts: Vec<Rect> = vec![];
                for _ in &self.windows {
                    layouts.push(Rect {
                        x:      (self.dimensions.x + x) + self.gaps,
                        y:      (self.dimensions.y) + self.gaps,
                        width:  width - (self.gaps * 2),
                        height: self.dimensions.height - (self.gaps * 2),
                    });
                    x += width;
                }
                self.layout_dimensions = layouts
            }
            Layout::Rows => {
                let height_f = self.dimensions.height as f32 / len as f32;
                let height = height_f.floor() as i32;

                let mut y = 0;
                let mut layouts: Vec<Rect> = vec![];
                for _ in &self.windows {
                    layouts.push(Rect {
                        x:      self.dimensions.x + self.gaps,
                        y:      self.dimensions.y + y + self.gaps,
                        width:  self.dimensions.width - (self.gaps * 2),
                        height: height - (self.gaps * 2),
                    });
                    y += height;
                }
                self.layout_dimensions = layouts
            }
        }
    }

    pub fn apply_layout(&mut self, new_focus: Option<usize>) {
        if let Layout::Monocle = self.layout {
            self.get_foreground_window();
            self.foreground_window.set_pos(
                self.layout_dimensions[0],
                Option::from(HWND_NOTOPMOST),
                None,
            );

            return;
        }

        let mut skipped = 0;
        for (i, w) in self.windows.iter().enumerate() {
            if w.should_tile() {
                if let Some(new_idx) = new_focus {
                    // Make sure this is focused
                    if i == new_idx {
                        w.set_pos(
                            self.layout_dimensions[new_idx],
                            None,
                            Option::from(SWP_NOMOVE as u32 | SWP_NOSIZE as u32),
                        );
                    } else {
                        w.set_pos(self.layout_dimensions[i - skipped], None, None)
                    }
                } else {
                    w.set_pos(self.layout_dimensions[i - skipped], None, None)
                }
            } else {
                skipped += 1
            }
        }
    }
}

pub const PADDING: i32 = 20;

impl Desktop {
    pub fn get_active_display_idx(&self) -> usize {
        let active_display = unsafe {
            let mut cursor_pos: POINT = mem::zeroed();
            GetCursorPos(&mut cursor_pos);

            MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST as u32)
        };

        for (i, display) in self.displays.iter().enumerate() {
            if display.hmonitor == active_display {
                return i;
            }
        }

        0
    }

    pub fn enumerate_display_monitors(&mut self) {
        self.displays.clear();

        unsafe {
            EnumDisplayMonitors(
                HDC(0),
                std::ptr::null_mut(),
                Some(enum_display_monitor),
                LPARAM(&mut self.displays as *mut Vec<Display> as isize),
            );
        }
    }

    pub fn get_visible_windows(&mut self) {
        let mut windows: Vec<Window> = vec![];

        unsafe {
            EnumWindows(
                Some(enum_window),
                LPARAM(&mut windows as *mut Vec<Window> as isize),
            );
        }

        for display in &mut self.displays {
            display.windows.clear();

            display.windows = windows
                .iter()
                .filter(|x| x.should_tile())
                .filter(|x| x.hmonitor == display.hmonitor)
                .map(|x| x.to_owned())
                .collect::<Vec<Window>>();
        }
    }

    pub fn focus_display(&mut self, from: usize, direction: CycleDirection) {
        let can_focus = self.displays.len() > 1;

        if can_focus {
            let to = match direction {
                CycleDirection::Previous => {
                    if from == 0 {
                        self.displays.len() - 1
                    } else {
                        from - 1
                    }
                }
                CycleDirection::Next => {
                    if from == self.displays.len() - 1 {
                        0
                    } else {
                        from + 1
                    }
                }
            };

            let target = self.displays[to].borrow_mut();
            if let Some(window) = target.windows.first() {
                window.set_foreground();
                target.follow_focus_with_mouse(0)
            } else {
                target.set_cursor_pos_to_centre();
                let mut enigo = Enigo::new();
                enigo.mouse_click(MouseButton::Left)
            }
        }
    }

    pub fn focus_display_number(&mut self, to: usize) {
        let can_focus = to <= self.displays.len() && to > 0;

        if can_focus {
            let to = to - 1;

            let target = self.displays[to].borrow_mut();
            if let Some(window) = target.windows.first() {
                window.set_foreground();
                target.follow_focus_with_mouse(0)
            } else {
                target.set_cursor_pos_to_centre();
                let mut enigo = Enigo::new();
                enigo.mouse_click(MouseButton::Left)
            }
        }
    }

    pub fn move_window_to_display(
        &mut self,
        window_idx: usize,
        from: usize,
        direction: CycleDirection,
    ) {
        let can_move = self.displays.len() > 1;

        if can_move {
            let to = match direction {
                CycleDirection::Previous => {
                    if from == 0 {
                        self.displays.len() - 1
                    } else {
                        from - 1
                    }
                }
                CycleDirection::Next => {
                    if from == self.displays.len() - 1 {
                        0
                    } else {
                        from + 1
                    }
                }
            };

            let window = {
                let origin = self.displays[from].borrow_mut();
                let window = origin.windows.remove(window_idx);
                origin.calculate_layout();
                origin.apply_layout(None);
                window
            };

            let target = self.displays[to].borrow_mut();
            target.windows.insert(0, window);
            target.calculate_layout();
            target.apply_layout(Option::from(0));
        }
    }

    pub fn move_window_to_display_number(&mut self, window_idx: usize, from: usize, to: usize) {
        let can_move = to <= self.displays.len() && to > 0;

        if can_move {
            let to = to - 1;

            let window = {
                let origin = self.displays[from].borrow_mut();
                let window = origin.windows.remove(window_idx);
                origin.calculate_layout();
                origin.apply_layout(None);
                window
            };

            let target = self.displays[to].borrow_mut();
            target.windows.insert(0, window);
            target.calculate_layout();
            target.apply_layout(Option::from(0));
        }
    }

    pub fn calculate_layouts(&mut self) {
        for display in &mut self.displays {
            display.calculate_layout()
        }
    }

    pub fn apply_layouts(&mut self, new_focus: Option<usize>) {
        for display in &mut self.displays {
            display.apply_layout(new_focus)
        }
    }
}

impl Default for Desktop {
    fn default() -> Self {
        let mut desktop = Desktop {
            displays: vec![],
            paused:   false,
        };

        desktop.enumerate_display_monitors();

        desktop.displays.sort_by(|x, y| {
            let ordering = y.dimensions.x.cmp(&x.dimensions.x);

            if ordering == Ordering::Equal {
                return y.dimensions.y.cmp(&x.dimensions.y);
            }

            ordering
        });

        desktop.get_visible_windows();
        for display in &mut desktop.displays {
            display.get_foreground_window()
        }

        desktop.calculate_layouts();
        desktop.apply_layouts(None);

        desktop
    }
}

extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = unsafe { &mut *(lparam.0 as *mut Vec<Window>) };

    let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY as u32) };

    let w = Window {
        hwnd,
        hmonitor,
        tile: true,
    };

    if w.is_visible() && !w.is_minimized() && w.should_manage(None) {
        windows.push(w)
    }

    true.into()
}

extern "system" fn enum_display_monitor(
    monitor: isize,
    _: HDC,
    _: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let displays = unsafe { &mut *(lparam.0 as *mut Vec<Display>) };

    let mut rect: Rect = unsafe {
        let mut info: MONITORINFO = mem::zeroed();
        info.cb_size = mem::size_of::<MONITORINFO>() as u32;

        GetMonitorInfoW(monitor, &mut info as *mut MONITORINFO as *mut _);

        info.rc_work.into()
    };

    rect.height -= PADDING * 2;
    rect.width -= PADDING * 2;
    rect.y += PADDING;
    rect.x += PADDING;

    displays.push(Display {
        dimensions:        rect,
        foreground_window: Window::default(),
        gaps:              5,
        hmonitor:          monitor,
        layout:            Layout::BSPV,
        layout_dimensions: vec![],
        windows:           vec![],
    });

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
