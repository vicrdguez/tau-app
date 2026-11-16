//! Identifiers the server hands out. Each is its own type so a conversation number can't be passed as an entry ID.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! text_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(text: impl Into<String>) -> Self {
                Self(text.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(text: &str) -> Self {
                Self(text.to_owned())
            }
        }
    };
}

macro_rules! number_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

text_id!(
    /// A bound project's name (`render-core`).
    Project
);
text_id!(
    /// A paired device's name (`laptop`), from the token or from `by` on an answer.
    Device
);
text_id!(
    /// A decision's subject, `${extension}:${key}` by convention (`w1:3:design`).
    Subject
);
text_id!(
    /// A decision's ID, `${subject}#${n}` (`w1:3:design#2`).
    DecisionId
);
text_id!(
    /// The ID this client gives one `input` or `continue`, sent unchanged on every resend.
    RequestId
);

number_id!(
    /// A conversation's number in its project's store.
    ConversationId
);
number_id!(
    /// An entry's ID in its project's store. Entry IDs grow with time across the store's conversations.
    EntryId
);
number_id!(
    /// A task's ID in its project's store.
    TaskId
);
number_id!(
    /// This device's ID for one outbox entry. Not a server ID.
    WriteId
);

impl Subject {
    /// The text before the first `:`, the extension that asks (`w1:3:design` → `w1`).
    pub fn extension(&self) -> &str {
        self.0.split(':').next().unwrap_or_default()
    }
}

impl DecisionId {
    pub fn subject(&self) -> Subject {
        Subject(match self.0.rfind('#') {
            Some(at) => self.0[..at].to_owned(),
            None => self.0.clone(),
        })
    }

    /// `n` of `subject#n`: how many times the subject was asked, this time included.
    pub fn request_number(&self) -> Option<u32> {
        self.0.rsplit_once('#')?.1.parse().ok()
    }
}

/// A conversation, named by its project.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConvRef {
    pub project: Project,
    pub id: ConversationId,
}

/// A decision, named by its project.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DecisionRef {
    pub project: Project,
    pub id: DecisionId,
}

impl ConvRef {
    pub fn new(project: impl Into<Project>, id: u64) -> Self {
        Self {
            project: project.into(),
            id: ConversationId(id),
        }
    }
}

impl DecisionRef {
    pub fn new(project: impl Into<Project>, id: impl Into<DecisionId>) -> Self {
        Self {
            project: project.into(),
            id: id.into(),
        }
    }
}

impl From<String> for Project {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl From<String> for DecisionId {
    fn from(text: String) -> Self {
        Self(text)
    }
}
