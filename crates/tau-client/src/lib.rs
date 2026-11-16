//! The tau desktop client's link to one tau server: calls, streams, the outbox and what the server last said.
//! See `README.md` for the public surface and how the pure engine and the threaded runtime split.

pub mod clock;
pub mod engine;
pub mod fixture;
pub mod http;
pub mod ids;
pub mod link;
pub mod model;
pub mod ops;
pub mod outbox;
pub mod shapes;
pub mod sse;
pub mod store;
pub mod streams;
pub mod transport;
pub mod wire;

pub use clock::{Mono, Now, Wall};
pub use engine::{LinkEvent, LinkPhase, LinkStatus, Notice, PairFailure, Slot, TranscriptKey};
pub use fixture::Fixture;
pub use ids::{
    ConvRef, ConversationId, DecisionId, DecisionRef, Device, EntryId, Project, Subject, TaskId,
    WriteId,
};
pub use link::{Link, LinkConfig};
pub use model::{Known, Label, Server};
pub use outbox::{NotSent, Outbox, OutboxEntry, StartWith, Write, WriteCall, WriteState};
