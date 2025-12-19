<h1 align="center">
 <a href="https://webrtc.rs"><img src="https://raw.githubusercontent.com/webrtc-rs/sansio/master/doc/sansio-black.png" alt="sansio"></a>
 <br>
</h1>
<p align="center">
 <a href="https://github.com/webrtc-rs/sansio/actions">
  <img src="https://github.com/webrtc-rs/sansio/workflows/cargo/badge.svg">
 </a>
 <a href="https://deps.rs/repo/github/webrtc-rs/sansio">
  <img src="https://deps.rs/repo/github/webrtc-rs/sansio/status.svg">
 </a>
 <a href="https://crates.io/crates/sansio">
  <img src="https://img.shields.io/crates/v/sansio.svg">
 </a>
 <a href="https://docs.rs/sansio">
  <img src="https://docs.rs/sansio/badge.svg">
 </a>
 <a href="https://doc.rust-lang.org/1.6.0/complement-project-faq.html#why-dual-mitasl2-license">
  <img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue" alt="License: MIT/Apache 2.0">
 </a>
</p>
<p align="center">
 Rust in Sans-IO
</p>

# Sans-IO Protocol

The `Protocol` trait provides a simpler, fully Sans-IO trait for protocol implementation.

## Example

```rust
use sansio::Protocol;
use std::collections::VecDeque;

#[derive(Debug)]
struct MyError;
impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "MyError")
    }
}
impl std::error::Error for MyError {}

/// A simple uppercase protocol: converts incoming strings to uppercase
struct UppercaseProtocol {
    routs: VecDeque<String>,
    wouts: VecDeque<String>,
}

impl UppercaseProtocol {
    fn new() -> Self {
        Self {
            wouts: VecDeque::new(),
        }
    }
}

impl Protocol<String, String, ()> for UppercaseProtocol {
    type Rout = String;
    type Wout = String;
    type Eout = ();
    type Error = MyError;

    fn handle_read(&mut self, msg: String) -> Result<(), Self::Error> {
        // Process incoming message
        self.routs.push_back(msg.to_uppercase());
        Ok(())
    }

    fn poll_read(&mut self) -> Option<Self::Rout> {
        // Return processed message
        self.routs.pop_front()
    }

    fn handle_write(&mut self, msg: String) -> Result<(), Self::Error> {
        // For this simple protocol, just pass through
        self.wouts.push_back(msg);
        Ok(())
    }

    fn poll_write(&mut self) -> Option<Self::Wout> {
        self.wouts.pop_front()
    }
}
```

## Quick Start

Add `sansio` to your `Cargo.toml`:

```toml
[dependencies]
sansio = "x.x.x"
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
