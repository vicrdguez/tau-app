//! The mode badge, the bottom bar's first segment (`grammar.md` §2).

use rune::{El, TextCase, row, text};

use crate::theme::Theme;

/// What the badge names. Every float that takes the keys names its own mode, never `NORMAL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Badge {
    Normal,
    Insert,
    Visual,
    VisualLine,
    Command,
    Hint,
    /// The answer readback and send-again: solid coral.
    Readback,
    /// Confirm and the not-recorded reply: the plain badge with a coral edge.
    ReadbackPlain,
    Outbox,
    Server,
    Peek,
}

impl Badge {
    pub fn word(self) -> &'static str {
        match self {
            Badge::Normal => "NORMAL",
            Badge::Insert => "INSERT",
            Badge::Visual => "VISUAL",
            Badge::VisualLine => "V-LINE",
            Badge::Command => "COMMAND",
            Badge::Hint => "HINT",
            Badge::Readback | Badge::ReadbackPlain => "READBACK",
            Badge::Outbox => "OUTBOX",
            Badge::Server => "SERVER",
            Badge::Peek => "PEEK",
        }
    }
}

pub fn badge(t: Theme, b: Badge) -> El {
    let (fill, fg) = match b {
        Badge::Readback => (t.c.state.attn, t.c.surface.read),
        _ => (t.c.surface.cursor_side, t.c.fg.default),
    };
    let edge = match b {
        Badge::Readback => None,
        Badge::Insert => Some(t.c.state.local),
        Badge::Visual | Badge::VisualLine => Some(t.c.state.stale),
        Badge::Command | Badge::Hint => Some(t.c.kind.literal),
        _ => Some(t.c.state.attn),
    };
    let word = text(b.word())
        .font(t.ty.badge)
        .color(fg)
        .text_case(TextCase::Uppercase);
    let el = row()
        .center_y()
        .h(t.l.height.badge)
        .pl(12.0)
        .pr(10.0)
        .fill(fill)
        .child(word);
    match edge {
        Some(c) => el.border_sides([0.0, 0.0, 0.0, 2.0], c),
        None => el,
    }
}
