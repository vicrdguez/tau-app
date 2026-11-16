//! The readback (`spec.md` §5.7, `grammar.md` §11): `␣ad` answers, `␣ar` sends an answer again,
//! and only a `y` sent at least 400 ms after the float was drawn, for the pick it last drew, sends.
//! A refusal turns the float into its reply; a server event can only turn `y` off.

mod keys;
mod view;

pub use keys::{again, answer, cancel, confirm, details, pick};
pub use view::thousands;

use rune::El;
use tau_client::model::Liveness;
use tau_client::wire::{DecisionRecord, DecisionStatus};
use tau_client::{DecisionRef, LinkPhase, WriteId};

use super::reader::record;
use super::standing::{answer_on, number, send_again_on};
use super::tokens_json;
use crate::app::{App, Stage};
use crate::copy::AnswerAction;
use crate::keys::{Ctx, Mode};
use crate::screens::{ScreenFloat, ScreenId, top_float};
use crate::theme::Theme;
use crate::time;
use crate::ui::{Badge, Scope};

/// What `y` sends.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    /// `␣ad`: the pick, the drafts and the basis.
    Answer,
    /// `␣ar` after the `then` task faulted: the record's own basis, choice and notes.
    AgainAfterFault(Box<DecisionRecord>),
    /// `␣ar` after a lost reply: the outbox entry's stored body.
    AgainUnknown(WriteId),
}

/// The reply the float turned into: a refusal's words, under `◇ NOT RECORDED`.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub meta: Option<String>,
    pub rows: Vec<(&'static str, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Reading,
    Sending(WriteId),
    Reply(Reply),
}

pub struct Readback {
    pub decision: DecisionRef,
    pub kind: Kind,
    pub pick: Option<usize>,
    /// The pick the float last drew, once drawn.
    pub drawn: Option<Option<usize>>,
    /// When it was first drawn, in the app's seconds.
    pub shown_at: Option<f64>,
    pub phase: Phase,
    /// `K`: the notes in full in the right column.
    pub notes_open: bool,
}

/// `tau.client_ms.readback_arm`, in seconds.
pub fn arm() -> f64 {
    tokens_json()["tau"]["client_ms"]["readback_arm"]
        .as_f64()
        .unwrap_or(400.0)
        / 1000.0
}

impl ScreenFloat for Readback {
    fn owner(&self) -> ScreenId {
        ScreenId::Decision
    }

    fn scope(&self) -> Scope {
        Scope {
            mode: Mode::Readback,
            ctx: match self.phase {
                Phase::Reply(_) => Ctx::Reply,
                _ => Ctx::Readback,
            },
        }
    }

    fn badge(&self) -> Badge {
        match self.phase {
            Phase::Reply(_) => Badge::ReadbackPlain,
            _ => Badge::Readback,
        }
    }

    fn view(&self, t: Theme, app: &App) -> El {
        view::view(t, app, self)
    }
}

/// Why `␣ad` or `␣ar` can't run now on a decision's view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Off {
    Offline,
    Revoked,
    Refused,
    NothingYet,
    Waiting,
    NotOpen,
    NotAnswered,
    DidNotFail,
}

impl Off {
    /// which-key's reason (`grammar.md` §5).
    pub fn reason(self) -> &'static str {
        match self {
            Off::Offline | Off::Revoked | Off::Refused => "offline",
            Off::NothingYet => "nothing received yet",
            Off::Waiting => "an answer is waiting for a reply",
            Off::NotOpen => "no longer open",
            Off::NotAnswered => "not answered",
            Off::DidNotFail => "its work did not fail",
        }
    }

    /// The echo when the key is pressed anyway (`client.md` §2.3 for offline).
    pub fn echo(self) -> &'static str {
        match self {
            Off::Offline => "offline · answer when connected",
            Off::Revoked => "revoked · nothing is sent",
            Off::Refused => "refused by the server · nothing is sent",
            other => other.reason(),
        }
    }
}

fn link_off(app: &App) -> Option<Off> {
    match app.link.status().phase {
        LinkPhase::Live => None,
        LinkPhase::Revoked => Some(Off::Revoked),
        LinkPhase::Refused { .. } => Some(Off::Refused),
        _ => Some(Off::Offline),
    }
}

fn liveness(app: &App, d: &DecisionRef) -> Option<Liveness> {
    app.link.server().inbox.value().map(|i| i.liveness(d))
}

/// Why `␣ad` would not open the readback for `d`.
pub fn answer_off(app: &App, d: &DecisionRef) -> Option<Off> {
    let off = link_off(app);
    let Some(view) = record(app, d) else {
        return Some(Off::NothingYet);
    };
    let unsettled = app.link.outbox().answer_unsettled(d);
    if unsettled.is_some() {
        return Some(Off::Waiting);
    }
    if !answer_on(view, liveness(app, d).as_ref(), unsettled) {
        return Some(Off::NotOpen);
    }
    off
}

/// Why `␣ar` would not open the send-again readback for `d`.
pub fn again_off(app: &App, d: &DecisionRef) -> Option<Off> {
    let Some(view) = record(app, d) else {
        return Some(Off::NothingYet);
    };
    if !send_again_on(view, app.link.outbox().answer_unsettled(d)) {
        // An open request reads as which-key's state 36 has it: its work did not fail.
        return Some(match view.decision.status {
            DecisionStatus::Superseded | DecisionStatus::Withdrawn => Off::NotAnswered,
            _ => Off::DidNotFail,
        });
    }
    link_off(app)
}

