//! The floats that need no server data: the palette, the `:` line, Confirm and `:messages`, and
//! which-key; and the slot for a screen's own floats (`crate::screens::ScreenFloat`), which the
//! app draws. A float that takes the keys is a root context: only its own keys act under it.

pub mod choices;
pub mod detail;
pub mod list_picker;
pub mod open_url;
pub mod outbox;
pub mod project_bind;
pub mod project_picker;
pub mod server;
pub mod sheet;
pub mod which_key;

pub use sheet::{Sheet, SheetRow};

use rune::{El, TextEdit, row, text, text_input};

use crate::keys::{Command, Ctx, Key, Mode, keymap};
use crate::screens::ScreenFloat;
use crate::theme::Theme;
use crate::ui::{Badge, Scope, Shell, float_hint, float_row, glyphs, shell, title};

/// What a picker lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerKind {
    /// `␣;`, `⌘K`: every command with its desc, `:` name and keys.
    Palette,
    /// `:messages`: the echo's history, newest last.
    Messages,
}

/// A picker: an input that filters rows, and a cursor over them.
#[derive(Clone, Debug)]
pub struct Picker {
    pub kind: PickerKind,
    pub query: TextEdit,
    pub cursor: usize,
}

/// The `:` line.
#[derive(Clone, Debug)]
pub struct CmdLine {
    pub line: TextEdit,
    /// The completion under the cursor.
    pub cursor: usize,
    /// How far back Up has walked the history.
    pub back: usize,
}

/// What Confirm's `y` does.
pub type OnYes = Box<dyn FnOnce(&mut crate::app::App, &mut rune::Tx)>;

/// The y/n float before an irreversible step.
pub struct Confirm {
    pub sentence: String,
    /// The word after `y`: `pairs`, `binds`.
    pub yes: &'static str,
    pub on_yes: Option<OnYes>,
}

pub enum Float {
    Picker(Picker),
    CmdLine(CmdLine),
    Confirm(Confirm),
    /// `:jumps`, the keys cheat sheet.
    Sheet(Sheet),
    /// A float a screen family owns: the peek float, the readback, a picker of its own.
    Screen(Box<dyn ScreenFloat>),
}

fn focused_edit() -> TextEdit {
    let mut e = TextEdit::new("");
    e.focused = true;
    e
}

impl Float {
    pub fn palette() -> Float {
        Float::Picker(Picker {
            kind: PickerKind::Palette,
            query: focused_edit(),
            cursor: 0,
        })
    }

    pub fn messages() -> Float {
        Float::Picker(Picker {
            kind: PickerKind::Messages,
            query: focused_edit(),
            cursor: 0,
        })
    }

    pub fn cmdline() -> Float {
        Float::CmdLine(CmdLine {
            line: focused_edit(),
            cursor: 0,
            back: 0,
        })
    }

    pub fn ctx(&self) -> Ctx {
        match self {
            Float::Picker(_) | Float::CmdLine(_) | Float::Sheet(_) => Ctx::Picker,
            Float::Confirm(_) => Ctx::Confirm,
            Float::Screen(f) => f.scope().ctx,
        }
    }

    pub fn mode(&self) -> Mode {
        match self {
            Float::Picker(_) | Float::CmdLine(_) | Float::Sheet(_) => Mode::Command,
            Float::Confirm(_) => Mode::Readback,
            Float::Screen(f) => f.scope().mode,
        }
    }

    pub fn badge(&self) -> Badge {
        match self {
            Float::Picker(_) | Float::CmdLine(_) | Float::Sheet(_) => Badge::Command,
            Float::Confirm(_) => Badge::ReadbackPlain,
            Float::Screen(f) => f.badge(),
        }
    }

    pub fn scope(&self) -> Scope {
        Scope {
            mode: self.mode(),
            ctx: self.ctx(),
        }
    }

    /// The text the float's input holds.
    pub fn edit_mut(&mut self) -> Option<&mut TextEdit> {
        match self {
            Float::Picker(p) => Some(&mut p.query),
            Float::CmdLine(c) => Some(&mut c.line),
            Float::Sheet(s) => Some(&mut s.query),
            Float::Confirm(_) => None,
            Float::Screen(f) => f.edit_mut(),
        }
    }
}

fn matches(query: &str, hay: &[&str]) -> bool {
    let q = query.trim().to_lowercase();
    q.is_empty() || hay.iter().any(|h| h.to_lowercase().contains(&q))
}

