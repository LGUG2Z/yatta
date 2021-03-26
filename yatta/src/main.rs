extern crate flexi_logger;
#[macro_use] extern crate num_derive;
extern crate num_traits;

use std::{
    borrow::BorrowMut,
    collections::HashMap,
    io::{BufRead, BufReader, ErrorKind},
    process::exit,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::{Context, Result};
use crossbeam_channel::{select, unbounded, Receiver, Sender};
use flexi_logger::{colored_detailed_format, Duplicate};
use lazy_static::lazy_static;
use log::{error, info};
use sysinfo::SystemExt;
use uds_windows::UnixListener;

use bindings::windows::win32::system_services::{HWND_TOP, SWP_NOMOVE, SWP_NOSIZE};
use yatta_core::{CycleDirection, Layout, OperationDirection, ResizeEdge, Sizing, SocketMessage};

use crate::{
    desktop::{Desktop, Display},
    rect::Rect,
    window::exe_name_from_path,
    windows_event::{WindowsEvent, WindowsEventListener, WindowsEventType},
};

mod desktop;
mod message_loop;
mod rect;
mod window;
mod windows_event;

lazy_static! {
    static ref YATTA_CHANNEL: Arc<Mutex<(Sender<Message>, Receiver<Message>)>> =
        Arc::new(Mutex::new(unbounded()));
    static ref FLOAT_CLASSES: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    static ref FLOAT_EXES: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    static ref FLOAT_TITLES: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    static ref DESKTOP_EXES: Arc<Mutex<HashMap<String, usize>>> =
        Arc::new(Mutex::new(HashMap::new()));
    static ref LAST_LAYOUT: Arc<Mutex<Layout>> = Arc::new(Mutex::new(Layout::BSPV));
}

#[derive(Clone, Debug)]
pub enum Message {
    WindowsEvent(WindowsEvent),
}

fn main() -> Result<()> {
    let home = dirs::home_dir().context("could not look up home directory")?;

    flexi_logger::Logger::with_str("debug")
        .format(colored_detailed_format)
        .log_to_file()
        .o_timestamp(false)
        .o_print_message(true)
        .directory(
            home.as_path()
                .to_str()
                .context("could not convert home directory path to string")?,
        )
        .duplicate_to_stdout(Duplicate::Info)
        .start()?;

    let mut system = sysinfo::System::new_all();
    system.refresh_processes();

    if system.get_process_by_name("yatta.exe").len() > 1 {
        error!("yatta.exe is already running, please exit the existing process before starting a new one");
        exit(1);
    }

    let desktop: Arc<Mutex<Desktop>> = Arc::new(Mutex::new(Desktop::default()));
    info!("started yatta");

    let listener = Arc::new(Mutex::new(WindowsEventListener::default()));
    listener.lock().unwrap().start();

    let mut socket = home.clone();
    socket.push("yatta.sock");
    let socket = socket.as_path();

    match std::fs::remove_file(&socket) {
        Ok(_) => {}
        Err(error) => match error.kind() {
            // Doing this because ::exists() doesn't work reliably on Windows via IntelliJ
            ErrorKind::NotFound => {}
            _ => {
                panic!(error);
            }
        },
    }

    let stream = match UnixListener::bind(&socket) {
        Err(error) => {
            panic!("failed to bind socket: {}", error)
        }
        Ok(stream) => stream,
    };

    info!(
        "listening for yattac messages on socket: {}",
        socket
            .to_str()
            .context("could not convert socket path to string")?
    );

    let desktop_clone = desktop.clone();
    thread::spawn(move || {
        for client in stream.incoming() {
            match client {
                Ok(stream) => {
                    let ls = Arc::clone(&listener);
                    handle_socket_message(stream, &desktop_clone, ls);
                }
                Err(err) => {
                    println!("Error: {}", err);
                    break;
                }
            }
        }
    });

    let yatta_receiver = YATTA_CHANNEL.lock().unwrap().1.clone();

    loop {
        select! {
                recv(yatta_receiver) -> maybe_msg => {
                    let msg = maybe_msg.unwrap();
                    let _ = match msg {
                        Message::WindowsEvent(ev) => {
                            let ws = Arc::clone(&desktop) ;
                            handle_windows_event_message(ev, ws)
                        },
                };
            }
        }
    }
}

fn handle_windows_event_message(ev: WindowsEvent, desktop: Arc<Mutex<Desktop>>) {
    let mut desktop = desktop.lock().unwrap();
    if desktop.paused {
        return;
    }

    // Make sure we discard any windows that no longer exist
    for display in &mut desktop.displays {
        display.windows.retain(|x| x.is_window());
    }

    let display_idx = desktop.get_active_display_idx();
    let display = desktop.displays[display_idx].borrow_mut();

    info!(
        "handling yatta channel message: {} ({})",
        ev.event_type, ev.event_code
    );

    match ev.event_type {
        WindowsEventType::ResizeStart => {
            let idx = ev.window.index(&display.windows);
            let old_position = display.layout_dimensions[idx.unwrap_or(0)];
            ev.window.set_pos(
                old_position,
                Option::from(HWND_TOP),
                Option::from(SWP_NOMOVE as u32 | SWP_NOSIZE as u32),
            )
        }
        WindowsEventType::ResizeEnd => {
            let idx = ev.window.index(&display.windows);
            let old_position = display.layout_dimensions[idx.unwrap_or(0)];
            let new_position = ev.window.info().window_rect;

            let mut resize = Rect::zero();
            resize.x = new_position.x - old_position.x;
            resize.y = new_position.y - old_position.y;
            resize.width = new_position.width - old_position.width;
            resize.height = new_position.height - old_position.height;

            if resize.width == 0 && resize.height == 0 {
                // TODO: Maybe get the cursor pos here and handle window dragging events as
                // moves or warps
                info!("ignoring probable window dragging event, only listening for window resizing events");
            } else {
                let mut ops = vec![];

                if resize.x != 0 {
                    resize.x *= 2;
                    let sizing = if resize.x > 0 {
                        Sizing::Decrease
                    } else {
                        Sizing::Increase
                    };

                    ops.push((ResizeEdge::Left, sizing, resize.x.abs()))
                }

                if resize.y != 0 {
                    resize.y *= 2;
                    let sizing = if resize.y > 0 {
                        Sizing::Decrease
                    } else {
                        Sizing::Increase
                    };

                    ops.push((ResizeEdge::Top, sizing, resize.y.abs()))
                }

                if resize.width != 0 && resize.x == 0 {
                    resize.width *= 2;
                    let sizing = if resize.width > 0 {
                        Sizing::Increase
                    } else {
                        Sizing::Decrease
                    };

                    ops.push((ResizeEdge::Right, sizing, resize.width.abs()))
                }

                if resize.height != 0 && resize.y == 0 {
                    resize.height *= 2;
                    let sizing = if resize.height > 0 {
                        Sizing::Increase
                    } else {
                        Sizing::Decrease
                    };

                    ops.push((ResizeEdge::Bottom, sizing, resize.height.abs()))
                }

                for (edge, sizing, step) in ops {
                    display.resize_window(edge, sizing, Option::from(step));
                }

                display.calculate_layout();
            }

            display.apply_layout(None);
        }
        WindowsEventType::Show => {
            if display.windows.is_empty() {
                display.windows.push(ev.window);
                display.calculate_layout();
                display.apply_layout(None);
            } else {
                // Some apps like Windows Terminal send multiple Events on startup, we don't
                // want dupes
                let mut contains = false;

                for window in &display.windows {
                    if window.hwnd == ev.window.hwnd {
                        contains = true;
                    }
                }

                if !contains {
                    let idx = display.get_foreground_window_index();
                    display.windows.insert(idx + 1, ev.window);
                    display.calculate_layout();
                    display.apply_layout(None);

                    if let Some(title) = ev.window.title() {
                        if let Ok(path) = ev.window.exe_path() {
                            info!(
                                "managing new window: {} - {} ({})",
                                &exe_name_from_path(&path),
                                &title,
                                ev.window.hwnd.0
                            );
                        }
                    }
                }
            }
        }
        WindowsEventType::Hide | WindowsEventType::Destroy => {
            let index = ev.window.index(&display.windows);
            let mut previous = index.unwrap_or(0);
            previous = if previous == 0 { 0 } else { previous - 1 };

            display.windows.retain(|x| !ev.window.eq(x));
            display.calculate_layout();
            display.apply_layout(Option::from(previous));
            if let Some(title) = ev.window.title() {
                info!("unmanaging window: {} ({})", &title, ev.window.hwnd.0);
            }
        }
        WindowsEventType::FocusChange => {
            display.calculate_layout();
            display.apply_layout(None);

            display.foreground_window = ev.window;
            if let Some(title) = ev.window.title() {
                if let Ok(path) = ev.window.exe_path() {
                    info!(
                        "focusing window: {} - {} ({})",
                        &exe_name_from_path(&path),
                        &title,
                        ev.window.hwnd.0
                    );
                }
            }
        }
    }
}

pub enum DirectionOperation {
    Focus,
    Move,
}

impl DirectionOperation {
    pub fn handle(self, display: &mut Display, idx: usize, new_idx: usize) {
        match self {
            DirectionOperation::Focus => {
                if let Some(window) = display.windows.get(new_idx) {
                    window.set_foreground();
                }
            }
            DirectionOperation::Move => {
                let window_resize = display.windows[idx].resize.clone();
                let new_window_resize = display.windows[new_idx].resize.clone();

                {
                    let window = display.windows[idx].borrow_mut();
                    window.resize = new_window_resize;
                }

                {
                    let new_window = display.windows[new_idx].borrow_mut();
                    new_window.resize = window_resize;
                }

                display.windows.swap(idx, new_idx);
                display.calculate_layout();
                display.apply_layout(Option::from(new_idx));
            }
        }

        display.follow_focus_with_mouse(new_idx);
    }
}

fn handle_socket_message(
    stream: uds_windows::UnixStream,
    desktop: &Arc<Mutex<Desktop>>,
    _listener: Arc<Mutex<WindowsEventListener>>,
) {
    let mut desktop = desktop.lock().unwrap();

    let stream = BufReader::new(stream);
    for line in stream.lines() {
        match line {
            Ok(socket_msg) => {
                if let Ok(msg) = SocketMessage::from_str(&socket_msg) {
                    if desktop.paused && !matches!(msg, SocketMessage::TogglePause) {
                        return;
                    }

                    let display_idx = desktop.get_active_display_idx();
                    let d = desktop.displays[display_idx].borrow_mut();

                    info!("handling yattac socket message: {:?}", &msg);
                    match msg {
                        SocketMessage::FocusWindow(direction) => match direction {
                            OperationDirection::Left => d.window_op_left(DirectionOperation::Focus),
                            OperationDirection::Right => {
                                d.window_op_right(DirectionOperation::Focus)
                            }
                            OperationDirection::Up => d.window_op_up(DirectionOperation::Focus),
                            OperationDirection::Down => d.window_op_down(DirectionOperation::Focus),
                            OperationDirection::Previous => {
                                d.window_op_previous(DirectionOperation::Focus)
                            }
                            OperationDirection::Next => d.window_op_next(DirectionOperation::Focus),
                        },
                        SocketMessage::Promote => {
                            let idx = d.get_foreground_window_index();
                            let window = d.windows.remove(idx);
                            d.windows.insert(0, window);
                            d.calculate_layout();
                            d.apply_layout(Option::from(0));
                            let window = d.windows.get(0).unwrap();
                            window.set_cursor_pos(d.layout_dimensions[0]);
                        }
                        SocketMessage::TogglePause => {
                            desktop.paused = !desktop.paused;
                        }
                        SocketMessage::ToggleMonocle => match d.layout {
                            Layout::Monocle => {
                                let idx = d.get_foreground_window_index();
                                if let Some(window) = d.windows.get(idx) {
                                    let window = window.clone();
                                    let last_desktop = LAST_LAYOUT.lock().unwrap();
                                    d.layout = *last_desktop;
                                    d.calculate_layout();
                                    d.apply_layout(None);

                                    // If we have monocle'd a floating window, we want to restore it
                                    // to the default floating position when toggling off monocle
                                    if !window.tile {
                                        let w2 = d.dimensions.width / 2;
                                        let h2 = d.dimensions.height / 2;
                                        let center = Rect {
                                            x:      d.dimensions.x
                                                + ((d.dimensions.width - w2) / 2),
                                            y:      d.dimensions.y
                                                + ((d.dimensions.height - h2) / 2),
                                            width:  w2,
                                            height: h2,
                                        };
                                        window.set_pos(center, None, None);
                                        window.set_cursor_pos(center);
                                    }
                                }
                            }
                            _ => {
                                let mut last_desktop = LAST_LAYOUT.lock().unwrap();
                                *last_desktop = d.layout;

                                d.layout = Layout::Monocle;
                                d.calculate_layout();
                                d.apply_layout(None);
                            }
                        },
                        SocketMessage::ToggleFloat => {
                            let idx = d.get_foreground_window_index();
                            let mut window = d.windows.remove(idx);
                            window.toggle_float();
                            d.windows.insert(idx, window);
                            d.calculate_layout();
                            d.apply_layout(None);

                            // Centre the window if we have disabled tiling
                            if !window.tile {
                                let w2 = d.dimensions.width / 2;
                                let h2 = d.dimensions.height / 2;
                                let center = Rect {
                                    x:      d.dimensions.x + ((d.dimensions.width - w2) / 2),
                                    y:      d.dimensions.y + ((d.dimensions.height - h2) / 2),
                                    width:  w2,
                                    height: h2,
                                };
                                window.set_pos(center, None, None);
                                window.set_cursor_pos(center);
                            } else {
                                // Make sure the mouse cursor goes back once we reenable tiling
                                window.set_cursor_pos(d.layout_dimensions[idx]);
                            }
                        }
                        SocketMessage::Retile => {
                            // Retiling should also rebalance the layout by resetting resizing
                            // adjustments
                            for window in d.windows.iter_mut() {
                                window.resize = None
                            }

                            d.get_foreground_window();
                            d.calculate_layout();
                            let idx = d.foreground_window.index(&d.windows);
                            d.apply_layout(idx);
                        }
                        SocketMessage::MoveWindow(direction) => match direction {
                            OperationDirection::Left => d.window_op_left(DirectionOperation::Move),
                            OperationDirection::Right => {
                                d.window_op_right(DirectionOperation::Move)
                            }
                            OperationDirection::Up => d.window_op_up(DirectionOperation::Move),
                            OperationDirection::Down => d.window_op_down(DirectionOperation::Move),
                            OperationDirection::Previous => {
                                d.window_op_previous(DirectionOperation::Move)
                            }
                            OperationDirection::Next => d.window_op_next(DirectionOperation::Move),
                        },
                        SocketMessage::MoveWindowToDisplay(direction) => {
                            let idx = d.get_foreground_window_index();
                            desktop.move_window_to_display(idx, display_idx, direction);
                        }
                        SocketMessage::MoveWindowToDisplayNumber(target) => {
                            let idx = d.get_foreground_window_index();
                            desktop.move_window_to_display_number(idx, display_idx, target);
                        }
                        SocketMessage::FocusDisplay(direction) => {
                            desktop.focus_display(display_idx, direction);
                        }
                        SocketMessage::FocusDisplayNumber(target) => {
                            desktop.focus_display_number(target);
                        }
                        SocketMessage::ResizeWindow(edge, sizing) => {
                            d.resize_window(edge, sizing, None);
                            d.calculate_layout();
                            d.apply_layout(None);
                        }
                        SocketMessage::GapSize(size) => {
                            d.gaps = size;
                            d.calculate_layout();
                            d.apply_layout(None);
                        }
                        SocketMessage::AdjustGaps(sizing) => {
                            match sizing {
                                Sizing::Increase => {
                                    d.gaps += 1;
                                }
                                Sizing::Decrease => {
                                    if d.gaps > 0 {
                                        d.gaps -= 1;
                                    }
                                }
                            }

                            d.calculate_layout();
                            d.apply_layout(None);
                        }
                        SocketMessage::Layout(layout) => {
                            // Layouts should always start in a balanced state
                            for window in d.windows.iter_mut() {
                                window.resize = None
                            }

                            d.layout = layout;
                            d.calculate_layout();
                            d.apply_layout(None);
                        }
                        SocketMessage::CycleLayout(direction) => {
                            // Layouts should always start in a balanced state
                            for window in d.windows.iter_mut() {
                                window.resize = None
                            }

                            match direction {
                                CycleDirection::Previous => d.layout.previous(),
                                CycleDirection::Next => d.layout.next(),
                            }

                            d.calculate_layout();
                            d.apply_layout(None);
                        }
                        SocketMessage::FloatClass(target) => {
                            let mut float_classes = FLOAT_CLASSES.lock().unwrap();
                            if !float_classes.contains(&target) {
                                float_classes.push(target)
                            }
                        }
                        SocketMessage::FloatExe(target) => {
                            let mut float_exes = FLOAT_EXES.lock().unwrap();
                            if !float_exes.contains(&target) {
                                float_exes.push(target)
                            }
                        }
                        SocketMessage::FloatTitle(target) => {
                            let mut float_titles = FLOAT_TITLES.lock().unwrap();
                            if !float_titles.contains(&target) {
                                float_titles.push(target)
                            }
                        }
                    }
                }
            }
            Err(error) => {
                error!("{}", error);
            }
        }
    }
}
