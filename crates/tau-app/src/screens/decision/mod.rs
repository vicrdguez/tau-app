//! A decision (`spec.md` §5.6) and its readback floats (§5.7): what am I asked, on exactly which
//! text, and what happens when I answer.
//!
//! - `doc`: the basis as blocks with their source ranges, the reader's motions over its visible
//!   text, and how a selection becomes a note's range (UTF-16 code units of `text`).
//! - `standing`: where the decision stands (open, answered, superseded, withdrawn, an answer with
//!   no reply), the `then` task's row of the table, and when `␣ad` and `␣ar` are on.
//! - `notes`: the drafts on this device per decision, with undo, saved at each change.
//! - `reader`: the cursor's keys, Visual, `c{motion}`, `␣mn`, `␣mw`, `]n [n`, `i`, `dd`, `u`.
//! - `view` and `margin`: the page and the notes margin, folded into the column under 1256 px.
//! - `readback`: `␣ad` and `␣ar`, the 400 ms arm, and every reply of §5.7's table.
//! - `scenes`: the mockups' states on the fixture link.
//!
//! The decision is read from the link's model (`decisions`, the inbox, the outbox) and written
//! only through `Link::write` and `Link::retry`; the link refetches it by §5.6's rules.

mod doc;
mod margin;
mod nav;
pub(crate) mod notes;
mod readback;
mod reader;
mod scenes;
mod standing;
pub(crate) mod state;
mod view;
mod yank;

#[cfg(test)]
mod doc_tests;
#[cfg(test)]
mod margin_tests;
#[cfg(test)]
mod standing_tests;
#[cfg(test)]
mod tests;

use rune::{El, Tx};
use tau_client::clock::{Clock, SystemClock};
use tau_client::{DecisionRef, LinkEvent, LinkPhase, Wall};

pub use state::State;

use super::Screen;
use crate::app::scenes::Scene;
use crate::app::{App, Handler, Invocation, Stage};
use crate::copy::{self, AnswerAction};
use crate::editor::EditMode;
use crate::frame::{Crumb, Room, Segment, SegmentKind};
use crate::keys::{Ctx, Key, Mode};
use crate::open::Item;
use crate::theme::Theme;
use crate::ui::Badge;
use reader::{record, shown};

/// The wall clock ages count from: the fixture's `now` in a scene.
fn now(app: &App) -> Wall {
    app.screens
        .decision
        .frozen_now
        .unwrap_or_else(|| SystemClock.now().wall)
}

pub struct Decision;

impl Screen for Decision {
    fn ctx(&self, app: &App) -> Ctx {
        if app.screens.decision.editing.is_some() {
            Ctx::Note
        } else {
            Ctx::Decision
        }
    }

    fn mode(&self, app: &App) -> Mode {
        let s = &app.screens.decision;
        if let Some(e) = &s.editing {
            return match e.editor.mode() {
                EditMode::Insert => Mode::Insert,
                EditMode::Normal => Mode::Normal,
                EditMode::Visual | EditMode::VisualLine => Mode::Visual,
            };
        }
        let visual = shown(app)
            .and_then(|d| s.page_of(&d))
            .is_some_and(|p| p.visual.is_some());
        if visual { Mode::Visual } else { Mode::Normal }
    }

    fn badge(&self, app: &App) -> Badge {
        let s = &app.screens.decision;
        if let Some(e) = &s.editing {
            return match e.editor.mode() {
                EditMode::Insert => Badge::Insert,
                EditMode::Normal => Badge::Normal,
                EditMode::Visual => Badge::Visual,
                EditMode::VisualLine => Badge::VisualLine,
            };
        }
        match shown(app)
            .and_then(|d| s.page_of(&d))
            .and_then(|p| p.visual)
        {
            Some((_, true)) => Badge::VisualLine,
            Some(_) => Badge::Visual,
            None => Badge::Normal,
        }
    }

    fn crumbs(&self, app: &App) -> Vec<Crumb> {
        match &app.stage {
            Stage::Item(Item::Decision { project, id }) => {
                vec![Crumb::Project(project.clone()), Crumb::Id(id.clone())]
            }
            _ => Vec::new(),
        }
    }

    fn segments(&self, app: &App) -> Vec<Segment> {
        segments(app)
    }

    fn has_details(&self, _: &App) -> bool {
        true
    }

