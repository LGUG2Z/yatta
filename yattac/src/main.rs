use std::io::Write;

use clap::Clap;
use uds_windows::UnixStream;

use yatta_core::{CycleDirection, Layout, OperationDirection, ResizeEdge, Sizing, SocketMessage};

#[derive(Clap)]
#[clap(version = "1.0", author = "Jade I. <jadeiqbal@fastmail.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    AdjustGaps(Sizing),
    Focus(OperationDirection),
    Move(OperationDirection),
    Resize(Resize),
    MoveToDisplay(CycleDirection),
    MoveToDisplayNumber(DisplayNumber),
    FocusDisplay(CycleDirection),
    FocusDisplayNumber(DisplayNumber),
    Promote,
    Retile,
    GapSize(Gap),
    Layout(Layout),
    CycleLayout(CycleDirection),
    ToggleFloat,
    TogglePause,
    ToggleMonocle,
    Start,
    Stop,
    FloatClass(FloatTarget),
    FloatExe(FloatTarget),
    FloatTitle(FloatTarget),
    SetWorkspace(WorkspaceIndex),
    MoveWindowToWorkspace(WorkspaceIndex),
    MoveWindowToWorkspaceAndFollow(WorkspaceIndex)
}

#[derive(Clap)]
struct Resize {
    edge:   ResizeEdge,
    sizing: Sizing,
}

#[derive(Clap)]
struct Gap {
    size: i32,
}

#[derive(Clap)]
struct WorkspaceIndex {
    index: usize
}

#[derive(Clap)]
struct DisplayNumber {
    target: usize,
}

#[derive(Clap)]
struct FloatTarget {
    id: String,
}

pub fn send_message(bytes: &[u8]) {
    let mut socket = dirs::home_dir().unwrap();
    socket.push("yatta.sock");
    let socket = socket.as_path();

    let mut stream = match UnixStream::connect(&socket) {
        Err(_) => panic!("server is not running"),
        Ok(stream) => stream,
    };

    if stream.write_all(&*bytes).is_err() {
        panic!("couldn't send message")
    }
}

fn main() {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Focus(direction) => {
            let bytes = SocketMessage::FocusWindow(direction).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::Promote => {
            let bytes = SocketMessage::Promote.as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::TogglePause => {
            let bytes = SocketMessage::TogglePause.as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::Retile => {
            let bytes = SocketMessage::Retile.as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::Move(direction) => {
            let bytes = SocketMessage::MoveWindow(direction).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::Resize(resize) => {
            let bytes = SocketMessage::ResizeWindow(resize.edge, resize.sizing)
                .as_bytes()
                .unwrap();
            send_message(&*bytes);
        }
        SubCommand::MoveToDisplay(direction) => {
            let bytes = SocketMessage::MoveWindowToDisplay(direction)
                .as_bytes()
                .unwrap();
            send_message(&*bytes);
        }
        SubCommand::MoveToDisplayNumber(display_number) => {
            let bytes = SocketMessage::MoveWindowToDisplayNumber(display_number.target)
                .as_bytes()
                .unwrap();
            send_message(&*bytes);
        }
        SubCommand::FocusDisplay(direction) => {
            let bytes = SocketMessage::FocusDisplay(direction).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::FocusDisplayNumber(display_number) => {
            let bytes = SocketMessage::FocusDisplayNumber(display_number.target)
                .as_bytes()
                .unwrap();
            send_message(&*bytes);
        }
        SubCommand::GapSize(gap) => {
            let bytes = SocketMessage::GapSize(gap.size).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::AdjustGaps(sizing) => {
            let bytes = SocketMessage::AdjustGaps(sizing).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::ToggleFloat => {
            let bytes = SocketMessage::ToggleFloat.as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::ToggleMonocle => {
            let bytes = SocketMessage::ToggleMonocle.as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::Layout(layout) => {
            let bytes = SocketMessage::Layout(layout).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::CycleLayout(direction) => {
            let bytes = SocketMessage::CycleLayout(direction).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::Start => {
            let script = r#"Start-Process yatta -WindowStyle hidden"#;
            match powershell_script::run(script, true) {
                Ok(output) => {
                    println!("{}", output);
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        SubCommand::Stop => {
            let bytes = SocketMessage::Stop.as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::FloatClass(target) => {
            let bytes = SocketMessage::FloatClass(target.id).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::FloatExe(target) => {
            let bytes = SocketMessage::FloatExe(target.id).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::FloatTitle(target) => {
            let bytes = SocketMessage::FloatTitle(target.id).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::SetWorkspace(index) => {
            let bytes = SocketMessage::SetWorkspace(index.index).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::MoveWindowToWorkspace(index) => {
            let bytes = SocketMessage::MoveWindowToWorkspace(index.index).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::MoveWindowToWorkspaceAndFollow(index) => {
            let bytes = SocketMessage::MoveWindowToWorkspaceAndFollow(index.index).as_bytes().unwrap();
            send_message(&*bytes);
        }
    }
}
