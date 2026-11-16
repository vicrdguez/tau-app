//! The conversation view's own state, per conversation, and what the drawn frame says of the one
//! the stage shows.

use std::collections::HashMap;

use rune::TextEdit;
use tau_client::clock::{Clock, SystemClock};
use tau_client::model::Transcript as Held;
use tau_client::shapes::{ConversationState, Docs};
use tau_client::streams::{Phase, StreamKey};
use tau_client::{ConvRef, ConversationId, EntryId, TaskId, Wall};

use super::transcript::{Folds, Item, Stop, build, stops};
use crate::app::{App, Stage, conv_ref};
use crate::editor::{Editor, Lines};

#[derive(Default)]
pub struct State {
    convs: HashMap<ConvRef, Conv>,
    /// The wall clock, pinned by scenes and tests to the fixture's `now`.
    pub clock: Option<Wall>,
    /// Where the transcript scrolls on the next frame.
    pub reveal: Option<Reveal>,
    /// The running count last drawn, so `poll` redraws only when it reads differently.
    pub ticked: Option<String>,
}

/// A scroll the next frame makes: keys have no `Tx`, so `poll` asks for it.
#[derive(Clone, Debug, PartialEq)]
pub enum Reveal {
    /// The stop at the transcript's top: a scene's starting view.
    Top(Stop),
    /// Just far enough to show the stop.
    Into(Stop),
    /// The end, following again.
    Bottom,
}

/// One conversation's view on this device.
pub struct Conv {
    /// The composer's draft, kept when the view closes (`spec.md` §3.3).
    pub composer: Editor,
    /// The composer has the keys.
    pub composing: bool,
    /// Where the cursor rests; `None` follows the last stop (`spec.md` §5.5 "Follow").
    pub cursor: Option<Stop>,
    pub folds: Folds,
    /// The run `␣cx` was sent for: stop stays off until the view shows it gone or a new one.
    pub stop_sent: Option<TaskId>,
    /// The newest entry when follow paused, for `12 new ↓`.
    pub paused_at: Option<EntryId>,
}

impl Default for Conv {
    fn default() -> Self {
        Conv {
            composer: Editor::new("", Lines::Multi),
            composing: false,
            cursor: None,
            folds: Folds::new(),
            stop_sent: None,
            paused_at: None,
        }
    }
}

impl State {
    pub(crate) fn saved(&self) -> Vec<(ConvRef, String, Option<Stop>)> {
        self.convs
            .iter()
            .map(|(id, c)| (id.clone(), c.composer.text().into(), c.cursor.clone()))
            .collect()
    }

    pub fn conv(&self, c: &ConvRef) -> Option<&Conv> {
        self.convs.get(c)
    }

    pub fn conv_mut(&mut self, c: &ConvRef) -> &mut Conv {
        self.convs.entry(c.clone()).or_default()
    }

    pub fn now(&self) -> Wall {
        self.clock.unwrap_or_else(|| SystemClock.now().wall)
    }
}

/// The conversation the stage shows, as the drawn frame has it.
pub struct Shown<'a> {
    pub conv: ConvRef,
    pub held: Option<&'a Held>,
    pub docs: Option<&'a Docs>,
    pub state: ConversationState,
    /// Its `open` stream is live, so `pi.live` is current (`spec.md` §4.9).
    pub up: bool,
    /// When its documents last came.
    pub at: Option<Wall>,
    pub owner: Option<ConversationId>,
}

pub fn shown(app: &App) -> Option<Shown<'_>> {
    let Stage::Item(item) = &app.stage else {
        return None;
    };
    let conv = conv_ref(item)?;
    let server = app.link.server();
    let held = server.transcripts.get(&conv);
    let docs = held.and_then(|h| h.docs.value());
    let up =
        app.link.status().streams.iter().any(|(k, s)| {
            *k == StreamKey::Open(conv.clone()) && matches!(s.phase, Phase::Live { .. })
        });
    let owner = server
        .conversations
        .get(&conv.project)
        .and_then(|k| k.value())
        .and_then(|rows| rows.iter().find(|r| r.conversation == conv.id))
        .and_then(|r| r.owner.as_ref())
        .map(|o| o.conversation);
    Some(Shown {
        state: docs.map_or(ConversationState::Idle, |d| d.live.state()),
        at: held.and_then(|h| h.docs.at()),
        held,
        docs,
        up,
        owner,
        conv,
    })
}

/// The transcript's items under this device's folds.
pub fn items(app: &App, s: &Shown) -> Vec<Item> {
    let folds = app.screens.conversation.conv(&s.conv).map(|c| &c.folds);
    match (s.held, folds) {
        (Some(h), Some(f)) => build(h, f),
        (Some(h), None) => build(h, &Folds::new()),
        (None, _) => Vec::new(),
    }
}

/// The stop the cursor is on: where it was left, else (follow, or its row gone) the last.
pub fn current_stop(app: &App, s: &Shown) -> Option<Stop> {
    let all = stops(&items(app, s));
    let left = app
        .screens
        .conversation
        .conv(&s.conv)
        .and_then(|c| c.cursor.clone());
    left.filter(|c| all.contains(c))
        .or_else(|| all.last().cloned())
}

/// Follow is on: the cursor tracks the last stop.
pub fn following(app: &App) -> bool {
    shown(app).is_none_or(|s| {
        app.screens
            .conversation
            .conv(&s.conv)
            .is_none_or(|c| c.cursor.is_none())
    })
}

/// The shown conversation's composer, for its field's `on_edit`.
pub fn composer_edit(app: &mut App) -> &mut TextEdit {
    let conv = match &app.stage {
        Stage::Item(item) => conv_ref(item),
        _ => None,
    };
    match conv {
        Some(c) => &mut app.screens.conversation.conv_mut(&c).composer.edit,
        None => {
            &mut app
                .screens
                .conversation
                .conv_mut(&ConvRef::new("", 0))
                .composer
                .edit
        }
    }
}

#[cfg(test)]
pub fn composer_text(app: &App) -> String {
    shown(app)
        .and_then(|s| app.screens.conversation.conv(&s.conv))
        .map(|c| c.composer.text().to_string())
        .unwrap_or_default()
}
