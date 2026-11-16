//! A picker of plain choices in the float shell (`spec.md` §6): the model and thinking pickers.
//! Rows are a label, an optional detail in `mono.small`, and `current` at the right; the input
//! filters them and the cursor is the one coral selection.

use rune::{El, TextEdit, row, text};

use crate::theme::Theme;
use crate::ui::{Scope, Shell, float_hint, float_row, shell, title};

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
    /// The input's count word: `5 models`.
    pub count: String,
    /// Stands in for the rows while there are none (`Nothing received from the server yet`).
    pub empty: Option<&'a str>,
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
    if c.rows.is_empty()
        && let Some(words) = c.empty
    {
        body = body.child(float_row(
            t,
            false,
            text(words).font(t.ty.ui).color(t.c.fg.alt),
        ));
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
    shell(
        t,
        Shell {
            tag: None,
            title: head,
            subtitle: None,
            meta: c.meta,
            body,
            input: Some(super::input(t, c.query, None, c.count)),
            hints: vec![
                float_hint(t, scope, &["<CR>"], "choose"),
                float_hint(t, scope, &["<Esc>"], "back"),
            ],
        },
    )
}
