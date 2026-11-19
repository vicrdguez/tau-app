//! The settings float (`␣hs`, `:settings`, `⌘,`): the theme, the reading width and motion, a
//! hairline, then one row per key of each command. Typing filters the rows. On a value row `C-h`
//! `C-l` (or Left Right) and `⏎` change it; on a key row `⏎` takes a new key and `C-x` puts back
//! the design's. Every change applies at once and is saved through `settings::change`.
//!
//! While a key row takes a new key, every key comes here before the keymap (`capture`): `⏎`
//! keeps the keys, `Esc` and `C-c` cancel, Backspace takes the last one back, auto-repeats and a
//! fifth key are ignored. A key the keymap refuses keeps the capture open with the reason on the
//! row and in the echo.

use rune::{El, TextEdit, Tx, col, el, row, text};

use super::{FILTER, Float, Input, Noun, input, matches, no_match};
use crate::app::{App, Invocation};
use crate::keys::{Code, Ctx, Key, KeyRow, Keymap, Mode, Refusal, display, written};
use crate::settings::{self, Edit, FILE, Field, Settings};
use crate::theme::Theme;
use crate::ui::{Scope, Shell, float_hint, float_hint_if, float_row, glyphs, shell, title};

/// A new key is at most this many keys long.
const MOST: usize = 4;
const SETTINGS: Noun = Noun::new("setting", "settings");

pub struct SettingsFloat {
    pub query: TextEdit,
    /// Among the rows the query keeps.
    pub cursor: usize,
    /// While the key row under the cursor takes a new key: the keys pressed so far.
    pub capture: Option<Vec<Key>>,
    /// Why what was asked of the row under the cursor didn't happen, until the cursor moves.
    pub note: Option<String>,
}

impl SettingsFloat {
    pub fn new(query: TextEdit) -> Self {
        SettingsFloat {
            query,
            cursor: 0,
            capture: None,
            note: None,
        }
    }
}

#[derive(Debug)]
pub enum Row {
    Field(Field),
    Key(KeyRow),
}

/// The values, then the keys, that `query` names by label, description, `:` name, key,
/// context or mode.
pub fn rows(km: Keymap, query: &str) -> Vec<Row> {
    let fields = Field::ALL
        .into_iter()
        .filter(|f| matches(query, &[f.label(), f.key()]))
        .map(Row::Field);
    let keys = km
        .key_rows()
        .into_iter()
        .filter(|r| {
            let mut hay = vec![r.command.desc.clone(), display(&r.now), written(&r.now)];
            if r.command.has_ex() {
                hay.push(format!(":{}", r.command.ex));
            }
            hay.extend(r.ctxs.iter().map(|c| c.name().to_string()));
            hay.extend(r.modes.iter().map(|m| mode_word(*m).to_string()));
            matches(query, &hay.iter().map(String::as_str).collect::<Vec<_>>())
        })
        .map(Row::Key);
    fields.chain(keys).collect()
}

fn mode_word(m: Mode) -> &'static str {
    match m {
        Mode::Normal => "normal",
        Mode::Visual => "visual",
        Mode::Insert => "insert",
        Mode::Command => "command",
        Mode::Hint => "hint",
        Mode::Readback => "readback",
        Mode::Peek => "peek",
        Mode::Outbox => "outbox",
        Mode::Server => "server",
    }
}

fn this(a: &mut App) -> Option<&mut SettingsFloat> {
    match a.floats.last_mut() {
        Some((Float::Settings(f), _)) => Some(f),
        _ => None,
    }
}

fn selected(a: &App) -> Option<Row> {
    let Some((Float::Settings(f), _)) = a.floats.last() else {
        return None;
    };
    rows(a.settings.keymap, &f.query.text)
        .into_iter()
        .nth(f.cursor)
}

/// `settings.keys`: move, change a value, take a new key, put back a key, close.
pub fn keys(a: &mut App, inv: &Invocation, _: &mut Tx) {
    let Some(&key) = inv.keys.first() else { return };
    if key == Key::plain(Code::Esc) || key == Key::ctrl('c') {
        a.close_float();
    } else if let Some(by) = super::cursor_step(key) {
        move_cursor(a, by);
    } else if key == Key::ctrl('l') {
        step(a, 1);
    } else if key == Key::ctrl('h') {
        step(a, -1);
    } else if key == Key::plain(Code::Enter) {
        act(a);
    } else if key == Key::ctrl('x') {
        put_back(a);
    }
}

