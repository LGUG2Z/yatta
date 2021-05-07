fn main() {
    windows::build!(
        Windows::Win32::DisplayDevices::{POINT, RECT},
        Windows::Win32::Dwm::*,
        Windows::Win32::Gdi::*,
        Windows::Win32::SystemServices::{
            PROCESS_ACCESS_RIGHTS,
            BOOL,
            PWSTR,
            QueryFullProcessImageNameW,
            PROCESS_NAME_FORMAT,
            OpenProcess
        },
        Windows::Win32::KeyboardAndMouseInput::SetFocus,
        Windows::Win32::WindowsAccessibility::{SetWinEventHook, HWINEVENTHOOK},
        Windows::Win32::WindowsAndMessaging::*,
    );
}
