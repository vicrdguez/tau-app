//! Small state marks and count tags (`visual-language.md` §6, `spec.md` §4.4). Each mark has its
//! own silhouette, so colour is never the only cue.

use rune::{Color, El, Icon, icon, row, text};

use crate::assets::{carbon, marks};
use crate::theme::Theme;

/// A state mark at a row's edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// An open decision: the coral filled diamond.
    Decision,
    /// A failed run, a faulted task: the rose heavy ×.
    Failed,
    /// A running conversation: the steel triangle.
    Running,
    /// An unknown outcome: the mauve outline `?`.
    Unknown,
    /// It passed: the olive checkmark.
    Ok,
}

impl Mark {
    fn glyph(self) -> (&'static Icon, f32) {
        match self {
            Mark::Decision => (&marks::DIAMOND, 9.0),
            Mark::Failed => (&marks::FAILED, 8.0),
            Mark::Running => (&marks::RUNNING, 8.0),
            Mark::Unknown => (&carbon::HELP, 12.0),
            Mark::Ok => (&carbon::CHECKMARK, 12.0),
        }
    }

    pub fn color(self, t: Theme) -> Color {
        match self {
            Mark::Decision => t.c.state.attn,
            Mark::Failed => t.c.state.blocked,
            Mark::Running => t.c.state.stale,
            Mark::Unknown => t.c.state.unknown,
            Mark::Ok => t.c.state.ok,
        }
    }
}

/// A mark centred in its 14 px slot.
pub fn mark(t: Theme, m: Mark) -> El {
    mark_in(m, m.color(t))
}

/// A mark in another colour (knocked out on a coral head).
pub fn mark_in(m: Mark, color: Color) -> El {
    let (g, size) = m.glyph();
    let slot = crate::theme::tokens().marks.slot;
    row()
        .size(slot, slot)
        .center()
        .child(icon(g, size).color(color))
}

/// What a count tag counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    /// Open decisions: `◆` coral.
    Needs,
    /// Failed runs and faulted tasks: `×` rose.
    Failed,
    /// Writes with an unknown outcome: `?` mauve.
    Unknown,
}

/// One count tag; `title` is the 18 px tag on Home's title. A zero count draws no tag.
pub fn count_tag(t: Theme, tag: Tag, n: u32, title: bool) -> Option<El> {
    if n == 0 {
        return None;
    }
    let ct = t.l.count_tag;
    let (fill, glyph, size) = match tag {
        Tag::Needs => (t.c.state.attn, &*marks::DIAMOND, ct.mark),
        Tag::Failed => (t.c.state.blocked, &*marks::FAILED, 8.0),
        Tag::Unknown => (t.c.state.unknown, &*carbon::HELP, ct.mark),
    };
    let knock = t.c.surface.read;
    Some(
        row()
            .center()
            .gap(4.0)
            .h(if title { ct.title_height } else { ct.height })
            .min_w(ct.min_width)
            .px(ct.padding_x)
            .radius(t.l.radius.pill)
            .fill(fill)
            .child(icon(glyph, size).color(knock))
            .child(text(n.to_string()).font(t.ty.key).color(knock)),
    )
}

/// The counts a view shows. `None` is "not received yet", which draws no tag (`spec.md` §4.8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub needs: Option<u32>,
    pub failed: Option<u32>,
    pub unknown: Option<u32>,
}

/// The tags for `counts`, 6 px apart, each only while above 0.
pub fn count_tags(t: Theme, counts: Counts) -> El {
    let mut out = row().center_y().gap(6.0);
    for (tag, n) in [
        (Tag::Needs, counts.needs),
        (Tag::Failed, counts.failed),
        (Tag::Unknown, counts.unknown),
    ] {
        if let Some(el) = n.and_then(|n| count_tag(t, tag, n, false)) {
            out = out.child(el);
        }
    }
    out
}
