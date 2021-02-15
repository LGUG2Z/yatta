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
use log::{debug, info};
use strum::Display;

use bindings::windows::win32::{
    system_services::{EVENT_MAX, EVENT_MIN, OBJID_WINDOW},
    windows_accessibility::SetWinEventHook,
    windows_and_messaging::HWND,
};

use crate::{message_loop, window::Window, Message, MESSAGE_CHANNEL};

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
        let message_sender = MESSAGE_CHANNEL.lock().unwrap().0.clone();

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

            info!("starting message loop");
            message_loop::start(|_| {
                if let Ok(event) = WINDOWS_EVENT_CHANNEL.lock().unwrap().1.try_recv() {
                    message_sender
                        .send(Message::WindowsEvent(event))
                        .expect("Failed to send WinEvent");
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

    let window = Window {
        hwnd,
        should_tile: true,
    };

    let event_code = unsafe { ::std::mem::transmute(event) };
    let event_type = match WindowsEventType::from_event_code(event_code) {
        Some(event) => event,
        None => {
            // Some apps like Firefox don't send ObjectCreate or ObjectShow on launch
            // This spams the message queue, but I don't know what else to do. On launch
            // it only sends the following WinEvents :/
            //
            // [yatta\src\windows_event.rs:110] event = 32780
            // [yatta\src\windows_event.rs:111] event_code = ObjectNameChange
            // [yatta\src\windows_event.rs:110] event = 32779
            // [yatta\src\windows_event.rs:111] event_code = ObjectLocationChange
            // [yatta\src\windows_event.rs:110] event = 32779
            // [yatta\src\windows_event.rs:111] event_code = ObjectLocationChange
            // [yatta\src\windows_event.rs:110] event = 32780
            // [yatta\src\windows_event.rs:111] event_code = ObjectNameChange
            if event_code == WinEventCode::ObjectNameChange
                && window.is_visible()
                && window.title().is_some()
                && window.title().unwrap().contains("Firefox")
            {
                WindowsEventType::Show
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

        // Need to expand this blacklist of windows that aren't visible but end up
        // forcing a redraw of the workspace

        // Not sure if this is needed after fixing the should_manage and is_cloaked fns
        let blacklist = vec![
            Some(String::from("Task Host Window")),
            Some(String::from("nsAppShell:EventWindow")), // Firefox
            Some(String::from("Firefox Media Keys")),     // Firefox
            Some(String::from("XCP")),
            Some(String::from("MCI command handling window")),
            Some(String::from("yatta")), // AHK script name
            Some(String::from("CSpNotify Notify Window")), // When starting Signal
            Some(String::from("Perfdisk PNP Window")),
            Some(String::from("Location Notification")),
            Some(String::from("Discord Updater")), // When starting Discord
        ];

        if !blacklist.contains(&window.title()) {
            WINDOWS_EVENT_CHANNEL
                .lock()
                .unwrap()
                .0
                .send(event)
                .expect("Failed to forward WindowsEvent");
        }
    } else {
        debug!("ignored event from {:?} {}", window.title(), event_code);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
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
            WinEventCode::ObjectShow
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
    ObjectDefactionChange                 = 0x8011,
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
