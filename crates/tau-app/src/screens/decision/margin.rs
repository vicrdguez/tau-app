//! The notes margin (`spec.md` §5.6 "The notes margin"): its head, the note on the whole, the
//! earlier requests, then each passage's note beside its passage, stacked 12 px apart; a note
//! pushed below its passage gets a leader. Under 1256 px the same items fold into the column.

use rune::{El, col, el, icon, row, text, text_input};

use super::state::{Anchor, Editing};
use crate::app::{App, input};
use crate::assets::carbon;
use crate::theme::Theme;
use crate::ui::{Scope, container, hint};

/// Notes stack this far apart.
pub const GAP: f32 = 12.0;
/// A note pushed further than this below its passage draws a leader.
const LEADER_AFTER: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    pub y: f32,
    pub leader: bool,
}

/// Places notes that want to sit at `want` (their passages' first lines, in order down the page),
/// `heights` tall, never above `floor` and never overlapping.
pub fn place(want: &[f32], heights: &[f32], floor: f32) -> Vec<Placed> {
    let mut next = floor;
    want.iter()
        .zip(heights)
        .map(|(&w, &h)| {
            let y = w.max(next);
            next = y + h + GAP;
            Placed {
                y,
                leader: y - w > LEADER_AFTER,
            }
        })
        .collect()
}

/// `Your notes · 3 · not sent`, or after an answer `Notes · 3 · sent by phone 13:30`.
pub fn head(t: Theme, mine: bool, words: &[String]) -> El {
    let mut r = row().center_y().h(16.0);
    if mine {
        r = r.child(icon(&carbon::EDIT, 12.0).color(t.c.state.local).mr(7.0));
    }
    let name = if mine { "Your notes" } else { "Notes" };
    r = r.child(
        text(name)
            .font(rune::TextStyle {
                weight: 600.0,
                ..t.ty.ui_small
            })
            .color(t.c.fg.alt),
    );
    for w in words {
        r = r
            .child(text("·").font(t.ty.ui_small).color(t.c.fg.muted).mx(6.0))
            .child(text(w.clone()).font(t.ty.ui_small).color(t.c.fg.muted));
    }
    r
}

/// The note on the whole: `on the whole` over its text.
pub fn whole(t: Theme, body: &str) -> El {
    col()
        .w_full()
        .child(text("on the whole").font(t.ty.ui_small).color(t.c.fg.muted))
        .child(
            text(body.to_string())
                .font(t.ty.ui)
                .color(t.c.fg.default)
                .wrap_anywhere()
                .mt(4.0),
        )
}

/// A note beside its passage: its head (`edit`, `not sent`) over its text.
pub fn note(t: Theme, state: &str, mine: bool, body: &str) -> El {
    let mut head = row().center_y().gap(7.0).h(16.0);
    if mine {
        head = head.child(icon(&carbon::EDIT, 12.0).color(t.c.state.local));
    }
    col()
        .w_full()
        .child(
            head.child(
                text(state.to_string())
                    .font(t.ty.ui_small)
                    .color(t.c.fg.muted),
            ),
        )
        .child(
            text(body.to_string())
                .font(t.ty.ui)
                .color(t.c.fg.default)
                .wrap_anywhere()
                .mt(3.0),
        )
}

/// The note being typed: the container with a coral edge while it has the caret in Insert, its
/// head, the text with the caret, and `Esc jk leave Insert` inside at the bottom right.
pub fn editing(t: Theme, scope: Scope, e: &Editing, keys: bool) -> El {
    let head = match &e.anchor {
        Anchor::Whole => "note on the whole",
        Anchor::Range { widened: true, .. } => "note on the whole paragraph",
        Anchor::Range { .. } => "note on the selection",
    };
    let mut edit = e.editor.edit.clone();
    edit.focused &= keys;
    let field = text_input(&edit, t.ty.ui)
        .wrap_anywhere()
        .caret(t.c.state.attn, t.c.surface.selection)
        .color(t.c.fg.default)
        .on_edit(|a: &mut App| a.screens.decision.edit_mut())
        .on_input_key(input::on_field_key)
        .label("Note")
        .key("decision-note");
    let inserting = keys && e.inserting();
    let foot = row()
        .center_y()
        .gap(6.0)
        .mt(8.0)
        .child(rune::spacer())
        .child(hint(t, scope, "<Esc>"))
        .child(hint(t, scope, "jk"))
        .child(text("leave Insert").font(t.ty.ui_small).color(t.c.fg.muted));
    container(
        t,
        inserting,
        col()
            .w_full()
            .child(
                row()
                    .center_y()
                    .gap(7.0)
                    .h(16.0)
                    .child(icon(&carbon::EDIT, 12.0).color(t.c.state.local))
                    .child(
                        text(format!("{head} · not sent"))
                            .font(t.ty.ui_small)
                            .color(t.c.fg.muted),
                    ),
            )
            .child(field.w_full().mt(3.0))
            .child(foot),
    )
    .px(14.0)
    .py(10.0)
}

/// A leader from the column's edge at the passage's first line, across the gap and down to the
/// note's head: three 1 px lines in `rule.note_leader`.
pub fn leader(t: Theme, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<El> {
    let c = t.c.rule.note_leader;
    let xm = x0 + 10.0;
    vec![
        el().absolute(x0, y0).size(xm - x0 + 1.0, 1.0).fill(c),
        el().absolute(xm, y0)
            .size(1.0, (y1 - y0).max(0.0) + 1.0)
            .fill(c),
        el().absolute(xm, y1).size(x1 - xm, 1.0).fill(c),
    ]
}
