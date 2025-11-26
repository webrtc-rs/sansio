//! # Pipeline - Chain of Handlers for Protocol Processing
//!
//! The pipeline is the fundamental abstraction in sansio for building complex protocol stacks
//! from simple, composable handlers. It implements an advanced form of the Intercepting Filter
//! pattern, giving you full control over how events flow through your protocol layers.
//!
//! ## Overview
//!
//! A pipeline is an ordered chain of [`Handler`]s that process messages bidirectionally:
//!
//! - **Inbound (handle_read)**: Data flows from first → last handler
//! - **Outbound (poll_write)**: Data flows from last → first handler
//!
//! Each handler transforms messages from one type to another, enabling you to build
//! layered protocol stacks (e.g., bytes → frames → messages → application data).
//!
//! ## Architecture
//!
//! ```text
//!                      Network I/O
//!                          ↓↑
//!                    ┌──────────┐
//!                    │ Pipeline │
//!                    └──────────┘
//!                          │
//!        Inbound (↓)       │       Outbound (↑)
//!                          │
//!      ┌───────────────────┼───────────────────┐
//!      │                   │                   │
//!      ↓                   ↑                   │
//!  ┌────────┐          ┌────────┐             │
//!  │Handler1│ ────→    │Handler1│             │
//!  │Bytes→  │          │  →Bytes│             │
//!  │Frames  │          │        │             │
//!  └────────┘          └────────┘             │
//!      │                   ↑                   │
//!      ↓                   │                   │
//!  ┌────────┐          ┌────────┐             │
//!  │Handler2│ ────→    │Handler2│             │
//!  │Frames→ │          │→Frames │             │
//!  │Messages│          │        │             │
//!  └────────┘          └────────┘             │
//!      │                   ↑                   │
//!      ↓                   │                   │
//!  ┌────────┐          ┌────────┐             │
//!  │Handler3│ ────→    │Handler3│             │
//!  │Messages│          │→Messages│            │
//!  │→App    │          │        │             │
//!  └────────┘          └────────┘             │
//!      │                   │                   │
//!      └───────────────────┴───────────────────┘
//!                  Application
//! ```
//!
//! ## Example: Building a Protocol Pipeline
//!
//! ```rust
//! use sansio::{Pipeline, InboundPipeline, Handler, Context};
//! use std::collections::VecDeque;
//!
//! # #[derive(Debug)]
//! # struct Frame { data: Vec<u8> }
//! # struct Message { text: String }
//!
//! // Handler 1: Bytes → Frames
//! struct FrameDecoder;
//! impl Handler for FrameDecoder {
//!     type Rin = Vec<u8>;
//!     type Rout = Frame;
//!     type Win = Frame;
//!     type Wout = Vec<u8>;
//!
//!     fn name(&self) -> &str { "FrameDecoder" }
//!
//!     fn handle_read(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, msg: Self::Rin) {
//!         // Decode bytes into frame
//!         let frame = Frame { data: msg };
//!         ctx.fire_handle_read(frame);
//!     }
//!
//!     fn poll_write(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> {
//!         // Encode frame to bytes
//!         ctx.fire_poll_write().map(|frame| frame.data)
//!     }
//! }
//!
//! // Handler 2: Frames → Messages
//! struct MessageCodec;
//! impl Handler for MessageCodec {
//!     type Rin = Frame;
//!     type Rout = Message;
//!     type Win = Message;
//!     type Wout = Frame;
//!
//!     fn name(&self) -> &str { "MessageCodec" }
//!
//!     fn handle_read(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, msg: Self::Rin) {
//!         // Decode frame into message
//!         if let Ok(text) = String::from_utf8(msg.data) {
//!             ctx.fire_handle_read(Message { text });
//!         }
//!     }
//!
//!     fn poll_write(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> {
//!         // Encode message to frame
//!         ctx.fire_poll_write().map(|msg| Frame { data: msg.text.into_bytes() })
//!     }
//! }
//!
//! // Build the pipeline
//! let pipeline: Pipeline<Vec<u8>, Message> = Pipeline::new();
//! pipeline
//!     .add_back(FrameDecoder)
//!     .add_back(MessageCodec);
//!
//! let pipeline = pipeline.finalize();
//!
//! // Use the pipeline
//! pipeline.handle_read(vec![72, 101, 108, 108, 111]); // "Hello"
//! ```
//!
//! ## Pipeline Lifecycle
//!
//! 1. **Construction**: Create with `Pipeline::new()`
//! 2. **Building**: Add handlers with `add_back()` or `add_front()`
//! 3. **Finalization**: Call `finalize()` to link handlers together
//! 4. **Usage**: Call `handle_read()`, `poll_write()`, etc.
//!
//! ## Event Flow
//!
//! The pipeline supports multiple event types:
//!
//! ### Transport Events
//! - `transport_active()`: Connection established
//! - `transport_inactive()`: Connection closed
//!
//! ### Data Events
//! - `handle_read(msg)`: Process inbound data
//! - `poll_write()`: Retrieve outbound data
//!
//! ### Time Events
//! - `handle_timeout(now)`: Handle timeout
//! - `poll_timeout(eto)`: Query next timeout
//!
//! ### Error Events
//! - `handle_error(err)`: Handle errors
//! - `handle_eof()`: Handle end-of-file
//! - `close()`: Initiate shutdown
//!
//! ## Type Safety
//!
//! Pipelines enforce type compatibility at compile time:
//!
//! ```rust,ignore
//! # use sansio::{Pipeline, Handler, Context};
//! # struct H1;
//! # impl Handler for H1 {
//! #     type Rin = Vec<u8>;
//! #     type Rout = String;  // Output: String
//! #     type Win = String;
//! #     type Wout = Vec<u8>;
//! #     fn name(&self) -> &str { "H1" }
//! #     fn handle_read(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, _: Self::Rin) {}
//! #     fn poll_write(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> { None }
//! # }
//! # struct H2;
//! # impl Handler for H2 {
//! #     type Rin = i32;      // Input: i32 (incompatible!)
//! #     type Rout = String;
//! #     type Win = String;
//! #     type Wout = i32;
//! #     fn name(&self) -> &str { "H2" }
//! #     fn handle_read(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, _: Self::Rin) {}
//! #     fn poll_write(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> { None }
//! # }
//! // This will fail at runtime: H1::Rout (String) != H2::Rin (i32)
//! let pipeline: Pipeline<Vec<u8>, String> = Pipeline::new();
//! pipeline.add_back(H1).add_back(H2); // Will panic when messages flow through!
//! ```
//!
//! ## Dynamic Pipeline Modification
//!
//! Pipelines can be modified at runtime:
//!
//! ```rust
//! # use sansio::{Pipeline, InboundPipeline, Handler, Context};
//! # struct MyHandler;
//! # impl Handler for MyHandler {
//! #     type Rin = String;
//! #     type Rout = String;
//! #     type Win = String;
//! #     type Wout = String;
//! #     fn name(&self) -> &str { "MyHandler" }
//! #     fn handle_read(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, msg: Self::Rin) {
//! #         ctx.fire_handle_read(msg);
//! #     }
//! #     fn poll_write(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> {
//! #         ctx.fire_poll_write()
//! #     }
//! # }
//! let pipeline: Pipeline<String, String> = Pipeline::new();
//! pipeline.add_back(MyHandler);
//! let pipeline = pipeline.finalize();
//!
//! // Use the pipeline
//! pipeline.transport_active();
//! pipeline.handle_read("Hello".to_string());
//!
//! // Remove handlers
//! pipeline.remove("MyHandler").unwrap();
//! ```
//!
//! ## Best Practices
//!
//! 1. **Ordering Matters**: Add handlers in the order they should process data
//! 2. **Single Responsibility**: Each handler should do one thing well
//! 3. **Type Flow**: Ensure handler types align: H1::Rout must equal H2::Rin
//! 4. **Finalize Once**: Call `finalize()` after building your pipeline
//! 5. **Event Propagation**: Most handlers should propagate events via `ctx.fire_*()`

