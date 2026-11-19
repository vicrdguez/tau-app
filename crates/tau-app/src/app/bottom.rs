//! The bottom float (`spec.md` §6): the top float, else which-key, else the float that just
//! closed, drawn inert on the page it closed on until its edge is down (`visual-language.md` §7).

use std::time::Duration;

use rune::{El, Tx};

use super::{App, Focus, Stage, input};
use crate::floats::{self, Float};
use crate::frame::BottomFloat;
use crate::keys::{Ctx, Key};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::ui::Scope;

/// What a float showed as it closed.
pub enum Closed {
    Float(Box<Float>, Focus),
    WhichKey(Vec<Key>, Scope),
}

/// A float on its way out: what it showed, the page it closed on, and when.
pub struct Leaving {
    closed: Closed,
    stage: Stage,
    at: f64,
}

impl App {
    /// A float closed on `stage`: drawn there, inert, until its edge is down.
    pub(super) fn leave(&mut self, closed: Closed, stage: Stage) -> &Closed {
        &self
            .leaving
            .insert(Leaving {
                closed,
                stage,
                at: self.now,
            })
            .closed
    }

    /// [`App::leave`] for a float its owner took off the stack to act on its choice.
    pub fn closed(&mut self, f: Float, opener: Focus) {
        self.leave(Closed::Float(Box::new(f), opener), self.stage.clone());
    }
}

/// Forgets the float that left once its edge is down, and wakes for that until then.
pub(super) fn poll(app: &mut App, tx: &mut Tx) -> bool {
    let Some(l) = &app.leaving else { return false };
    let down = l.at + f64::from(app.theme().motion.panel.settling_duration(1e-4));
    if app.now < down {
        tx.wake_after(Duration::from_secs_f64(down - app.now));
        return false;
    }
    app.leaving = None;
    true
}

pub(super) fn view(
    t: Theme,
    app: &App,
    page: &dyn Screen,
    scope: Scope,
    width: f32,
) -> Option<BottomFloat> {
    if let Some((f, opener)) = app.floats.last() {
        let f = float(t, app, page, f, *opener)
            .on_layout_tx(|_: &mut App, _, tx| input::reveal_float_cursor(tx));
        return Some(BottomFloat::Shown(f));
    }
    if app.which_key() {
        return which_key(t, app, page, app.keys.pending(), scope, width).map(BottomFloat::Shown);
    }
    let l = app
        .leaving
        .as_ref()
        .filter(|l| t.motion.on && l.stage == app.stage)?;
    match &l.closed {
        Closed::Float(f, opener) => Some(float(t, app, page, f, *opener)),
        Closed::WhichKey(keys, scope) => which_key(t, app, page, keys, *scope, width),
    }
    .map(BottomFloat::Leaving)
}

fn float(t: Theme, app: &App, page: &dyn Screen, f: &Float, opener: Focus) -> El {
    match f {
        Float::Screen(f) => f.view(t, app),
        f => {
            let ctx = match opener {
                Focus::Sidebar => Ctx::Sidebar,
                Focus::Stage => page.ctx(app),
            };
            floats::view(t, f, ctx, &app.messages, &app.settings)
        }
    }
}

fn which_key(
    t: Theme,
    app: &App,
    page: &dyn Screen,
    keys: &[Key],
    scope: Scope,
    width: f32,
) -> Option<El> {
    let wk = app.settings.keymap.which_key(keys, scope.mode, scope.ctx)?;
    let why = |id: &str| page.unavailable(app, id);
    Some(floats::which_key::view(t, &wk, scope.ctx, &why, width))
}
