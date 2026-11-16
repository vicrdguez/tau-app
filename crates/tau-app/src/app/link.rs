//! The link to the server in the app: rune's waker and window focus go to it, `poll` pumps it,
//! hands its events to the screens, and refreshes `App::server` from it. While the link isn't
//! paired the stage shows Connect. The stage's conversation and decision follow what the stage
//! shows.

use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::Duration;

use rune::Tx;
use tau_client::clock::{Clock, SystemClock};
use tau_client::{DecisionRef, Fixture, Label, Link, LinkConfig, LinkEvent, LinkPhase, Slot};

use super::server::{conv_ref, decision_ref};
use super::{App, Stage};
use crate::copy::{self, AnswerAction};
use crate::screens;

static FIXTURE: LazyLock<Fixture> = LazyLock::new(Fixture::design);

/// `--demo` and headless renders: the design's fixture as the server.
pub fn demo() -> Link {
    Link::fixture(FIXTURE.clone())
}

/// The window's link: the server in `~/.config/tau/client.toml`, or none yet.
pub fn standard() -> Link {
    Link::open(LinkConfig::standard())
}

/// `AppSpec::setup`: worker threads wake the window when something waits for `pump`.
pub fn setup(app: &mut App, waker: rune::Waker) {
    app.link.set_waker(move || waker());
}

/// The window gained or lost focus: the link polls only while focused (`client.md` §5).
pub fn on_focus(app: &mut App, focused: bool, _tx: &mut Tx) {
    app.link.set_focused(focused);
    for id in screens::ALL {
        screens::get(id).on_focus(app, focused);
    }
    if !focused {
        super::local::save(app);
    }
}

pub fn on_close(app: &mut App, tx: &mut Tx) {
    on_focus(app, false, tx);
}

/// Applies what the link received, hands its events to the screens and refreshes what the frame
/// shows. True when anything did change. A frozen app (a scene) leaves the link alone.
pub fn pump(app: &mut App, tx: &mut Tx) -> bool {
    if !app.live {
        return false;
    }
    let mut changed = app.link.pump();
    let mut said = Vec::new();
    for event in app.link.take_events() {
        changed = true;
        // One stream end per stream says one thing: echo it once.
        if let Some(words) = echo(app, &event)
            && !said.contains(&words)
        {
            app.say(words.clone());
            said.push(words);
        }
        for id in screens::ALL {
            screens::get(id).on_event(app, &event);
        }
    }
    let before = app.server.clone();
    refresh(app);
    changed |= app.server != before;
    // Words that count on the local clock: `offline 6 m`, `reconnecting in 4 s`.
    if matches!(
        app.link.status().phase,
        LinkPhase::Offline { .. } | LinkPhase::Reconnecting { .. }
    ) {
        tx.wake_after(Duration::from_secs(1));
    }
    changed
}

/// The event's echo, in the copy's words. An answer's echo is the plain answer's; the decision
/// screen says its own after this one when it sent again after a fault.
fn echo(app: &App, event: &LinkEvent) -> Option<String> {
    let body = match event {
        LinkEvent::Write { id, .. } => app.link.outbox().get(*id).map(|e| e.body.clone()),
        _ => None,
    }
    .unwrap_or_default();
    let device = app.link.server().device.as_ref().map(ToString::to_string);
    copy::event_echo(event, &body, device.as_deref(), AnswerAction::Answer)
}

/// Refreshes `App::server` from the link and asks for what the open items lack; Connect while
/// the link isn't paired.
pub fn refresh(app: &mut App) {
    if matches!(
        app.link.status().phase,
        LinkPhase::NotPaired | LinkPhase::Pairing
    ) && app.stage != Stage::Connect
    {
        app.show(Stage::Connect);
    }
    let now = SystemClock.now().wall;
    app.server.update(
        &app.link,
        &app.open,
        &app.hidden,
        now,
        app.screens.decision.notes.count(),
    );
    if app.link.status().phase == LinkPhase::Live {
        ask_for_labels(app);
    }
}

/// An open conversation's label from its first input; an open decision's title from `decision`
/// when the inbox doesn't list it, once per launch.
fn ask_for_labels(app: &mut App) {
    let items: Vec<_> = app.open.iter().cloned().collect();
    for item in items {
        if let Some(conv) = conv_ref(&item)
            && app.link.server().label(&conv) == Label::NotYet
        {
            app.link.want_label(conv);
        } else if let Some(d) = decision_ref(&item)
            && app.server.label(&item).is_none()
            && !app.link.server().decisions.contains_key(&d)
            && app.asked.insert(d.clone())
        {
            app.link.fetch_decision(d);
        }
    }
}

/// The link holds the `open` stream of the conversation the stage shows, and watches the decision
/// it shows.
pub fn follow_stage(app: &mut App) {
    let item = match &app.stage {
        Stage::Item(item) => Some(item),
        _ => None,
    };
    app.link.show(Slot::Stage, item.and_then(conv_ref));
    app.link.watch_decision(item.and_then(decision_ref));
}

/// Pumps until nothing changes and refreshes `App::server`, as frames would: for scenes and tests.
pub fn settle(app: &mut App) {
    app.link.set_focused(true);
    for _ in 0..30 {
        if !app.link.pump() {
            break;
        }
        refresh(app);
    }
    let _ = app.link.take_events();
    refresh(app);
}

/// Decisions asked for once this launch.
pub type Asked = HashSet<DecisionRef>;