use std::{cell::RefCell, error::Error, rc::Rc, time::Instant};

use crate::{handler::Handler, pipeline_internal::PipelineInternal};

/// Inbound operations for a pipeline.
///
/// The `InboundPipeline` trait defines operations for pushing data and events
/// into the pipeline (from the network/transport toward the application).
///
/// These methods are typically called by your I/O layer when:
/// - Data arrives from the network
/// - Connection state changes
/// - Timeouts occur
/// - Errors happen
///
/// # Type Parameters
///
/// - `R`: The input message type (what you push into the pipeline)
///
/// # Example
///
/// ```rust
/// use sansio::{Pipeline, InboundPipeline, Handler, Context};
/// use std::time::Instant;
///
/// // Simple pass-through handler for the example
/// struct SimpleHandler;
/// impl Handler for SimpleHandler {
///     type Rin = Vec<u8>;
///     type Rout = String;
///     type Win = String;
///     type Wout = Vec<u8>;
///     fn name(&self) -> &str { "SimpleHandler" }
///     fn handle_read(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, msg: Self::Rin) {
///         // Convert Vec<u8> to String
///         if let Ok(s) = String::from_utf8(msg) {
///             ctx.fire_handle_read(s);
///         }
///     }
///     fn poll_write(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> {
///         // Convert String to Vec<u8>
///         ctx.fire_poll_write().map(|s| s.into_bytes())
///     }
/// }
///
/// let pipeline: Pipeline<Vec<u8>, String> = Pipeline::new();
/// pipeline.add_back(SimpleHandler);
/// let pipeline = pipeline.finalize();
///
/// // Push data into the pipeline
/// pipeline.transport_active();
/// pipeline.handle_read(vec![72, 101, 108, 108, 111]); // "Hello"
/// pipeline.handle_timeout(Instant::now());
/// ```
pub trait InboundPipeline<R> {
    /// Notifies the pipeline that the transport is active (connected).
    ///
    /// Call this when a connection is established. The event will propagate
    /// through all handlers in the pipeline.
    fn transport_active(&self);

