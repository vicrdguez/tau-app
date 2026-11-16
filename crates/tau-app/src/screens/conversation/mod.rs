//! A conversation (`spec.md` §5.5): what this agent is doing, and how I steer it.
//!
//! The stage shows one conversation through its `open` stream (`App::show` points the link's
//! `Slot::Stage` at it): the title block and its facts line, the transcript, then the queue, the
//! unsent rows from the outbox and the composer at the column's foot. The header's segments are
//! `␣cx stop` while the drawn frame shows a run and `␣cc continue` after a failed or stopped one;
//! each reads `· off` while the view's stream is down, and stop also from the press until the
//! view shows that run gone or a new one (FC4). The details panel (`␣td`) holds the settings, the
//! decisions and the events.
//!
//! - `transcript` turns the held entries and `pi.live` into items and cursor stops; it is pure.
//! - `state` keeps each conversation's composer, cursor, folds and sent stop, and reads what the
//!   drawn frame says of the shown one.
//! - `view` and `details` draw; `floats` holds the model and thinking pickers and a record's data.
//!
//! The cursor rests on a header line, a tool row, a fold or one top-level Markdown block of a
//! message body (a paragraph, a list item, a code block): the design's line cursor inside text
//! stands on the block that holds the line. `pi.reset` and `pi.compaction` are quiet chrome lines,
//! `context reset` and `earlier context summarised`, whose text `za` or Enter opens. A press on a
//! tool row, tool group or thinking line toggles it and places the cursor there. Enter and `za`
//! toggle the fold under the reader cursor, never while composing. An opened tool row shows its
//! full input above the output when its summary omits fields or lines.

mod details;
mod floats;
mod nav;
mod scenes;
mod state;
pub(crate) mod transcript;
mod view;
mod yank;

use std::time::Duration;

use rune::{El, Tx};
use serde_json::Value;
use tau_client::model::Older;
use tau_client::shapes::{ConversationState, EntryBody};
use tau_client::wire::InputMode;
use tau_client::{
    EntryId, LinkEvent, NotSent, TaskId, TranscriptKey, Write, WriteCall, WriteState,
};

use self::floats::{Pick, Picker, RecordData, picker_keys};
use self::state::{Reveal, current_stop, following, shown};
use self::transcript::{Item, Outcome, Part, Stop, foldables, stops};
use super::Screen;
use crate::app::scenes::Scene;
use crate::app::{App, Handler, Invocation, Stage};
use crate::editor::{EditMode, Reply};
use crate::floats::Float;
use crate::frame::{Crumb, Segment, SegmentKind};
use crate::keys::{Code, Ctx, Key, Mode};
use crate::open::Item as OpenItem;
use crate::theme::Theme;
use crate::time;

pub use state::State;

#[cfg(test)]
use self::floats::{picker_cursor, picker_rows, record_float_text};
#[cfg(test)]
use self::state::{composer_edit, composer_text, items};

pub struct Conversation;

impl Screen for Conversation {
    fn ctx(&self, app: &App) -> Ctx {
        if composing(app) {
            Ctx::Composer
        } else {
            Ctx::Conversation
        }
    }

    fn mode(&self, app: &App) -> Mode {
        let Some(c) = shown(app).and_then(|s| app.screens.conversation.conv(&s.conv)) else {
            return Mode::Normal;
        };
        if !c.composing {
            return Mode::Normal;
        }
        match c.composer.mode() {
            EditMode::Insert => Mode::Insert,
            EditMode::Normal => Mode::Normal,
            EditMode::Visual | EditMode::VisualLine => Mode::Visual,
        }
    }

    fn crumbs(&self, app: &App) -> Vec<Crumb> {
        match &app.stage {
            Stage::Item(OpenItem::Conversation { project, id }) => {
                vec![Crumb::Project(project.clone()), Crumb::Id(format!("#{id}"))]
            }
            _ => Vec::new(),
        }
    }

