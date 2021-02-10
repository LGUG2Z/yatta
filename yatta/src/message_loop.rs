use std::{thread, time::Duration};

use bindings::windows::win32::{
    system_services::PM_REMOVE,
    windows_and_messaging::{DispatchMessageW, PeekMessageW, TranslateMessage, HWND, MSG},
};

pub fn start(cb: impl Fn(Option<MSG>) -> bool) {
    start_with_sleep(10, cb);
}

pub fn start_with_sleep(sleep: u64, cb: impl Fn(Option<MSG>) -> bool) {
    let mut msg: MSG = MSG::default();
    loop {
        let mut value: Option<MSG> = None;
        unsafe {
            if !bool::from(!PeekMessageW(&mut msg, HWND(0), 0, 0, PM_REMOVE as u32)) {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);

                value = Some(msg.clone());
            }
        }

        thread::sleep(Duration::from_millis(sleep));

        if !cb(value) {
            break;
        }
    }
}
