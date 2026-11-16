//! The one container style, its small text field, and the notice band (`visual-language.md`
//! §1.1, `spec.md` §4).

use rune::{El, row, text};

use super::keys::{Scope, chip};
use crate::theme::Theme;

/// The container: `surface.raised`, a hairline edge (coral while it has the caret in Insert),
/// `radius.surface`. It holds your message, the composer, a note being typed and opened tool
/// output; nothing else gets a box.
pub fn container(t: Theme, focused: bool, child: El) -> El {
    let edge = if focused {
        t.c.state.attn
    } else {
        t.c.border.hairline
    };
    row()
        .fill(t.c.surface.raised)
        .border(1.0, edge)
        .radius(t.l.radius.surface)
        .padding(t.l.space[4])
        .child(child.grow(1.0))
}

/// The small text field: `surface.sunken`, a hairline edge (coral with the caret in Insert),
/// `radius.pill`, 26 px tall, 10 px in. `input` is the field itself (`text_input`).
pub fn text_field(t: Theme, focused: bool, input: El) -> El {
    let edge = if focused {
        t.c.state.attn
    } else {
        t.c.border.hairline
    };
    row()
        .center_y()
        .h(t.l.text_field.height)
        .px(t.l.text_field.padding_x)
        .fill(t.c.surface.sunken)
        .border(1.0, edge)
        .radius(t.l.radius.pill)
        .child(input.grow(1.0).min_w(0.0))
}

/// A notice band's action at its right: a chip and a word (`␣rc reconnect`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoticeAction {
    pub seq: &'static str,
    pub word: &'static str,
}

/// The notice band at the stage's top: one line (38 px) or two (56 px), `surface.side`, pushing
/// the page down. Anything the user must act on is a band, never only an echo. A `rose` band (a
/// refusal, a revoke) carries a 2 px `state.blocked` rule at its top.
pub fn notice(
    t: Theme,
    scope: Scope,
    mark: Option<El>,
    sentence: &str,
    action: Option<NoticeAction>,
    rose: bool,
) -> El {
    let band = t.l.notice_band;
    let mut r = row()
        .center_y()
        .gap(10.0)
        .w_full()
        .min_h(band.one_line)
        .max_h(band.two_lines)
        .px(40.0)
        .py(10.0)
        .fill(t.c.surface.side);
    if rose {
        r = r.border_sides([2.0, 0.0, 0.0, 0.0], t.c.state.blocked);
    }
    if let Some(m) = mark {
        r = r.child(m);
    }
    r = r.child(
        text(sentence.to_string())
            .font(t.ty.ui)
            .color(t.c.fg.default)
            .wrap()
            .max_w(900.0)
            .shrink()
            .min_w(0.0),
    );
    if let Some(a) = action {
        r = r.child(rune::spacer()).child(
            row()
                .center_y()
                .gap(6.0)
                .child(chip(t, scope, a.seq, false))
                .child(text(a.word).font(t.ty.ui_small).color(t.c.fg.alt)),
        );
    }
    r
}
