//! Find, Filter, and list search share the same rows and input layout.
use super::{Input, input, no_match};
use crate::{
    assets::{carbon, marks},
    content,
    theme::Theme,
    ui::{self, Scope, Shell},
};
use rune::{El, Icon, col, icon};

/// What lives in a project, by the icon the sidebar gives its kind (`visual-language.md` §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Conversation,
    Decision,
    Event,
}

/// One row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// Words alone: Filter's and list search's rows.
    Plain(String),
    /// A project: its folder and its name in sage.
    Project(String),
    /// Its kind's icon, its label, and its project in sage in a slot at the right, so the
    /// projects line up down the list.
    In {
        kind: Kind,
        label: String,
        project: String,
    },
}

/// What a list picker shows and says.
pub struct List<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub rows: &'a [Line],
    pub cursor: usize,
    pub input: Input<'a>,
    /// The body's one line when the query keeps no row: `Nothing matches "zzzz"`.
    pub none: String,
}

/// The project slot's width: a name of about 20 characters before it is cut.
const PROJECT: f32 = 160.0;

pub fn view(t: Theme, scope: Scope, l: List) -> El {
    let mut body = col().w_full();
    for (i, row) in l.rows.iter().enumerate() {
        body = body.child(ui::float_row(t, i == l.cursor, line(t, row)));
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

fn line(t: Theme, row: &Line) -> El {
    let lead = |glyph: &'static Icon| Some(icon(glyph, 14.0).color(t.c.fg.muted));
    match row {
        Line::Plain(label) => content::line(t, label, t.ty.ui, t.c.fg.default),
        Line::Project(name) => ui::slot_row(
            lead(&carbon::FOLDER),
            content::line(t, name, t.ty.ui, t.c.kind.worker),
            Vec::new(),
            12.0,
        ),
        Line::In {
            kind,
            label,
            project,
        } => ui::slot_row(
            lead(match kind {
                Kind::Conversation => &carbon::CHAT,
                Kind::Decision => &marks::DIAMOND_OUTLINE,
                Kind::Event => &carbon::TIME,
            }),
            content::line(t, label, t.ty.ui, t.c.fg.default),
            vec![content::line(t, project, t.ty.ui, t.c.kind.worker).w(PROJECT)],
            12.0,
        ),
    }
}
