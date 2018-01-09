//! Windows-specific types for signal handling.
//!
//! This module is only defined on Windows and contains the primary `Event` type
//! for receiving notifications of events. These events are listened for via the
//! `SetConsoleCtrlHandler` function which receives events of the type
//! `CTRL_C_EVENT` and `CTRL_BREAK_EVENT`

#![cfg(windows)]
extern crate winapi;

use std::io;
use std::ops::Deref;
use std::sync::{Once, ONCE_INIT};

use may::sync::Mutex;
use may::sync::mpsc::{self, Receiver, Sender};
use self::winapi::shared::minwindef::*;

pub use self::winapi::um::wincon::{CTRL_BREAK_EVENT, CTRL_C_EVENT};

extern "system" {
    fn SetConsoleCtrlHandler(HandlerRoutine: usize, Add: BOOL) -> BOOL;
}

// Number of different windows signals
// only CTRL_C and CTRL_BREAK supported
const SIGNUM: usize = 2;
const CTRL_C_SLOT: usize = 0;
const CTRL_BREAK_SLOT: usize = 1;

struct SignalInfo {
    // The ones interested in this signal
    recipients: Mutex<Vec<Box<Sender<()>>>>,
}

impl Default for SignalInfo {
    fn default() -> SignalInfo {
        SignalInfo {
            recipients: Mutex::new(Vec::new()),
        }
    }
}

struct Globals {
    signals: [SignalInfo; SIGNUM],
}

static mut GLOBALS: *mut Globals = 0 as *mut Globals;

fn globals() -> &'static Globals {
    static INIT: Once = ONCE_INIT;

    unsafe {
        INIT.call_once(|| {
            let globals = Globals {
                signals: Default::default(),
            };
            GLOBALS = Box::into_raw(Box::new(globals));
        });

        let rc = SetConsoleCtrlHandler(handler as usize, TRUE);
        if rc == 0 {
            Box::from_raw(GLOBALS);
            GLOBALS = 0 as *mut _;
            // return Err(io::Error::last_os_error())
            panic!("failed to set console handler");
        }

        &*GLOBALS
    }
}

/// global signal handler for CTRL_C and CTRL_BREAK
unsafe extern "system" fn handler(ty: DWORD) -> BOOL {
    let event = match ty {
        CTRL_C_EVENT => CTRL_C_SLOT,
        CTRL_BREAK_EVENT => CTRL_BREAK_SLOT,
        _ => return FALSE,
    };

    let slot = match (*GLOBALS).signals.get(event) {
        Some(slot) => slot,
        None => unreachable!(),
    };

    // broadcast the signal
    for tx in slot.recipients.lock().unwrap().iter() {
        tx.send(()).unwrap();
    }

    TRUE
}

/// An implementation of `Stream` for receiving a particular type of signal.
///
/// This structure deref to mpsc::Receiver<()> and represents notifications
/// of the current process receiving a particular signal. The signal being
/// listened for is passed to `Signal::new`, and every signal is then
/// yielded as each element for the stream.
///
pub struct Signal {
    signal: usize,
    // Used only as an identifier. We place the real sender into a Box, so it
    // stays on the same address forever. That gives us a unique pointer, so we
    // can use this to identify the sender in a Vec and delete it when we are
    // dropped.
    id: *const Sender<()>,
    rx: Receiver<()>,
}

// The raw pointer prevents the compiler from determining it as Send
// automatically. But the only thing we use the raw pointer for is to identify
// the correct Box to delete, not manipulate any data through that.
unsafe impl Send for Signal {}

impl Signal {
    /// Creates a new stream which will receive notifications when the current
    /// process receives the signal `signal`.
    ///
    /// The `Signal` stream is an infinite stream which will receive
    /// notifications whenever a signal is received.
    ///
    /// A `Signal` stream can be created for a particular signal number
    /// multiple times. When a signal is received then all the associated
    /// channels will receive the signal notification.
    pub fn new(signal: DWORD) -> io::Result<Signal> {
        let slot = match signal {
            CTRL_C_EVENT => CTRL_C_SLOT,
            CTRL_BREAK_EVENT => CTRL_BREAK_SLOT,
            _ => return Err(io::Error::new(io::ErrorKind::Other, "invalide signal")),
        };

        let (tx, rx) = mpsc::channel();
        let tx = Box::new(tx);
        let id: *const _ = &*tx;
        globals().signals[slot].recipients.lock().unwrap().push(tx);
        Ok(Signal {
            rx: rx,
            id: id,
            signal: slot,
        })
    }
}

impl Deref for Signal {
    type Target = mpsc::Receiver<()>;
    fn deref(&self) -> &mpsc::Receiver<()> {
        &self.rx
    }
}

impl Drop for Signal {
    fn drop(&mut self) {
        let idx = self.signal as usize;
        let mut list = globals().signals[idx].recipients.lock().unwrap();
        list.retain(|sender| &**sender as *const _ != self.id);
    }
}
