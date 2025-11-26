use bytes::BytesMut;
use clap::Parser;
use log::{trace, warn};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{ErrorKind, Read, Write},
    net::SocketAddr,
    net::{TcpListener, TcpStream},
    rc::Rc,
    rc::Weak,
    str::FromStr,
    time::{Duration, Instant},
};
use wg::WaitGroup;

use sansio::{Context, Handler, InboundPipeline, OutboundPipeline, Pipeline, LocalExecutorBuilder, spawn_local};

mod helpers;

use helpers::{
    byte_to_message_decoder::{LineBasedFrameDecoder, TaggedByteToMessageCodec, TerminatorType},
    string_codec::TaggedStringCodec,
    transport::{Protocol, TaggedBytesMut, TaggedString, TransportContext},
};

////////////////////////////////////////////////////////////////////////////////////////////////////
struct Shared {
    peers: HashMap<SocketAddr, Weak<dyn OutboundPipeline<TaggedBytesMut, TaggedString>>>,
}

impl Shared {
    /// Create a new, empty, instance of `Shared`.
    fn new() -> Self {
        Shared {
            peers: HashMap::new(),
        }
    }

    fn contains(&self, peer: &SocketAddr) -> bool {
        self.peers.contains_key(peer)
    }

    fn join(
        &mut self,
        peer: SocketAddr,
        pipeline: Weak<dyn OutboundPipeline<TaggedBytesMut, TaggedString>>,
    ) {
        println!("{} joined", peer);
        self.peers.insert(peer, pipeline);
    }

    fn leave(&mut self, peer: &SocketAddr) {
        println!("{} left", peer);
        self.peers.remove(peer);
    }

