use std::io::Write;

use clap::Clap;
use uds_windows::UnixStream;

use yatta_core::{OperationDirection, SocketMessage};

#[derive(Clap)]
#[clap(version = "1.0", author = "Jade I. <jadeiqbal@fastmail.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Focus(Direction),
    Move(Direction),
    Promote,
    TogglePause,
    Retile,
}

#[derive(Clap)]
struct Direction {
    pub direction: OperationDirection,
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

            let bytes = SocketMessage::FocusWindow(direction.direction)
                .as_bytes()
                .unwrap();

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

            let bytes = SocketMessage::ReTile.as_bytes().unwrap();

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

            let bytes = SocketMessage::MoveWindow(direction.direction)
                .as_bytes()
                .unwrap();

            match stream.write_all(&*bytes) {
                Err(_) => panic!("couldn't send message"),
                Ok(_) => {}
            }
        }
    }
}