/// Keys no command took: Down Up move, Right Left change a value, and the rest type in the
/// filter, which starts the rows over.
pub fn other_keys(a: &mut App, keys: &[Key]) -> bool {
    match keys {
        [k] if *k == Key::plain(Code::Down) => move_cursor(a, 1),
        [k] if *k == Key::plain(Code::Up) => move_cursor(a, -1),
        [k] if *k == Key::plain(Code::Right) => step(a, 1),
        [k] if *k == Key::plain(Code::Left) => step(a, -1),
        _ => {
            if let Some(f) = this(a) {
                f.cursor = 0;
                f.note = None;
            }
            return false;
        }
    }
    true
}

fn move_cursor(a: &mut App, by: i64) {
    let len = match a.floats.last() {
        Some((Float::Settings(f), _)) => rows(a.settings.keymap, &f.query.text).len(),
        _ => return,
    };
    if let Some(f) = this(a) {
        f.cursor = (f.cursor as i64 + by).clamp(0, len.saturating_sub(1) as i64) as usize;
        f.note = None;
    }
}

fn step(a: &mut App, by: i32) {
    if let Some(Row::Field(field)) = selected(a) {
        settings::change(a, field.step(&a.settings, by), Edit::Field(field));
    }
}

/// `⏎`, or a press on the row: a value steps on; a key row starts taking a new key.
fn act(a: &mut App) {
    match selected(a) {
        Some(Row::Field(_)) => step(a, 1),
        Some(Row::Key(KeyRow {
            fixed: Some(fixed), ..
        })) => refuse(a, Refusal::Fixed(fixed)),
        Some(Row::Key(_)) => {
            if let Some(f) = this(a) {
                f.capture = Some(Vec::new());
                f.note = None;
            }
        }
        None => {}
    }
}

/// `C-x`: the row's design key, checked like any other move.
fn put_back(a: &mut App) {
    let Some(Row::Key(row)) = selected(a) else {
        return;
    };
    match a
        .settings
        .keymap
        .rebind(&row.command.id, &row.design, &row.design)
    {
        Ok(km) => {
            a.say(format!(
                "{} is {} again",
                row.command.desc,
                display(&row.design)
            ));
            keep(a, &row, km);
        }
        Err(refusal) => refuse(a, refusal),
    }
}

fn keep(a: &mut App, row: &KeyRow, keymap: Keymap) {
    if let Some(f) = this(a) {
        f.capture = None;
        f.note = None;
    }
    let new = Settings {
        keymap,
        ..a.settings
    };
    settings::change(
        a,
        new,
        Edit::Key {
            command: row.command.id.as_str(),
            design: row.design.clone(),
        },
    );
}

fn refuse(a: &mut App, refusal: Refusal) {
    let why = refusal.to_string();
    if let Some(f) = this(a) {
        f.note = Some(why.clone());
    }
    a.say(why);
}

/// Whether a key row is taking a new key, so keys come to `capture` first.
pub fn capturing(a: &App) -> bool {
    matches!(
        a.floats.last(),
        Some((
            Float::Settings(SettingsFloat {
                capture: Some(_),
                ..
            }),
            _
        ))
    )
}

/// Every key while a key row takes a new key, before the keymap sees it.
pub fn capture(a: &mut App, key: Key, repeat: bool) -> bool {
    let Some(f) = this(a) else { return false };
    let Some(keys) = f.capture.as_mut() else {
        return false;
    };
    if repeat {
        return true;
    }
    if key == Key::plain(Code::Enter) {
        commit(a);
        return true;
    }
    if key == Key::plain(Code::Esc) || key == Key::ctrl('c') {
        f.capture = None;
    } else if key == Key::plain(Code::Backspace) {
        keys.pop();
    } else if keys.len() < MOST {
        keys.push(key);
    }
    f.note = None;
    true
}

/// `⏎` in capture: the keys taken become the row's, unless the keymap refuses them.
fn commit(a: &mut App) {
    let typed = this(a).and_then(|f| f.capture.clone()).unwrap_or_default();
    let Some(Row::Key(row)) = selected(a) else {
        return;
    };
    if typed.is_empty() {
        if let Some(f) = this(a) {
            f.capture = None;
        }
        return;
    }
    match a
        .settings
        .keymap
        .rebind(&row.command.id, &row.design, &typed)
    {
        Ok(km) => {
            a.say(format!("{} is now {}", row.command.desc, display(&typed)));
            keep(a, &row, km);
        }
        Err(refusal) => refuse(a, refusal),
    }
}

/// A press on row `i`: it is selected and acts as `⏎` does.
fn choose(a: &mut App, i: usize) {
    if let Some(f) = this(a) {
        f.cursor = i;
        f.capture = None;
        f.note = None;
    }
    act(a);
}

