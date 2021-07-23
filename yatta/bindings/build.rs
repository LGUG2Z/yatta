fn main() {
    windows::build!(
        Windows::Win32::Foundation::{
            POINT,
            RECT,
            BOOL,
            PWSTR,
            HWND,
            LPARAM,
        },
        Windows::Win32::Graphics::Dwm::*,
        Windows::Win32::Graphics::Gdi::*,
        Windows::Win32::System::Threading::{
            PROCESS_ACCESS_RIGHTS,
            PROCESS_NAME_FORMAT,
            OpenProcess,
            QueryFullProcessImageNameW,
        },
        Windows::Win32::UI::KeyboardAndMouseInput::SetFocus,
        Windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK},
        Windows::Win32::UI::WindowsAndMessaging::*,
    );
}
