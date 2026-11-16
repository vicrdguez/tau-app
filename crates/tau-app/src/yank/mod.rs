//! Yanking text out of the readers (`grammar.md` §4): `y{obj}` into the app's register, `␣y{obj}`
//! onto the system clipboard as well; doubled (`yy`, `␣yy`) for the block under the cursor, and in
//! Visual the selection. The text is the source bytes of the object (`visual-language.md` §3);
//! each reader finds its objects through `Screen::yank`.
//!
//! Once `y` waits for its object it takes the next keys itself, so `i` and `a` start an object
//! rather than run their own commands.

pub mod objects;

use rune::Tx;

use crate::app::{App, Handler, Invocation};
use crate::keys::{Code, Ctx, Key, Mode};
use crate::screens;

/// What `y` takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Object {
    Word {
        around: bool,
    },
    Sentence {
        around: bool,
    },
    Paragraph {
        around: bool,
    },
    /// A conversation's message: an input, a turn's text, a record's.
    Message {
        around: bool,
    },
    /// The block under the cursor (`yy`).
    Line,
    Selection,
}

/// An operator waiting for its object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending {
    clipboard: bool,
    /// `i` (false) or `a` (true) typed, waiting for the object's letter.
    around: Option<bool>,
}

pub fn handlers() -> Vec<(&'static str, Handler)> {
    vec![
        ("yank", start),
        ("yank.clipboard", start),
        ("textobject.tau", |a, _, _| {
            a.say("a text object follows an operator: yank, or a note's change")
        }),
    ]
}

/// The operator as the bottom bar shows it while it waits: `y`, `␣yi`.
pub fn shown(app: &App) -> String {
    let Some(p) = app.yank else {
        return String::new();
    };
    let op = if p.clipboard { "␣y" } else { "y" };
    let half = match p.around {
        Some(true) => "a",
        Some(false) => "i",
        None => "",
    };
    format!("{op}{half}")
}

fn start(app: &mut App, inv: &Invocation, tx: &mut Tx) {
    let clipboard = inv.command.id == "yank.clipboard";
    let scope = app.scope();
    let in_reader = app.keymap.chain(scope.ctx).contains(&Ctx::Reader);
    if !in_reader || screens::holder(app) != Some(screens::of(&app.stage)) {
        app.say("nothing to yank here");
        return;
    }
    if scope.mode == Mode::Visual {
        take(app, Object::Selection, clipboard, tx);
    } else {
        app.yank = Some(Pending {
            clipboard,
            around: None,
        });
    }
}

/// A key while an operator waits: true when it took the key. `Esc`, or a key that names no
/// object, drops the operator.
pub fn key(app: &mut App, key: Key, tx: &mut Tx) -> bool {
    let Some(p) = app.yank.take() else {
        return false;
    };
    if key == Key::plain(Code::Esc) {
        return true;
    }
    let obj = match (p.around, key.typed()) {
        (None, Some('y')) => Object::Line,
        (None, Some(' ')) if p.clipboard => {
            app.yank = Some(p);
            return true;
        }
        (None, Some(c @ ('i' | 'a'))) => {
            app.yank = Some(Pending {
                around: Some(c == 'a'),
                ..p
            });
            return true;
        }
        (Some(around), Some('w')) => Object::Word { around },
        (Some(around), Some('s')) => Object::Sentence { around },
        (Some(around), Some('p')) => Object::Paragraph { around },
        (Some(around), Some('m')) => Object::Message { around },
        _ => return true,
    };
    take(app, obj, p.clipboard, tx);
    true
}

fn take(app: &mut App, obj: Object, clipboard: bool, tx: &mut Tx) {
    let Some(text) = screens::page(app).yank(app, obj) else {
        app.say("nothing to yank here");
        return;
    };
    let lines = text.lines().count();
    let size = if lines > 1 {
        format!("{lines} lines")
    } else {
        format!("{} characters", text.chars().count())
    };
    if clipboard {
        tx.copy(text.clone());
        app.say(format!("copied {size} to the clipboard"));
    } else {
        app.say(format!("yanked {size}"));
    }
    app.register = Some(text);
}

#[cfg(test)]
mod tests;
