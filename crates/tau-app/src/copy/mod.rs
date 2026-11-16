//! Plain UI words for client outcomes. No I/O, widget types, or client state changes.
//!
//! Refusals preserve server sentences verbatim and never append punctuation to them.
//! Field errors and bind's stored project remain separate so a screen can place them.
//! `bad_cursor` has no message because paging restarts. `idle` and `event_log`
//! have no message because this client never provokes them.
//!
//! Outbox rows separate the unknown mark, call, state, and optional key hint.
//! Draw `unknown` as `?`, and draw `retry.key` as a chip followed by its
//! `words`. For a manual retry, join state and hint with ` · `.
//! Never put the key into an echo.
//!
//! Event echoes omit Started, Superseded, Revoked, Paired, PairFailed (Connect shows it), this device's
//! DecisionLeft, unrecognized stream endings, in-flight writes, ordinary done
//! writes, unknown writes, and not-landed settlements. Those update views,
//! bands, or outbox rows instead. Refusal echoes contain only the heading;
//! field errors and stored records belong in the originating view.
//!
//! AnswerAction distinguishes a send-again after a fault from an answer or
//! a resend after no reply. Only the former says that work runs again.
//! Event payloads omit the original request and device name, so callers supply
//! those from their outbox and session. Connect calls pair_failure with the URL it paired to.

mod connect;
mod events;
mod outbox;
mod project;
mod refusals;
pub use project::project_not_sent;

pub use connect::{no_server, pair_failure};
pub use events::event_echo;
pub use outbox::{KeyHint, OutboxRow, outbox_row, start_unknown};
pub use refusals::{AnswerAction, RefusalCopy, device_refused, refusal};

#[cfg(test)]
mod tests;
