#[macro_use] extern crate num_derive;
extern crate num_traits;

use std::{
    io::{BufRead, BufReader, ErrorKind},
    sync::{Arc, Mutex},
    thread,
};

use crossbeam_channel::{select, unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use uds_windows::UnixListener;

use crate::{
    windows_event::{WindowsEvent, WindowsEventListener, WindowsEventType},
    workspace::Workspace,
};
use anyhow::Result;
use log::{error, info};
use yatta_core::SocketMessage;

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
                    if window.0 == ev.window.0 {
                        contains = true;
                    }
                }

                if !contains {
                    let idx = workspace.get_foreground_window_index();
                    workspace.windows.insert(idx + 1, ev.window);
                    workspace.calculate_layout();
                    workspace.apply_layout(None);
                } else {
                    info!(
                        "did not retile on show event as window is already shown: {:?}",
                        &ev
                    );
                }
            }
        }
        WindowsEventType::Hide | WindowsEventType::Destroy => {
            let index = ev.window.get_index(&workspace.windows);

            workspace.windows.retain(|x| !ev.window.eq(x));
            workspace.calculate_layout();
            workspace.apply_layout(index);
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
                        SocketMessage::FocusWindow(direction) => {
                            let idx = workspace.get_foreground_window_index();
                            let new_idx = direction.destination(idx, workspace.windows.len());
                            workspace.windows.get(new_idx).unwrap().set_foreground();
                            workspace.calculate_layout();
                            let window = workspace.windows.get(new_idx).unwrap();

                            // Mouse follows focus
                            window.set_cursor_pos(workspace.layout[new_idx]);
                        }
                        SocketMessage::SwapWindow(direction) => {
                            let idx = workspace.get_foreground_window_index();
                            let new_idx = direction.destination(idx, workspace.windows.len());
                            workspace.windows.swap(idx, new_idx);
                            workspace.calculate_layout();
                            workspace.apply_layout(Option::from(new_idx));

                            let window = workspace.windows.get(new_idx).unwrap();
                            window.set_cursor_pos(workspace.layout[new_idx]);
                        }
                        SocketMessage::Promote => {
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
                        SocketMessage::ReTile => {
                            workspace.get_visible_windows();
                            workspace.get_foreground_window();
                            workspace.calculate_layout();
                            let idx = workspace.foreground_window.get_index(&workspace.windows);
                            workspace.apply_layout(idx);
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
