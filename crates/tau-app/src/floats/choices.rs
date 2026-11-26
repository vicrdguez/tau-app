//! A picker of plain choices in the float shell (`spec.md` §6): the model and thinking pickers.
//! Rows are a label, an optional detail in `mono.small`, and `current` at the right; the input
//! filters them and the cursor is the one coral selection.

use rune::{El, TextEdit, row, text};

use super::{FILTER, Input, Noun, input, no_match};
use crate::theme::Theme;
use crate::ui::{Scope, Shell, float_hint, float_hint_if, float_row, shell, title};

/// One row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub label: String,
    /// A model's ID, in `mono.small` `fg.muted`.
    pub detail: Option<String>,
    pub current: bool,
}

/// What the float says around its rows.
pub struct Choices<'a> {
    pub title: &'a str,
    /// `for #5`: `#5` in orange mono.
    pub subject: Option<String>,
    /// `w1 may set the model again when it next sends work here`.
    pub meta: Option<String>,
    pub rows: &'a [Choice],
    pub cursor: usize,
    pub query: &'a TextEdit,
    /// What a row is: `5 models`.
    pub noun: Noun,
    /// The rows before the query: `1 of 5 models`.
    pub total: usize,
    /// Stands in for the rows while there are none (`Nothing received from the server yet`).
    pub empty: Option<&'a str>,
    pub refresh: Option<El>,
}

pub fn view(t: Theme, scope: Scope, c: Choices) -> El {
    let mut head = row().center_y().gap(10.0).child(title(t, c.title));
    if let Some(id) = c.subject {
        head = head.child(
            row()
                .center_y()
                .gap(5.0)
                .child(text("for").font(t.ty.ui_small).color(t.c.fg.muted))
                .child(text(id).font(t.ty.mono_small).color(t.c.kind.literal)),
        );
    }
    let mut body = rune::col().w_full();
    if c.rows.is_empty() {
        if let Some(words) = c.empty {
            body = body.child(float_row(
                t,
                false,
                text(words).font(t.ty.ui).color(t.c.fg.alt),
            ));
        } else if !c.query.text.trim().is_empty() {
            body = body.child(no_match(t, c.noun.none(&c.query.text)));
        }
    }
    for (i, choice) in c.rows.iter().enumerate() {
        let mut content = row().center_y().gap(12.0).child(
            text(choice.label.clone())
                .font(t.ty.ui)
                .color(t.c.fg.default)
                .truncate()
                .shrink()
                .min_w(0.0),
        );
        if let Some(d) = &choice.detail {
            content = content.child(
                text(d.clone())
                    .font(t.ty.mono_small)
                    .color(t.c.fg.muted)
                    .truncate()
                    .shrink()
                    .min_w(0.0),
            );
        }
        content = content.child(rune::spacer());
        if choice.current {
            content = content.child(text("current").font(t.ty.ui_small).color(t.c.fg.muted));
        }
        body = body.child(float_row(t, i == c.cursor, content));
    }
    let mut hints = Vec::new();
    hints.extend(c.refresh);
    hints.push(float_hint_if(
        t,
        scope,
        &["<CR>"],
        "choose",
        !c.rows.is_empty(),
    ));
    hints.push(float_hint(t, scope, &["<Esc>"], "back"));
    shell(
        t,
        Shell {
            tag: None,
            title: head,
            subtitle: None,
            meta: c.meta,
            body,
            input: Some(input(
                t,
                Input {
                    edit: c.query,
                    label: &format!("Filter {}", c.noun.many),
                    placeholder: FILTER,
                    count: c.noun.kept(&c.query.text, c.rows.len(), c.total),
                },
            )),
            hints,
        },
    )
}
