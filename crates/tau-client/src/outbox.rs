//! The outbox (`client.md` §2): every write, with its exact body, kept on this device until it is settled.

use crate::clock::{Mono, Wall};
use crate::ids::{ConvRef, ConversationId, DecisionId, DecisionRef, EntryId, Project, WriteId};
use crate::wire::{Annotation, DecisionRecord, InputMode, Refusal, ThinkingLevel, Unknown};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::time::Duration;

/// A write the UI asks for.
#[derive(Clone, Debug, PartialEq)]
pub enum Write {
    Input {
        conv: ConvRef,
        text: String,
        mode: InputMode,
    },
    Continue {
        conv: ConvRef,
    },
    Stop {
        conv: ConvRef,
    },
    SetModel {
        conv: ConvRef,
        model: String,
        thinking: Option<ThinkingLevel>,
    },
    SetOptions {
        project: Project,
        extension: String,
        patch: Map<String, Value>,
    },
    Bind {
        project: Project,
        workdir: String,
        extensions: Vec<String>,
    },
    AddExtensions {
        project: Project,
        extensions: Vec<String>,
    },
    Answer {
        decision: DecisionRef,
        basis: EntryId,
        choice: String,
        annotations: Vec<Annotation>,
    },
    /// The start page's `⌘⏎`: `create`, then, only on its result, `input` (`client.md` §2.4).
    Start {
        project: Project,
        with: StartWith,
        text: String,
    },
}

/// What a new conversation starts with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartWith {
    Workflow(String),
    /// The bare agent on this model.
    Model(String),
}

/// Why a write was not taken. Nothing was sent and nothing entered the outbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotSent {
    /// Empty or white-space text.
    Empty,
    /// Over 1 MiB: `Over 1 MiB · nothing was sent`.
    TooLarge,
    /// No stream is connected; only an input waits for the server.
    Offline,
    /// A `create` from the start page is in flight: one press makes at most one `create`.
    Starting,
    /// This decision has an answer whose outcome is unknown, or one still sending.
    AnswerUnsettled,
    /// The server refused this device on a call; writes wait until a call succeeds again.
    Refused,
    Revoked,
    NotPaired,
    /// The server speaks another protocol; nothing is sent.
    OtherProtocol,
    /// This entry can't be sent again from the outbox (`client.md` §2.1).
    NoRetry,
    NoSuchEntry,
}

/// The write calls, each with its row of `client.md` §2.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WriteCall {
    Input,
    Continue,
    Stop,
    SetModel,
    SetOptions,
    Bind,
    AddExtensions,
    Answer,
    Create,
}

/// What may send an entry again after an unknown outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resends {
    /// The schedule of §2.2 sends it again by itself, within 10 minutes and the same launch.
    pub automatic: bool,
    /// `␣rt` sends it again with the same body.
    pub retry: Retry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retry {
    Yes,
    /// Not once restored from an earlier launch: another device may have set a newer value.
    NotRestored,
    /// Never from the outbox: an answer is sent again only through the readback (`␣ar`), and `create` and `stop`
    /// not at all.
    No,
}

impl WriteCall {
    /// The table of `client.md` §2.1.
    pub fn resends(self) -> Resends {
        use Retry::*;
        let (automatic, retry) = match self {
            WriteCall::Input | WriteCall::Continue => (true, Yes),
            WriteCall::SetModel | WriteCall::SetOptions => (true, NotRestored),
            WriteCall::Bind | WriteCall::AddExtensions => (true, Yes),
            WriteCall::Answer | WriteCall::Create | WriteCall::Stop => (false, No),
        };
        Resends { automatic, retry }
    }
}

/// The automatic resend schedule (`client.md` §2.2): the first try 1 s after the unknown outcome (or at the next
/// stream connection), then 2, 4, 8, 16 and 30 s.
pub const RESEND_DELAYS: [Duration; 6] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(16),
    Duration::from_secs(30),
];
/// An entry older than this is never sent automatically.
pub const AUTO_RESEND_WINDOW: Duration = Duration::from_secs(600);
/// A done entry leaves the outbox after this.
pub const DONE_LINGERS: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub id: WriteId,
    pub call: WriteCall,
    /// The exact body, sent unchanged on every try.
    pub body: Value,
    pub state: WriteState,
    pub created: Wall,
    /// When it was last sent.
    pub sent_at: Option<Wall>,
    /// Automatic resends made so far.
    pub tries: u32,
    /// Restored from an earlier launch: never sent automatically.
    pub restored: bool,
    /// Made by the start page (`create` and its `input`).
    pub from_start: bool,
    #[serde(skip)]
    pub(crate) created_mono: Option<Mono>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WriteState {
    /// Not sent yet. `next` says when it goes: at the next connection (`waiting for the server`), or only on `␣rt`.
    Waiting {
        next: Next,
    },
    Sending,
    Done {
        at: Wall,
        result: Value,
    },
    Refused(Refusal),
    /// Sent, and no reply proved the outcome.
    Unknown {
        reason: Unknown,
        next: Next,
    },
    /// An answer whose outcome the server later proved (`client.md` §2.3).
    Settled(Settlement),
}

