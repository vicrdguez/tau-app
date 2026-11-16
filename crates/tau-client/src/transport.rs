//! How the runtime reaches a server: one small trait, carried out over HTTP or answered from the fixture.

use crate::engine::StreamEvent;
use crate::wire::{Outcome, Request, Token};

/// Where a call goes: the server's URL and, for every call but `pair`, the device token.
#[derive(Clone, Debug)]
pub struct Target {
    pub url: String,
    pub token: Option<Token>,
}

/// Receives one call's outcome, once, from any thread.
pub type Reply = Box<dyn FnOnce(Outcome) + Send>;
/// Receives a stream's events, from any thread, until the stream ends or is closed.
pub type Deliver = Box<dyn Fn(StreamEvent) + Send + Sync>;

pub trait Transport: Send + Sync {
    /// Starts one call. The outcome goes to `reply`; nothing here blocks the caller.
    fn call(&self, target: &Target, request: Request, reply: Reply);
    /// Opens one stream. Its events go to `deliver` until it ends or the handle closes it.
    fn open(&self, target: &Target, request: Request, deliver: Deliver) -> Box<dyn StreamHandle>;
}

/// An open stream. Closing it stops its delivery and frees its connection within about a second.
pub trait StreamHandle: Send {
    fn close(&self);
}