    fn segments(&self, app: &App) -> Vec<Segment> {
        let Some(s) = shown(app) else {
            return Vec::new();
        };
        let Some(docs) = s.docs else {
            return Vec::new();
        };
        let sent = app
            .screens
            .conversation
            .conv(&s.conv)
            .and_then(|c| c.stop_sent);
        header(&s.state, docs.live.run.as_ref().map(|r| r.task), s.up, sent)
    }

    fn has_details(&self, _: &App) -> bool {
        true
    }

    fn details(&self, t: Theme, app: &App) -> El {
        details::view(t, app)
    }

    fn view(&self, t: Theme, app: &App) -> El {
        view::page(t, app)
    }

    fn commands(&self) -> Vec<(&'static str, Handler)> {
        vec![
            ("composer.focus", |a, _, _| compose(a)),
            ("composer.send", |a, _, _| {
                let mode = match shown(a).map(|s| s.state) {
                    Some(ConversationState::Running) => InputMode::Steer,
                    _ => InputMode::Send,
                };
                send(a, mode);
            }),
            ("composer.followup", |a, _, _| send(a, InputMode::FollowUp)),
            ("editor.newline", |a, _, _| {
                if let Some(c) = shown_conv(a) {
                    a.screens.conversation.conv_mut(&c).composer.newline();
                }
            }),
            ("float.close", |a, _, _| {
                if let Some(c) = shown_conv(a) {
                    a.screens.conversation.conv_mut(&c).composing = false;
                }
            }),
            ("conv.stop", stop),
            ("conv.continue", continue_run),
            ("conv.model", |a, _, _| Picker::open(a, Pick::Model)),
            ("conv.thinking", |a, _, _| Picker::open(a, Pick::Thinking)),
            ("message.next", |a, inv, _| motion(a, inv, Stop::is_head)),
            ("toolcall.next", |a, inv, _| motion(a, inv, Stop::is_tool)),
            ("fold.toggle", fold),
            ("list.open", |a, inv, _| {
                if inv.applies
                    && !composing(a)
                    && let Some(stop) = shown(a).and_then(|s| current_stop(a, &s))
                {
                    toggle(a, stop);
                }
            }),
            ("toggle.follow", |a, _, _| {
                if following(a) {
                    pause(a);
                } else {
                    resume(a);
                }
            }),
            ("goto.linked", goto_owner),
            ("link.external", |a, _, _| {
                let url = yank::link(a);
                crate::floats::open_url::confirm(a, url);
            }),
            ("peek", peek),
            ("peek.keys", |a, _, _| {
                a.close_float();
            }),
            ("picker.keys", picker_keys),
            ("recover.retry", retry),
        ]
    }

    fn anchor(&self, app: &App) -> Option<super::Anchor> {
        nav::anchor(app)
    }

    fn land(&self, app: &mut App, anchor: &super::Anchor) {
        nav::land(app, anchor);
    }

    fn targets(&self, app: &App) -> Vec<super::HintTarget> {
        nav::targets(app)
    }

    fn yank(&self, app: &mut App, obj: crate::yank::Object) -> Option<String> {
        yank::text(app, obj)
    }

    fn keys(&self, app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
        if let Some((Float::Screen(_), _)) = app.floats.last() {
            if let Some(p) = crate::screens::top_float::<Picker>(app) {
                p.cursor = 0;
                return false;
            }
            return true;
        }
        let Some(conv) = shown_conv(app) else {
            return true;
        };
        let now = app.now;
        let c = app.screens.conversation.conv_mut(&conv);
        if c.composing {
            let ed = &mut c.composer;
            if ed.mode() == EditMode::Insert {
                return keys.iter().all(|k| ed.insert(*k, now) == Reply::Handled);
            }
            for k in keys {
                ed.normal(count, *k);
            }
            return true;
        }
        reader_motion(app, keys, count.unwrap_or(1));
        true
    }

