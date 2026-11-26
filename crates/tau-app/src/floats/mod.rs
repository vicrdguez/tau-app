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
pub mod settings;
pub mod sheet;
pub mod which_key;

pub use sheet::{Sheet, SheetRow};

use rune::{El, TextEdit, row, text, text_input};

use crate::keys::{Command, Ctx, Key, Keymap, Mode};
use crate::screens::ScreenFloat;
use crate::settings::Settings;
use crate::theme::Theme;
use crate::ui::{
    Badge, Scope, Shell, float_hint, float_hint_if, float_row, printed_keys, shell, title,
};

pub(crate) fn model_refresh(t: Theme, scope: Scope, app: &crate::app::App) -> El {
    let seq = "<C-r>";
    let pending = app.link.models_refreshing();
    let word = if pending {
        "Refreshing models…"
    } else {
        "Refresh models"
    };
    let key = t.keymap.chip(seq, scope.mode, scope.ctx);
    crate::ui::control(
        float_hint_if(t, scope, &[seq], word, !pending),
        crate::ui::with_keys(word, &[&key]),
        !pending,
        |a: &mut crate::app::App, tx| crate::app::commands::run_id(a, "picker.refresh-models", tx),
    )
    .key("refresh-models")
}

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
    /// `␣hs`, `:settings`, `⌘,`.
    Settings(settings::SettingsFloat),
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

    /// `:messages`, on the newest of `count`.
    pub fn messages(count: usize) -> Float {
        Float::Picker(Picker {
            kind: PickerKind::Messages,
            query: focused_edit(),
            cursor: count.saturating_sub(1),
        })
    }

    pub fn settings() -> Float {
        Float::Settings(settings::SettingsFloat::new(focused_edit()))
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
            Float::Settings(_) => Ctx::Settings,
            Float::Screen(f) => f.scope().ctx,
        }
    }

    pub fn mode(&self) -> Mode {
        match self {
            Float::Picker(_) | Float::CmdLine(_) | Float::Sheet(_) | Float::Settings(_) => {
                Mode::Command
            }
            Float::Confirm(_) => Mode::Readback,
            Float::Screen(f) => f.scope().mode,
        }
    }

    pub fn badge(&self) -> Badge {
        match self {
            Float::Picker(_) | Float::CmdLine(_) | Float::Sheet(_) | Float::Settings(_) => {
                Badge::Command
            }
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
            Float::Settings(s) => Some(&mut s.query),
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
pub fn unavailable(km: Keymap, command: &Command, ctx: Ctx) -> Option<String> {
    if km.applies(command, ctx) {
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
            Ctx::Settings => (&[], &["the settings float"]),
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
pub fn palette_rows(km: Keymap, query: &str) -> Vec<&'static Command> {
    km.palette()
        .filter(|c| matches(query, &[&c.desc, &c.ex]))
        .collect()
}

/// The `:` line's completions: commands whose name starts with the word typed.
pub fn completions(km: Keymap, line: &str) -> Vec<&'static Command> {
    let word = line.split_whitespace().next().unwrap_or("");
    km.ex_completions(word)
}

/// The messages a `:messages` picker shows for `query`.
pub fn message_rows<'a>(history: &'a [String], query: &str) -> Vec<&'a String> {
    history.iter().filter(|m| matches(query, &[m])).collect()
}

/// A float's input (`spec.md` §6).
pub struct Input<'a> {
    pub edit: &'a TextEdit,
    /// What it filters, for a screen reader: `Filter projects`.
    pub label: &'a str,
    /// Shown while it is empty: `type to filter`, or `type to search` in Find.
    pub placeholder: &'a str,
    /// After it, muted: `3 projects`, `1 of 3 projects` ([`Noun::kept`]).
    pub count: String,
}

/// A picker's input: `float.input` with a coral caret, then the count.
pub fn input(t: Theme, i: Input) -> El {
    prefixed(t, i, None)
}

/// The input with `prefix` typed before the caret, as the `:` line has it. `on_edit` reaches the
/// top float's field.
fn prefixed(t: Theme, i: Input, prefix: Option<&str>) -> El {
    let field = text_input(i.edit, t.ty.float_input)
        .single_line()
        .caret(t.c.state.attn, t.c.surface.selection)
        .color(t.c.fg.default)
        .on_edit(|s: &mut crate::app::App| s.float_edit())
        .on_input_key(crate::app::input::on_field_key)
        // What a query keeps changes the float's height: its edge glides there.
        .animate_edit_layout()
        .label(i.label)
        .key("float-input");
    let mut typed = row().center_y();
    if let Some(p) = prefix {
        typed = typed.child(
            text(p.to_string())
                .font(t.ty.float_input)
                .color(t.c.fg.default),
        );
    }
    typed = typed.child(field.min_w(2.0).shrink());
    // Beside the field rather than its own placeholder: a field is as wide as its text, and
    // the count must follow the words.
    if i.edit.text.is_empty() && !i.placeholder.is_empty() {
        typed = typed.child(
            text(i.placeholder.to_string())
                .font(t.ty.float_input)
                .color(t.c.fg.muted)
                .truncate(),
        );
    }
    row()
        .center_y()
        .gap(10.0)
        .child(typed.shrink().min_w(0.0))
        .child(text(i.count).font(t.ty.ui_small).color(t.c.fg.muted))
        .child(rune::spacer())
}

