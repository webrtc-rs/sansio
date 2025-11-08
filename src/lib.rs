//! ### What is SansIO?
//! SansIO is an IO-free Rust networking framework that makes it easy to build protocols, application clients/servers.
//!
//! It's like [Netty](https://netty.io) or [Wangle](https://github.com/facebook/wangle), but in Rust.
//!
//! ### What is a Pipeline?
//! The fundamental abstraction of SansIO is the [Pipeline](crate::Pipeline).
//! It offers immense flexibility to customize how requests and responses are handled by your service.
//! Once you have fully understood this abstraction,
//! you will be able to write all sorts of sophisticated protocols, application clients/servers.
//!
//! A [Pipeline](crate::Pipeline) is a chain of request/response [handlers](crate::Handler) that handle inbound request and
//! outbound response. Once you chain handlers together, it provides an agile way to convert
//! a raw data stream into the desired message type and the inverse -- desired message type to raw data stream.
//! Pipeline implements an advanced form of the Intercepting Filter pattern to give a user full control
//! over how an event is handled and how the handlers in a pipeline interact with each other.
//!
//! A [Handler](crate::Handler) should do one and only one function - just like the UNIX philosophy. If you have a handler that
//! is doing more than one function than you should split it into individual handlers. This is really important for
//! maintainability and flexibility as its common to change your protocol for one reason or the other.
//!
//! ### How does an event flow in a Pipeline?
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
//!   |          Context.fire_read()                      |               |
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
//!                   | read()                            | poll_write()
//!                   |                                  \|/
//!   +---------------+-----------------------------------+---------------+
//!   |               |                                   |               |
//!   |        Internal I/O Threads (Transport Implementation)            |
//!   +-------------------------------------------------------------------+
//! ```
//!
//! ### Echo Server Example
//! Let's look at how to write an echo server.
//!
//! Here's the main piece of code in our echo server; it receives a string from inbound direction in the pipeline,
//! prints it to stdout and sends it back to outbound direction in the pipeline. It's really important to add the
//! line delimiter because our pipeline will use a line decoder.
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
//! This needs to be the final handler in the pipeline. Now the definition of the pipeline is needed to handle the requests and responses.
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
//! It is very important to be strict in the order of insertion as they are ordered by insertion. The pipeline has 4 handlers:
//!
//! * ByteToMessageCodecHandler
//!     * Inbound: receives a zero-copy byte buffer and splits on line-endings
//!     * Outbound: just passes the byte buffer to network transport layer
//! * StringCodecHandler
//!     * Inbound: receives a byte buffer and decodes it into a std::string and pass up to the EchoHandler.
//!     * Outbound: receives a std::string and encodes it into a byte buffer and pass down to the ByteToMessageCodec.
//! * EchoHandler
//!     * Inbound: receives a std::string and writes it to the pipeline, which will send the message outbound.
//!     * Outbound: receives a std::string and forwards it to StringCodec.
//!
//! Now that all needs to be done is plug the pipeline factory into a Bootstrap run function and that’s pretty much it.
//!
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
//!         let delay_from_now = eto.checked_duration_since(Instant::now()).unwrap_or(Duration::from_secs(0));
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
#![warn(rust_2018_idioms)]
#![allow(dead_code)]
#![warn(missing_docs)]

pub(crate) mod handler;
pub(crate) mod handler_internal;
pub(crate) mod pipeline;
pub(crate) mod pipeline_internal;
pub(crate) mod protocol;

pub use handler::{Context, Handler};
pub use pipeline::{InboundPipeline, OutboundPipeline, Pipeline};
pub use protocol::Protocol;
