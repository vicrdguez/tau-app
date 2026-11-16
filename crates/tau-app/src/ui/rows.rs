//! Lines of chrome: a row with fixed right slots and a label cut with `…`, group heads, and facts
//! lines that drop whole parts when they don't fit.

use rune::{El, row, text};

use crate::theme::Theme;

/// A row: an optional lead (mark or icon), the label (it takes the rest and is cut with `…`),
/// then fixed slots at the right, `gap` apart.
pub fn slot_row(lead: Option<El>, label: El, slots: Vec<El>, gap: f32) -> El {
    let mut r = row().center_y().gap(gap).w_full();
    if let Some(lead) = lead {
        r = r.child(lead);
    }
    r = r.child(label.grow(1.0).shrink().min_w(0.0));
    for s in slots {
        r = r.child(s);
    }
    r
}

/// A label: one plain line in `ui`, cut at its end.
pub fn label(t: Theme, s: &str, color: rune::Color) -> El {
    text(s.to_string()).font(t.ty.ui).color(color).truncate()
}

/// A group head: `group` in steel, then its count in `fg.alt`.
pub fn group_head(t: Theme, name: &str, count: Option<u32>) -> El {
    let mut r = row()
        .center_y()
        .gap(7.0)
        .h(24.0)
        .child(text(name.to_string()).font(t.ty.group).color(t.c.kind.you));
    if let Some(n) = count {
        r = r.child(text(n.to_string()).font(t.ty.group).color(t.c.fg.alt));
    }
    r
}

/// One part of a facts line.
pub struct Part {
    pub el: El,
}

impl Part {
    pub fn text(t: Theme, s: &str) -> Part {
        Part {
            el: text(s.to_string()).font(t.ty.ui_small).color(t.c.fg.alt),
        }
    }
}

/// A facts line: parts joined by ` · `, one line tall. A part that doesn't fit drops whole, with
/// everything after it, never cut mid-word (`visual-language.md` §3).
pub fn facts(t: Theme, parts: Vec<Part>) -> El {
    let h = t.ty.ui_small.line_height;
    let mut line = row().flex_wrap().h(h).clip_rounded(0.0).min_w(0.0);
    for (i, p) in parts.into_iter().enumerate() {
        let mut part = row().center_y().h(h);
        if i > 0 {
            part = part.child(text("·").font(t.ty.ui_small).color(t.c.fg.muted).mx(6.0));
        }
        line = line.child(part.child(p.el));
    }
    line
}
