//! Definition of the core `Protocol` trait to sansio
//!
//! The [`Protocol`] trait provides the necessary abstractions for defining
//! sans-io protocol. It is simple but powerful and is used as the foundation
//! for the rest of sansio library.

use std::time::Instant;

/// The `Protocol` trait is a simplified interface making it easy to write
/// network protocols in a modular and reusable way, decoupled from the
/// underlying network and timer, etc. It is one of sans-io fundamental abstractions.
pub trait Protocol {
    /// Associated transmit type
    type Transmit;

    /// Associated event type
    type Event;

    /// Associated error type
    type Error;

    /// Handles transmit
    fn handle_transmit(&mut self, transmit: Self::Transmit) -> Result<(), Self::Error>;

    /// Polls transmit
    fn poll_transmit(&mut self) -> Option<Self::Transmit>;

    /// Handles event
    fn handle_event(&mut self, _evt: Self::Event) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Polls event
    fn poll_event(&mut self) -> Option<Self::Event> {
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

    /// Closes protocol
    fn close(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<P> Protocol for &mut P
where
    P: Protocol + ?Sized,
{
    type Transmit = P::Transmit;
    type Event = P::Event;
    type Error = P::Error;

    fn handle_transmit(&mut self, msg: P::Transmit) -> Result<(), P::Error> {
        (**self).handle_transmit(msg)
    }

    fn poll_transmit(&mut self) -> Option<P::Transmit> {
        (**self).poll_transmit()
    }

    fn handle_event(&mut self, evt: P::Event) -> Result<(), P::Error> {
        (**self).handle_event(evt)
    }

    fn poll_event(&mut self) -> Option<P::Event> {
        (**self).poll_event()
    }

    fn handle_timeout(&mut self, now: Instant) -> Result<(), P::Error> {
        (**self).handle_timeout(now)
    }

    fn poll_timeout(&mut self) -> Option<Instant> {
        (**self).poll_timeout()
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        (**self).close()
    }
}

impl<P> Protocol for Box<P>
where
    P: Protocol + ?Sized,
{
    type Transmit = P::Transmit;
    type Event = P::Event;
    type Error = P::Error;

    fn handle_transmit(&mut self, msg: P::Transmit) -> Result<(), P::Error> {
        (**self).handle_transmit(msg)
    }

    fn poll_transmit(&mut self) -> Option<P::Transmit> {
        (**self).poll_transmit()
    }

    fn handle_event(&mut self, evt: P::Event) -> Result<(), P::Error> {
        (**self).handle_event(evt)
    }

    fn poll_event(&mut self) -> Option<P::Event> {
        (**self).poll_event()
    }

    fn handle_timeout(&mut self, now: Instant) -> Result<(), P::Error> {
        (**self).handle_timeout(now)
    }

    fn poll_timeout(&mut self) -> Option<Instant> {
        (**self).poll_timeout()
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        (**self).close()
    }
}
