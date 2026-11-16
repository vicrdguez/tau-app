//! Readback commands: opening, choosing, cancelling and confirming.

use tau_client::{DecisionRef, NotSent, Write};

use super::super::reader::{self, record, shown};
use super::super::state::Cursor;
use super::{Kind, Phase, Readback, again_off, answer_off, arm, link_off};
use crate::app::{App, Invocation};
use crate::copy::AnswerAction;
use crate::floats::Float;
use crate::keys::Key;
use crate::screens::top_float;

/// `␣ad`: the readback, its pick preselected when the cursor is on a choice.
pub fn answer(app: &mut App, inv: &Invocation, _: &mut rune::Tx) {
    let Some(d) = shown(app).filter(|_| inv.applies) else {
        app.say("open it first · ⏎");
        return;
    };
    if let Some(off) = answer_off(app, &d) {
        app.say(off.echo());
        return;
    }
    let pick = match reader::cursor(app, &d) {
        Some(Cursor::Choice(i)) => Some(i),
        _ => None,
    };
    open(app, d, Kind::Answer, pick);
}

/// `␣ar`: send the answer again, after a fault or a lost reply.
pub fn again(app: &mut App, inv: &Invocation, _: &mut rune::Tx) {
    let Some(d) = shown(app).filter(|_| inv.applies) else {
        app.say("open it first · ⏎");
        return;
    };
    if let Some(off) = again_off(app, &d) {
        app.say(off.echo());
        return;
    }
    let Some(view) = record(app, &d) else { return };
    let kind = match app.link.outbox().answer_unsettled(&d) {
        Some(e) => Kind::AgainUnknown(e.id),
        None => Kind::AgainAfterFault(Box::new(view.decision.clone())),
    };
    open(app, d, kind, None);
}

fn open(app: &mut App, decision: DecisionRef, kind: Kind, pick: Option<usize>) {
    app.screens.decision.reveal = true;
    app.open_float(Float::Screen(Box::new(Readback {
        decision,
        kind,
        pick,
        drawn: None,
        shown_at: None,
        phase: Phase::Reading,
        notes_open: false,
    })));
}

/// `j k` and `1`–`9`: the pick. Each redraws, and `y` waits for that draw.
pub fn pick(app: &mut App, inv: &Invocation, _: &mut rune::Tx) {
    let Some(&key) = inv.keys.first() else { return };
    let choices = top_float::<Readback>(app)
        .map(|f| f.decision.clone())
        .and_then(|d| record(app, &d).map(|v| v.decision.choices.len()))
        .unwrap_or(0);
    let Some(f) = top_float::<Readback>(app) else {
        return;
    };
    if f.phase != Phase::Reading || f.kind != Kind::Answer || choices == 0 {
        return;
    }
    let last = choices - 1;
    f.pick = if key == Key::char('j') {
        Some(f.pick.map_or(0, |p| (p + 1).min(last)))
    } else if key == Key::char('k') {
        Some(f.pick.map_or(0, |p| p.saturating_sub(1)))
    } else {
        match key.digit() {
            Some(n) if choices <= 9 && n >= 1 && (n as usize) <= choices => Some(n as usize - 1),
            _ => f.pick,
        }
    };
}

/// `K`: every note in full.
pub fn details(app: &mut App, _: &Invocation, _: &mut rune::Tx) {
    if let Some(f) = top_float::<Readback>(app) {
        f.notes_open = !f.notes_open;
    }
}

/// `n Esc q C-c`, and the reply's `Esc q`.
pub fn cancel(app: &mut App, _: &Invocation, _: &mut rune::Tx) {
    if top_float::<Readback>(app).is_some() {
        app.close_float();
    }
}

/// `y`: sends what the float last drew, once it has been on screen for the arm time.
pub fn confirm(app: &mut App, _: &Invocation, _: &mut rune::Tx) {
    let now = app.now;
    let Some(f) = top_float::<Readback>(app) else {
        return;
    };
    if f.phase != Phase::Reading {
        return;
    }
    let armed = f.shown_at.is_some_and(|at| now - at >= arm() - 1e-9) && f.drawn == Some(f.pick);
    if !armed {
        app.say("read it first");
        return;
    }
    let (d, kind, pick) = (f.decision.clone(), f.kind.clone(), f.pick);
    if let Some(off) = link_off(app) {
        app.say(off.echo());
        return;
    }
    let sent = match kind {
        Kind::Answer => {
            let Some(view) = record(app, &d) else { return };
            let Some(choice) = pick.and_then(|p| view.decision.choices.get(p)).cloned() else {
                app.say("pick a choice first");
                return;
            };
            let basis = view.decision.basis;
            let annotations = app.screens.decision.notes.annotations(&d);
            app.link
                .write(Write::Answer {
                    decision: d.clone(),
                    basis,
                    choice,
                    annotations,
                })
                .map(|id| (id, AnswerAction::Answer))
        }
        Kind::AgainAfterFault(r) => app
            .link
            .write(Write::Answer {
                decision: d.clone(),
                basis: r.basis,
                choice: r.choice.clone().unwrap_or_default(),
                annotations: r.annotations.clone(),
            })
            .map(|id| (id, AnswerAction::SendAgainAfterFault)),
        Kind::AgainUnknown(id) => app.link.retry(id).map(|()| (id, AnswerAction::Answer)),
    };
    match sent {
        Ok((id, action)) => {
            app.screens.decision.sent.insert(id, (d, action));
            if let Some(f) = top_float::<Readback>(app) {
                f.phase = Phase::Sending(id);
            }
        }
        Err(why) => app.say(not_sent(&why)),
    }
}

fn not_sent(why: &NotSent) -> &'static str {
    match why {
        NotSent::AnswerUnsettled => "an answer is waiting for a reply",
        NotSent::Revoked => "revoked · nothing is sent",
        NotSent::Refused => "refused by the server · nothing is sent",
        NotSent::OtherProtocol => "the server speaks another protocol · nothing is sent",
        NotSent::NotPaired => "not paired · nothing is sent",
        _ => "offline · answer when connected",
    }
}
