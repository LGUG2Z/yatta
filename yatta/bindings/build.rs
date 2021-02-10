fn main() {
    windows::build!(
        windows::win32::display_devices::*,
        windows::win32::dwm::*,
        windows::win32::gdi::*,
        windows::win32::menus_and_resources::*,
        windows::win32::system_services::*,
        windows::win32::keyboard_and_mouse_input::*,
        windows::win32::windows_accessibility::*,
        windows::win32::windows_and_messaging::*,
    );
}
