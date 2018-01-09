//! Unix-specific types for signal handling.
//!
//! This module is only defined on Unix platforms and contains the primary
//! `Signal` type for receiving notifications of signals.

#![cfg(unix)]

pub extern crate libc;
use std::io;
use std::mem;
use std::ops::Deref;
use std::cell::UnsafeCell;
use std::sync::{Once, ONCE_INIT};

use may::sync::Mutex;
use may::sync::mpsc::{self, Receiver, Sender};

use self::libc::c_int;
pub use self::libc::{SIGUSR1, SIGUSR2, SIGINT, SIGTERM};
pub use self::libc::{SIGALRM, SIGHUP, SIGPIPE, SIGQUIT, SIGTRAP};

// Number of different unix signals
const SIGNUM: usize = 32;

struct SignalInfo {
    // The ones interested in this signal
    recipients: Mutex<Vec<Box<Sender<()>>>>,
    init: Once,
    initialized: UnsafeCell<bool>,
    prev: UnsafeCell<libc::sigaction>,
}

impl Default for SignalInfo {
    fn default() -> SignalInfo {
        SignalInfo {
            init: ONCE_INIT,
            initialized: UnsafeCell::new(false),
            recipients: Mutex::new(Vec::new()),
            prev: UnsafeCell::new(unsafe { mem::zeroed() }),
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
        &*GLOBALS
    }
}

/// Our global signal handler for all signals registered by this module.
///
/// The purpose of this signal handler is to primarily:
///
/// 1. Flag that our specific signal was received (e.g. store an atomic flag)
/// 2. Wake up driver tasks by writing a byte to a pipe
///
/// Those two operations shoudl both be async-signal safe. After that's done we
/// just try to call a previous signal handler, if any, to be "good denizens of
/// the internet"
extern "C" fn handler(signum: c_int, info: *mut libc::siginfo_t, ptr: *mut libc::c_void) {
    type FnSigaction = extern "C" fn(c_int, *mut libc::siginfo_t, *mut libc::c_void);
    type FnHandler = extern "C" fn(c_int);
    unsafe {
        let slot = match (*GLOBALS).signals.get(signum as usize) {
            Some(slot) => slot,
            None => return,
        };

        // broadcast the signal
        for tx in slot.recipients.lock().unwrap().iter() {
            tx.send(()).unwrap();
        }

        let fnptr = (*slot.prev.get()).sa_sigaction;
        if fnptr == 0 || fnptr == libc::SIG_DFL || fnptr == libc::SIG_IGN {
            return;
        }
        if (*slot.prev.get()).sa_flags & libc::SA_SIGINFO == 0 {
            let action = mem::transmute::<usize, FnHandler>(fnptr);
            action(signum)
        } else {
            let action = mem::transmute::<usize, FnSigaction>(fnptr);
            action(signum, info, ptr)
        }
    }
}

/// Enable this module to receive signal notifications for the `signal`
/// provided.
///
/// This will register the signal handler if it hasn't already been registered,
/// returning any error along the way if that fails.
fn signal_enable(signal: c_int) -> io::Result<()> {
    let siginfo = match globals().signals.get(signal as usize) {
        Some(slot) => slot,
        None => return Err(io::Error::new(io::ErrorKind::Other, "signal too large")),
    };
    unsafe {
        let mut err = None;
        siginfo.init.call_once(|| {
            let mut new: libc::sigaction = mem::zeroed();
            new.sa_sigaction = handler as usize;
            new.sa_flags = libc::SA_RESTART | libc::SA_SIGINFO | libc::SA_NOCLDSTOP;
            if libc::sigaction(signal, &new, &mut *siginfo.prev.get()) != 0 {
                err = Some(io::Error::last_os_error());
            } else {
                *siginfo.initialized.get() = true;
            }
        });
        if let Some(err) = err {
            return Err(err);
        }
        if *siginfo.initialized.get() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to register signal handler",
            ))
        }
    }
}

/// An implementation of `Stream` for receiving a particular type of signal.
///
/// This structure deref to mpsc::Receiver<()> and represents notifications
/// of the current process receiving a particular signal. The signal being
/// listened for is passed to `Signal::new`, and every signal is then
/// yielded as each element for the stream.
///
pub struct Signal {
    signal: c_int,
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
    /// notifications whenever a signal is received. More documentation can be
    /// found on `Signal` itself, but to reiterate:
    ///
    /// * Once a signal handler is registered with the process the underlying
    ///   libc signal handler is never unregistered.
    ///
    /// A `Signal` stream can be created for a particular signal number
    /// multiple times. When a signal is received then all the associated
    /// channels will receive the signal notification.
    pub fn new(signal: c_int) -> io::Result<Signal> {
        // Turn the signal delivery on once we are ready for it
        try!(signal_enable(signal));

        // One wakeup in a queue is enough, no need for us to buffer up any
        // more.
        let (tx, rx) = mpsc::channel();
        let tx = Box::new(tx);
        let id: *const _ = &*tx;
        let idx = signal as usize;
        globals().signals[idx].recipients.lock().unwrap().push(tx);
        Ok(Signal {
            rx: rx,
            id: id,
            signal: signal,
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