    fn reveal_target(&self, app: &App, tx: &mut Tx) {
        if following(app) {
            tx.scroll_to_bottom(view::TRANSCRIPT);
        } else if let Some(s) = shown(app)
            && let Some(stop) = current_stop(app, &s)
        {
            tx.scroll_into_view(stop.key());
        }
    }

    fn shown(&self, app: &mut App) {
        if let Some(c) = shown_conv(app) {
            app.screens.conversation.conv_mut(&c).composing = false;
        }
    }

    fn on_event(&self, app: &mut App, event: &LinkEvent) {
        // A refused input comes back to its composer as a draft (`spec.md` §5.2).
        if let LinkEvent::Write {
            id,
            call: WriteCall::Input,
            state: WriteState::Refused(_),
        } = event
            && let Some(entry) = app.link.outbox().get(*id)
            && let (Some(conv), Some(text)) = (entry.conversation(), entry.text())
        {
            let text = text.to_string();
            let ed = &mut app.screens.conversation.conv_mut(&conv).composer;
            if ed.text().trim().is_empty() {
                ed.edit.replace_text(&text);
            }
        }
    }

    fn scenes(&self) -> Vec<Scene> {
        scenes::all()
    }

    fn poll(&self, app: &mut App, now: f64, tx: &mut Tx) -> bool {
        let Some(conv) = shown_conv(app) else {
            return false;
        };
        let mut changed = false;
        if let Some(reveal) = app.screens.conversation.reveal.take() {
            match reveal {
                Reveal::Top(stop) => tx.scroll_key_to(stop.key(), 0.0),
                Reveal::Into(stop) => tx.scroll_into_view(stop.key()),
                Reveal::Bottom => tx.scroll_to_bottom(view::TRANSCRIPT),
            }
            changed = true;
        }
        let ed = &mut app.screens.conversation.conv_mut(&conv).composer;
        if let Some(due) = ed.jk_deadline() {
            tx.wake_after(Duration::from_secs_f64((due - now).max(0.0)));
        }
        changed |= ed.tick(now);
        let count = running_count(app);
        if let Some((text, step)) = &count {
            tx.wake_after(*step);
            changed |= app.screens.conversation.ticked.as_ref() != Some(text);
        }
        app.screens.conversation.ticked = count.map(|(t, _)| t);
        changed
    }
}

/// The header's segments from the drawn state: running `␣cx stop`, failed or stopped `␣cc
/// continue`, idle none. Each reads `· off` while the view's stream is down, and stop also once
/// sent for the drawn run (`spec.md` §4.2, §4.9; FC4).
pub fn header(
    state: &ConversationState,
    run: Option<TaskId>,
    up: bool,
    stop_sent: Option<TaskId>,
) -> Vec<Segment> {
    let (seq, word, on) = match state {
        ConversationState::Running => ("<leader>cx", "stop", up && stop_sent != run),
        ConversationState::Failed { .. } | ConversationState::Stopped => {
            ("<leader>cc", "continue", up)
        }
        ConversationState::Idle => return Vec::new(),
    };
    vec![Segment {
        seq,
        word: word.into(),
        kind: if on {
            SegmentKind::Quiet
        } else {
            SegmentKind::Off
        },
    }]
}

/// The composer's caption: what `⌘⏎` does in the drawn state, its mode word apart.
pub fn caption(state: &ConversationState, model: &str) -> (&'static str, String) {
    match state {
        ConversationState::Running => (
            "steer",
            "joins this run after the current tool round".to_string(),
        ),
        ConversationState::Failed { .. } => {
            ("send", "starts a new run · the last run failed".to_string())
        }
        ConversationState::Stopped | ConversationState::Idle => {
            ("send", format!("starts a run on {model}"))
        }
    }
}