    /// Notifies the pipeline that the transport is inactive (disconnected).
    ///
    /// Call this when a connection is closed. The event will propagate
    /// through all handlers in the pipeline.
    fn transport_inactive(&self);

    /// Pushes an incoming message into the pipeline.
    ///
    /// The message will flow through handlers from first to last, with each
    /// handler potentially transforming it.
    ///
    /// # Parameters
    ///
    /// - `msg`: The incoming message (type `R`)
    fn handle_read(&self, msg: R);

    /// Polls the pipeline for an outgoing message.
    ///
    /// This retrieves messages that should be sent over the transport.
    /// Messages flow from last handler to first (outbound direction).
    ///
    /// # Returns
    ///
    /// - `Some(R)`: A message ready to send
    /// - `None`: No message available
    fn poll_write(&self) -> Option<R>;

    /// Handles a timeout event.
    ///
    /// Call this periodically or when a timeout fires. Handlers can use this
    /// to perform time-based operations (keepalives, retransmissions, etc.).
    ///
    /// # Parameters
    ///
    /// - `now`: The current timestamp
    fn handle_timeout(&self, now: Instant);

    /// Polls for the next timeout deadline.
    ///
    /// Updates `eto` to the earliest time any handler needs `handle_timeout` called.
    /// Use this to set your I/O timer.
    ///
    /// # Parameters
    ///
    /// - `eto`: Mutable reference to earliest timeout. Will be updated to the minimum across all handlers.
    fn poll_timeout(&self, eto: &mut Instant);

    /// Handles an end-of-file event.
    ///
    /// Call this when the input stream is closed (e.g., TCP FIN received).
    fn handle_eof(&self);

    /// Handles an error event.
    ///
    /// Call this when an error occurs in I/O or processing.
    ///
    /// # Parameters
    ///
    /// - `err`: The error to propagate through the pipeline
    fn handle_error(&self, err: Box<dyn Error>);
}

/// Outbound operations for a pipeline.
///
/// The `OutboundPipeline` trait defines operations for pushing data from the
/// application into the pipeline (toward the network/transport).
///
/// These methods are typically called by your application when you want to:
/// - Send data over the network
/// - Close the connection
///
/// # Type Parameters
///
/// - `R`: The input message type (bottom of pipeline)
/// - `W`: The write message type (top of pipeline, from application)
///
/// # Example
///
/// ```rust
/// use sansio::{Pipeline, OutboundPipeline};
///
/// let pipeline: Pipeline<Vec<u8>, String> = Pipeline::new();
/// // ... add handlers ...
/// let pipeline = pipeline.finalize();
///
/// // Send data from application
/// pipeline.write("Hello, World!".to_string());
/// pipeline.close();
/// ```
pub trait OutboundPipeline<R, W> {
    /// Writes a message into the pipeline.
    ///
    /// The message will flow through handlers from last to first (outbound),
    /// with each handler potentially transforming it. Eventually it can be
    /// retrieved via `poll_write()`.
    ///
    /// # Parameters
    ///
    /// - `msg`: The message to write (type `W`)
    fn write(&self, msg: W);

