//! Find, Filter, and list search share the same plain rows and input layout.
use super::{Input, input, no_match};
use crate::{
    content,
    theme::Theme,
    ui::{self, Scope, Shell},
};
use rune::{El, col};

/// What a list picker shows and says.
pub struct List<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub rows: &'a [String],
    pub cursor: usize,
    pub input: Input<'a>,
    /// The body's one line when the query keeps no row: `Nothing matches "zzzz"`.
    pub none: String,
}

pub fn view(t: Theme, scope: Scope, l: List) -> El {
    let mut body = col().w_full();
    for (i, label) in l.rows.iter().enumerate() {
        body = body.child(ui::float_row(
            t,
            i == l.cursor,
            content::line(t, label, t.ty.ui, t.c.fg.default),
        ));
    }
    if l.rows.is_empty() && !l.input.edit.text.trim().is_empty() {
        body = body.child(no_match(t, l.none));
    }
    ui::shell(
        t,
        Shell {
            tag: None,
            title: ui::title(t, l.title),
            subtitle: Some(l.subtitle.into()),
            meta: None,
            body,
            input: Some(input(t, l.input)),
            hints: vec![
                ui::float_hint_if(t, scope, &["<CR>"], "choose", !l.rows.is_empty()),
                ui::float_hint(t, scope, &["<Esc>"], "back"),
            ],
        },
    )
}
