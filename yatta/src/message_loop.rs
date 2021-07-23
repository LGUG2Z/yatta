use std::{thread, time::Duration};

use bindings::Windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE},
};

pub fn start(cb: impl Fn(Option<MSG>) -> bool) {
    start_with_sleep(10, cb);
}

pub fn start_with_sleep(sleep: u64, cb: impl Fn(Option<MSG>) -> bool) {
    let mut msg: MSG = MSG::default();
    loop {
        let mut value: Option<MSG> = None;
        unsafe {
            if !bool::from(!PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE)) {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                value = Some(msg);
            }
        }

        thread::sleep(Duration::from_millis(sleep));

        if !cb(value) {
            break;
        }
    }
}
