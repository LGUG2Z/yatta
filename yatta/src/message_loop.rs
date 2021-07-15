use std::{thread, time::Duration};

use bindings::Windows::Win32::WindowsAndMessaging::{
    DispatchMessageW,
    PeekMessageW,
    TranslateMessage,
    HWND,
    MSG,
    PEEK_MESSAGE_REMOVE_TYPE,
};

pub fn start(cb: impl Fn(Option<MSG>) -> bool) {
    start_with_sleep(10, cb);
}

pub fn start_with_sleep(sleep: u64, cb: impl Fn(Option<MSG>) -> bool) {
    let mut msg: MSG = MSG::default();
    loop {
        let mut value: Option<MSG> = None;
        unsafe {
            if !bool::from(!PeekMessageW(
                &mut msg,
                HWND(0),
                0,
                0,
                PEEK_MESSAGE_REMOVE_TYPE::PM_REMOVE,
            )) {
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
