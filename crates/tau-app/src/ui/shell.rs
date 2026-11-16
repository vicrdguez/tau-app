//! The one shell every bottom float shares (`spec.md` §6): head, body, foot; `surface.raised`, a
//! hairline, a 2 px coral top rule, `radius.surface` and the upward shadow.

use rune::{El, col, el, row, text};

use crate::theme::Theme;

/// What a float shows. The frame places it over the stage and clips the page at its top edge.
pub struct Shell {
    /// The solid coral verb tag, or the neutral outline `◇ NOT RECORDED`.
    pub tag: Option<El>,
    /// `float.title`; it may carry an `#id`, a project or a key glyph.
    pub title: El,
    pub subtitle: Option<String>,
    pub meta: Option<String>,
    pub body: El,
    /// The foot's input at the left (`float.input`), with any count after it.
    pub input: Option<El>,
    /// The foot's hints at the right, 16 px apart.
    pub hints: Vec<El>,
}

/// A float's title in `float.title`.
pub fn title(t: Theme, s: &str) -> El {
    text(s.to_string())
        .font(t.ty.float_title)
        .color(t.c.fg.default)
}

pub fn shell(t: Theme, s: Shell) -> El {
    let f = t.l.float;
    let mut head = row()
        .center_y()
        .gap(10.0)
        .px(f.padding_x)
        .pb(8.0)
        .min_h(28.0);
    if let Some(tag) = s.tag {
        head = head.child(tag);
    }
    head = head.child(s.title);
    if let Some(sub) = s.subtitle {
        head = head.child(
            text(sub)
                .font(t.ty.ui_small)
                .color(t.c.fg.muted)
                .truncate()
                .shrink()
                .min_w(0.0),
        );
    }
    if let Some(meta) = s.meta {
        head = head
            .child(rune::spacer())
            .child(text(meta).font(t.ty.ui_small).color(t.c.fg.muted));
    }
    let mut float = col()
        .w_full()
        .fill(t.c.surface.raised)
        .border(1.0, t.c.border.hairline)
        .radius(t.l.radius.surface)
        .box_shadow(t.c.float_shadow)
        .pt(f.padding_top + 2.0)
        .pb(f.padding_bottom + 1.0)
        .child(head)
        .child(
            col()
                .w_full()
                .py(2.0)
                .min_h(0.0)
                .shrink()
                .clip_rounded(0.0)
                .child(s.body),
        );
    if s.input.is_some() || !s.hints.is_empty() {
        let mut foot = row()
            .center_y()
            .gap(t.l.gaps.float_hints)
            .mx(f.padding_x)
            .mt(8.0)
            .pt(9.0)
            .min_h(f.foot_min_height)
            .border_sides([1.0, 0.0, 0.0, 0.0], t.c.border.hairline)
            .border_layout();
        foot = match s.input {
            Some(input) => foot.child(input.grow(1.0).min_w(0.0)),
            None => foot.child(rune::spacer()),
        };
        let mut hints = row().center_y().gap(t.l.gaps.float_hints);
        for h in s.hints {
            hints = hints.child(h);
        }
        float = float.child(foot.child(hints));
    }
    // The coral top rule: a top-only border over the hairline, narrowing round the corners as a
    // CSS `border-top` does.
    float.child(
        el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0))
            .border_sides([2.0, 0.0, 0.0, 0.0], t.c.state.attn)
            .radius(t.l.radius.surface)
            .inert(),
    )
}

/// A float's rows body: 30 px rows, 24 px in. The selection is a `surface.cursor.side` band with
/// the 2 px coral bar.
pub fn float_row(t: Theme, selected: bool, content: El) -> El {
    let r = row()
        .center_y()
        .gap(12.0)
        .h(t.l.float.row)
        .px(t.l.float.padding_x)
        .w_full()
        .child(content.grow(1.0).min_w(0.0));
    if selected {
        r.fill(t.c.surface.cursor_side)
            .border_sides([0.0, 0.0, 0.0, 2.0], t.c.state.attn)
    } else {
        r
    }
}

/// A key-value grid row: a 96 px steel label, then the value in `fg`.
pub fn kv_row(t: Theme, label: &str, value: El) -> El {
    row()
        .center_y()
        .min_h(t.l.float.row)
        .px(t.l.float.padding_x)
        .child(
            text(label.to_string())
                .font(t.ty.ui_small)
                .color(t.c.kind.you)
                .w(t.l.float.kv_label)
                .wrap(),
        )
        .child(value.grow(1.0).min_w(0.0))
}
