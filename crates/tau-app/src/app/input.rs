//! Keys, from rune's event to what they do: the keymap engine resolves each key for the region
//! that has the keys; a command goes to its handler, and what is left goes to the region's own
//! handler (the sidebar's motions, a float's input).

use rune::{KeyEvent, Tx};

use super::commands::{self, Invocation};
use super::{App, Focus};
use crate::floats::Float;
use crate::frame::Row;
use crate::keys::{Code, Key, Outcome, from_rune};
use crate::screens;

/// The window's `on_key`. What a key changes lands at once (`visual-language.md` §7: rows never
/// move, the sidebar shows and hides with no slide, `␣ht` switches within one frame), except a
/// float it opens, which fades in.
pub fn on_key(app: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let floats = app.floats.len();
    let handled = match from_rune(ev, tx.key_base()) {
        Some(key) => press(app, key, tx.key_repeat(), tx),
        None => false,
    };
    if app.floats.len() > floats {
        tx.animated();
    } else {
        tx.instant();
    }
    handled
}

/// A focused field's `on_input_key`: the same keys, before the field types them.
pub fn on_field_key(app: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    on_key(app, ev, tx)
}

/// One key. False hands it on (a float's field types it).
pub fn press(app: &mut App, key: Key, repeat: bool, tx: &mut Tx) -> bool {
    if app.keys.is_idle() {
        app.echo = None;
        if crate::yank::key(app, key, tx) {
            return true;
        }
    }
    let scope = app.scope();
    match app
        .keys
        .press(app.keymap, scope.mode, scope.ctx, key, repeat, app.now)
    {
        Outcome::Pending => {
            tx.wake_after(super::which_key_delay(app.keymap));
            true
        }
        Outcome::Cancelled | Outcome::Ignored => true,
        Outcome::Command {
            command,
            count,
            applies,
            keys,
        } => {
            commands::run(
                app,
                &Invocation {
                    command,
                    count,
                    applies,
                    keys,
                },
                tx,
            );
            true
        }
        Outcome::Builtin { keys, count } | Outcome::Unbound { keys, count } => {
            let from = crate::nav::is_jump(&keys).then(|| crate::nav::here(app));
            let taken = region(app, &keys, count);
            if let Some(from) = from {
                crate::nav::left(app, from);
            }
            taken
        }
    }
}

/// Keys no command took, for the region that has them: a screen family's (its page or its float),
/// a frame float's, or the sidebar's.
fn region(app: &mut App, keys: &[Key], count: Option<u32>) -> bool {
    if app.hints.is_some() {
        return true;
    }
    if let Some(screen) = screens::holder(app) {
        return screens::get(screen).keys(app, keys, count);
    }
    if let Some((f, _)) = app.floats.last_mut() {
        return float_keys(f, &app.history, keys);
    }
    if app.focus == Focus::Sidebar {
        sidebar_motion(app, keys, count.unwrap_or(1));
    }
    true
}

/// A float's own keys: typing goes to its field; Up and Down walk the `:` line's history.
fn float_keys(f: &mut Float, history: &[String], keys: &[Key]) -> bool {
    let [key] = keys else { return true };
    match f {
        Float::CmdLine(c) if matches!(key.code, Code::Up | Code::Down) => {
            let back = if key.code == Code::Up {
                (c.back + 1).min(history.len())
            } else {
                c.back.saturating_sub(1)
            };
            c.back = back;
            let line = if back == 0 {
                String::new()
            } else {
                history[history.len() - back].clone()
            };
            c.line.replace_text(line);
            c.cursor = 0;
            true
        }
        Float::Picker(p) => {
            p.cursor = 0;
            false
        }
        Float::Sheet(s) => {
            s.cursor = 0;
            false
        }
        Float::CmdLine(c) => {
            c.cursor = 0;
            false
        }
        Float::Confirm(_) | Float::Screen(_) => true,
    }
}

/// `j k gg G` over the sidebar's rows.
fn sidebar_motion(app: &mut App, keys: &[Key], count: u32) {
    let (rows, _) = app.sidebar_rows();
    let at = rows
        .iter()
        .position(|r| *r == app.sidebar_cursor)
        .unwrap_or(0);
    let last = rows.len() - 1;
    let to = match keys {
        [k] if *k == Key::char('j') => (at + count as usize).min(last),
        [k] if *k == Key::char('k') => at.saturating_sub(count as usize),
        [g, h] if *g == Key::char('g') && *h == Key::char('g') => 0,
        [k] if *k == Key::char('G') => last,
        _ => at,
    };
    app.sidebar_cursor = rows[to].clone();
}

/// Moves the sidebar's cursor to a row that still exists after an item closed.
pub(super) fn keep_cursor(app: &mut App, was: usize) {
    let (rows, _) = app.sidebar_rows();
    if !rows.contains(&app.sidebar_cursor) {
        app.sidebar_cursor = rows
            .get(was)
            .cloned()
            .filter(|r| *r != Row::Server)
            .or_else(|| rows.get(was.saturating_sub(1)).cloned())
            .unwrap_or(Row::New);
    }
}