    /// Initiates pipeline close.
    ///
    /// Sends a close event through the pipeline, allowing handlers to
    /// flush buffers and clean up resources.
    fn close(&self);
}

/// A pipeline of handlers for processing protocol messages.
///
/// The `Pipeline` is the main abstraction for building protocol stacks in sansio.
/// It maintains an ordered chain of [`Handler`]s and routes messages bidirectionally
/// through them.
///
/// # Type Parameters
///
/// - `R`: Input type at the bottom of the pipeline (from network/transport)
/// - `W`: Write type at the top of the pipeline (from application)
///
/// # Lifecycle
///
/// 1. **Create**: `Pipeline::new()`
/// 2. **Build**: Add handlers with `add_back()` or `add_front()`
/// 3. **Finalize**: Call `finalize()` to link handlers and wrap in `Rc`
/// 4. **Use**: Call methods from `InboundPipeline` and `OutboundPipeline` traits
///
/// # Example
///
/// ```rust
/// use sansio::{Pipeline, Handler, Context, InboundPipeline};
///
/// # struct EchoHandler;
/// # impl Handler for EchoHandler {
/// #     type Rin = String;
/// #     type Rout = String;
/// #     type Win = String;
/// #     type Wout = String;
/// #     fn name(&self) -> &str { "EchoHandler" }
/// #     fn handle_read(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, msg: Self::Rin) {
/// #         ctx.fire_handle_read(msg);
/// #     }
/// #     fn poll_write(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> {
/// #         ctx.fire_poll_write()
/// #     }
/// # }
/// // Create and build pipeline
/// let pipeline: Pipeline<String, String> = Pipeline::new();
/// pipeline.add_back(EchoHandler);
///
/// // Finalize before use
/// let pipeline = pipeline.finalize();
///
/// // Use the pipeline
/// pipeline.transport_active();
/// pipeline.handle_read("Hello".to_string());
/// if let Some(msg) = pipeline.poll_write() {
///     println!("Got: {}", msg);
/// }
/// ```
///
/// # Thread Safety
///
/// Pipelines use `Rc` (not `Arc`) and are **not** thread-safe. They are designed
/// for single-threaded I/O loops. For multi-threaded usage, create separate
/// pipelines per thread.
pub struct Pipeline<R, W> {
    internal: RefCell<PipelineInternal<R, W>>,
}

impl<R: 'static, W: 'static> Default for Pipeline<R, W> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: 'static, W: 'static> Pipeline<R, W> {
    /// Creates a new empty pipeline.
    ///
    /// The pipeline starts with no handlers. Use [`add_back`] or [`add_front`]
    /// to add handlers, then call [`finalize`] before using it.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sansio::Pipeline;
    ///
    /// let pipeline: Pipeline<Vec<u8>, String> = Pipeline::new();
    /// ```
    ///
    /// [`add_back`]: Pipeline::add_back
    /// [`add_front`]: Pipeline::add_front
    /// [`finalize`]: Pipeline::finalize
    pub fn new() -> Self {
        Self {
            internal: RefCell::new(PipelineInternal::new()),
        }
    }

    /// Appends a handler at the end of the pipeline.
    ///
    /// Handlers are processed in the order they are added:
    /// - Inbound: first → last
    /// - Outbound: last → first
    ///
    /// Returns `&self` for method chaining.
    ///
    /// # Parameters
    ///
    /// - `handler`: The handler to append
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sansio::{Pipeline, Handler, Context};
    /// # struct H1;
    /// # impl Handler for H1 {
    /// #     type Rin = String;
    /// #     type Rout = String;
    /// #     type Win = String;
    /// #     type Wout = String;
    /// #     fn name(&self) -> &str { "H1" }
    /// #     fn handle_read(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, _: Self::Rin) {}
    /// #     fn poll_write(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> { None }
    /// # }
    /// # struct H2;
    /// # impl Handler for H2 {
    /// #     type Rin = String;
    /// #     type Rout = String;
    /// #     type Win = String;
    /// #     type Wout = String;
    /// #     fn name(&self) -> &str { "H2" }
    /// #     fn handle_read(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, _: Self::Rin) {}
    /// #     fn poll_write(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> { None }
    /// # }
    /// let pipeline: Pipeline<String, String> = Pipeline::new();
    /// pipeline
    ///     .add_back(H1)  // First handler
    ///     .add_back(H2); // Second handler
    /// ```
    pub fn add_back(&self, handler: impl Handler + 'static) -> &Self {
        {
            let mut internal = self.internal.borrow_mut();
            internal.add_back(handler);
        }
        self
    }