/// Why `command` can't run where the keys are (`ctx`), or `None` when it can. A command of the
/// views says which to open (`open a decision first`); any other names where it runs.
pub fn unavailable(command: &Command, ctx: Ctx) -> Option<String> {
    if keymap().applies(command, ctx) {
        return None;
    }
    let mut views = Vec::new();
    let mut places = Vec::new();
    for c in &command.ctx {
        let (view, place): (&[&str], &[&str]) = match c {
            Ctx::Conversation => (&["a conversation"], &[]),
            Ctx::Decision => (&["a decision"], &[]),
            Ctx::Project => (&["a project"], &[]),
            Ctx::Reader => (&["a conversation", "a decision"], &[]),
            Ctx::Global => (&[], &["anywhere"]),
            Ctx::List => (&[], &["a list"]),
            Ctx::Sidebar => (&[], &["the sidebar"]),
            Ctx::Editor => (&[], &["an editor"]),
            Ctx::Composer => (&[], &["the composer"]),
            Ctx::Note => (&[], &["a note"]),
            Ctx::Field => (&[], &["an option field"]),
            Ctx::Connect => (&[], &["Connect"]),
            Ctx::Start => (&[], &["the start page"]),
            Ctx::Hint => (&[], &["hint mode"]),
            Ctx::Picker => (&[], &["a picker"]),
            Ctx::Bind => (&[], &["the bind float"]),
            Ctx::Readback => (&[], &["the readback"]),
            Ctx::Reply => (&[], &["the reply"]),
            Ctx::Confirm => (&[], &["Confirm"]),
            Ctx::Outbox => (&[], &["the outbox"]),
            Ctx::Server => (&[], &["the Server float"]),
            Ctx::Peek => (&[], &["the peek"]),
        };
        views.extend_from_slice(view);
        places.extend_from_slice(place);
    }
    Some(if places.is_empty() {
        format!("open {} first", either(&views))
    } else {
        views.extend(places);
        format!("only in {}", either(&views))
    })
}

/// `a`, `a or b`, `a, b or c`.
fn either(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// The palette's rows for `query`, in the keymap's order.
pub fn palette_rows(query: &str) -> Vec<&'static Command> {
    keymap()
        .palette()
        .filter(|c| matches(query, &[&c.desc, &c.ex]))
        .collect()
}

/// The `:` line's completions: commands whose name starts with the word typed.
pub fn completions(line: &str) -> Vec<&'static Command> {
    let word = line.split_whitespace().next().unwrap_or("");
    keymap().ex_completions(word)
}

/// The messages a `:messages` picker shows for `query`.
pub fn message_rows<'a>(history: &'a [String], query: &str) -> Vec<&'a String> {
    history.iter().filter(|m| matches(query, &[m])).collect()
}