/// A noun as a count says it: `1 project`, `3 projects`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noun {
    pub one: &'static str,
    pub many: &'static str,
}

impl Noun {
    pub const fn new(one: &'static str, many: &'static str) -> Noun {
        Noun { one, many }
    }

    pub fn count(self, n: usize) -> String {
        plural(n, self.one, self.many)
    }

    /// What a picker's foot says of its rows: all of them, `3 projects`, until a query is typed;
    /// then how many it keeps, `1 of 3 projects`.
    pub fn kept(self, query: &str, kept: usize, total: usize) -> String {
        if query.trim().is_empty() {
            self.count(total)
        } else {
            format!("{kept} of {}", self.count(total))
        }
    }

    /// `No project matches "zzzz"`.
    pub fn none(self, query: &str) -> String {
        format!("No {} matches \"{query}\"", self.one)
    }
}

/// The one line a picker's body shows when its query keeps no row.
pub fn no_match(t: Theme, words: String) -> El {
    float_row(
        t,
        false,
        text(words).font(t.ty.ui).color(t.c.fg.alt).truncate(),
    )
}

/// The placeholder of every picker but Find.
pub const FILTER: &str = "type to filter";
const COMMANDS: Noun = Noun::new("command", "commands");
const MESSAGES: Noun = Noun::new("message", "messages");

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The top float in its shell. `ctx` is where the keys were before it opened, for the palette's
/// "runs here" test; `history` is the echo's history.
pub fn view(t: Theme, f: &Float, ctx: Ctx, history: &[String], settings: &Settings) -> El {
    let scope = f.scope();
    let hint = |seqs: &[&str], word: &str| float_hint(t, scope, seqs, word);
    match f {
        Float::Picker(p) if p.kind == PickerKind::Palette => {
            let rows = palette_rows(t.keymap, &p.query.text);
            let mut body = rune::col().w_full();
            for (i, c) in rows.iter().enumerate() {
                let here = unavailable(t.keymap, c, ctx).is_none();
                let fg = if here { t.c.fg.default } else { t.c.fg.muted };
                let mut keys = row().center_y().gap(8.0).w(120.0);
                for k in t.keymap.keys_of(c, Mode::Normal) {
                    keys = keys.child(printed_keys(&[k], t.ty.mono_small, t.c.fg.muted));
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
                            .children(unavailable(t.keymap, c, ctx).map(|r| {
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
            if rows.is_empty() {
                body = body.child(no_match(t, COMMANDS.none(&p.query.text)));
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
                        Input {
                            edit: &p.query,
                            label: "Filter commands",
                            placeholder: FILTER,
                            count: COMMANDS.kept(
                                &p.query.text,
                                rows.len(),
                                palette_rows(t.keymap, "").len(),
                            ),
                        },
                    )),
                    hints: vec![
                        float_hint_if(t, scope, &["<CR>"], "choose", !rows.is_empty()),
                        hint(&["<Esc>"], "back"),
                    ],
                },
            )
        }
        Float::Picker(p) => {
            let rows = message_rows(history, &p.query.text);
            let mut body = rune::col().w_full();
            for (i, m) in rows.iter().enumerate() {
                body = body.child(float_row(
                    t,
                    i == p.cursor,
                    text((*m).clone())
                        .font(t.ty.ui)
                        .color(t.c.fg.default)
                        .truncate(),
                ));
            }
            if rows.is_empty() && !history.is_empty() {
                body = body.child(no_match(t, MESSAGES.none(&p.query.text)));
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
                        Input {
                            edit: &p.query,
                            label: "Filter messages",
                            placeholder: FILTER,
                            count: MESSAGES.kept(&p.query.text, rows.len(), history.len()),
                        },
                    )),
                    hints: vec![hint(&["<Esc>"], "back")],
                },
            )
        }
        Float::CmdLine(c) => {
            let rows = completions(t.keymap, &c.line.text);
            let mut body = rune::col().w_full();
            for (i, cmd) in rows.iter().enumerate() {
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
                    input: Some(prefixed(
                        t,
                        Input {
                            edit: &c.line,
                            label: "Command line",
                            placeholder: "",
                            count: String::new(),
                        },
                        Some(":"),
                    )),
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
        Float::Settings(s) => settings::view(t, s, settings),
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
mod settings_tests;
#[cfg(test)]
mod tests;
