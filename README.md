# may_signal

Signal handling library

You can use this library to wait for a signal in coroutine context without blocking the thread.

[![Build Status](https://travis-ci.org/Xudong-Huang/may_signal.svg?branch=master)](https://travis-ci.org/Xudong-Huang/may_signal)

[Documentation](https://docs.rs/may_signal)

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies]
may_signal = "0.1"
```

Next you can use the API directly:

```rust,no_run
#[macro_use]
extern crate may;
extern crate may_signal;

fn main() {
    let s = may_signal::ctrl_c();
    for _ in s.iter().take(3) {
        println!("CTRL_C pressed!");
    }

    join!(
        {
            let s = may_signal::ctrl_c();
            for _ in s.iter().take(3) {
                println!("CTRL_C pressed! in coroutine 0");
            }
        },
        {
            let s = may_signal::ctrl_c();
            for _ in s.iter().take(4) {
                println!("CTRL_C pressed! in coroutine 1");
            }
        }
    );

    #[cfg(unix)]
    {
        let sig_int = may_signal::Signal::new(may_signal::unix::SIGINT).unwrap();
        let sig_trm = may_signal::Signal::new(may_signal::unix::SIGTERM).unwrap();
        for _ in 0..3 {
            select!(
                _ = sig_int.recv().unwrap() => println!("SIGINT received"),
                _ = sig_trm.recv().unwrap() => println!("SIGTRM received")
            );
        }
    }
}
```

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.