    fn view(&self, t: Theme, app: &App) -> El {
        view::page(t, app)
    }

    fn details(&self, t: Theme, app: &App) -> El {
        view::details(t, app)
    }

    fn unavailable(&self, app: &App, id: &str) -> Option<String> {
        let d = shown(app)?;
        let off = match id {
            "auth.answer" => readback::answer_off(app, &d),
            "auth.again" => readback::again_off(app, &d),
            _ => None,
        };
        off.map(|o| o.reason().to_string())
    }

    fn commands(&self) -> Vec<(&'static str, Handler)> {
        vec![
            ("auth.answer", readback::answer),
            ("auth.again", readback::again),
            ("readback.confirm", readback::confirm),
            ("readback.cancel", readback::cancel),
            ("readback.pick", readback::pick),
            ("readback.details", readback::details),
            ("reply.keys", readback::cancel),
            ("note.annotate", |a, _, _| reader::operator(a)),
            ("note.here", |a, _, _| reader::here(a)),
            ("note.whole", |a, _, _| reader::whole(a)),
            ("note.next", |a, inv, _| {
                let forward = inv.keys.first() != Some(&Key::char('['));
                reader::next_note(a, forward, inv.count.unwrap_or(1));
            }),
            ("draft.edit", |a, _, _| reader::edit(a)),
            ("draft.discard", |a, _, _| reader::discard(a)),
            ("draft.undo", |a, _, _| reader::undo(a, true)),
            ("draft.redo", |a, _, _| reader::undo(a, false)),
            ("toggle.source", toggle_source),
            ("goto.linked", goto),
            ("link.external", |a, _, _| {
                let url = nav::link(a);
                crate::floats::open_url::confirm(a, url);
            }),
            ("list.open", |_, _, _| {}),
            ("fold.toggle", fold),
            ("toggle.details", |a, inv, tx| {
                if inv.applies {
                    a.details_visible = !a.details_visible;
                    // The margin folds or unfolds and the text reflows; the cursor's paragraph holds.
                    tx.keep_on_screen("decision-cursor");
                }
            }),
            ("float.close", close),
            ("editor.newline", |a, _, _| {
                if let Some(e) = a.screens.decision.editing.as_mut() {
                    e.editor.newline();
                }
            }),
        ]
    }

