//! `Server`: everything the server said, as last received. The UI reads it and never writes it.
//!
//! Each thing fetched is [`Known`]: not received yet, held as of a time, or gone from the server. The UI must
//! never draw calm before data arrives (`spec.md` §4.8), so "not yet" is its own state, not an empty list.

use crate::clock::{Mono, Wall};
use crate::ids::{
    ConvRef, ConversationId, DecisionId, DecisionRef, Device, EntryId, Project, Subject,
};
use crate::shapes::{Docs, Entry, EntryBody, label};
use crate::wire::{
    ConversationRow, DecisionView, ExtensionInfo, Fault, ModelInfo, OpenDecision, OptionsItem,
    ProjectView,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Known<T> {
    #[default]
    NotYet,
    Have {
        value: T,
        at: Wall,
    },
    /// The server said it has no such object (`unknown_project`, `unknown_conversation`, `unknown_decision`).
    Gone {
        message: String,
    },
}

impl<T> Known<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Known::Have { value, .. } => Some(value),
            _ => None,
        }
    }

    pub fn at(&self) -> Option<Wall> {
        match self {
            Known::Have { at, .. } => Some(*at),
            _ => None,
        }
    }
}

/// The one read-only model of one server.
#[derive(Clone, Debug, Default)]
pub struct Server {
    pub url: Option<String>,
    pub device: Option<Device>,
    pub projects: Known<Vec<ProjectView>>,
    pub extensions: Known<Vec<ExtensionInfo>>,
    pub models: Known<Vec<ModelInfo>>,
    /// Open decisions across projects, live while the inbox stream is up; `at` is the last frame's time.
    pub inbox: Known<Inbox>,
    /// Per project, every page of `conversations`, newest first.
    pub conversations: BTreeMap<Project, Known<Vec<ConversationRow>>>,
    /// Per project, every page of `faults`, newest first.
    pub faults: BTreeMap<Project, Known<Vec<Fault>>>,
    pub options: BTreeMap<Project, Known<Vec<OptionsItem>>>,
    /// Per project, its event log.
    pub timelines: BTreeMap<Project, Transcript>,
    /// Conversations this device has watched. Entries stay after the stream closes.
    pub transcripts: HashMap<ConvRef, Transcript>,
    pub decisions: HashMap<DecisionRef, Known<DecisionView>>,
    pub labels: Labels,
}

/// The inbox stream's value: per project, per subject, the open decision.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inbox {
    pub projects: BTreeMap<Project, BTreeMap<Subject, OpenDecision>>,
}

/// What the inbox says about one decision (`spec.md` §5.6 "Liveness").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Liveness {
    Open,
    /// Its subject now has another open decision.
    Superseded {
        by: DecisionId,
    },
    /// Its subject has no open decision: answered, withdrawn, or never listed.
    NotOpen,
}

impl Inbox {
    /// The `◆` count: open decisions of every project.
    pub fn count(&self) -> usize {
        self.projects.values().map(BTreeMap::len).sum()
    }

    /// Needs you: every open decision, oldest `requestedAt` first.
    pub fn oldest_first(&self) -> Vec<(&Project, &OpenDecision)> {
        let mut all: Vec<_> = self
            .projects
            .iter()
            .flat_map(|(p, subjects)| subjects.values().map(move |d| (p, d)))
            .collect();
        all.sort_by(|a, b| {
            a.1.requested_at
                .cmp(&b.1.requested_at)
                .then_with(|| a.1.id.cmp(&b.1.id))
        });
        all
    }

    pub fn liveness(&self, decision: &DecisionRef) -> Liveness {
        let open = self
            .projects
            .get(&decision.project)
            .and_then(|s| s.get(&decision.id.subject()));
        match open {
            Some(d) if d.id == decision.id => Liveness::Open,
            Some(d) => Liveness::Superseded { by: d.id.clone() },
            None => Liveness::NotOpen,
        }
    }

    pub fn contains(&self, decision: &DecisionRef) -> bool {
        self.liveness(decision) == Liveness::Open
    }
}

/// A watched conversation or a project's event log: every entry seen, by ID, and the documents as last received.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transcript {
    /// The conversation the stream names (for a timeline, the log's).
    pub conversation: Option<ConversationId>,
    /// Kept for good: the window dropping its oldest never drops them here.
    pub entries: BTreeMap<EntryId, Entry>,
    pub docs: Known<Docs>,
    /// The newest entry ID before the stream's window, as last received.
    pub before: Option<EntryId>,
    /// Entries missed while away, being fetched (`client.md` §3.3): the UI draws one chrome line there.
    pub gap: Option<Gap>,
    pub older: Older,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    pub min: EntryId,
    pub max: EntryId,
}

/// Scrolling back past the oldest held entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Older {
    /// Older entries may exist; none asked for yet.
    #[default]
    Unknown,
    /// A page is on its way.
    Fetching,
    /// More pages wait behind this cursor.
    More { cursor: String, max: EntryId },
    /// The oldest entry is held: nothing is older.
    Complete,
}

impl Transcript {
    pub fn newest(&self) -> Option<EntryId> {
        self.entries.keys().next_back().copied()
    }

    pub fn oldest(&self) -> Option<EntryId> {
        self.entries.keys().next().copied()
    }

    /// True when the first entry of the conversation is held, so its first input is known.
    pub fn has_start(&self) -> bool {
        self.older == Older::Complete
    }

    /// The first input's label, when the conversation's start is held and holds one.
    pub fn label(&self) -> Option<String> {
        if !self.has_start() {
            return None;
        }
        self.entries
            .values()
            .find(|e| matches!(e.body, EntryBody::User { .. }))
            .and_then(|e| e.user_text().map(label))
    }
}

/// Labels found for good, and conversations asked about that had no input yet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Labels {
    pub found: BTreeMap<ConvRef, String>,
    pub(crate) asked: HashMap<ConvRef, Mono>,
}

/// What a conversation's row is called.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    Found(String),
    /// Asked, and it has no input yet: `empty conversation`.
    Empty,
    NotYet,
}

impl Server {
    pub fn label(&self, conv: &ConvRef) -> Label {
        if let Some(text) = self.labels.found.get(conv) {
            Label::Found(text.clone())
        } else if self.labels.asked.contains_key(conv) {
            Label::Empty
        } else {
            Label::NotYet
        }
    }

    /// The `◆` count, once the inbox has said.
    pub fn open_decisions(&self) -> Option<usize> {
        self.inbox.value().map(Inbox::count)
    }
}