/// Why a write the view asked for was not taken, in the copy's words.
fn not_sent(e: &NotSent) -> String {
    match e {
        NotSent::TooLarge => "Over 1 MiB · nothing was sent",
        NotSent::Offline => "offline · nothing was sent",
        NotSent::Refused => "the server refused this device · writes wait until it accepts one",
        NotSent::Revoked => "this device was revoked · nothing was sent",
        NotSent::NotPaired => "not connected · nothing was sent",
        NotSent::OtherProtocol => "the server speaks another protocol · nothing was sent",
        NotSent::NoRetry | NotSent::NoSuchEntry => "nothing to send again",
        NotSent::Empty | NotSent::Starting | NotSent::AnswerUnsettled => "nothing was sent",
    }
    .to_string()
}

fn shown_conv(app: &App) -> Option<tau_client::ConvRef> {
    match &app.stage {
        Stage::Item(item) => crate::app::conv_ref(item),
        _ => None,
    }
}

fn composing(app: &App) -> bool {
    shown_conv(app)
        .and_then(|c| app.screens.conversation.conv(&c))
        .is_some_and(|c| c.composing)
}

/// `i a o`: the composer takes the keys, in Insert.
fn compose(app: &mut App) {
    if let Some(c) = shown_conv(app) {
        let conv = app.screens.conversation.conv_mut(&c);
        conv.composing = true;
        conv.composer.edit.focused = true;
        conv.composer.enter_insert();
    }
}

/// `⌘⏎` and `M-⏎`: the composer's text as an `input` in `mode`; empty or white-space text sends
/// nothing. The outbox keeps it until the server answers (`client.md` §2).
fn send(app: &mut App, mode: InputMode) {
    let Some(conv) = shown_conv(app) else { return };
    let text = app
        .screens
        .conversation
        .conv_mut(&conv)
        .composer
        .text()
        .to_string();
    if text.trim().is_empty() {
        return;
    }
    match app.link.write(Write::Input {
        conv: conv.clone(),
        text,
        mode,
    }) {
        Ok(_) => {
            let ed = &mut app.screens.conversation.conv_mut(&conv).composer;
            ed.edit.replace_text("");
        }
        Err(NotSent::Empty) => {}
        Err(e) => app.say(not_sent(&e)),
    }
}

/// `␣cx`: stops the drawn run, once.
fn stop(app: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(s) = shown(app) else { return };
    let run = s.docs.and_then(|d| d.live.run.as_ref()).map(|r| r.task);
    let sent = app
        .screens
        .conversation
        .conv(&s.conv)
        .and_then(|c| c.stop_sent);
    let on = header(&s.state, run, s.up, sent)
        .first()
        .is_some_and(|seg| seg.seq == "<leader>cx" && seg.kind == SegmentKind::Quiet);
    if !inv.applies || !on {
        return;
    }
    let conv = s.conv.clone();
    match app.link.write(Write::Stop { conv: conv.clone() }) {
        Ok(_) => app.screens.conversation.conv_mut(&conv).stop_sent = run,
        Err(e) => app.say(not_sent(&e)),
    }
}

/// `␣cc`: runs a failed or stopped run again on the same context.
fn continue_run(app: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(s) = shown(app) else { return };
    let on = header(&s.state, None, s.up, None)
        .first()
        .is_some_and(|seg| seg.seq == "<leader>cc" && seg.kind == SegmentKind::Quiet);
    if !inv.applies || !on {
        return;
    }
    let conv = s.conv.clone();
    if let Err(e) = app.link.write(Write::Continue { conv }) {
        app.say(not_sent(&e));
    }
}

/// `␣rt` in the view: the first of its unsent inputs that can be sent again, same body.
fn retry(app: &mut App, _: &Invocation, _: &mut Tx) {
    let Some(conv) = shown_conv(app) else { return };
    let id = app
        .link
        .outbox()
        .unsent_inputs(&conv)
        .into_iter()
        .find(|e| e.can_retry())
        .map(|e| e.id);
    match id {
        Some(id) => {
            if let Err(e) = app.link.retry(id) {
                app.say(not_sent(&e));
            }
        }
        None => app.say("nothing to send again"),
    }
}

