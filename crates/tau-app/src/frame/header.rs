//! The stage header (`spec.md` §4.2): the sidebar toggle and the breadcrumb at the left, the view's
//! action segments at the right.

use rune::{El, Tx, icon, row, text};

use crate::assets::{carbon, marks};
use crate::theme::Theme;
use crate::ui::{Scope, chip, glyphs};

/// One part of the breadcrumb.
#[derive(Clone, Debug)]
pub enum Crumb {
    Text(String),
    /// A project's name, in sage.
    Project(String),
    /// `#4` or a decision ID, in orange mono.
    Id(String),
    /// A quiet pressable `show all` after a place's filter, that resets it.
    ShowAll(fn(&mut crate::app::App)),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentKind {
    /// Quiet `ui` in `fg.alt` with a key chip: `␣cx stop`.
    Quiet,
    /// `◆ ␣ad Answer`: solid coral only while `␣ad` would open the readback, else a muted outline.
    Answer,
    /// `<verb> · off`, with a dashed edge and no chip.
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub seq: &'static str,
    pub word: String,
    pub kind: SegmentKind,
}

pub struct Header {
    /// The sidebar is drawn beside the stage (the toggle reads `side-panel--close`).
    pub sidebar_shown: bool,
    pub crumbs: Vec<Crumb>,
    pub segments: Vec<Segment>,
    pub scope: Scope,
    /// No float or which-key is up: the coral Answer may be solid (`spec.md` §4.6).
    pub answer_solid: bool,
}

/// `on_toggle` runs when the sidebar toggle is clicked.
pub fn header<S: 'static>(t: Theme, h: &Header, on_toggle: fn(&mut S, &mut Tx)) -> El {
    let l = t.l.stage_header;
    let left = if h.sidebar_shown {
        l.padding_x[0]
    } else {
        l.left_when_sidebar_hidden
    };
    let toggle = if h.sidebar_shown {
        &carbon::SIDE_PANEL_CLOSE
    } else {
        &carbon::SIDE_PANEL_OPEN
    };
    let mut crumbs = row().center_y().gap(7.0).shrink().min_w(0.0);
    for (i, c) in h.crumbs.iter().enumerate() {
        if let Crumb::ShowAll(run) = *c {
            crumbs = crumbs.child(
                text("show all")
                    .font(t.ty.ui_small)
                    .color(t.c.fg.muted)
                    .ml(12.0 - 7.0)
                    .on_press(move |a: &mut crate::app::App| {
                        if a.floats.is_empty() {
                            run(a);
                        }
                    }),
            );
            continue;
        }
        if i > 0 {
            crumbs = crumbs.child(text("›").font(t.ty.ui).color(t.c.fg.muted));
        }
        crumbs = crumbs.child(match c {
            Crumb::Text(s) => text(s.clone()).font(t.ty.ui).color(t.c.fg.default),
            Crumb::Project(s) => text(s.clone()).font(t.ty.ui).color(t.c.kind.worker),
            Crumb::Id(s) => text(s.clone())
                .font(t.ty.mono_small)
                .color(t.c.kind.literal),
            Crumb::ShowAll(_) => unreachable!("drawn above"),
        });
    }
    let mut segments = row().center_y().gap(t.l.gaps.segment);
    for s in &h.segments {
        segments = segments.child(segment(t, h.scope, h.answer_solid, s));
    }
    row()
        .center_y()
        .gap(10.0)
        .pl(left)
        .pr(l.padding_x[1])
        .fill(t.c.surface.read)
        .child(
            icon(toggle, 16.0)
                .color(t.c.fg.muted)
                .on_press_tx(move |st: &mut S, tx: &mut Tx| on_toggle(st, tx)),
        )
        .child(crumbs)
        .child(rune::spacer())
        .child(segments)
}

fn segment(t: Theme, scope: Scope, answer_solid: bool, s: &Segment) -> El {
    let key = t.keymap.chip(s.seq, scope.mode, scope.ctx);
    let lit = key.lit;
    let pill = t.l.radius.pill;
    match s.kind {
        SegmentKind::Off => row()
            .center_y()
            .gap(6.0)
            .h(t.l.height.answer_segment)
            .px(10.0)
            .border(1.0, t.c.border.hairline)
            .border_dash(3.0, 2.0)
            .radius(pill)
            .child(
                text(format!("{} · off", s.word))
                    .font(t.ty.ui)
                    .color(t.c.fg.muted),
            ),
        SegmentKind::Answer if lit && answer_solid => {
            let knock = t.c.surface.read;
            row()
                .center_y()
                .gap(7.0)
                .h(t.l.height.answer_segment)
                .pl(9.0)
                .pr(10.0)
                .radius(pill)
                .fill(t.c.state.attn)
                .child(icon(&marks::DIAMOND, 11.0).color(knock))
                .child(
                    glyphs(&key.printed, t.ty.key, knock)
                        .h(18.0)
                        .px(5.0)
                        .radius(pill)
                        .fill(t.c.surface.read.alpha(0.16)),
                )
                .child(text(s.word.clone()).font(t.ty.ui_strong).color(knock))
        }
        SegmentKind::Answer => row()
            .center_y()
            .gap(7.0)
            .h(t.l.height.answer_segment)
            .pl(9.0)
            .pr(10.0)
            .radius(pill)
            .border(1.0, t.c.border.hairline)
            .child(icon(&marks::DIAMOND, 11.0).color(t.c.fg.muted))
            .child(chip(t, scope, s.seq, false))
            .child(text(s.word.clone()).font(t.ty.ui).color(t.c.fg.muted)),
        SegmentKind::Quiet => row()
            .center_y()
            .gap(7.0)
            .child(chip(t, scope, s.seq, false))
            .child(text(s.word.clone()).font(t.ty.ui).color(if lit {
                t.c.fg.alt
            } else {
                t.c.fg.muted
            })),
    }
}
