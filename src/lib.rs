//! Signal handling library
//!
//! This crate implements signal handling by using may which is a stackful
//! coroutine library in Rust. The primary type exported from this
//! crate, `unix::Signal`, allows receiving for arbitrary signals on Unix
//! platforms, receiving them in coroutine fashion.
//!
//! # Examples
//!
//! Print out all ctrl-C notifications received
//!
//! ```rust,no_run
//! extern crate may_signal;
//!
//! fn main() {
//!     // Create an infinite stream of "Ctrl+C" notifications. Each item received
//!     // on this stream represent a ctrl-c signal.
//!     let ctrl_c = may_signal::ctrl_c();
//!
//!     // Process each ctrl-c as it comes in
//!     for _ in ctrl_c.iter() {
//!         println!("ctrl-c received!");
//!     };
//! }
//! ```
//!

#![doc(html_root_url = "https://docs.rs/may_signal/0.1")]
#![deny(missing_docs)]

// #[macro_use]
#[doc(hidden)]
extern crate may;

pub mod unix;
pub mod windows;

#[cfg(unix)]
pub use unix::Signal;
#[cfg(windows)]
pub use windows::Signal;

/// Creates a stream which receives "ctrl-c" notifications sent to a process.
///
/// In general signals are handled very differently across Unix and Windows, but
/// this is somewhat cross platform in terms of how it can be handled. A ctrl-c
/// event to a console process can be represented as a stream for both Windows
/// and Unix.
///
/// This function returns a signal receiver on which you can get all the signal
/// events.
pub fn ctrl_c() -> Signal {
    use std::io;

    #[cfg(unix)]
    fn ctrl_c_imp() -> io::Result<Signal> {
        Signal::new(unix::SIGINT)
    }

    #[cfg(windows)]
    fn ctrl_c_imp() -> io::Result<Signal> {
        Signal::new(windows::CTRL_C_EVENT)
    }

    return ctrl_c_imp().expect("failed to create Signal");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_ctrl_c() {
        let _s = ctrl_c();
    }
}
