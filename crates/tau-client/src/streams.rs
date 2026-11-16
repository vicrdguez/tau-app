//! Supervision of one stream (`client.md` §3.5, §3.6): connecting, live, backing off, or ended by a revoke.

use crate::clock::{Mono, Wall};
use crate::ids::{ConvRef, Project};
use crate::sse::EndReason;
use crate::wire::{Refusal, Unknown};
use std::time::Duration;

/// The streams this client holds: the inbox, one timeline per project, and an `open` per conversation shown.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StreamKey {
    Inbox,
    Timeline(Project),
    Open(ConvRef),
}

/// One attempt to hold a stream. A message from an attempt the stream no longer holds changes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId(pub u64);

/// A stream that delivers nothing, frame or heartbeat, for this long is closed and reopened.
pub const SILENCE: Duration = Duration::from_secs(45);
/// After this many `end: failed` in a row, a notice band.
pub const FAILURES_NOTICED: u32 = 3;

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    /// Asked for; no first frame yet.
    Connecting { id: StreamId, since: Mono },
    /// The first frame came. `last` is when anything last came.
    Live {
        id: StreamId,
        since: Wall,
        last: Mono,
    },
    /// Waiting to reopen.
    Backoff { due: Mono },
    /// Ended `revoked`: never reopened.
    Revoked,
}

/// Why a stream is not up.
#[derive(Clone, Debug, PartialEq)]
pub enum EndCause {
    /// The connection closed with no `end`: a crash or the network.
    Cut(Option<String>),
    /// Nothing came for 45 s.
    Silent,
    End(EndReason),
    /// Refused before it began (a JSON reply instead of a stream).
    Refused(Refusal),
    /// Neither a stream nor a reply this client can read.
    Unknown(Unknown),
    /// A frame that could not be read or applied.
    BadFrame(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stream {
    pub phase: Phase,
    /// Cuts and ends since the last first frame: the backoff step.
    pub failures: u32,
    /// `end: failed` in a row.
    pub failed_in_row: u32,
    pub last_cause: Option<EndCause>,
}

/// The backoff after `failures` failures in a row: 1, 2, 4, 8, 16, 30 s, then every 30 s.
pub fn backoff(failures: u32) -> Duration {
    Duration::from_secs(match failures {
        0 | 1 => 1,
        2 => 2,
        3 => 4,
        4 => 8,
        5 => 16,
        _ => 30,
    })
}

impl Stream {
    pub fn connecting(id: StreamId, now: Mono) -> Stream {
        Stream {
            phase: Phase::Connecting { id, since: now },
            failures: 0,
            failed_in_row: 0,
            last_cause: None,
        }
    }

    /// The attempt this stream holds now, if any.
    pub fn attempt(&self) -> Option<StreamId> {
        match self.phase {
            Phase::Connecting { id, .. } | Phase::Live { id, .. } => Some(id),
            _ => None,
        }
    }

    pub fn is_live(&self) -> bool {
        matches!(self.phase, Phase::Live { .. })
    }

    /// When silence would close it.
    pub fn silent_at(&self) -> Option<Mono> {
        match self.phase {
            Phase::Connecting { since, .. } => Some(since.after(SILENCE)),
            Phase::Live { last, .. } => Some(last.after(SILENCE)),
            _ => None,
        }
    }

    pub fn keeps_failing(&self) -> bool {
        self.failed_in_row >= FAILURES_NOTICED
    }

    pub(crate) fn reopen(&mut self, id: StreamId, now: Mono) {
        self.phase = Phase::Connecting { id, since: now };
    }

    pub(crate) fn first_frame(&mut self, now: Mono, wall: Wall) {
        self.failures = 0;
        if let Phase::Connecting { id, .. } | Phase::Live { id, .. } = self.phase {
            self.phase = Phase::Live {
                id,
                since: wall,
                last: now,
            };
        }
    }

    pub(crate) fn heard(&mut self, now: Mono) {
        if let Phase::Live { last, .. } = &mut self.phase {
            *last = now;
        }
    }

    /// Records an end and schedules the reopen.
    pub(crate) fn ended(&mut self, cause: EndCause, now: Mono) {
        if cause == EndCause::End(EndReason::Revoked) {
            self.phase = Phase::Revoked;
        } else {
            self.failures += 1;
            self.phase = Phase::Backoff {
                due: now.after(backoff(self.failures)),
            };
        }
        self.failed_in_row = if cause == EndCause::End(EndReason::Failed) {
            self.failed_in_row + 1
        } else {
            0
        };
        self.last_cause = Some(cause);
    }
}
