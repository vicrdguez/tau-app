//! The sidebar (`spec.md` §4.1): the brand and the server line, `+ New` and `Find`, the places,
//! the open items by project, and `Server` at the bottom.

use rune::{El, Icon, ScrollSpec, TextCase, Tx, col, icon, row, text};

use super::{Place, ServerPhase, TroubleMark};
use crate::assets::{brand, carbon, marks};
use crate::open::Item;
use crate::theme::Theme;
use crate::ui::{Counts, Mark, Scope, Tag, chip, count_tag, mark, with_keys};

/// A row of the sidebar, for its cursor and the current row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    New,
    Find,
    Place(Place),
    Item(Item),
    Server,
    /// A count tag's click target, not a cursor row.
    Count(Tag),
}

/// One open item as the sidebar draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenRow {
    pub item: Item,
    /// A conversation's label or a decision's title; `None` is `empty conversation`.
    pub label: Option<String>,
    /// Its state mark at the right; none before anything is received.
    pub mark: Option<Mark>,
    /// The server no longer has it: its label in `fg.muted`.
    pub gone: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OpenGroup {
    pub project: String,
    pub items: Vec<OpenRow>,
}

pub struct Sidebar<'a> {
    pub server: &'a ServerPhase,
    pub counts: Counts,
    /// What the stage shows: its row has the neutral `surface.current` fill.
    pub current: Option<Row>,
    /// The sidebar's cursor while it has the keys: the 2 px coral bar.
    pub cursor: Option<Row>,
    pub open: &'a [OpenGroup],
    /// Who has the keys, for the two chips.
    pub scope: Scope,
}

/// What a click on a row does; the app gives it the row.
pub type OnRow<S> = fn(&mut S, &Row, &mut Tx);

pub(super) fn tags<S: 'static>(t: Theme, counts: Counts, on_row: OnRow<S>) -> El {
    let mut tags = row().center_y().gap(6.0);
    for (tag, n) in [
        (Tag::Needs, counts.needs),
        (Tag::Failed, counts.failed),
        (Tag::Unknown, counts.unknown),
    ] {
        if let Some(el) = n.and_then(|n| count_tag(t, tag, n, false)) {
            tags = tags.child(el.on_press_tx(move |s: &mut S, tx| on_row(s, &Row::Count(tag), tx)));
        }
    }
    tags
}

/// The rune key of a sidebar row, for hint mode's labels.
pub fn row_key(r: &Row) -> String {
    format!("side-{r:?}")
}

pub fn sidebar<S: 'static>(t: Theme, s: &Sidebar, on_row: OnRow<S>) -> El {
    let l = t.l.sidebar;
    let line = |r: Row, lead: El, body: El, right: Option<El>| -> El {
        let current = s.current.as_ref() == Some(&r);
        let cursor = s.cursor.as_ref() == Some(&r);
        let pressed = r.clone();
        let mut el = row()
            .key(row_key(&r))
            .on_press_tx(move |st: &mut S, tx: &mut Tx| on_row(st, &pressed, tx))
            .center_y()
            .h(l.row)
            .px(l.row_padding_x)
            .gap(10.0)
            .w_full()
            .child(lead)
            .child(body.grow(1.0).shrink().min_w(0.0));
        if let Some(right) = right {
            el = el.child(right);
        }
        if current || cursor {
            el = el.fill(t.c.surface.current);
        }
        if cursor
            && matches!(
                s.scope.ctx,
                crate::keys::Ctx::Sidebar | crate::keys::Ctx::Peek
            )
        {
            el = el.border_sides([0.0, 0.0, 0.0, 2.0], t.c.state.attn);
        }
        el
    };
    let is_on = |r: &Row| s.current.as_ref() == Some(r) || s.cursor.as_ref() == Some(r);
    let row_icon = |g: &'static Icon, on: bool| {
        icon(g, l.row_icon).color(if on { t.c.fg.alt } else { t.c.fg.muted })
    };
    let row_text = |words: &str, on: bool| {
        text(words.to_string())
            .font(t.ty.ui)
            .color(if on { t.c.fg.default } else { t.c.fg.alt })
            .truncate()
    };
    let place = |r: Row, g: &'static Icon, words: &str, right: Option<El>| {
        let on = is_on(&r);
        line(r, row_icon(g, on), row_text(words, on), right)
    };

    let brand = row()
        .mt(l.brand_top)
        .pl(8.0)
        .pr(4.0)
        .gap(9.0)
        .center_y()
        .child(icon(&brand::AVATAR, l.avatar))
        .child(
            col()
                .shrink()
                .min_w(0.0)
                .child(text("tau").font(t.ty.sidebar_brand).color(t.c.fg.default))
                .child(
                    server_line(t, s.server)
                        .mt(1.0)
                        .on_press_tx(move |st: &mut S, tx| on_row(st, &Row::Server, tx)),
                ),
        );

    let action = |r: Row, g: &'static Icon, words: &str, seq: &str| {
        let key = t.keymap.chip(seq, s.scope.mode, s.scope.ctx);
        place(r, g, words, Some(chip(t, s.scope, seq, false))).label(with_keys(words, &[&key]))
    };
    let actions = col()
        .mt(18.0)
        .child(action(Row::New, &carbon::ADD, "New", "<leader>cn"))
        .child(action(Row::Find, &carbon::SEARCH, "Find", "<leader>."));

    let places = col()
        .mt(12.0)
        .child(place(
            Row::Place(Place::Home),
            &carbon::HOME,
            "Home",
            Some(tags(t, s.counts, on_row)),
        ))
        .child(place(
            Row::Place(Place::Conversations),
            &carbon::CHAT,
            "Conversations",
            None,
        ))
        .child(place(
            Row::Place(Place::Timeline),
            &carbon::TIME,
            "Timeline",
            None,
        ))
        .child(place(
            Row::Place(Place::Projects),
            &carbon::FOLDER,
            "Projects",
            None,
        ));

    let mut column = col()
        .w_full()
        .h_full()
        .px(l.padding_x)
        .fill(t.c.surface.side)
        .child(brand)
        .child(actions)
        .child(places);

    if !s.open.is_empty() {
        let mut items = col().w_full();
        for g in s.open {
            items = items.child(
                row()
                    .center_y()
                    .h(l.group_label)
                    .px(l.row_padding_x)
                    .mt(10.0)
                    .child(
                        text(g.project.clone())
                            .font(t.ty.sidebar_group)
                            .color(t.c.kind.worker)
                            .truncate(),
                    ),
            );
            for o in &g.items {
                items = items.child(open_row(t, o, is_on(&Row::Item(o.item.clone())), &line));
            }
        }
        column = column
            .child(
                row().mt(l.section_label_above).px(l.row_padding_x).child(
                    text("Open")
                        .font(t.ty.sidebar_section)
                        .color(t.c.fg.muted)
                        .text_case(TextCase::Uppercase),
                ),
            )
            .child(
                items
                    .scroll(ScrollSpec {
                        stick_bottom: false,
                    })
                    .grow(1.0)
                    .min_h(0.0),
            );
    } else {
        column = column.child(rune::spacer());
    }
    column.child(
        place(Row::Server, &carbon::SETTINGS, "Server", None)
            .mt(4.0)
            .mb(10.0),
    )
}