    /// Inserts a handler at the beginning of the pipeline.
    ///
    /// The new handler will be the first to process inbound messages.
    ///
    /// Returns `&self` for method chaining.
    ///
    /// # Parameters
    ///
    /// - `handler`: The handler to prepend
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sansio::{Pipeline, Handler, Context};
    /// # struct H1;
    /// # impl Handler for H1 {
    /// #     type Rin = String;
    /// #     type Rout = String;
    /// #     type Win = String;
    /// #     type Wout = String;
    /// #     fn name(&self) -> &str { "H1" }
    /// #     fn handle_read(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, _: Self::Rin) {}
    /// #     fn poll_write(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> { None }
    /// # }
    /// let pipeline: Pipeline<String, String> = Pipeline::new();
    /// pipeline.add_front(H1); // Will be first in chain
    /// ```
    pub fn add_front(&self, handler: impl Handler + 'static) -> &Self {
        {
            let mut internal = self.internal.borrow_mut();
            internal.add_front(handler);
        }
        self
    }

    /// Removes the last handler from the pipeline.
    ///
    /// # Returns
    ///
    /// - `Ok(&self)`: Handler removed successfully (for chaining)
    /// - `Err(io::Error)`: Pipeline is empty
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sansio::{Pipeline, Handler, Context};
    /// # struct H1;
    /// # impl Handler for H1 {
    /// #     type Rin = String;
    /// #     type Rout = String;
    /// #     type Win = String;
    /// #     type Wout = String;
    /// #     fn name(&self) -> &str { "H1" }
    /// #     fn handle_read(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, _: Self::Rin) {}
    /// #     fn poll_write(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> { None }
    /// # }
    /// let pipeline: Pipeline<String, String> = Pipeline::new();
    /// pipeline.add_back(H1);
    /// pipeline.remove_back().unwrap();
    /// ```
    pub fn remove_back(&self) -> Result<&Self, std::io::Error> {
        let result = {
            let mut internal = self.internal.borrow_mut();
            internal.remove_back()
        };
        match result {
            Ok(()) => Ok(self),
            Err(err) => Err(err),
        }
    }

    /// Removes the first handler from the pipeline.
    ///
    /// # Returns
    ///
    /// - `Ok(&self)`: Handler removed successfully (for chaining)
    /// - `Err(io::Error)`: Pipeline is empty
    pub fn remove_front(&self) -> Result<&Self, std::io::Error> {
        let result = {
            let mut internal = self.internal.borrow_mut();
            internal.remove_front()
        };
        match result {
            Ok(()) => Ok(self),
            Err(err) => Err(err),
        }
    }

