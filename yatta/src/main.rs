#[macro_use] extern crate num_derive;
extern crate num_traits;

use std::{
    io::{BufRead, BufReader, ErrorKind},
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Result;
use crossbeam_channel::{select, unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use log::{debug, error, info};
use uds_windows::UnixListener;

use yatta_core::{OperationDirection, Orientation, Sizing, SocketMessage};

use crate::{
    rect::Rect,
    windows_event::{WindowsEvent, WindowsEventListener, WindowsEventType},
    workspace::Workspace,
};

mod message_loop;
mod rect;
mod window;
mod windows_event;
mod workspace;

lazy_static! {
    static ref MESSAGE_CHANNEL: Arc<Mutex<(Sender<Message>, Receiver<Message>)>> =
        Arc::new(Mutex::new(unbounded()));
}

#[derive(Clone, Debug)]
pub enum Message {
    WindowsEvent(WindowsEvent),
}

fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "INFO");
    env_logger::init();
    let workspace: Arc<Mutex<Workspace>> = Arc::new(Mutex::new(Workspace::default()));
    info!("loaded workspace: {:?}", &workspace.lock().unwrap());

    let listener = Arc::new(Mutex::new(WindowsEventListener::default()));
    listener.lock().unwrap().start();

    let mut socket = dirs::home_dir().unwrap();
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
        Err(err) => {
            dbg!(err);
            panic!("failed to bind socket")
        }
        Ok(stream) => stream,
    };

    info!("starting listening on socket: {}", socket.to_str().unwrap());

    let workspace_clone = workspace.clone();
    thread::spawn(move || {
        for client in stream.incoming() {
            match client {
                Ok(stream) => {
                    let ls = Arc::clone(&listener);
                    handle_socket_message(stream, &workspace_clone, ls);
                }
                Err(err) => {
                    println!("Error: {}", err);
                    break;
                }
            }
        }
    });

    let message_receiver = MESSAGE_CHANNEL.lock().unwrap().1.clone();

    loop {
        select! {
                recv(message_receiver) -> maybe_msg => {
                    let msg = maybe_msg.unwrap();
                    let _ = match msg {
                        Message::WindowsEvent(ev) => {
                            let ws = Arc::clone(&workspace) ;
                            handle_windows_event_message(ev, ws)
                    }
                };
            }
        }
    }
}

fn handle_windows_event_message(ev: WindowsEvent, workspace: Arc<Mutex<Workspace>>) {
    let mut workspace = workspace.lock().unwrap();
    info!("handling windows event: {:?}", &ev);
    match ev.event_type {
        WindowsEventType::Show => {
            if workspace.windows.is_empty() {
                workspace.windows.push(ev.window);
                workspace.calculate_layout();
                workspace.apply_layout(None);
            } else {
                // Some apps like Windows Terminal send multiple Events on startup, we don't
                // want dupes
                let mut contains = false;

                for window in &workspace.windows {
                    if window.hwnd == ev.window.hwnd {
                        contains = true;
                    }
                }

                if !contains {
                    let idx = workspace.get_foreground_window_index();
                    workspace.windows.insert(idx + 1, ev.window);
                    workspace.calculate_layout();
                    workspace.apply_layout(None);
                } else {
                    debug!(
                        "did not retile on show event as window is already shown: {:?}",
                        &ev
                    );
                }
            }
        }
        WindowsEventType::Hide | WindowsEventType::Destroy => {
            let index = ev.window.get_index(&workspace.windows);
            let mut previous = index.unwrap_or(0);
            previous = if previous == 0 { 0 } else { previous - 1 };

            workspace.windows.retain(|x| !ev.window.eq(x));
            workspace.calculate_layout();
            workspace.apply_layout(Option::from(previous));
        }
        WindowsEventType::FocusChange => {
            let mut current = workspace.windows.clone();
            workspace.get_visible_windows();
            current.retain(|x| workspace.windows.contains(x));
            workspace.windows = current;
            workspace.calculate_layout();
            workspace.apply_layout(None);

            workspace.foreground_window = ev.window;
        }
    }
}

pub enum DirectionOperation {
    Focus,
    Move,
}

impl DirectionOperation {
    pub fn handle(self, workspace: &mut Workspace, idx: usize, new_idx: usize) {
        match self {
            DirectionOperation::Focus => {
                workspace.windows.get(new_idx).unwrap().set_foreground();
                workspace.calculate_layout();
            }
            DirectionOperation::Move => {
                workspace.windows.swap(idx, new_idx);
                workspace.calculate_layout();
                workspace.apply_layout(Option::from(new_idx));
            }
        }

        workspace.follow_focus_with_mouse(new_idx);
    }
}