/// The shown conversation's stops and the cursor's place among them, owned, so a key can then
/// change the view's state.
struct Walk {
    conv: tau_client::ConvRef,
    stops: Vec<Stop>,
    at: usize,
    newest: Option<EntryId>,
    /// Older entries may wait behind the oldest held one.
    older: bool,
}

fn walk(app: &App) -> Option<Walk> {
    let s = shown(app)?;
    let stops = stops(&state::items(app, &s));
    let at = current_stop(app, &s).and_then(|c| stops.iter().position(|x| *x == c))?;
    Some(Walk {
        at,
        newest: s.held.and_then(|h| h.newest()),
        older: s
            .held
            .is_some_and(|h| matches!(h.older, Older::Unknown | Older::More { .. })),
        conv: s.conv,
        stops,
    })
}

/// The cursor onto stop `to`: the last stop follows again, any other pauses follow.
fn set_cursor(app: &mut App, w: &Walk, to: usize) {
    if to + 1 == w.stops.len() {
        resume(app);
        return;
    }
    let c = app.screens.conversation.conv_mut(&w.conv);
    if c.cursor.is_none() {
        c.paused_at = w.newest;
    }
    c.cursor = Some(w.stops[to].clone());
    app.screens.conversation.reveal = Some(Reveal::Into(w.stops[to].clone()));
}

/// Follow pauses where the cursor is.
fn pause(app: &mut App) {
    let Some(w) = walk(app) else { return };
    let c = app.screens.conversation.conv_mut(&w.conv);
    c.cursor = Some(w.stops[w.at].clone());
    c.paused_at = w.newest;
}

/// `G`, `␣tf`: the cursor on the last stop, following new ones.
fn resume(app: &mut App) {
    if let Some(c) = shown_conv(app) {
        let conv = app.screens.conversation.conv_mut(&c);
        conv.cursor = None;
        conv.paused_at = None;
        app.screens.conversation.reveal = Some(Reveal::Bottom);
    }
}

/// `j k gg G C-d C-u` over the stops. Reaching the first stop asks for older entries, when the
/// server may hold some (`client.md` §3.3).
fn reader_motion(app: &mut App, keys: &[Key], count: u32) {
    let Some(w) = walk(app) else { return };
    let n = count as usize;
    let page = 10 * n;
    let last = w.stops.len() - 1;
    let to = match keys {
        [k] if *k == Key::char('j') || *k == Key::plain(Code::Down) => (w.at + n).min(last),
        [k] if *k == Key::char('k') || *k == Key::plain(Code::Up) => w.at.saturating_sub(n),
        [k] if *k == Key::ctrl('d') => (w.at + page).min(last),
        [k] if *k == Key::ctrl('u') => w.at.saturating_sub(page),
        [g, h] if *g == Key::char('g') && *h == Key::char('g') => 0,
        [k] if *k == Key::char('G') => last,
        _ => return,
    };
    if to == 0 && w.older {
        app.link
            .page_older(TranscriptKey::Conversation(w.conv.clone()));
    }
    set_cursor(app, &w, to);
}

/// `]m [m ]t [t`: the start of the next object, or of the one the cursor is in or the one before.
fn motion(app: &mut App, inv: &Invocation, is: fn(&Stop) -> bool) {
    let Some(w) = walk(app) else { return };
    let back = inv.keys.first() == Some(&Key::char('['));
    let mut to = w.at;
    for _ in 0..inv.count.unwrap_or(1) {
        let next = if back {
            (0..to).rev().find(|&i| is(&w.stops[i]))
        } else {
            (to + 1..w.stops.len()).find(|&i| is(&w.stops[i]))
        };
        match next {
            Some(i) => to = i,
            None => break,
        }
    }
    if to != w.at {
        set_cursor(app, &w, to);
    }
}

