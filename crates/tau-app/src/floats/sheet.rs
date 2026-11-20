//! A picker over rows made when it opened: `:jumps` and the keys cheat sheet (`␣hk`). Typing
//! filters the rows; `C-j C-k` move, `⏎` chooses (the jumps go there), `Esc` closes.

use rune::{El, TextEdit, col, row, text, text_input};

use crate::app::App;
use crate::keys::{Ctx, Keymap, Mode, Target, display};
use crate::theme::Theme;
use crate::ui::{self, Scope, Shell, glyphs};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetRow {
    /// Printed keys (`␣hk`), or empty.
    pub keys: String,
    pub text: String,
    /// Muted, at the right.
    pub meta: String,
}

pub struct Sheet {
    pub title: &'static str,
    pub subtitle: String,
    pub rows: Vec<SheetRow>,
    pub query: TextEdit,
    /// Among the rows the query keeps.
    pub cursor: usize,
    /// What `⏎` does with the chosen row's index in `rows`; `None` only closes.
    pub choose: Option<fn(&mut App, usize)>,
}

impl Sheet {
    pub fn new(title: &'static str, subtitle: String, rows: Vec<SheetRow>) -> Sheet {
        let mut query = TextEdit::new("");
        query.focused = true;
        Sheet {
            title,
            subtitle,
            rows,
            query,
            cursor: 0,
            choose: None,
        }
    }

    /// `␣hk`: the keys cheat sheet, at the keys this device types.
    pub fn keys(km: Keymap) -> Sheet {
        Sheet::new("Keys", "every key in keymap.toml".into(), keys_rows(km))
    }

    /// The indices of the rows the query keeps, in order.
    pub fn shown(&self) -> Vec<usize> {
        let q = self.query.text.trim().to_lowercase();
        (0..self.rows.len())
            .filter(|&i| {
                let r = &self.rows[i];
                q.is_empty()
                    || [&r.keys, &r.text, &r.meta]
                        .iter()
                        .any(|s| s.to_lowercase().contains(&q))
            })
            .collect()
    }
}

/// The cheat sheet's rows, generated from `keymap.toml` (`grammar.md` §12 KL10): each command's
/// keys in each context it binds them, contexts in the table's tree order, commands in its order.
pub fn keys_rows(km: Keymap) -> Vec<SheetRow> {
    let mut rows = Vec::new();
    for ctx in Ctx::ALL {
        for (i, c) in km.commands().iter().enumerate() {
            let mut modes: Vec<(Mode, Vec<String>)> = Vec::new();
            for b in km
                .bindings()
                .filter(|b| b.ctx == ctx && b.target == Target::Command(i))
            {
                let shown = display(b.seq);
                match modes.iter_mut().find(|(m, _)| *m == b.mode) {
                    Some((_, keys)) if !keys.contains(&shown) => keys.push(shown),
                    Some(_) => {}
                    None => modes.push((b.mode, vec![shown])),
                }
            }
            // One row per set of keys, naming its modes unless it is Normal's alone.
            let mut merged: Vec<(Vec<Mode>, Vec<String>)> = Vec::new();
            for (mode, keys) in modes {
                match merged.iter_mut().find(|(_, k)| *k == keys) {
                    Some((ms, _)) => ms.push(mode),
                    None => merged.push((vec![mode], keys)),
                }
            }
            for (modes, keys) in merged {
                let meta = if modes == [Mode::Normal] {
                    ctx.name().to_string()
                } else {
                    let words: Vec<&str> = modes.into_iter().map(mode_word).collect();
                    format!("{} · {}", ctx.name(), words.join(", "))
                };
                rows.push(SheetRow {
                    keys: keys.join(" "),
                    text: c.desc.clone(),
                    meta,
                });
            }
        }
    }
    rows
}

fn mode_word(mode: Mode) -> &'static str {
    match mode {
        Mode::Normal => "normal",
        Mode::Visual => "visual",
        Mode::Insert => "insert",
        Mode::Command => "picker",
        Mode::Hint => "hint",
        Mode::Readback => "readback",
        Mode::Peek => "peek",
        Mode::Outbox => "outbox",
        Mode::Server => "server",
    }
}

/// How many rows the body shows around the cursor.
const WINDOW: usize = 10;

pub fn view(t: Theme, scope: Scope, s: &Sheet) -> El {
    let shown = s.shown();
    let start = s
        .cursor
        .saturating_sub(WINDOW / 2)
        .min(shown.len().saturating_sub(WINDOW));
    let mut body = col().w_full();
    for (at, &i) in shown.iter().enumerate().skip(start).take(WINDOW) {
        let r = &s.rows[i];
        let mut content = row().center_y().gap(12.0);
        if !s.rows.iter().all(|r| r.keys.is_empty()) {
            content = content.child(
                glyphs(&r.keys, t.ty.mono_small, t.c.kind.literal)
                    .w(120.0)
                    .min_w(120.0),
            );
        }
        content = content
            .child(
                text(r.text.clone())
                    .font(t.ty.ui)
                    .color(t.c.fg.default)
                    .truncate()
                    .grow(1.0)
                    .shrink()
                    .min_w(0.0),
            )
            .child(
                text(r.meta.clone())
                    .font(t.ty.ui_small)
                    .color(t.c.fg.muted)
                    .truncate(),
            );
        body = body.child(ui::float_row(t, at == s.cursor, content));
    }
    let input = row()
        .center_y()
        .gap(10.0)
        .child(
            text_input(&s.query, t.ty.float_input)
                .single_line()
                .caret(t.c.state.attn, t.c.surface.selection)
                .color(t.c.fg.default)
                .on_edit(|a: &mut App| a.float_edit())
                .on_input_key(crate::app::input::on_field_key)
                .label(format!("{} query", s.title))
                .key("sheet-input")
                .min_w(2.0)
                .shrink(),
        )
        .child(
            text(format!("{} of {}", shown.len(), s.rows.len()))
                .font(t.ty.ui_small)
                .color(t.c.fg.muted),
        );
    let mut hints = vec![ui::float_hint(t, scope, &["<C-j>", "<C-k>"], "move")];
    if s.choose.is_some() {
        hints.push(ui::float_hint(t, scope, &["<CR>"], "goes there"));
    }
    hints.push(ui::float_hint(t, scope, &["<Esc>"], "back"));
    ui::shell(
        t,
        Shell {
            tag: None,
            title: ui::title(t, s.title),
            subtitle: Some(s.subtitle.clone()),
            meta: None,
            body,
            input: Some(input),
            hints,
        },
    )
}

#[cfg(test)]
#[path = "sheet_tests.rs"]
mod tests;