fn handle_socket_message(
    stream: uds_windows::UnixStream,
    workspace: &Arc<Mutex<Workspace>>,
    _listener: Arc<Mutex<WindowsEventListener>>,
) {
    let mut workspace = workspace.lock().unwrap();
    let stream = BufReader::new(stream);
    for line in stream.lines() {
        match line {
            Ok(socket_msg) => {
                if let Ok(msg) = SocketMessage::from_str(&socket_msg) {
                    info!("handling socket message: {:?}", &msg);
                    match msg {
                        SocketMessage::FocusWindow(direction) => match direction {
                            OperationDirection::Left => {
                                workspace.window_op_left(DirectionOperation::Focus)
                            }
                            OperationDirection::Right => {
                                workspace.window_op_right(DirectionOperation::Focus)
                            }
                            OperationDirection::Up => {
                                workspace.window_op_up(DirectionOperation::Focus)
                            }
                            OperationDirection::Down => {
                                workspace.window_op_down(DirectionOperation::Focus)
                            }
                            OperationDirection::Previous => {
                                workspace.window_op_previous(DirectionOperation::Focus)
                            }
                            OperationDirection::Next => {
                                workspace.window_op_next(DirectionOperation::Focus)
                            }
                        },
                        SocketMessage::Promote => {
                            workspace.get_foreground_window();
                            let idx = workspace.get_foreground_window_index();
                            let window = workspace.windows.remove(idx);
                            workspace.windows.insert(0, window);
                            workspace.calculate_layout();
                            workspace.apply_layout(Option::from(0));
                            let window = workspace.windows.get(0).unwrap();
                            window.set_cursor_pos(workspace.layout[0]);
                        }
                        SocketMessage::TogglePause => {
                            unimplemented!();
                        }
                        SocketMessage::ToggleFloat => {
                            let idx = workspace.get_foreground_window_index();
                            let mut window = workspace.windows.remove(idx);
                            window.toggle_float();
                            workspace.windows.insert(idx, window);
                            workspace.calculate_layout();
                            workspace.apply_layout(None);
                            // Centre the window if we have disabled tiling
                            if !window.should_tile {
                                let w2 = workspace.dimensions.width / 2;
                                let h2 = workspace.dimensions.height / 2;
                                let center = Rect {
                                    x:      workspace.dimensions.x
                                        + ((workspace.dimensions.width - w2) / 2),
                                    y:      workspace.dimensions.y
                                        + ((workspace.dimensions.height - h2) / 2),
                                    width:  w2,
                                    height: h2,
                                };
                                window.set_pos(center, None, None);
                                window.set_cursor_pos(center);
                            } else {
                                // Make sure the mouse cursor goes back once we reenable tiling
                                window.set_cursor_pos(workspace.layout[idx]);
                            }
                        }
                        SocketMessage::Retile => {
                            workspace.get_visible_windows();
                            workspace.get_foreground_window();
                            workspace.calculate_layout();
                            let idx = workspace.foreground_window.get_index(&workspace.windows);
                            workspace.apply_layout(idx);
                        }
                        SocketMessage::MoveWindow(direction) => match direction {
                            OperationDirection::Left => {
                                workspace.window_op_left(DirectionOperation::Move)
                            }
                            OperationDirection::Right => {
                                workspace.window_op_right(DirectionOperation::Move)
                            }
                            OperationDirection::Up => {
                                workspace.window_op_up(DirectionOperation::Move)
                            }
                            OperationDirection::Down => {
                                workspace.window_op_down(DirectionOperation::Move)
                            }
                            OperationDirection::Previous => {
                                workspace.window_op_previous(DirectionOperation::Move)
                            }
                            OperationDirection::Next => {
                                workspace.window_op_next(DirectionOperation::Move)
                            }
                        },
                        SocketMessage::SetGapSize(size) => {
                            workspace.set_gaps(size);
                            workspace.calculate_layout();
                            workspace.apply_layout(None);
                        }
                        SocketMessage::AdjustGaps(sizing) => {
                            let gaps = workspace.gaps;

                            match sizing {
                                Sizing::Increase => {
                                    workspace.set_gaps(gaps + 1);
                                }
                                Sizing::Decrease => {
                                    workspace.set_gaps(gaps - 1);
                                }
                            }

                            workspace.calculate_layout();
                            workspace.apply_layout(None);
                        }
                        SocketMessage::SetOrientation(orientation) => {
                            workspace.orientation = orientation;
                            workspace.calculate_layout();
                            workspace.apply_layout(None);
                        }
                        SocketMessage::ToggleOrientation => {
                            workspace.orientation = match workspace.orientation {
                                Orientation::Horizontal => Orientation::Vertical,
                                Orientation::Vertical => Orientation::Horizontal,
                            };

                            workspace.calculate_layout();
                            workspace.apply_layout(None);
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