pub fn view(t: Theme, f: &SettingsFloat, s: &Settings) -> El {
    let scope = Scope {
        mode: Mode::Command,
        ctx: Ctx::Settings,
    };
    let rows = rows(s.keymap, &f.query.text);
    let mut body = col().w_full();
    for (i, r) in rows.iter().enumerate() {
        if i > 0 && matches!(r, Row::Key(_)) && matches!(rows[i - 1], Row::Field(_)) {
            body = body.child(
                el().h(1.0)
                    .mx(t.l.float.padding_x)
                    .fill(t.c.border.hairline),
            );
        }
        let here = i == f.cursor;
        let content = match r {
            Row::Field(field) => line(
                t,
                field.label(),
                text(format!("· {}", field.choices()))
                    .font(t.ty.ui_small)
                    .color(t.c.fg.muted),
                String::new(),
                glyphs(&field.shown(s), t.ty.mono_small, t.c.fg.muted),
            ),
            Row::Key(k) => {
                let note = here.then_some(f.note.as_deref()).flatten();
                let mut about = format!(
                    "{} · {}",
                    k.ctxs
                        .iter()
                        .map(|c| c.name())
                        .collect::<Vec<_>>()
                        .join(", "),
                    k.modes
                        .iter()
                        .map(|m| mode_word(*m))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if k.now != k.design {
                    about.push_str(&format!(" · yours, was {}", display(&k.design)));
                }
                let about = match note {
                    Some(why) => text(format!("· {why}"))
                        .font(t.ty.ui_small)
                        .color(t.c.text.failed),
                    None => text(format!("· {about}"))
                        .font(t.ty.ui_small)
                        .color(t.c.fg.muted),
                };
                let key = match f.capture.as_ref().filter(|_| here) {
                    Some(typed) if typed.is_empty() => {
                        text("press a key…").font(t.ty.ui_small).color(t.c.fg.muted)
                    }
                    Some(typed) => glyphs(&display(typed), t.ty.mono_small, t.c.fg.default),
                    None => glyphs(&display(&k.now), t.ty.mono_small, t.c.fg.muted),
                };
                let ex = if k.command.has_ex() {
                    format!(":{}", k.command.ex)
                } else {
                    String::new()
                };
                line(t, &k.command.desc, about, ex, key)
            }
        };
        body = body.child(
            float_row(t, here, content).on_press_tx(move |a: &mut App, tx| {
                choose(a, i);
                tx.instant();
            }),
        );
    }
    let hint = |seqs: &[&str], word: &str| float_hint(t, scope, seqs, word);
    let mut hints = match (f.capture.as_ref(), rows.get(f.cursor)) {
        (Some(_), _) => vec![hint(&["<CR>"], "keep"), hint(&["<Esc>"], "cancel")],
        (None, Some(Row::Field(_))) => {
            vec![
                hint(&["<C-h>", "<C-l>"], "change"),
                hint(&["<Esc>"], "back"),
            ]
        }
        (None, row) => vec![
            float_hint_if(t, scope, &["<CR>"], "new key", row.is_some()),
            float_hint_if(t, scope, &["<C-x>"], "default", row.is_some()),
            hint(&["<Esc>"], "back"),
        ],
    };
    if rows.is_empty() {
        body = body.child(no_match(t, SETTINGS.none(&f.query.text)));
    }
    if f.capture.as_ref().is_some_and(|k| k.len() == MOST) {
        hints.insert(
            0,
            text(format!("{MOST} keys at most"))
                .font(t.ty.ui_small)
                .color(t.c.fg.muted),
        );
    }
    // While a row takes a new key the filter has no caret: the keys go to the row.
    let mut query = f.query.clone();
    query.focused = f.capture.is_none();
    shell(
        t,
        Shell {
            tag: None,
            title: title(t, "Settings"),
            subtitle: None,
            meta: Some(FILE.to_string()),
            body,
            input: Some(input(
                t,
                Input {
                    edit: &query,
                    label: "Filter settings",
                    placeholder: FILTER,
                    count: SETTINGS.kept(&f.query.text, rows.len(), self::rows(s.keymap, "").len()),
                },
            )),
            hints,
        },
    )
}

/// One row as the palette lays it out: the label with `about` after it, the `:` name, the key.
fn line(t: Theme, label: &str, about: El, ex: String, key: El) -> El {
    row()
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
                    text(label.to_string())
                        .font(t.ty.ui)
                        .color(t.c.fg.default)
                        .truncate()
                        .shrink()
                        .min_w(0.0),
                )
                .child(about.truncate().shrink().min_w(0.0)),
        )
        .child(text(ex).font(t.ty.mono_small).color(t.c.fg.alt).w(160.0))
        .child(row().center_y().w(120.0).child(key))
}