    /// Send message to every peer, except for the sender.
    fn broadcast(&self, sender: SocketAddr, msg: TaggedString) {
        print!("broadcast message: {}", msg.message);
        for (peer, pipeline) in self.peers.iter() {
            if *peer != sender {
                if let Some(pipeline) = pipeline.upgrade() {
                    let _ = pipeline.write(TaggedString {
                        now: msg.now,
                        transport: TransportContext {
                            local_addr: msg.transport.local_addr,
                            peer_addr: *peer,
                            ecn: msg.transport.ecn,
                            protocol: msg.transport.protocol,
                        },
                        message: msg.message.clone(),
                    });
                }
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
struct ChatHandler {
    state: Rc<RefCell<Shared>>,
    pipeline: Weak<dyn OutboundPipeline<TaggedBytesMut, TaggedString>>,
    peer_addr: Option<SocketAddr>,
}

impl ChatHandler {
    fn new(
        state: Rc<RefCell<Shared>>,
        pipeline: Weak<dyn OutboundPipeline<TaggedBytesMut, TaggedString>>,
    ) -> Self {
        ChatHandler {
            state,
            pipeline,
            peer_addr: None,
        }
    }
}

impl Handler for ChatHandler {
    type Rin = TaggedString;
    type Rout = Self::Rin;
    type Win = TaggedString;
    type Wout = Self::Win;

    fn name(&self) -> &str {
        "ChatHandler"
    }

    fn handle_read(
        &mut self,
        _ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>,
        msg: Self::Rin,
    ) {
        let peer_addr = msg.transport.peer_addr;
        println!(
            "received: {} from {:?} to {}",
            msg.message, peer_addr, msg.transport.local_addr
        );

        let mut s = self.state.borrow_mut();
        if !s.contains(&peer_addr) {
            s.join(peer_addr, self.pipeline.clone());
            self.peer_addr = Some(peer_addr);
        }
        s.broadcast(
            peer_addr,
            TaggedString {
                now: Instant::now(),
                transport: TransportContext {
                    local_addr: msg.transport.local_addr,
                    ecn: msg.transport.ecn,
                    ..Default::default()
                },
                message: format!("{}\r\n", msg.message),
            },
        );
    }

    fn handle_eof(&mut self, ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>) {
        // first leave itself from state, otherwise, it may still receive message from broadcast,
        // which may cause data racing.
        if let Some(peer_addr) = self.peer_addr {
            let mut s = self.state.borrow_mut();
            s.leave(&peer_addr);
        }
        ctx.fire_handle_close();
    }

    fn handle_timeout(
        &mut self,
        _ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>,
        _now: Instant,
    ) {
        //last handler, no need to fire_handle_timeout
    }
    fn poll_timeout(
        &mut self,
        _ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>,
        _eto: &mut Instant,
    ) {
        //last handler, no need to fire_poll_timeout
    }

    fn poll_write(
        &mut self,
        ctx: &Context<Self::Rin, Self::Rout, Self::Win, Self::Wout>,
    ) -> Option<Self::Wout> {
        ctx.fire_poll_write()
    }
}

#[derive(Parser)]
#[command(name = "Chat Server TCP")]
#[command(author = "Rusty Rain <y@liu.mx>")]
#[command(version = "0.1.0")]
#[command(about = "An example of chat server tcp", long_about = None)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
    #[arg(long, default_value_t = format!("0.0.0.0"))]
    host: String,
    #[arg(long, default_value_t = 8080)]
    port: u16,
    #[arg(long, default_value_t = format!("INFO"))]
    log_level: String,
}

fn build_pipeline(state: Rc<RefCell<Shared>>) -> Rc<Pipeline<TaggedBytesMut, TaggedString>> {
    let pipeline: Rc<Pipeline<TaggedBytesMut, TaggedString>> = Rc::new(Pipeline::new());

    let line_based_frame_decoder_handler = TaggedByteToMessageCodec::new(Box::new(
        LineBasedFrameDecoder::new(8192, true, TerminatorType::BOTH),
    ));
    let string_codec_handler = TaggedStringCodec::new();
    let pipeline_wr = Rc::downgrade(&pipeline);
    let chat_handler = ChatHandler::new(state, pipeline_wr);

    pipeline.add_back(line_based_frame_decoder_handler);
    pipeline.add_back(string_codec_handler);
    pipeline.add_back(chat_handler);
    pipeline.update()
}

fn write_socket_output(
    socket: &mut TcpStream,
    pipeline: &Rc<Pipeline<TaggedBytesMut, TaggedString>>,
) -> anyhow::Result<()> {
    while let Some(transmit) = pipeline.poll_write() {
        match socket.write(&transmit.message) {
            Ok(n) => {
                trace!("socket write {} bytes", n);
            }
            Err(err) => {
                warn!("socket write error {}", err);
                break;
            }
        }
    }

    Ok(())
}

fn read_socket_input(socket: &mut TcpStream, buf: &mut [u8]) -> Option<TaggedBytesMut> {
    match socket.read(buf) {
        Ok(n) => Some(TaggedBytesMut {
            now: Instant::now(),
            transport: TransportContext {
                local_addr: socket.local_addr().unwrap(),
                peer_addr: socket.peer_addr().unwrap(),
                protocol: Protocol::TCP,
                ecn: None,
            },
            message: BytesMut::from(&buf[..n]),
        }),

        Err(e) => match e.kind() {
            // Expected error for set_read_timeout(). One for windows, one for the rest.
            ErrorKind::WouldBlock | ErrorKind::TimedOut => None,
            _ => panic!("UdpSocket read failed: {e:?}"),
        },
    }
}

fn run_pipeline(
    mut socket: TcpStream,
    stop_rx: crossbeam_channel::Receiver<()>,
    state: Rc<RefCell<Shared>>,
) -> anyhow::Result<()> {
    let mut buf = vec![0; 2000];

    let pipeline = build_pipeline(state);
    pipeline.transport_active();
    loop {
        // Check cancellation
        match stop_rx.try_recv() {
            Ok(_) => break,
            Err(err) => {
                if err.is_disconnected() {
                    break;
                }
            }
        };

        write_socket_output(&mut socket, &pipeline)?;

        // Poll pipeline to get next timeout
        let mut eto = Instant::now() + Duration::from_millis(100);
        pipeline.poll_timeout(&mut eto);

        let delay_from_now = eto
            .checked_duration_since(Instant::now())
            .unwrap_or(Duration::from_secs(0));
        if delay_from_now.is_zero() {
            pipeline.handle_timeout(Instant::now());
            continue;
        }

        socket
            .set_read_timeout(Some(delay_from_now))
            .expect("setting socket read timeout");

        if let Some(input) = read_socket_input(&mut socket, &mut buf) {
            pipeline.handle_read(input);
        }

        // Drive time forward
        pipeline.handle_timeout(Instant::now());
    }
    pipeline.transport_inactive();

    println!("server on {} is gracefully down", socket.local_addr()?);
    Ok(())
}

async fn run(stop_rx: crossbeam_channel::Receiver<()>, host: String, port: u16) -> anyhow::Result<()> {
    let wait_group = WaitGroup::new();

    // Create the shared state. This is how all the peers communicate.
    // The server task will hold a handle to this. For every new client, the
    // `state` handle is cloned and passed into the handler that processes the
    // client connection.
    let state = Rc::new(RefCell::new(Shared::new()));

    let listener = TcpListener::bind(format!("{host}:{port}"))?;
    listener.set_nonblocking(true)?;

    let listener_stop_rx = stop_rx.clone();
    loop {
        // Check cancellation
        match listener_stop_rx.try_recv() {
            Ok(_) => break,
            Err(err) => {
                if err.is_disconnected() {
                    break;
                }
            }
        };

        match listener.accept() {
            Ok((stream, addr)) => {
                println!("Connection from {addr}");
                let worker = wait_group.add(1);
                let stream_stop_rx = listener_stop_rx.clone();
                let state_clone = state.clone();
                spawn_local( async move {
                    if let Err(err) = run_pipeline(stream, stream_stop_rx, state_clone) {
                        eprintln!("run got error: {}", err);
                    }
                    worker.done();
                }).detach();
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                eprintln!("Accept error: {e}");
                break;
            }
        }
    }

    println!("Wait for Gracefully Shutdown...");
    wait_group.wait();

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let host = cli.host;
    let port = cli.port;
    let log_level = log::LevelFilter::from_str(&cli.log_level)?;
    if cli.debug {
        env_logger::Builder::new()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{}:{} [{}] {} - {}",
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    record.level(),
                    chrono::Local::now().format("%H:%M:%S.%6f"),
                    record.args()
                )
            })
            .filter(None, log_level)
            .init();
    }

    let (stop_tx, stop_rx) = crossbeam_channel::bounded::<()>(1);

    println!("Press Ctrl-C to stop");
    println!("try `nc {} {}` in another shell", host, port);
    std::thread::spawn(move || {
        let mut stop_tx = Some(stop_tx);
        ctrlc::set_handler(move || {
            if let Some(stop_tx) = stop_tx.take() {
                let _ = stop_tx.send(());
            }
        })
        .expect("Error setting Ctrl-C handler");
    });

    LocalExecutorBuilder::default().run(async move {
        if let Err(err) = run(stop_rx, host, port).await {
            eprintln!("run got error: {}", err);
        }
    });

   Ok(())
}
