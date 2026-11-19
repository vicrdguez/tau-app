//! A picker over rows made when it opened: `:jumps` and the keys cheat sheet (`␣hk`). Typing
//! filters the rows; `C-j C-k` move, `⏎` chooses (the jumps go there), `Esc` closes.

use rune::{El, TextEdit, col, row, text};

use super::{FILTER, Input, Noun, input, no_match};
use crate::app::App;
use crate::keys::{Ctx, Key, Keymap, Mode, Target, display};
use crate::theme::Theme;
use crate::ui::{self, Scope, Shell};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetRow {
    /// The keys of the row's command as this device types them; none for a jump.
    pub keys: Vec<&'static [Key]>,
    pub text: String,
    /// Muted, at the right.
    pub meta: String,
}

impl SheetRow {
    /// The keys as the row prints them, alternatives apart: `␣; ⌘K`.
    pub fn printed(&self) -> String {
        let printed: Vec<String> = self.keys.iter().map(|k| display(k)).collect();
        printed.join(" ")
    }
}

pub struct Sheet {
    pub title: &'static str,
    /// What a row is, for the count: `162 keys`.
    pub noun: Noun,
    pub subtitle: String,
    pub rows: Vec<SheetRow>,
    pub query: TextEdit,
    /// Among the rows the query keeps.
    pub cursor: usize,
    /// What `⏎` does with the chosen row's index in `rows`; `None` only closes.
    pub choose: Option<fn(&mut App, usize)>,
}

impl Sheet {
    pub fn new(title: &'static str, noun: Noun, subtitle: String, rows: Vec<SheetRow>) -> Sheet {
        let mut query = TextEdit::new("");
        query.focused = true;
        Sheet {
            title,
            noun,
            subtitle,
            rows,
            query,
            cursor: 0,
            choose: None,
        }
    }

    /// `␣hk`: the keys cheat sheet, at the keys this device types.
    pub fn keys(km: Keymap) -> Sheet {
        Sheet::new(
            "Keys",
            Noun::new("key", "keys"),
            "every key in keymap.toml".into(),
            keys_rows(km),
        )
    }

    /// The indices of the rows the query keeps, in order.
    pub fn shown(&self) -> Vec<usize> {
        let q = self.query.text.trim().to_lowercase();
        (0..self.rows.len())
            .filter(|&i| {
                let r = &self.rows[i];
                q.is_empty()
                    || [&r.printed(), &r.text, &r.meta]
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
            let mut modes: Vec<(Mode, Vec<&'static [Key]>)> = Vec::new();
            for b in km
                .bindings()
                .filter(|b| b.ctx == ctx && b.target == Target::Command(i))
            {
                match modes.iter_mut().find(|(m, _)| *m == b.mode) {
                    Some((_, keys)) if !keys.contains(&b.seq) => keys.push(b.seq),
                    Some(_) => {}
                    None => modes.push((b.mode, vec![b.seq])),
                }
            }
            // One row per set of keys, naming its modes unless it is Normal's alone.
            let mut merged: Vec<(Vec<Mode>, Vec<&'static [Key]>)> = Vec::new();
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
                    keys,
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

pub fn view(t: Theme, scope: Scope, s: &Sheet) -> El {
    let shown = s.shown();
    let mut body = col().w_full();
    for (at, &i) in shown.iter().enumerate() {
        let r = &s.rows[i];
        let mut content = row().center_y().gap(12.0);
        if !s.rows.iter().all(|r| r.keys.is_empty()) {
            content = content.child(
                ui::printed_keys(&r.keys, t.ty.mono_small, t.c.kind.literal)
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
    if shown.is_empty() && !s.rows.is_empty() {
        body = body.child(no_match(t, s.noun.none(&s.query.text)));
    }
    let label = format!("Filter {}", s.noun.many);
    let input = input(
        t,
        Input {
            edit: &s.query,
            label: &label,
            placeholder: FILTER,
            count: s.noun.kept(&s.query.text, shown.len(), s.rows.len()),
        },
    );
    let mut hints = vec![ui::float_hint(t, scope, &["<C-j>", "<C-k>"], "move")];
    if s.choose.is_some() {
        hints.push(ui::float_hint_if(
            t,
            scope,
            &["<CR>"],
            "goes there",
            !shown.is_empty(),
        ));
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
