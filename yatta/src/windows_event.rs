use std::{
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc,
        Mutex,
    },
    thread,
    time::Duration,
};

use crossbeam_channel::{unbounded, Receiver, Sender};
use lazy_static::lazy_static;
use log::{error, info};
use strum::Display;

use bindings::windows::win32::{
    system_services::{EVENT_MAX, EVENT_MIN, OBJID_WINDOW},
    windows_accessibility::SetWinEventHook,
    windows_and_messaging::HWND,
};

use crate::{
    message_loop,
    window::{exe_name_from_path, Window},
    Message,
    YATTA_CHANNEL,
};

lazy_static! {
    static ref WINDOWS_EVENT_CHANNEL: Arc<Mutex<(Sender<WindowsEvent>, Receiver<WindowsEvent>)>> =
        Arc::new(Mutex::new(unbounded()));
}

#[derive(Debug, Clone)]
pub struct WindowsEventListener {
    hook: Arc<AtomicIsize>,
}

impl Default for WindowsEventListener {
    fn default() -> Self {
        Self {
            hook: Arc::new(AtomicIsize::new(0)),
        }
    }
}

impl WindowsEventListener {
    pub fn start(&self) {
        let hook = self.hook.clone();
        let yatta_sender = YATTA_CHANNEL.lock().unwrap().0.clone();

        thread::spawn(move || unsafe {
            let hook_ref = SetWinEventHook(
                EVENT_MIN as u32,
                EVENT_MAX as u32,
                0,
                Some(handler),
                0,
                0,
                0,
            );

            hook.store(hook_ref, Ordering::SeqCst);

            info!("starting windows event listener");
            message_loop::start(|_| {
                if let Ok(event) = WINDOWS_EVENT_CHANNEL.lock().unwrap().1.try_recv() {
                    match yatta_sender.send(Message::WindowsEvent(event)) {
                        Ok(_) => {}
                        Err(error) => {
                            error!("could not send windows event to yatta channel: {}", error)
                        }
                    }
                }

                thread::sleep(Duration::from_millis(10));

                true
            });
        });
    }
}