    fn globals(&self) -> Vec<(&'static str, Handler)> {
        // Away from a decision's view the answer commands only say where they run (`grammar.md`
        // §11 rule 2).
        vec![("auth.answer", elsewhere), ("auth.again", elsewhere)]
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

    fn object_key(&self, app: &mut App, key: Key) -> bool {
        app.scope().ctx == Ctx::Decision && reader::object_key(app, key)
    }

    fn keys(&self, app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
        if app.screens.decision.editing.is_some() {
            reader::editor_key(app, keys, count)
        } else {
            reader::key(app, keys, count)
        }
    }

    fn room(&self, _: &App) -> Room {
        Room::Lift(Some("decision-cursor".into()))
    }

    fn shown(&self, app: &mut App) {
        if shown(app).is_none() {
            return;
        }
        app.screens.decision.notes.load(app.link.store());
        fetch_earlier(app);
        app.screens.decision.reveal = true;
    }

    fn on_event(&self, app: &mut App, event: &LinkEvent) {
        match event {
            LinkEvent::Write { id, state, .. } => {
                let action = app.screens.decision.sent.get(id).map(|(_, a)| *a);
                if action == Some(AnswerAction::SendAgainAfterFault)
                    && matches!(state, tau_client::WriteState::Done { .. })
                {
                    let body = app.link.outbox().get(*id).map(|e| e.body.clone());
                    let device = app.link.server().device.as_ref().map(ToString::to_string);
                    if let Some(words) = copy::event_echo(
                        event,
                        &body.unwrap_or_default(),
                        device.as_deref(),
                        AnswerAction::SendAgainAfterFault,
                    ) {
                        app.say(words);
                    }
                }
                readback::on_write(app, *id, state);
            }
            LinkEvent::DecisionLeft {
                decision,
                ours: false,
            } => {
                app.screens.decision.left.insert(decision.clone());
            }
            _ => {}
        }
    }

    fn scenes(&self) -> Vec<Scene> {
        scenes::all()
    }

    fn poll(&self, app: &mut App, now: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        if let Some(e) = app.screens.decision.editing.as_mut() {
            if let Some(due) = e.editor.jk_deadline() {
                tx.wake_after(std::time::Duration::from_secs_f64((due - now).max(0.0)));
            }
            changed |= e.editor.tick(now);
        }
        changed |= readback::poll(app, now);
        fetch_earlier(app);
        if app.screens.decision.reveal && shown(app).is_some() {
            app.screens.decision.reveal = false;
            tx.scroll_into_view("decision-cursor");
        }
        changed
    }
}

/// The header's segments (`spec.md` §4.2): `◆ ␣ad Answer` while the decision is open, `· off`
/// while `␣ad` can't open the readback; `␣ar send again` while it is live.
fn segments(app: &App) -> Vec<Segment> {
    let Some(d) = shown(app) else {
        return Vec::new();
    };
    let Some(v) = record(app, &d) else {
        return Vec::new();
    };
    let live = app.link.server().inbox.value().map(|i| i.liveness(&d));
    let unsettled = app.link.outbox().answer_unsettled(&d);
    let up = app.link.status().phase == LinkPhase::Live;
    let st = standing::standing(v, live.as_ref(), unsettled);
    let mut out = Vec::new();
    if matches!(
        st,
        standing::Standing::Open { .. } | standing::Standing::Unknown { .. }
    ) {
        let on = up && standing::answer_on(v, live.as_ref(), unsettled);
        out.push(Segment {
            seq: "<leader>ad",
            word: "Answer".to_string(),
            kind: if on {
                SegmentKind::Answer
            } else {
                SegmentKind::Off
            },
        });
    }
    if standing::send_again_on(v, unsettled) {
        out.push(Segment {
            seq: "<leader>ar",
            word: "send again".to_string(),
            kind: if up {
                SegmentKind::Quiet
            } else {
                SegmentKind::Off
            },
        });
    }
    out
}

/// Asks once per launch, while the link is live, for the earlier requests the margin names.
fn fetch_earlier(app: &mut App) {
    if app.link.status().phase != LinkPhase::Live {
        return;
    }
    let Some(d) = shown(app) else { return };
    let Some(n) = d.id.request_number() else {
        return;
    };
    let subject = d.id.subject();
    for i in 1..n {
        let earlier = DecisionRef::new(d.project.clone(), format!("{subject}#{i}"));
        if !app.link.server().decisions.contains_key(&earlier)
            && app.screens.decision.asked.insert(earlier.clone())
        {
            app.link.fetch_decision(earlier);
        }
    }
}

fn elsewhere(app: &mut App, _: &Invocation, _: &mut Tx) {
    app.say("open it first · ⏎");
}

/// `␣ts`: the basis as plain source text, or back to Markdown; the cursor keeps its place.
fn toggle_source(app: &mut App, _: &Invocation, _: &mut Tx) {
    if let Some(d) = shown(app) {
        let p = app.screens.decision.page(&d);
        p.source = !p.source;
        p.visual = None;
        app.screens.decision.reveal = true;
    }
}

/// `gd`: the decision's conversation.
fn goto(app: &mut App, _: &Invocation, _: &mut Tx) {
    let Some(d) = shown(app) else { return };
    let Some(conv) = record(app, &d).map(|v| v.decision.conversation) else {
        return;
    };
    app.show(Stage::Item(Item::Conversation {
        project: d.project.to_string(),
        id: conv.0,
    }));
}

/// `za`: opens or folds every earlier request in the margin.
fn fold(app: &mut App, _: &Invocation, _: &mut Tx) {
    let Some(d) = shown(app) else { return };
    let Some(n) = d.id.request_number() else {
        return;
    };
    let subject = d.id.subject();
    let ids: Vec<tau_client::DecisionId> = (1..n)
        .map(|i| tau_client::DecisionId::from(format!("{subject}#{i}")))
        .collect();
    let p = app.screens.decision.page(&d);
    if p.earlier.is_empty() {
        p.earlier.extend(ids);
    } else {
        p.earlier.clear();
    }
}

/// `Esc` and `q`: the note editor's Normal mode back to the view, keeping its text; Visual and a
/// pending operator cancelled.
fn close(app: &mut App, _: &Invocation, _: &mut Tx) {
    if app.screens.decision.editing.is_some() {
        reader::finish(app);
        return;
    }
    if let Some(d) = shown(app) {
        let p = app.screens.decision.page(&d);
        p.visual = None;
        p.op = None;
    }
}
