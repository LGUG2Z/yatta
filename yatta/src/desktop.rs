use std::{borrow::BorrowMut, cmp::Ordering, mem};

use enigo::{Enigo, MouseButton, MouseControllable};

use bindings::Windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, POINT, RECT},
    Graphics::Gdi::{
        EnumDisplayMonitors,
        GetMonitorInfoW,
        MonitorFromPoint,
        MonitorFromWindow,
        HDC,
        HMONITOR,
        MONITORINFO,
        MONITOR_DEFAULTTONEAREST,
        MONITOR_DEFAULTTOPRIMARY,
    },
    UI::WindowsAndMessaging::{
        EnumWindows,
        GetCursorPos,
        SetCursorPos,
        HWND_NOTOPMOST,
        SWP_NOMOVE,
        SWP_NOSIZE,
    },
};
use yatta_core::{CycleDirection, Layout, ResizeEdge, Sizing};

use crate::{rect::Rect, window::Window, DirectionOperation, PADDING};

#[derive(Debug, Clone)]
pub struct Desktop {
    pub displays: Vec<Display>,
    pub paused:   bool,
}

#[derive(Debug, Clone)]
pub struct Display {
    pub windows:           Vec<Window>,
    pub hmonitor:          HMONITOR,
    dimensions:            Rect,
    pub layout:            Layout,
    pub layout_dimensions: Vec<Rect>,
    pub foreground_window: Window,
    pub gaps:              i32,
    pub padding:           i32,
    pub resize_step:       i32,
}