/// The float's input: `float.input` with a coral caret. `on_edit` reaches the same field.
fn input(t: Theme, edit: &TextEdit, prefix: Option<&str>, count: String) -> El {
    let field = text_input(edit, t.ty.float_input)
        .single_line()
        .caret(t.c.state.attn, t.c.surface.selection)
        .color(t.c.fg.default)
        .on_edit(|s: &mut crate::app::App| s.float_edit())
        .on_input_key(crate::app::input::on_field_key)
        .key("float-input");
    let mut typed = row().center_y();
    if let Some(p) = prefix {
        typed = typed.child(
            text(p.to_string())
                .font(t.ty.float_input)
                .color(t.c.fg.default),
        );
    }
    row()
        .center_y()
        .gap(10.0)
        .child(typed.child(field.min_w(2.0).shrink()).shrink().min_w(0.0))
        .child(text(count).font(t.ty.ui_small).color(t.c.fg.muted))
        .child(rune::spacer())
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The top float in its shell. `ctx` is where the keys were before it opened, for the palette's
/// "runs here" test; `history` is the echo's history.
pub fn view(t: Theme, f: &Float, ctx: Ctx, history: &[String]) -> El {
    let scope = f.scope();
    let hint = |seqs: &[&str], word: &str| float_hint(t, scope, seqs, word);
    match f {
        Float::Picker(p) if p.kind == PickerKind::Palette => {
            let rows = palette_rows(&p.query.text);
            let mut body = rune::col().w_full();
            for (i, c) in rows.iter().enumerate().take(12) {
                let here = unavailable(c, ctx).is_none();
                let fg = if here { t.c.fg.default } else { t.c.fg.muted };
                let mut keys = row().center_y().gap(8.0).w(120.0);
                for k in keymap().keys_of(c, Mode::Normal) {
                    keys = keys.child(glyphs(&k, t.ty.mono_small, t.c.fg.muted));
                }
                let content = row()
                    .center_y()
                    .gap(12.0)
                    .child(
                        row()
                            .center_y()
                            .gap(8.0)
                            .grow(1.0)
                            .shrink()
                            .min_w(0.0)
                            .child(
                                text(c.desc.clone())
                                    .font(t.ty.ui)
                                    .color(fg)
                                    .truncate()
                                    .shrink()
                                    .min_w(0.0),
                            )
                            .children(unavailable(c, ctx).map(|r| {
                                text(format!("· {r}"))
                                    .font(t.ty.ui_small)
                                    .color(t.c.fg.muted)
                            })),
                    )
                    .child(
                        text(format!(":{}", c.ex))
                            .font(t.ty.mono_small)
                            .color(if here { t.c.fg.alt } else { t.c.fg.muted })
                            .w(160.0),
                    )
                    .child(keys);
                body = body.child(float_row(t, i == p.cursor, content));
            }
            shell(
                t,
                Shell {
                    tag: None,
                    title: title(t, "Commands"),
                    subtitle: None,
                    meta: None,
                    body,
                    input: Some(input(
                        t,
                        &p.query,
                        None,
                        plural(rows.len(), "command", "commands"),
                    )),
                    hints: vec![hint(&["<CR>"], "choose"), hint(&["<Esc>"], "back")],
                },
            )
        }
        Float::Picker(p) => {
            let rows = message_rows(history, &p.query.text);
            let mut body = rune::col().w_full();
            for (i, m) in rows.iter().enumerate().skip(rows.len().saturating_sub(12)) {
                body = body.child(float_row(
                    t,
                    i == p.cursor,
                    text((*m).clone())
                        .font(t.ty.ui)
                        .color(t.c.fg.default)
                        .truncate(),
                ));
            }
            shell(
                t,
                Shell {
                    tag: None,
                    title: title(t, "Messages"),
                    subtitle: None,
                    meta: None,
                    body,
                    input: Some(input(
                        t,
                        &p.query,
                        None,
                        plural(rows.len(), "message", "messages"),
                    )),
                    hints: vec![hint(&["<Esc>"], "back")],
                },
            )
        }
        Float::CmdLine(c) => {
            let rows = completions(&c.line.text);
            let mut body = rune::col().w_full();
            for (i, cmd) in rows.iter().enumerate().take(8) {
                let content = row()
                    .center_y()
                    .gap(12.0)
                    .child(
                        text(format!(":{}", cmd.ex))
                            .font(t.ty.mono_small)
                            .color(t.c.kind.literal)
                            .w(160.0),
                    )
                    .child(
                        text(cmd.desc.clone())
                            .font(t.ty.ui)
                            .color(t.c.fg.default)
                            .truncate()
                            .shrink()
                            .min_w(0.0),
                    );
                body = body.child(float_row(t, i == c.cursor, content));
            }
            shell(
                t,
                Shell {
                    tag: None,
                    title: title(t, "Command line"),
                    subtitle: None,
                    meta: None,
                    body,
                    input: Some(input(t, &c.line, Some(":"), String::new())),
                    hints: vec![
                        hint(&["<Tab>"], "complete"),
                        hint(&["<CR>"], "run"),
                        hint(&["<Esc>"], "back"),
                    ],
                },
            )
        }
        Float::Confirm(c) => shell(
            t,
            Shell {
                tag: None,
                title: title(t, "Confirm"),
                subtitle: None,
                meta: None,
                body: text(c.sentence.clone())
                    .font(t.ty.prose_small)
                    .color(t.c.fg.default)
                    .wrap_anywhere()
                    .px(t.l.float.padding_x)
                    .pt(4.0)
                    .pb(8.0),
                input: None,
                hints: vec![hint(&["y"], c.yes), hint(&["n"], "cancels")],
            },
        ),
        Float::Sheet(s) => sheet::view(t, scope, s),
        // The app draws a screen's float: it reads the app's state.
        Float::Screen(_) => rune::el(),
    }
}

/// Whether `key` moves a picker's cursor (`C-j` down, `C-k` up).
pub fn cursor_step(key: Key) -> Option<i64> {
    if key == Key::ctrl('j') {
        Some(1)
    } else if key == Key::ctrl('k') {
        Some(-1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests;
