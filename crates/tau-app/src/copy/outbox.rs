//! State words and retry chips for entries in the outbox.

use serde_json::Value;
use tau_client::{
    Wall,
    outbox::{AUTO_RESEND_WINDOW, Next, OutboxEntry, Settlement, WriteCall, WriteState},
    wire::CallName,
};

use super::{AnswerAction, RefusalCopy, refusal};
use crate::time;

/// A key and its word, drawn as a chip (`key` is a keymap sequence) and a muted word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyHint {
    pub key: &'static str,
    pub words: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutboxRow {
    pub call: &'static str,
    pub state: String,
    pub unknown: bool,
    pub retry: Option<KeyHint>,
    pub refusal: Option<RefusalCopy>,
}

/// The UI supplies now; retry eligibility remains the client's decision.
pub fn outbox_row(entry: &OutboxEntry, now: Wall) -> OutboxRow {
    let manual = entry.restored
        || time::between(entry.created, now) > AUTO_RESEND_WINDOW
        || matches!(
            entry.state,
            WriteState::Waiting { next: Next::Manual }
                | WriteState::Unknown {
                    next: Next::Manual,
                    ..
                }
        );
    let refused = match &entry.state {
        WriteState::Refused(r) => Some(refusal(
            call_name(entry.call),
            &entry.body,
            r,
            AnswerAction::Answer,
        )),
        _ => None,
    };
    let state = match &entry.state {
        WriteState::Sending => "sending…".into(),
        WriteState::Done { .. } => "done".into(),
        WriteState::Refused(_) => refused
            .as_ref()
            .and_then(|r| r.message.clone())
            .unwrap_or_default(),
        WriteState::Settled(s) => match s {
            Settlement::Landed(_) => "recorded".into(),
            Settlement::NotLanded => "not recorded".into(),
            Settlement::AnsweredElsewhere(d) => super::refusals::conflict(d),
            Settlement::NoLongerOpen(_) => "Not recorded. This request is no longer open.".into(),
        },
        WriteState::Waiting { .. } | WriteState::Unknown { .. } => {
            if entry.restored && matches!(entry.call, WriteCall::SetModel | WriteCall::SetOptions) {
                "may not have been applied · set it again".into()
            } else if matches!(entry.state, WriteState::Unknown { .. }) {
                match entry.call {
                    WriteCall::Answer => "no reply · open the decision to send it again".into(),
                    WriteCall::Create => "No reply · a conversation may have been created · try again from the start page".into(),
                    WriteCall::Stop => "No reply · the run may have stopped".into(),
                    _ if manual => "not sent automatically".into(),
                    WriteCall::Input if entry.from_start => "no reply · resends when connected".into(),
                    _ => "no reply".into(),
                }
            } else if manual {
                "not sent automatically".into()
            } else {
                "waiting for the server".into()
            }
        }
    };
    OutboxRow {
        call: call_word(entry.call),
        state,
        unknown: entry.is_unknown(),
        retry: entry.can_retry().then_some(KeyHint {
            key: "<leader>rt",
            words: if manual { "retries" } else { "retry" },
        }),
        refusal: refused,
    }
}

/// The start page's warning is distinct from create's outbox row.
pub fn start_unknown(body: &Value) -> String {
    let project = body["project"].as_str().unwrap_or("the project");
    format!(
        "? No reply · a conversation may have been created; it would show as empty conversation in {project}"
    )
}

fn call_word(call: WriteCall) -> &'static str {
    match call {
        WriteCall::Input => "input",
        WriteCall::Continue => "continue",
        WriteCall::Stop => "stop",
        WriteCall::SetModel => "set model",
        WriteCall::SetOptions => "set options",
        WriteCall::Bind => "bind",
        WriteCall::AddExtensions => "add extensions",
        WriteCall::Answer => "answer",
        WriteCall::Create => "create",
    }
}

pub(super) fn call_name(call: WriteCall) -> CallName {
    match call {
        WriteCall::Input => CallName::Input,
        WriteCall::Continue => CallName::Continue,
        WriteCall::Stop => CallName::Stop,
        WriteCall::SetModel => CallName::SetModel,
        WriteCall::SetOptions => CallName::SetOptions,
        WriteCall::Bind => CallName::Bind,
        WriteCall::AddExtensions => CallName::AddExtensions,
        WriteCall::Answer => CallName::Answer,
        WriteCall::Create => CallName::Create,
    }
}