impl Display {
    pub fn get_dimensions(&self) -> Rect {
        let mut rect = self.dimensions;

        let padding = PADDING.lock().unwrap();

        rect.height -= *padding * 2;
        rect.width -= *padding * 2;
        rect.y += *padding;
        rect.x += *padding;

        rect
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

    pub fn set_cursor_pos_to_centre(&self) {
        unsafe {
            SetCursorPos(
                self.get_dimensions().x + (self.get_dimensions().width / 2),
                self.get_dimensions().y + (self.get_dimensions().height / 2),
            );
        }
    }

    pub fn follow_focus_with_mouse(&mut self, idx: usize) {
        if let Some(window) = self.windows.get(idx) {
            window.set_cursor_pos(self.layout_dimensions[idx]);
        };
    }

    pub fn resize_window(&mut self, edge: ResizeEdge, sizing: Sizing, step: Option<i32>) {
        let resize_step = if let Some(step) = step {
            step
        } else {
            self.resize_step
        };

        let idx = self.get_foreground_window_index();
        let can_resize = match self.layout {
            Layout::BSPV => match edge {
                ResizeEdge::Left => !self.windows.is_empty() && idx != 0,
                ResizeEdge::Top => self.windows.len() > 2 && idx != 0 && idx != 1,
                ResizeEdge::Right => {
                    self.windows.len() > 1 && idx % 2 == 0 && idx != self.windows.len() - 1
                }
                ResizeEdge::Bottom => {
                    self.windows.len() > 2 && idx != self.windows.len() - 1 && idx % 2 != 0
                }
            },
            Layout::BSPH => match edge {
                ResizeEdge::Left => self.windows.len() > 2 && idx != 0 && idx != 1,
                ResizeEdge::Top => self.windows.len() > 1 && idx != 0,
                ResizeEdge::Right => {
                    self.windows.len() > 2 && idx != self.windows.len() - 1 && idx % 2 != 0
                }
                ResizeEdge::Bottom => {
                    self.windows.len() > 1 && idx % 2 == 0 && idx != self.windows.len() - 1
                }
            },
            _ => false,
        };

        if can_resize {
            let vertical = match self.layout {
                Layout::BSPV => 1,
                Layout::BSPH => 0,
                _ => unreachable!(),
            };

            // We want to reference the layout dimensions from a state where it's as if no
            // ressize adjustments have been applied
            let layout = bsp(
                0,
                self.windows.len(),
                self.get_dimensions(),
                vertical,
                self.gaps,
                vec![],
            )[idx];

            if self.windows[idx].resize.is_none() {
                self.windows[idx].resize = Option::from(Rect::zero())
            }

            if let Some(r) = self.windows[idx].resize.borrow_mut() {
                let max_divisor = 1.005;
                match edge {
                    ResizeEdge::Left => match sizing {
                        Sizing::Increase => {
                            // Some final checks to make sure the user can't infinitely resize to
                            // the point of pushing other windows out of bounds

                            // Note: These checks cannot take into account the changes made to the
                            // edges of adjacent windows at operation time, so it is still possible
                            // to push windows out of bounds by maxing out an Increase Left on a
                            // Window with index 1, and then maxing out a Decrease Right on a Window
                            // with index 0. I don't think it's worth trying to defensively program
                            // against this; if people end up in this situation they are better off
                            // just hitting the retile command
                            let diff = ((r.x + -resize_step) as f32).abs();
                            let max = layout.width as f32 / max_divisor;
                            if diff < max {
                                r.x += -resize_step;
                            }
                        }
                        Sizing::Decrease => {
                            let diff = ((r.x - -resize_step) as f32).abs();
                            let max = layout.width as f32 / max_divisor;
                            if diff < max {
                                r.x -= -resize_step;
                            }
                        }
                    },
                    ResizeEdge::Top => match sizing {
                        Sizing::Increase => {
                            let diff = ((r.y + resize_step) as f32).abs();
                            let max = layout.height as f32 / max_divisor;
                            if diff < max {
                                r.y += -resize_step;
                            }
                        }
                        Sizing::Decrease => {
                            let diff = ((r.y - resize_step) as f32).abs();
                            let max = layout.height as f32 / max_divisor;
                            if diff < max {
                                r.y -= -resize_step;
                            }
                        }
                    },
                    ResizeEdge::Right => match sizing {
                        Sizing::Increase => {
                            let diff = ((r.width + resize_step) as f32).abs();
                            let max = layout.width as f32 / max_divisor;
                            if diff < max {
                                r.width += resize_step;
                            }
                        }
                        Sizing::Decrease => {
                            let diff = ((r.width - resize_step) as f32).abs();
                            let max = layout.width as f32 / max_divisor;
                            if diff < max {
                                r.width -= resize_step;
                            }
                        }
                    },
                    ResizeEdge::Bottom => match sizing {
                        Sizing::Increase => {
                            let diff = ((r.height + resize_step) as f32).abs();
                            let max = layout.height as f32 / max_divisor;
                            if diff < max {
                                r.height += resize_step;
                            }
                        }
                        Sizing::Decrease => {
                            let diff = ((r.height - resize_step) as f32).abs();
                            let max = layout.height as f32 / max_divisor;
                            if diff < max {
                                r.height -= resize_step;
                            }
                        }
                    },
                };
            };
        }
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

    fn calculate_resize_adjustments(&self) -> Vec<Option<Rect>> {
        let windows: Vec<&Window> = self.windows.iter().filter(|x| x.should_tile()).collect();
        let resize_dimensions: Vec<Option<Rect>> = windows.iter().map(|x| x.resize).collect();
        let mut resize_adjustments = resize_dimensions.clone();

        for (i, opt) in resize_dimensions.iter().enumerate() {
            if let Some(resize_ref) = opt {
                if i > 0 {
                    if resize_ref.x != 0 {
                        let range = if i == 1 {
                            0..1
                        } else if i & 1 != 0 {
                            i - 1..i
                        } else {
                            i - 2..i
                        };

                        for n in range {
                            let should_adjust = match self.layout {
                                Layout::BSPV => n & 1 == 0,
                                Layout::BSPH => n & 1 == 1,
                                _ => unreachable!(),
                            };

                            if should_adjust {
                                if let Some(adjacent_resize) = resize_adjustments[n].borrow_mut() {
                                    adjacent_resize.width += resize_ref.x;
                                } else {
                                    resize_adjustments[n] = Option::from(Rect {
                                        x:      0,
                                        y:      0,
                                        width:  resize_ref.x,
                                        height: 0,
                                    });
                                }
                            }
                        }

                        if let Some(rr) = resize_adjustments[i].borrow_mut() {
                            rr.x = 0;
                        }
                    }

                    if resize_ref.y != 0 {
                        let range = if i == 1 {
                            0..1
                        } else if i & 1 == 0 {
                            i - 1..i
                        } else {
                            i - 2..i
                        };

                        for n in range {
                            let should_adjust = match self.layout {
                                Layout::BSPV => n & 1 == 1,
                                Layout::BSPH => n & 1 == 0,
                                _ => unreachable!(),
                            };

                            if should_adjust {
                                if let Some(adjacent_resize) = resize_adjustments[n].borrow_mut() {
                                    adjacent_resize.height += resize_ref.y;
                                } else {
                                    resize_adjustments[n] = Option::from(Rect {
                                        x:      0,
                                        y:      0,
                                        width:  0,
                                        height: resize_ref.y,
                                    });
                                }
                            }
                        }

                        if let Some(resize) = resize_adjustments[i].borrow_mut() {
                            resize.y = 0;
                        }
                    }
                }
            }
        }

        resize_adjustments
    }

    pub fn calculate_layout(&mut self) {
        let len = self.windows.iter().filter(|x| x.should_tile()).count();

        match self.layout {
            Layout::Monocle => {
                self.layout_dimensions = bsp(0, 1, self.get_dimensions(), 1, self.gaps, vec![]);
            }
            Layout::BSPV => {
                let resize_adjustments = self.calculate_resize_adjustments();
                self.layout_dimensions = bsp(
                    0,
                    len,
                    self.get_dimensions(),
                    1,
                    self.gaps,
                    resize_adjustments,
                );
            }
            Layout::BSPH => {
                let resize_adjustments = self.calculate_resize_adjustments();
                self.layout_dimensions = bsp(
                    0,
                    len,
                    self.get_dimensions(),
                    0,
                    self.gaps,
                    resize_adjustments,
                );
            }
            Layout::Columns => {
                let width_f = self.get_dimensions().width as f32 / len as f32;
                let width = width_f.floor() as i32;

                let mut x = 0;
                let mut layouts: Vec<Rect> = vec![];
                for _ in &self.windows {
                    layouts.push(Rect {
                        x:      (self.get_dimensions().x + x) + self.gaps,
                        y:      (self.get_dimensions().y) + self.gaps,
                        width:  width - (self.gaps * 2),
                        height: self.get_dimensions().height - (self.gaps * 2),
                    });
                    x += width;
                }
                self.layout_dimensions = layouts
            }
            Layout::Rows => {
                let height_f = self.get_dimensions().height as f32 / len as f32;
                let height = height_f.floor() as i32;

                let mut y = 0;
                let mut layouts: Vec<Rect> = vec![];
                for _ in &self.windows {
                    layouts.push(Rect {
                        x:      self.get_dimensions().x + self.gaps,
                        y:      self.get_dimensions().y + y + self.gaps,
                        width:  self.get_dimensions().width - (self.gaps * 2),
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
                            Option::from(SWP_NOMOVE | SWP_NOSIZE),
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

impl Desktop {
    pub fn get_active_display_idx(&self) -> usize {
        let active_display = unsafe {
            let mut cursor_pos: POINT = mem::zeroed();
            GetCursorPos(&mut cursor_pos);

            MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST)
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

    let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY) };

    let w = Window {
        hwnd,
        hmonitor,
        tile: true,
        resize: None,
    };

    if w.is_visible() && !w.is_minimized() && w.should_manage(None) {
        windows.push(w)
    }

    true.into()
}

extern "system" fn enum_display_monitor(
    monitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let displays = unsafe { &mut *(lparam.0 as *mut Vec<Display>) };

    let rect: Rect = unsafe {
        let mut info: MONITORINFO = mem::zeroed();
        info.cbSize = mem::size_of::<MONITORINFO>() as u32;

        GetMonitorInfoW(monitor, &mut info as *mut MONITORINFO as *mut _);

        info.rcWork.into()
    };

    let padding = PADDING.lock().unwrap();

    displays.push(Display {
        dimensions:        rect,
        foreground_window: Window::default(),
        gaps:              5,
        padding:           *padding,
        resize_step:       50,
        hmonitor:          monitor,
        layout:            Layout::BSPV,
        layout_dimensions: vec![],
        windows:           vec![],
    });

    true.into()
}

fn bsp(
    i: usize,
    window_count: usize,
    area: Rect,
    vertical: usize,
    gaps: i32,
    resize_dimensions: Vec<Option<Rect>>,
) -> Vec<Rect> {
    let mut a = area;

    let resized = if let Some(Some(r)) = resize_dimensions.get(i) {
        a.x += r.x;
        a.y += r.y;
        a.width += r.width;
        a.height += r.height;
        a
    } else {
        area
    };

    if window_count == 0 {
        vec![]
    } else if window_count == 1 {
        vec![Rect {
            x:      resized.x + gaps,
            y:      resized.y + gaps,
            width:  resized.width - gaps * 2,
            height: resized.height - gaps * 2,
        }]
    } else if i % 2 == vertical {
        let mut res = vec![Rect {
            x:      resized.x + gaps,
            y:      resized.y + gaps,
            width:  resized.width - gaps * 2,
            height: resized.height / 2 - gaps * 2,
        }];
        res.append(&mut bsp(
            i + 1,
            window_count - 1,
            Rect {
                x:      area.x,
                y:      area.y + resized.height / 2,
                width:  area.width,
                height: area.height - resized.height / 2,
            },
            vertical,
            gaps,
            resize_dimensions,
        ));
        res
    } else {
        let mut res = vec![Rect {
            x:      resized.x + gaps,
            y:      resized.y + gaps,
            width:  resized.width / 2 - gaps * 2,
            height: resized.height - gaps * 2,
        }];
        res.append(&mut bsp(
            i + 1,
            window_count - 1,
            Rect {
                x:      area.x + resized.width / 2,
                y:      area.y,
                width:  area.width - resized.width / 2,
                height: area.height,
            },
            vertical,
            gaps,
            resize_dimensions,
        ));
        res
    }
}