extern "system" fn handler(
    _h_win_event_hook: isize,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if id_object != OBJID_WINDOW {
        return;
    }

    let window = Window { hwnd, tile: true };

    let event_code = unsafe { ::std::mem::transmute(event) };
    let event_type = match WindowsEventType::from_event_code(event_code) {
        Some(event) => event,
        None => {
            // Some apps like Firefox don't send ObjectCreate or ObjectShow on launch
            // This spams the message queue, but I don't know what else to do. On launch
            // it only sends the following WinEvents :/
            //
            // [yatta\src\windows_event.rs:110] event = 32780 ObjectNameChange
            // [yatta\src\windows_event.rs:110] event = 32779 ObjectLocationChange
            let object_name_change_on_launch = vec!["firefox.exe", "idea64.exe"];
            if let Ok(path) = window.exe_path() {
                if event_code == WinEventCode::ObjectNameChange {
                    if object_name_change_on_launch.contains(&&*exe_name_from_path(&path)) {
                        WindowsEventType::Show
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            }
        }
    };

    if window.should_manage(Option::from(event_type)) {
        let event = WindowsEvent {
            event_type,
            event_code,
            window,
            title: window.title(),
        };

        WINDOWS_EVENT_CHANNEL
            .lock()
            .unwrap()
            .0
            .send(event)
            .expect("Failed to forward WindowsEvent");
    }
}

#[derive(Clone, Copy, Debug, Display, PartialEq)]
pub enum WindowsEventType {
    Destroy,
    FocusChange,
    Hide,
    Show,
}

impl WindowsEventType {
    pub fn from_event_code(event_code: WinEventCode) -> Option<Self> {
        match event_code {
            WinEventCode::ObjectDestroy => Some(Self::Destroy),

            WinEventCode::ObjectCloaked
            | WinEventCode::ObjectHide
            | WinEventCode::SystemMinimizeStart => Some(Self::Hide),

            // WinEventCode::ObjectCreate |
            WinEventCode::SystemDesktopSwitch
            | WinEventCode::ObjectShow
            | WinEventCode::ObjectUncloaked
            | WinEventCode::SystemMinimizeEnd => Some(Self::Show),

            WinEventCode::ObjectFocus | WinEventCode::SystemForeground => Some(Self::FocusChange),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WindowsEvent {
    pub event_type: WindowsEventType,
    pub event_code: WinEventCode,
    pub window:     Window,
    pub title:      Option<String>,
}

#[derive(Clone, Copy, FromPrimitive, ToPrimitive, PartialEq, Display, Debug)]
#[repr(u32)]
#[allow(dead_code)]
pub enum WinEventCode {
    ObjectAcceleratorChange               = 0x8012,
    ObjectCloaked                         = 0x8017,
    ObjectContentScrolled                 = 0x8015,
    ObjectCreate                          = 0x8000,
    ObjectDefActionChange                 = 0x8011,
    ObjectDescriptionChange               = 0x800D,
    ObjectDestroy                         = 0x8001,
    ObjectDragStart                       = 0x8021,
    ObjectDragCancel                      = 0x8022,
    ObjectDragComplete                    = 0x8023,
    ObjectDragEnter                       = 0x8024,
    ObjectDragLeave                       = 0x8025,
    ObjectDragDropped                     = 0x8026,
    ObjectEnd                             = 0x80FF,
    ObjectFocus                           = 0x8005,
    ObjectHelpChange                      = 0x8010,
    ObjectHide                            = 0x8003,
    ObjectHostedObjectsInvalidated        = 0x8020,
    ObjectImeHide                         = 0x8028,
    ObjectImeShow                         = 0x8027,
    ObjectImeChange                       = 0x8029,
    ObjectInvoked                         = 0x8013,
    ObjectLiveRegionChanged               = 0x8019,
    ObjectLocationChange                  = 0x800B,
    ObjectNameChange                      = 0x800C,
    ObjectParentChange                    = 0x800F,
    ObjectReorder                         = 0x8004,
    ObjectSelection                       = 0x8006,
    ObjectSelectionAdd                    = 0x8007,
    ObjectSelectionRemove                 = 0x8008,
    ObjectSelectionWithin                 = 0x8009,
    ObjectShow                            = 0x8002,
    ObjectStateChange                     = 0x800A,
    ObjectTextEditConversionTargetChanged = 0x8030,
    ObjectTextSelectionChanged            = 0x8014,
    ObjectUncloaked                       = 0x8018,
    ObjectValueChange                     = 0x800E,
    SystemAlert                           = 0x0002,
    SystemArrangementPreview              = 0x8016,
    SystemCaptureEnd                      = 0x0009,
    SystemCaptureStart                    = 0x0008,
    SystemContextHelpEnd                  = 0x000D,
    SystemContextHelpStart                = 0x000C,
    SystemDesktopSwitch                   = 0x0020,
    SystemDialogEnd                       = 0x0011,
    SystemDialogStart                     = 0x0010,
    SystemDragDropEnd                     = 0x000F,
    SystemDragDropStart                   = 0x000E,
    SystemEnd                             = 0x00FF,
    SystemForeground                      = 0x0003,
    SystemMenuPopupEnd                    = 0x0007,
    SystemMenuPopupStart                  = 0x0006,
    SystemMenuEnd                         = 0x0005,
    SystemMenuStart                       = 0x0004,
    SystemMinimizeEnd                     = 0x0017,
    SystemMinimizeStart                   = 0x0016,
    SystemMoveSizeEnd                     = 0x000B,
    SystemMoveSizeStart                   = 0x000A,
    SystemScrollingEnd                    = 0x0013,
    SystemScrollingStart                  = 0x0012,
    SystemSound                           = 0x0001,
    SystemSwitchEnd                       = 0x0015,
    SystemSwitchStart                     = 0x0014,
}