fn open_row(t: Theme, o: &OpenRow, on: bool, line: &dyn Fn(Row, El, El, Option<El>) -> El) -> El {
    let l = t.l.sidebar;
    let fg = if o.gone {
        t.c.fg.muted
    } else if on {
        t.c.fg.default
    } else {
        t.c.fg.alt
    };
    let icon_color = if on { t.c.fg.alt } else { t.c.fg.muted };
    let id = |s: String| text(s).font(t.ty.mono_small).color(t.c.kind.literal);
    let words = |s: &str, c| {
        text(s.to_string())
            .font(t.ty.ui)
            .color(c)
            .truncate()
            .shrink()
            .min_w(0.0)
    };
    let (glyph, body): (&'static Icon, El) = match &o.item {
        Item::Conversation { id: n, .. } => {
            let label = match &o.label {
                Some(label) => words(label, fg),
                None => words("empty conversation", t.c.fg.muted),
            };
            (
                &carbon::CHAT,
                row()
                    .center_y()
                    .gap(6.0)
                    .min_w(0.0)
                    .child(id(format!("#{n}")))
                    .child(label),
            )
        }
        Item::Decision { id: did, .. } => {
            let n = did
                .rsplit_once('#')
                .map_or(String::new(), |(_, n)| format!("#{n}"));
            let title = o.label.clone().unwrap_or_else(|| did.clone());
            (
                &marks::DIAMOND_OUTLINE,
                row()
                    .center_y()
                    .gap(6.0)
                    .min_w(0.0)
                    .child(words(&title, fg))
                    .child(id(n)),
            )
        }
        Item::Project(p) => (&carbon::FOLDER, words(p, fg)),
    };
    let mut right = row().w(14.0);
    if let Some(m) = o.mark {
        right = right.child(mark(t, m));
    }
    line(
        Row::Item(o.item.clone()),
        icon(glyph, l.item_icon).color(icon_color),
        body,
        Some(right),
    )
    .gap(8.0)
}

/// The server line: calm in `fg.muted` (`laptop → host ⌄`), or `fg` with its mark.
fn server_line(t: Theme, phase: &ServerPhase) -> El {
    let style = t.ty.sidebar_server;
    match phase {
        ServerPhase::NotConnected => text("not connected").font(style).color(t.c.fg.muted),
        ServerPhase::Calm { device, host } => row()
            .center_y()
            .gap(2.0)
            .child(
                text(format!("{device} → {host}"))
                    .font(style)
                    .color(t.c.fg.muted)
                    .truncate()
                    .shrink()
                    .min_w(0.0),
            )
            .child(icon(&carbon::CHEVRON_DOWN, 12.0).color(t.c.fg.muted)),
        ServerPhase::Trouble { mark: m, words } => row()
            .center_y()
            .gap(5.0)
            .child(trouble_mark(t, *m))
            .child(text(words.clone()).font(style).color(t.c.fg.default)),
    }
}

/// The mark of a server state that isn't calm.
pub fn trouble_mark(t: Theme, m: TroubleMark) -> El {
    match m {
        TroubleMark::Ring => icon(&marks::RING, 12.0).color(t.c.state.stale),
        TroubleMark::Time => icon(&carbon::TIME, 12.0).color(t.c.state.stale),
        TroubleMark::Failed => mark(t, Mark::Failed),
    }
}
