use std::io::Write;

use clap::Clap;
use uds_windows::UnixStream;

use yatta_core::{CycleDirection, Layout, OperationDirection, Sizing, SocketMessage};

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
    GapSize(Gap),
    Layout(Layout),
    CycleLayout(CycleDirection),
    ToggleFloat,
    TogglePause,
    Start,
    Stop,
    FloatClass(Target),
    FloatExe(Target),
}

#[derive(Clap)]
struct Gap {
    size: i32,
}

#[derive(Clap)]
struct Target {
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
            let script = r#"Stop-Process -Name yatta"#;
            match powershell_script::run(script, true) {
                Ok(output) => {
                    println!("{}", output);
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }
        SubCommand::FloatClass(target) => {
            let bytes = SocketMessage::FloatClass(target.id).as_bytes().unwrap();
            send_message(&*bytes);
        }
        SubCommand::FloatExe(target) => {
            let bytes = SocketMessage::FloatExe(target.id).as_bytes().unwrap();
            send_message(&*bytes);
        }
    }
}
