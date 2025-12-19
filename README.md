<h1 align="center">
 <a href="https://webrtc.rs"><img src="https://raw.githubusercontent.com/webrtc-rs/sansio/master/sansio-black.png" alt="sansio"></a>
 <br>
</h1>
<p align="center">
 <a href="https://github.com/webrtc-rs/sansio/actions">
  <img src="https://github.com/webrtc-rs/sansio/workflows/cargo/badge.svg">
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

# What is Sans-IO?

**Sans-IO** (French for "without I/O") is an architectural pattern for writing protocol implementations that are
completely decoupled from I/O operations. This makes protocols:

- **Testable**: Test protocol logic without mocking sockets or async runtimes
- **Portable**: Run the same protocol code in sync, async, embedded, or WASM environments
- **Composable**: Stack and combine protocol layers easily
- **Debuggable**: Step through protocol state machines without I/O side effects

## Features

- Clean push-pull API for message handling
- Zero-cost abstractions with generic type parameters
- Support for custom events and timeout handling
- Works with any I/O backend (sync, async, embedded)
- No dependencies, minimal footprint
- Fully documented with examples
- **`no_std` by default** - works in any environment

## no_std Support

This crate is `no_std` by default and works seamlessly in any environment - embedded systems,
bare-metal applications, WASM, or standard applications.

### Time Handling

The `Time` associated type is fully generic, so you can use any time representation that fits
your environment:

**Using `std::time::Instant`:**

```rust
impl Protocol<Vec<u8>, Vec<u8>, ()> for MyProtocol {
    type Time = std::time::Instant;
    // ...
}
```

**Using tick counts (embedded):**

```rust
impl Protocol<Vec<u8>, Vec<u8>, ()> for MyProtocol {
    type Time = u64;  // System ticks
    // ...
}
```

**Using milliseconds:**

```rust
impl Protocol<Vec<u8>, Vec<u8>, ()> for MyProtocol {
    type Time = i64;  // Milliseconds since epoch
    // ...
}
```

**No timeout needed:**

```rust
impl Protocol<Vec<u8>, Vec<u8>, ()> for MyProtocol {
    type Time = ();  // Unit type when timeouts aren't used
    // ...
}
```

## The Protocol Trait

The `Protocol` trait provides a simplified Sans-IO interface for building network protocols:

```rust
pub trait Protocol<Rin, Win, Ein> {
    type Rout;  // Output read type
    type Wout;  // Output write type
    type Eout;  // Output event type
    type Error; // Error type
    type Time;  // Time type (u64, Instant, i64, (), etc.)

    // Push data into protocol
    fn handle_read(&mut self, msg: Rin) -> Result<(), Self::Error>;
    fn handle_write(&mut self, msg: Win) -> Result<(), Self::Error>;
    fn handle_event(&mut self, evt: Ein) -> Result<(), Self::Error>;
    fn handle_timeout(&mut self, now: Self::Time) -> Result<(), Self::Error>;

    // Pull results from protocol
    fn poll_read(&mut self) -> Option<Self::Rout>;
    fn poll_write(&mut self) -> Option<Self::Wout>;
    fn poll_event(&mut self) -> Option<Self::Eout>;
    fn poll_timeout(&mut self) -> Option<Self::Time>;

    // Lifecycle
    fn close(&mut self) -> Result<(), Self::Error>;
}
```

## Quick Start

Add `sansio` to your `Cargo.toml`:

```toml
[dependencies]
sansio = "0.10"
```

## Example: Uppercase Protocol

A simple protocol that converts incoming strings to uppercase:

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
            routs: VecDeque::new(),
            wouts: VecDeque::new(),
        }
    }
}

impl Protocol<String, String, ()> for UppercaseProtocol {
    type Rout = String;
    type Wout = String;
    type Eout = ();
    type Error = MyError;
    type Time = ();  // No timeout handling needed

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

// Usage example
fn main() {
    let mut protocol = UppercaseProtocol::new();

    // Push data in
    protocol.handle_read("hello world".to_string()).unwrap();

    // Pull results out
    assert_eq!(protocol.poll_read(), Some("HELLO WORLD".to_string()));
}
```

## Testing Made Easy

Sans-IO protocols are trivial to test because they don't involve any I/O:

```rust
#[test]
fn test_uppercase_protocol() {
    let mut protocol = UppercaseProtocol::new();

    // Test single message
    protocol.handle_read("test".to_string()).unwrap();
    assert_eq!(protocol.poll_read(), Some("TEST".to_string()));

    // Test multiple messages
    protocol.handle_read("hello".to_string()).unwrap();
    protocol.handle_read("world".to_string()).unwrap();
    assert_eq!(protocol.poll_read(), Some("HELLO".to_string()));
    assert_eq!(protocol.poll_read(), Some("WORLD".to_string()));
    assert_eq!(protocol.poll_read(), None);
}
```

## Use Cases

- **Network Protocols**: HTTP, WebSocket, custom TCP/UDP protocols
- **Message Parsers**: Protocol buffers, JSON-RPC, custom formats
- **State Machines**: Connection handling, handshakes, negotiations
- **Protocol Testing**: Unit test protocol logic without network I/O
- **Embedded Systems**: Protocols that need to work with both blocking and non-blocking I/O
- **WASM**: Browser-based protocol implementations

## Why Sans-IO?

Traditional protocol implementations mix I/O and protocol logic:

```rust
// Traditional approach - tightly coupled to async I/O
async fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0u8; 1024];
    while let Ok(n) = stream.read(&mut buf).await {
        // Protocol logic mixed with I/O
        let response = process(&buf[..n]);
        stream.write_all(&response).await.unwrap();
    }
}
```

Sans-IO separates concerns:

```rust
// Sans-IO approach - protocol logic is independent
struct MyProtocol {
    /* ... */
}
impl Protocol<Vec<u8>, Vec<u8>, ()> for MyProtocol { /* ... */ }

// I/O layer is separate and can be swapped
async fn handle_connection(mut stream: TcpStream, mut protocol: MyProtocol) {
    let mut buf = [0u8; 1024];
    while let Ok(n) = stream.read(&mut buf).await {
        protocol.handle_read(buf[..n].to_vec()).unwrap();
        while let Some(response) = protocol.poll_write() {
            stream.write_all(&response).await.unwrap();
        }
    }
}
```

Benefits:

- Protocol logic can be tested without async runtime
- Same protocol works with sync, async, or embedded I/O
- Easier to debug and reason about
- Protocol layers can be composed and reused

## Documentation

Full API documentation is available at [docs.rs/sansio](https://docs.rs/sansio)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