/// When an unsettled entry is sent next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Next {
    At(Mono),
    AtConnection,
    /// Not sent automatically: `␣rt` retries when the call allows it.
    Manual,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Settlement {
    /// It landed: the inbox dropped the decision and `decision` shows this device's choice and notes.
    Landed(DecisionRecord),
    /// A fresh inbox still lists the decision open: `␣ad` is live again.
    NotLanded,
    /// Another device answered it first, or with other fields: the copy of `conflict`.
    AnsweredElsewhere(DecisionRecord),
    /// Superseded or withdrawn: the copy of `not_open`.
    NoLongerOpen(DecisionRecord),
}

/// Every write of this device, oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outbox {
    pub(crate) entries: Vec<OutboxEntry>,
    pub(crate) next_id: u64,
}

impl OutboxEntry {
    pub fn project(&self) -> Option<Project> {
        self.body
            .get("project")
            .and_then(Value::as_str)
            .map(Project::from)
    }

    pub fn conversation(&self) -> Option<ConvRef> {
        let id = self.body.get("conversation").and_then(Value::as_u64)?;
        Some(ConvRef {
            project: self.project()?,
            id: ConversationId(id),
        })
    }

    pub fn decision(&self) -> Option<DecisionRef> {
        let id = self.body.get("decision").and_then(Value::as_str)?;
        Some(DecisionRef {
            project: self.project()?,
            id: DecisionId::from(id),
        })
    }

    /// The text of an `input`.
    pub fn text(&self) -> Option<&str> {
        self.body.get("text").and_then(Value::as_str)
    }

    pub fn mode(&self) -> InputMode {
        match self.body.get("whenBusy").and_then(Value::as_str) {
            Some("steer") => InputMode::Steer,
            Some("followUp") => InputMode::FollowUp,
            _ => InputMode::Send,
        }
    }

    /// Counted by the `?` tag.
    pub fn is_unknown(&self) -> bool {
        matches!(self.state, WriteState::Unknown { .. })
    }

    /// Not yet proved done or refused.
    pub fn is_unsettled(&self) -> bool {
        matches!(
            self.state,
            WriteState::Waiting { .. } | WriteState::Sending | WriteState::Unknown { .. }
        )
    }

    /// `␣rt` may send it again now.
    pub fn can_retry(&self) -> bool {
        let unsettled = matches!(
            self.state,
            WriteState::Waiting { .. } | WriteState::Unknown { .. }
        );
        unsettled
            && match self.call.resends().retry {
                Retry::Yes => true,
                Retry::NotRestored => !self.restored,
                Retry::No => false,
            }
    }

    /// `␣ar` may send this answer's stored body again, through the readback.
    pub fn can_send_again(&self) -> bool {
        self.call == WriteCall::Answer && self.is_unknown()
    }
}

impl Outbox {
    pub fn entries(&self) -> &[OutboxEntry] {
        &self.entries
    }

    pub fn get(&self, id: WriteId) -> Option<&OutboxEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// The `?` count.
    pub fn unknown_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_unknown()).count()
    }

    /// The start page's `create` in flight: the page reads `starting…`.
    pub fn starting(&self) -> bool {
        self.entries
            .iter()
            .any(|e| e.call == WriteCall::Create && e.state == WriteState::Sending)
    }

    /// An answer for this decision still sending or with an unknown outcome: `␣ad` is off.
    pub fn answer_unsettled(&self, decision: &DecisionRef) -> Option<&OutboxEntry> {
        self.entries.iter().find(|e| {
            e.call == WriteCall::Answer
                && matches!(e.state, WriteState::Sending | WriteState::Unknown { .. })
                && e.decision().as_ref() == Some(decision)
        })
    }

    /// Unsettled inputs of one conversation, oldest first: the unsent rows above its composer.
    pub fn unsent_inputs(&self, conv: &ConvRef) -> Vec<&OutboxEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.call == WriteCall::Input
                    && e.is_unsettled()
                    && e.conversation().as_ref() == Some(conv)
            })
            .collect()
    }

    /// What is written to disk: everything but done entries, which leave within a minute anyway.
    pub fn persisted(&self) -> Vec<OutboxEntry> {
        self.entries
            .iter()
            .filter(|e| !matches!(e.state, WriteState::Done { .. }))
            .cloned()
            .collect()
    }

    /// The outbox of an earlier launch: nothing in it is sent automatically.
    pub fn restore(entries: Vec<OutboxEntry>) -> Outbox {
        let entries: Vec<OutboxEntry> = entries
            .into_iter()
            .filter(|e| !matches!(e.state, WriteState::Done { .. }))
            .map(|mut e| {
                e.restored = true;
                e.created_mono = None;
                e.state = match e.state {
                    WriteState::Sending => WriteState::Unknown {
                        reason: Unknown::Restart,
                        next: Next::Manual,
                    },
                    WriteState::Waiting { .. } => WriteState::Waiting { next: Next::Manual },
                    WriteState::Unknown { reason, .. } => WriteState::Unknown {
                        reason,
                        next: Next::Manual,
                    },
                    settled => settled,
                };
                e
            })
            .collect();
        let next_id = entries.iter().map(|e| e.id.0 + 1).max().unwrap_or(1);
        Outbox { entries, next_id }
    }
}
