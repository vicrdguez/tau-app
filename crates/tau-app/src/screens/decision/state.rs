//! The decision family's state: per decision, the reader's cursor and view; the note drafts; the
//! note being typed; and the answers this view sent, so their replies land where they were made.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::ops::Range;

use tau_client::{DecisionId, DecisionRef, Wall, WriteId};

use super::notes::Notes;
use crate::copy::AnswerAction;
use crate::editor::{EditMode, Editor, Lines};

/// The reader's cursor: a caret in the basis (a byte of `text`), or a choice row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Cursor {
    Text(usize),
    Choice(usize),
}

/// An operator waiting for its motion or object (`c`, `gc`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Annotate,
    /// `ci` or `ca`, waiting for `w p s`.
    Object {
        around: bool,
    },
}

/// One decision as the reader left it.
#[derive(Clone, Debug, Default)]
pub struct Page {
    /// `None` until the basis arrives: the first line of its content.
    pub cursor: Option<Cursor>,
    /// Visual's anchor, and whether it is linewise (`V`).
    pub visual: Option<(usize, bool)>,
    pub op: Option<Op>,
    /// `␣ts`: the basis as plain source text.
    pub source: bool,
    /// Earlier requests opened in the margin.
    pub earlier: BTreeSet<DecisionId>,
}

/// Where a note being typed sits.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Anchor {
    Whole,
    /// UTF-16 code units of `text`.
    Range {
        range: Range<u64>,
        widened: bool,
    },
}

/// The note being typed, in the margin at its place.
pub struct Editing {
    pub decision: DecisionRef,
    pub anchor: Anchor,
    /// The draft it edits, by index into the decision's drafts; `None` for a new note.
    pub draft: Option<usize>,
    pub editor: Editor,
}

impl Editing {
    pub fn new(decision: DecisionRef, anchor: Anchor, draft: Option<usize>, text: &str) -> Self {
        let mut editor = Editor::inserting(text, Lines::Multi);
        editor.edit.focused = true;
        Editing {
            decision,
            anchor,
            draft,
            editor,
        }
    }

    pub fn inserting(&self) -> bool {
        self.editor.mode() == EditMode::Insert
    }
}

#[derive(Default)]
pub struct State {
    pub pages: HashMap<DecisionRef, Page>,
    pub notes: Notes,
    pub editing: Option<Editing>,
    /// Answers this view sent, with how their echo reads.
    pub sent: HashMap<WriteId, (DecisionRef, AnswerAction)>,
    /// Decisions that left the inbox, answered elsewhere or withdrawn, while this device watched:
    /// their view says so in a band (`spec.md` §5.6 "Liveness").
    pub left: HashSet<DecisionRef>,
    /// Earlier requests asked for this launch.
    pub asked: HashSet<DecisionRef>,
    /// Ask the page to scroll its cursor into view on the next frame.
    pub reveal: bool,
    /// The clock the facts line counts ages from: the fixture's `now` in scenes, else the system's.
    pub frozen_now: Option<Wall>,
    /// What the note field edits when no note is being typed.
    scratch: rune::TextEdit,
}

impl State {
    pub fn page(&mut self, d: &DecisionRef) -> &mut Page {
        self.pages.entry(d.clone()).or_default()
    }

    pub fn page_of(&self, d: &DecisionRef) -> Option<&Page> {
        self.pages.get(d)
    }

    /// The note field's text, for its `on_edit`.
    pub fn edit_mut(&mut self) -> &mut rune::TextEdit {
        match self.editing.as_mut() {
            Some(e) => &mut e.editor.edit,
            None => &mut self.scratch,
        }
    }
}