/// `za zo zc` on the row under the cursor; `zR` opens every row, `zM` closes them.
fn fold(app: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(s) = shown(app) else { return };
    let items = state::items(app, &s);
    let rows = foldables(&items);
    let key = inv.keys.get(1).copied();
    let conv = s.conv.clone();
    let cursor = current_stop(app, &s);
    let following = following(app);
    if key.is_none_or(|k| k == Key::char('a')) {
        if let Some(stop) = cursor {
            toggle(app, stop);
        }
        return;
    }
    let c = app.screens.conversation.conv_mut(&conv);
    match key {
        Some(k) if k == Key::char('R') || k == Key::char('M') => {
            let open = k == Key::char('R');
            for (stop, _) in rows {
                c.folds.insert(stop, open);
            }
        }
        _ => {
            let Some((stop, _)) = cursor.and_then(|at| rows.into_iter().find(|(f, _)| *f == at))
            else {
                return;
            };
            let to = match key {
                Some(k) if k == Key::char('o') => true,
                Some(k) if k == Key::char('c') => false,
                _ => return,
            };
            c.folds.insert(stop.clone(), to);
            // A fold that opens below the followed cursor keeps the cursor on it.
            if following {
                c.cursor = Some(stop);
            }
        }
    }
}

/// Toggles a fold and gives its stop the reader cursor, including when pressed from the composer.
fn toggle(app: &mut App, stop: Stop) {
    let Some(s) = shown(app) else { return };
    let Some((_, open)) = foldables(&state::items(app, &s))
        .into_iter()
        .find(|(f, _)| *f == stop)
    else {
        return;
    };
    let newest = s.held.and_then(|h| h.newest());
    let conv = s.conv.clone();
    let c = app.screens.conversation.conv_mut(&conv);
    if c.cursor.is_none() {
        c.paused_at = newest;
    }
    c.folds.insert(stop.clone(), !open);
    c.cursor = Some(stop.clone());
    c.composing = false;
    app.focus = crate::app::Focus::Stage;
    app.screens.conversation.reveal = Some(Reveal::Into(stop));
}

/// `gd` in an owned conversation: its owner.
fn goto_owner(app: &mut App, _: &Invocation, _: &mut Tx) {
    let Some(s) = shown(app) else { return };
    match s.owner {
        Some(owner) => {
            let project = s.conv.project.to_string();
            app.show(Stage::Item(OpenItem::Conversation {
                project,
                id: owner.0,
            }));
        }
        None => app.say("nothing linked here"),
    }
}

/// `K` on a record: its `data` as JSON, with the task that wrote it.
fn peek(app: &mut App, _: &Invocation, _: &mut Tx) {
    let Some(s) = shown(app) else { return };
    let Some(at) = current_stop(app, &s) else {
        return;
    };
    let entry = match &at {
        Stop::Head(e) | Stop::Text(e, _, _) => *e,
        _ => return,
    };
    let Some(e) = s.held.and_then(|h| h.entries.get(&entry)) else {
        return;
    };
    if !matches!(e.body, EntryBody::Record { .. }) {
        return;
    }
    let data = e.raw.get("data").cloned().unwrap_or(Value::Null);
    let mut subtitle = format!("entry {}", e.id);
    if let Some(task) = e.by_task {
        subtitle.push_str(&format!(" · by task {task}"));
    }
    app.open_float(Float::Screen(Box::new(RecordData {
        kind: e.kind.clone(),
        subtitle,
        json: serde_json::to_string_pretty(&data).unwrap_or_default(),
    })));
}

/// The first running row's count and how soon it next reads differently, while the view's
/// stream is up; a count stops while it is down (`spec.md` §4.9).
fn running_count(app: &App) -> Option<(String, Duration)> {
    let s = shown(app)?;
    if !s.up {
        return None;
    }
    let started = state::items(app, &s).iter().find_map(|i| match i {
        Item::Turn { parts, .. } => parts.iter().find_map(|p| match p {
            Part::Tool(r) if r.outcome == Outcome::Running => r.started,
            _ => None,
        }),
        _ => None,
    })?;
    let span = time::between(tau_client::Wall(started), app.screens.conversation.now());
    let step = if span < Duration::from_secs(10) {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(1)
    };
    Some((time::span(span), step))
}

#[cfg(test)]
mod tests;
