#![warn(
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    unreachable_pub
)]
#![forbid(unsafe_code)]
// `rustdoc::broken_intra_doc_links` is checked on CI

//! Definition of the core `Handler` trait to sansio
//!
//! The [`Handler`] trait provides the necessary abstractions for defining
//! sansio pipeline. It is simple but powerful and is used as the foundation
//! for the rest of sansio pipeline.

use std::time::Instant;

/// The `Handler` trait is a simplified interface making it easy to write
/// network applications in a modular and reusable way, decoupled from the
/// underlying protocol. It is one of sansio fundamental abstractions.
pub trait Handler<Rin, Win, Ein> {
    /// Associated output read type
    type Rout;
    /// Associated output write type
    type Wout;
    /// Associated output event type
    type Eout;
    /// Associated result error type
    type Error;

    /// Handles Rin and returns Rout for next inbound handler handling
    fn handle_read(&mut self, msg: Rin) -> Result<(), Self::Error>;

    /// Polls Rout from internal queue for next inbound handler handling
    fn poll_read(&mut self) -> Option<Self::Rout>;

    /// Handles Win and returns Wout for next outbound handler handling
    fn handle_write(&mut self, msg: Win) -> Result<(), Self::Error>;

    /// Polls Wout from internal queue for next outbound handler handling
    fn poll_write(&mut self) -> Option<Self::Wout>;

    /// Handles event
    fn handle_event(&mut self, _evt: Ein) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Polls event
    fn poll_event(&mut self) -> Option<Self::Eout> {
        None
    }

    /// Handles timeout
    fn handle_timeout(&mut self, _now: Instant) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Polls timeout
    fn poll_timeout(&mut self) -> Option<Instant> {
        None
    }
}

impl<H, Rin, Win, Ein> Handler<Rin, Win, Ein> for &mut H
where
    H: Handler<Rin, Win, Ein> + ?Sized,
{
    type Rout = H::Rout;
    type Wout = H::Wout;
    type Eout = H::Eout;
    type Error = H::Error;

    fn handle_read(&mut self, msg: Rin) -> Result<(), H::Error> {
        (**self).handle_read(msg)
    }

    fn poll_read(&mut self) -> Option<H::Rout> {
        (**self).poll_read()
    }

    fn handle_write(&mut self, msg: Win) -> Result<(), H::Error> {
        (**self).handle_write(msg)
    }

    fn poll_write(&mut self) -> Option<H::Wout> {
        (**self).poll_write()
    }

    fn handle_event(&mut self, evt: Ein) -> Result<(), H::Error> {
        (**self).handle_event(evt)
    }

    fn poll_event(&mut self) -> Option<H::Eout> {
        (**self).poll_event()
    }

    fn handle_timeout(&mut self, now: Instant) -> Result<(), H::Error> {
        (**self).handle_timeout(now)
    }

    fn poll_timeout(&mut self) -> Option<Instant> {
        (**self).poll_timeout()
    }
}

impl<H, Rin, Win, Ein> Handler<Rin, Win, Ein> for Box<H>
where
    H: Handler<Rin, Win, Ein> + ?Sized,
{
    type Rout = H::Rout;
    type Wout = H::Wout;
    type Eout = H::Eout;
    type Error = H::Error;

    fn handle_read(&mut self, msg: Rin) -> Result<(), H::Error> {
        (**self).handle_read(msg)
    }

    fn poll_read(&mut self) -> Option<H::Rout> {
        (**self).poll_read()
    }

    fn handle_write(&mut self, msg: Win) -> Result<(), H::Error> {
        (**self).handle_write(msg)
    }

    fn poll_write(&mut self) -> Option<H::Wout> {
        (**self).poll_write()
    }

    fn handle_event(&mut self, evt: Ein) -> Result<(), H::Error> {
        (**self).handle_event(evt)
    }

    fn poll_event(&mut self) -> Option<H::Eout> {
        (**self).poll_event()
    }

    fn handle_timeout(&mut self, now: Instant) -> Result<(), H::Error> {
        (**self).handle_timeout(now)
    }

    fn poll_timeout(&mut self) -> Option<Instant> {
        (**self).poll_timeout()
    }
}