/// Every frame, before the view: notes when the float was first drawn and which pick it draws;
/// turns `y` off when the inbox says the request is no longer open. True redraws.
pub fn poll(app: &mut App, now: f64) -> bool {
    let live = top_float::<Readback>(app).map(|f| f.decision.clone());
    let Some(d) = live else { return false };
    let liveness = liveness(app, &d);
    let Some(f) = top_float::<Readback>(app) else {
        return false;
    };
    let mut changed = false;
    if f.phase == Phase::Reading && f.kind == Kind::Answer && liveness != Some(Liveness::Open) {
        f.phase = Phase::Reply(not_open(match &liveness {
            Some(Liveness::Superseded { by }) => Some(number(by.as_str())),
            _ => None,
        }));
        changed = true;
    }
    if f.shown_at.is_none() {
        f.shown_at = Some(now);
        changed = true;
    }
    if f.drawn != Some(f.pick) {
        f.drawn = Some(f.pick);
        changed = true;
    }
    changed
}

fn not_open(open: Option<String>) -> Reply {
    let mut words = "Not recorded. This request is no longer open.".to_string();
    if let Some(n) = open {
        words.push_str(&format!(" {n} is open now."));
    }
    Reply {
        meta: None,
        rows: vec![("reply", words)],
    }
}

/// The float's reply to a refusal of the answer it sent (`spec.md` §5.7, the table).
pub fn refused(
    app: &mut App,
    id: WriteId,
    refusal: &tau_client::wire::Refusal,
    action: AnswerAction,
) {
    let body = app
        .link
        .outbox()
        .get(id)
        .map(|e| e.body.clone())
        .unwrap_or_default();
    let words = crate::copy::refusal(tau_client::wire::CallName::Answer, &body, refusal, action);
    let d = app.screens.decision.sent.get(&id).map(|(d, _)| d.clone());
    let ours = d.as_ref().map(|d| yours(app, d, &body)).unwrap_or_default();
    let mut rows = vec![("reply", words.message.unwrap_or_default())];
    let mut meta = None;
    if let Some(r) = refusal.decision() {
        let notes = r.annotations.len();
        if let Some(choice) = &r.choice {
            let n = r
                .choices
                .iter()
                .position(|c| c == choice)
                .map_or(0, |i| i + 1);
            rows.push(("their choice", format!("{n} {choice}")));
        }
        rows.push(("their notes", plural(notes, "note", "notes")));
        meta = Some(answered_meta(&r));
    } else if let Some(open) = words.open {
        rows[0].1.push_str(&format!(" {open} is open now."));
    }
    if action == AnswerAction::Answer {
        rows.push(("yours", ours));
    }
    if let Some(f) = top_float::<Readback>(app).filter(|f| f.phase == Phase::Sending(id)) {
        f.phase = Phase::Reply(Reply { meta, rows });
    }
}

fn yours(app: &App, d: &DecisionRef, body: &serde_json::Value) -> String {
    let choice = body["choice"].as_str().unwrap_or("none");
    let n = app.screens.decision.notes.of(d).len();
    if n == 0 {
        format!("your choice {choice} was not sent")
    } else {
        format!(
            "your choice {choice} and your {} were not sent; they stay on this device",
            plural(n, "note", "notes")
        )
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn answered_meta(r: &DecisionRecord) -> String {
    let mut s = "answered".to_string();
    if let Some(by) = &r.by {
        s.push_str(&format!(" · {by}"));
    }
    if let Some(at) = r.at {
        s.push_str(&format!(" {}", time::clock(tau_client::Wall(at))));
    }
    s
}

/// What the readback's reply turns into once the link reports the write it sent: the float
/// closes on a result or an unknown outcome, and becomes the reply on a refusal.
pub fn on_write(app: &mut App, id: WriteId, state: &tau_client::WriteState) {
    use tau_client::WriteState;
    // A lost reply proved landed clears the drafts, whichever launch sent it (`client.md` §2.3).
    if let WriteState::Settled(tau_client::outbox::Settlement::Landed(_)) = state
        && let Some(d) = app.link.outbox().get(id).and_then(|e| e.decision())
    {
        let store = app.link.store();
        app.screens.decision.notes.clear(&d, store);
    }
    let Some((d, action)) = app.screens.decision.sent.get(&id).cloned() else {
        return;
    };
    let ours = top_float::<Readback>(app).is_some_and(|f| f.phase == Phase::Sending(id));
    match state {
        WriteState::Done { .. } => {
            if ours {
                app.close_float();
            }
            app.screens.decision.sent.remove(&id);
            let store = app.link.store();
            app.screens.decision.notes.clear(&d, store);
            if action == AnswerAction::SendAgainAfterFault {
                hide_faults(app, &d);
            }
        }
        WriteState::Refused(r) => {
            if ours {
                refused(app, id, r, action);
            }
            app.screens.decision.sent.remove(&id);
        }
        WriteState::Unknown { .. } => {
            if ours {
                app.close_float();
            }
        }
        WriteState::Settled(_) => {
            app.screens.decision.sent.remove(&id);
        }
        WriteState::Waiting { .. } | WriteState::Sending => {}
    }
    if app.stage
        == Stage::Item(crate::open::Item::Decision {
            project: d.project.to_string(),
            id: d.id.to_string(),
        })
    {
        app.screens.decision.reveal = true;
    }
}

/// After a send-again, the old fault is hidden on this device (`spec.md` §5.7).
fn hide_faults(app: &mut App, d: &DecisionRef) {
    let tasks: Vec<_> = app
        .link
        .server()
        .faults
        .get(&d.project)
        .and_then(|k| k.value())
        .into_iter()
        .flatten()
        .filter(|f| f.decision.as_ref() == Some(&d.id))
        .map(|f| f.task)
        .collect();
    for task in tasks {
        app.hidden.faults.insert((d.project.clone(), task));
    }
}
