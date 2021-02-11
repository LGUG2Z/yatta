use std::io::Write;

use clap::Clap;
use uds_windows::UnixStream;

use yatta_core::{OperationDirection, Sizing, SocketMessage};

#[derive(Clap)]
#[clap(version = "1.0", author = "Jade I. <jadeiqbal@fastmail.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Focus(OperationDirection),
    Move(OperationDirection),
    Promote,
    TogglePause,
    ToggleFloat,
    Retile,
    SetGapSize(Gaps),
    AdjustGaps(Sizing),
}

#[derive(Clap)]
struct Gaps {
    pub size: i32,
}

fn main() {
    let opts: Opts = Opts::parse();

    let mut socket = dirs::home_dir().unwrap();
    socket.push("yatta.sock");
    let socket = socket.as_path();

    match opts.subcmd {
        SubCommand::Focus(direction) => {
            let mut stream = match UnixStream::connect(&socket) {
                Err(_) => panic!("server is not running"),
                Ok(stream) => stream,
            };

            let bytes = SocketMessage::FocusWindow(direction).as_bytes().unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
        SubCommand::Promote => {
            let mut stream = match UnixStream::connect(&socket) {
                Err(_) => panic!("server is not running"),
                Ok(stream) => stream,
            };

            let bytes = SocketMessage::Promote.as_bytes().unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
        SubCommand::TogglePause => {
            let mut stream = match UnixStream::connect(&socket) {
                Err(_) => panic!("server is not running"),
                Ok(stream) => stream,
            };

            let bytes = SocketMessage::TogglePause.as_bytes().unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
        SubCommand::Retile => {
            let mut stream = match UnixStream::connect(&socket) {
                Err(_) => panic!("server is not running"),
                Ok(stream) => stream,
            };

            let bytes = SocketMessage::Retile.as_bytes().unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
        SubCommand::Move(direction) => {
            let mut stream = match UnixStream::connect(&socket) {
                Err(_) => panic!("server is not running"),
                Ok(stream) => stream,
            };

            let bytes = SocketMessage::MoveWindow(direction).as_bytes().unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
        SubCommand::SetGapSize(gaps) => {
            let mut stream = match UnixStream::connect(&socket) {
                Err(_) => panic!("server is not running"),
                Ok(stream) => stream,
            };

            let bytes = SocketMessage::SetGapSize(gaps.size).as_bytes().unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
        SubCommand::AdjustGaps(sizing) => {
            let mut stream = match UnixStream::connect(&socket) {
                Err(_) => panic!("server is not running"),
                Ok(stream) => stream,
            };

            let bytes = SocketMessage::AdjustGaps(sizing).as_bytes().unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
        SubCommand::ToggleFloat => {
            let mut stream = match UnixStream::connect(&socket) {
                Err(_) => panic!("server is not running"),
                Ok(stream) => stream,
            };

            let bytes = SocketMessage::ToggleFloat.as_bytes().unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
    }
}
