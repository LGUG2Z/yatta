use std::io::Write;

use clap::Clap;
use uds_windows::UnixStream;

use yatta_core::{OperationDirection, Orientation, Sizing, SocketMessage};

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
    Promote,
    Retile,
    SetGapSize(Gap),
    SetOrientation(Orientation),
    ToggleFloat,
    ToggleOrientation,
    TogglePause,
}

#[derive(Clap)]
struct Gap {
    size: i32,
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
        SubCommand::SetGapSize(gap) => {
            let bytes = SocketMessage::SetGapSize(gap.size).as_bytes().unwrap();
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
        SubCommand::SetOrientation(orientation) => {
            let bytes = SocketMessage::SetOrientation(orientation)
                .as_bytes()
                .unwrap();
            send_message(&*bytes);
        }
        SubCommand::ToggleOrientation => {
            let bytes = SocketMessage::ToggleOrientation.as_bytes().unwrap();
            send_message(&*bytes);
        }
    }
}
