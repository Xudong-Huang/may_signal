# may_signal

Signal handling library

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
fn main() {
    // Create an infinite stream of "Ctrl+C" notifications.
    let ctrl_c = may_signal::ctrl_c();

    // Process each ctrl-c as it comes in
    for _ in ctrl_c.iter() {
        println!("ctrl-c received!");
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
