//! `K` on something whose whole value the page doesn't draw (`grammar.md` §4): a record's `data`
//! as JSON. The text is shown as sent, in `mono.small`, with no Markdown.

use rune::{El, row, text};

use crate::theme::Theme;
use crate::ui::{Scope, Shell, float_hint, shell, title};

pub struct Detail<'a> {
    /// A record's kind, in orange mono.
    pub kind: &'a str,
    /// `entry 34 · by task 18`.
    pub subtitle: String,
    pub text: &'a str,
}

pub fn view(t: Theme, scope: Scope, d: Detail) -> El {
    let head = row().center_y().gap(10.0).child(title(t, "Details")).child(
        text(d.kind.to_string())
            .font(t.ty.mono_small)
            .color(t.c.kind.literal),
    );
    shell(
        t,
        Shell {
            tag: None,
            title: head,
            subtitle: Some(d.subtitle),
            meta: None,
            body: text(d.text.to_string())
                .font(t.ty.mono_small)
                .color(t.c.fg.alt)
                .wrap()
                .px(t.l.float.padding_x)
                .py(4.0),
            input: None,
            hints: vec![float_hint(t, scope, &["<Esc>"], "closes")],
        },
    )
}