    /// Removes a specific handler by name.
    ///
    /// Searches for a handler with the given name and removes it.
    ///
    /// # Parameters
    ///
    /// - `handler_name`: The name of the handler to remove (from [`Handler::name`])
    ///
    /// # Returns
    ///
    /// - `Ok(&self)`: Handler removed successfully (for chaining)
    /// - `Err(io::Error)`: Handler not found
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sansio::{Pipeline, Handler, Context};
    /// # struct MyHandler;
    /// # impl Handler for MyHandler {
    /// #     type Rin = String;
    /// #     type Rout = String;
    /// #     type Win = String;
    /// #     type Wout = String;
    /// #     fn name(&self) -> &str { "MyHandler" }
    /// #     fn handle_read(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, _: Self::Rin) {}
    /// #     fn poll_write(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> { None }
    /// # }
    /// let pipeline: Pipeline<String, String> = Pipeline::new();
    /// pipeline.add_back(MyHandler);
    /// let pipeline = pipeline.finalize();
    ///
    /// pipeline.remove("MyHandler").unwrap();
    /// ```
    pub fn remove(&self, handler_name: &str) -> Result<&Self, std::io::Error> {
        let result = {
            let mut internal = self.internal.borrow_mut();
            internal.remove(handler_name)
        };
        match result {
            Ok(()) => Ok(self),
            Err(err) => Err(err),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    /// Returns the number of handlers in the pipeline.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sansio::{Pipeline, Handler, Context};
    /// # struct H1;
    /// # impl Handler for H1 {
    /// #     type Rin = String;
    /// #     type Rout = String;
    /// #     type Win = String;
    /// #     type Wout = String;
    /// #     fn name(&self) -> &str { "H1" }
    /// #     fn handle_read(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, _: Self::Rin) {}
    /// #     fn poll_write(&mut self, _: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> { None }
    /// # }
    /// let pipeline: Pipeline<String, String> = Pipeline::new();
    /// assert_eq!(pipeline.len(), 0);
    ///
    /// pipeline.add_back(H1);
    /// assert_eq!(pipeline.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        let internal = self.internal.borrow();
        internal.len()
    }

    /// Updates an `Rc`-wrapped pipeline's internal structure.
    ///
    /// Called internally by [`finalize`]. You typically don't need to call this directly.
    ///
    /// [`finalize`]: Pipeline::finalize
    pub fn update(self: Rc<Self>) -> Rc<Self> {
        {
            let internal = self.internal.borrow();
            internal.finalize();
        }
        self
    }

    /// Finalizes the pipeline, making it ready for use.
    ///
    /// This method:
    /// 1. Wraps the pipeline in `Rc` for shared ownership
    /// 2. Links handlers together internally
    /// 3. Returns the finalized `Rc<Pipeline>`
    ///
    /// **You must call this before using the pipeline.**
    ///
    /// # Example
    ///
    /// ```rust
    /// # use sansio::{Pipeline, InboundPipeline, Handler, Context};
    /// # struct H1;
    /// # impl Handler for H1 {
    /// #     type Rin = String;
    /// #     type Rout = String;
    /// #     type Win = String;
    /// #     type Wout = String;
    /// #     fn name(&self) -> &str { "H1" }
    /// #     fn handle_read(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>, msg: Self::Rin) {
    /// #         ctx.fire_handle_read(msg);
    /// #     }
    /// #     fn poll_write(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) -> Option<Self::Wout> {
    /// #         ctx.fire_poll_write()
    /// #     }
    /// # }
    /// let pipeline: Pipeline<String, String> = Pipeline::new();
    /// pipeline.add_back(H1);
    ///
    /// // Finalize before use
    /// let pipeline = pipeline.finalize();
    ///
    /// // Now ready to use
    /// pipeline.handle_read("Hello".to_string());
    /// ```
    pub fn finalize(self) -> Rc<Self> {
        let pipeline = Rc::new(self);
        pipeline.update()
    }
}

impl<R: 'static, W: 'static> InboundPipeline<R> for Pipeline<R, W> {
    /// Transport is active now, which means it is connected.
    fn transport_active(&self) {
        let internal = self.internal.borrow();
        internal.transport_active();
    }

    /// Transport is inactive now, which means it is disconnected.
    fn transport_inactive(&self) {
        let internal = self.internal.borrow();
        internal.transport_inactive();
    }

    /// Handles an incoming message.
    fn handle_read(&self, msg: R) {
        let internal = self.internal.borrow();
        internal.handle_read(msg);
    }

    /// Polls an outgoing message
    fn poll_write(&self) -> Option<R> {
        let internal = self.internal.borrow();
        internal.poll_write()
    }

    /// Handles a timeout event.
    fn handle_timeout(&self, now: Instant) {
        let internal = self.internal.borrow();
        internal.handle_timeout(now);
    }

    /// Polls earliest timeout (eto) in its inbound operations.
    fn poll_timeout(&self, eto: &mut Instant) {
        let internal = self.internal.borrow();
        internal.poll_timeout(eto);
    }

    /// Handles an EOF event.
    fn handle_eof(&self) {
        let internal = self.internal.borrow();
        internal.handle_eof();
    }

    /// Handles an Error in one of its inbound operations.
    fn handle_error(&self, err: Box<dyn Error>) {
        let internal = self.internal.borrow();
        internal.handle_error(err);
    }
}

impl<R: 'static, W: 'static> OutboundPipeline<R, W> for Pipeline<R, W> {
    /// Writes a message to pipeline
    fn write(&self, msg: W) {
        let internal = self.internal.borrow();
        internal.write(msg);
    }

    /// Writes a close event.
    fn close(&self) {
        let internal = self.internal.borrow();
        internal.handle_close();
    }
}
