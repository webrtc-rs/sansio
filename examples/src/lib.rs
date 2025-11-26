//! # Sansio Examples
//!
//! This crate contains examples demonstrating how to use the sansio ecosystem.
//!
//! ## Running Examples
//!
//! Run an example with:
//!
//! ```bash
//! cargo run --example chat_server_udp
//! ```
//!
//! Or with a specific runtime:
//!
//! ```bash
//! cargo run --example chat_server_udp --no-default-features --features runtime-tokio
//! ```

#![warn(rust_2018_idioms)]
#![allow(dead_code)]

// Helper modules that examples can use
pub mod helpers;
