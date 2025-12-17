//! # SansIO - IO-Free Networking Framework
//!
//! `sansio` is a Sans-IO networking framework for Rust that separates protocol logic from I/O operations,
//! making it easy to build modular, reusable, and testable network protocols.
//!
//! Inspired by [Netty](https://netty.io) and [Wangle](https://github.com/facebook/wangle),
//! `sansio` brings the power of pipeline-based protocol composition to Rust.
//!
//! ## Core Concepts
//!
//! ### Pipeline
//!
//! The [`Pipeline`] is the fundamental abstraction. It's a chain of [`Handler`]s that process
//! inbound and outbound data. Pipelines implement an advanced form of the Intercepting Filter
//! pattern, giving you full control over how events flow through your protocol stack.
//!
//! **Key Benefits:**
//! - Modular: Each handler does one thing well (UNIX philosophy)
//! - Composable: Chain handlers to build complex protocols
//! - Flexible: Easy to add, remove, or reorder handlers
//! - Testable: Test handlers in isolation without I/O
//!
//! ### Handler
//!
//! A [`Handler`] processes messages flowing through the pipeline. Each handler has four associated types:
//! - `Rin`: Input type for inbound messages
//! - `Rout`: Output type for inbound messages
//! - `Win`: Input type for outbound messages
//! - `Wout`: Output type for outbound messages
//!
//! **Best Practice:** Keep handlers focused on a single responsibility. If a handler does multiple
//! things, split it into separate handlers.
//!
//! ### Protocol
//!
//! The [`Protocol`] trait provides a simpler alternative to [`Handler`] for building Sans-IO protocols.
//! It's fully decoupled from I/O, timers, and other runtime dependencies, making protocols easy to
//! test and reuse across different runtime environments.
//!
//! ## Event Flow
//!
//! Messages flow through the pipeline in two directions:
//! - **Inbound** (bottom-up): Network → Handler 1 → Handler 2 → ... → Handler N
//! - **Outbound** (top-down): Handler N → ... → Handler 2 → Handler 1 → Network
//!
//! ```text
//!                                                       | write()
//!   +---------------------------------------------------+---------------+
//!   |                             Pipeline              |               |
//!   |                                                  \|/              |
//!   |    +----------+----------+------------+-----------+----------+    |
//!   |    |                       Handler  N                        |    |
//!   |    +----------+----------+------------+-----------+----------+    |
//!   |              /|\                                  |               |
//!   |               |                                   |               |
//!   |               |                                   |               |
//!   |               |                                  \|/              |
//!   |    +----------+----------+------------+-----------+----------+    |
//!   |    |                       Handler N-1                       |    |
//!   |    +----------+----------+------------+-----------+----------+    |
//!   |              /|\                                  |               |
//!   |               |                                   |               |
//!   |               |                         Context.fire_poll_write() |
//!   |               |                                   |               |
//!   |               |                                   |               |
//!   |          Context.fire_handle_read()               |               |
//!   |               |                                   |               |
//!   |               |                                  \|/              |
//!   |    +----------+----------+------------+-----------+----------+    |
//!   |    |                       Handler  2                        |    |
//!   |    +----------+----------+------------+-----------+----------+    |
//!   |              /|\                                  |               |
//!   |               |                                   |               |
//!   |               |                                   |               |
//!   |               |                                  \|/              |
//!   |    +----------+----------+------------+-----------+----------+    |
//!   |    |                       Handler  1                        |    |
//!   |    +----------+----------+------------+-----------+----------+    |
//!   |              /|\                                  |               |
//!   +---------------+-----------------------------------+---------------+
//!                   | handle_read()                     | poll_write()
//!                   |                                  \|/
//!   +---------------+-----------------------------------+---------------+
//!   |               |                                   |               |
//!   |        Internal I/O Threads (Transport Implementation)            |
//!   +-------------------------------------------------------------------+
//! ```
//!
//! ## Example: Echo Server
//!
//! Here's a complete example showing how to build a simple echo server using the pipeline pattern.
//!
//! ### Step 1: Define the Handler
//!
//! The echo handler receives strings, prints them, and sends them back:
//! ```ignore
//! struct EchoServerHandler {
//!     transmits: VecDeque<String>,
//! }
//!
//! impl Handler for EchoServerHandler {
//!     type Rin = TaggedString;
//!     type Rout = Self::Rin;
//!     type Win = TaggedString;
//!     type Wout = Self::Win;
//!
//!     fn name(&self) -> &str {
//!         "EchoServerHandler"
//!     }
//!
//!     fn handle_read(
//!         &mut self,
//!         _ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>,
//!         msg: Self::Rin,
//!     ) {
//!         println!("handling {}", msg.message);
//!         self.transmits.push_back(format!("{}\r\n", msg.message));
//!     }
//!
//!     fn poll_write(
//!         &mut self,
//!         ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>,
//!     ) -> Option<Self::Wout> {
//!         if let Some(msg) = ctx.fire_poll_write() {
//!             self.transmits.push_back(msg);
//!         }
//!         self.transmits.pop_front()
//!     }
//! }
//! ```
//!
//! ### Step 2: Build the Pipeline
//!
//! Chain handlers together to form a complete protocol stack:
//! ```ignore
//! fn build_pipeline() -> Rc<Pipeline<BytesMut,String>> {
//!     let pipeline: Pipeline<BytesMut,String> = Pipeline::new();
//!
//!     let line_based_frame_decoder_handler = ByteToMessageCodecHandler::new(Box::new(
//!         LineBasedFrameDecoder::new(8192, true, TerminatorType::BOTH),
//!     ));
//!     let string_codec_handler = StringCodecHandler::new();
//!     let echo_server_handler = EchoServerHandler::new();
//!
//!     pipeline.add_back(line_based_frame_decoder_handler);
//!     pipeline.add_back(string_codec_handler);
//!     pipeline.add_back(echo_server_handler);
//!     pipeline.finalize()
//! }
//! ```
//!
//! **Handler Responsibilities:**
//! - **LineBasedFrameDecoder**: Splits byte stream on `\n` or `\r\n`
//! - **StringCodec**: Converts bytes ↔ UTF-8 strings
//! - **EchoHandler**: Application logic (echo messages back)
//!
//! **Important:** Handler order matters! They're processed in insertion order.
//!
//! ### Step 3.1: Run the Event Loop with sync fn
//!
//! The pipeline is pure protocol logic - you provide the I/O:
//! ```ignore
//! fn run(socket: UdpSocket, cancel_rx: crossbeam_channel::Receiver<()>) {
//!     let mut buf = vec![0; 2000];
//!
//!     let pipeline = build_pipeline();
//!     pipeline.transport_active();
//!     loop {
//!         // Check cancellation
//!         if cancel_rx.try_recv().is_ok() {
//!             break;
//!         }
//!
//!         // Poll pipeline to write transmit to socket
//!         while let Some(transmit) = pipeline.poll_write() {
//!             socket.send(transmit)?;
//!         }
//!
//!         // Poll pipeline to get next timeout
//!         let mut eto = Instant::now() + Duration::from_millis(100);
//!         pipeline.poll_timeout(&mut eto);
//!
//!         let delay_from_now = eto
//!             .checked_duration_since(Instant::now())
//!             .unwrap_or(Duration::from_secs(0));
//!         if delay_from_now.is_zero() {
//!            pipeline.handle_timeout(Instant::now());
//!            continue;
//!         }
//!
//!         socket.set_read_timeout(Some(delay_from_now)).expect("setting socket read timeout");
//!         if let Ok(n) = socket.recv_from(buf) {
//!             pipeline.handle_read(&buf[..n]);
//!         }
//!
//!         // Drive time forward
//!         pipeline.handle_timeout(Instant::now());
//!     }
//!     pipeline.transport_inactive();
//! }
//! ```
//! ### Step 3.2: Run the Event Loop with async fn
//!
//! The pipeline is pure protocol logic - you provide the I/O:
//! ```ignore
//! async fn run(socket: UdpSocket,
//!              mut close_rx: broadcast::Receiver<()>,
//!              mut write_notify: Notify) {
//!     let mut buf = vec![0; 2000];
//!
//!     let pipeline = build_pipeline();
//!     pipeline.transport_active();
//!     loop {
//!         // Poll pipeline to write transmit to socket
//!         while let Some(transmit) = pipeline.poll_write() {
//!             socket.send(transmit)?;
//!         }
//!
//!         // Poll pipeline to get next timeout
//!         let mut eto = Instant::now() + Duration::from_secs(86400);
//!         pipeline.poll_timeout(&mut eto);
//!
//!         let delay_from_now = eto
//!             .checked_duration_since(Instant::now())
//!             .unwrap_or(Duration::from_secs(0));
//!         if delay_from_now.is_zero() {
//!            pipeline.handle_timeout(Instant::now());
//!            continue;
//!         }
//!
//!         let timer = tokio::time::sleep(delay_from_now);
//!         tokio::pin!(timer);
//!
//!         tokio::select! {
//!             _ = close_rx.recv() => break,
//!             _ = write_notify.notified() => {},
//!             _ = timer.as_mut() => pipeline.handle_timeout(Instant::now()),
//!             n = socket.recv(&mut buf) => pipeline.handle_read(&buf[..n]),
//!         }
//!     }
//!     pipeline.transport_inactive();
//! }
//! ```
//!
//! ## Design Philosophy
//!
//! `sansio` follows the Sans-IO pattern, which separates protocol logic from I/O concerns:
//!
//! **Benefits:**
//! - **Testable**: Test protocol logic without real network I/O
//! - **Flexible**: Use with any I/O model (sync, async, embedded)
//! - **Reusable**: Same protocol code across different environments
//! - **Debuggable**: Easier to reason about and debug
//!
//! **Trade-offs:**
//! - You manage the I/O loop yourself
//! - More control means more responsibility
//! - Steeper learning curve for simple cases
//!
//! For most applications, the benefits far outweigh the trade-offs, especially as your
//! protocol logic becomes more complex.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/webrtc-rs/sansio/master/doc/sansio-white.png"
)]
#![warn(rust_2018_idioms)]
#![allow(dead_code)]
#![warn(missing_docs)]

// ========================================
// Module Declarations
// ========================================

/// Handler implementations for protocol pipelines
pub(crate) mod handler;

/// Internal handler types (not part of public API)
pub(crate) mod handler_internal;

/// Pipeline implementations for chaining handlers
pub(crate) mod pipeline;

/// Internal pipeline types (not part of public API)
pub(crate) mod pipeline_internal;

/// Protocol trait for building Sans-IO protocols
pub(crate) mod protocol;

// ========================================
// Public Exports
// ========================================

/// Handler and context types for building protocol pipelines
pub use handler::{Context, Handler};

/// Pipeline traits for inbound and outbound message processing
pub use pipeline::{InboundPipeline, OutboundPipeline, Pipeline};

/// Protocol trait for Sans-IO protocol implementations
pub use protocol::Protocol;
