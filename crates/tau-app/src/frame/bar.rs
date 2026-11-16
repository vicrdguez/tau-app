//! The bottom bar (`spec.md` §4.3): the mode badge, pending keys, the echo, and at the right the
//! save state, then, while the sidebar isn't drawn, the count tags and any server trouble.

use rune::{El, row, text};

use super::ServerPhase;
use super::sidebar::{OnRow, Row, tags, trouble_mark};
use crate::theme::Theme;
use crate::ui::{Badge, Counts, badge, glyphs};

pub struct Bar<'a> {
    pub badge: Badge,
    /// The pending keys in their printed form (`2␣a`).
    pub pending: String,
    pub echo: Option<&'a str>,
    /// The save state of option drafts (`✎ 2 not saved :w`), when there are any.
    pub save: Option<El>,
    /// While the sidebar isn't drawn: the count tags and the server line when it isn't calm.
    pub right: Option<(Counts, &'a ServerPhase)>,
}

pub fn bar<S: 'static>(t: Theme, b: Bar, on_row: OnRow<S>) -> El {
    let mut r = row()
        .center_y()
        .fill(t.c.surface.bar)
        .child(badge(t, b.badge));
    if !b.pending.is_empty() {
        r = r.child(
            glyphs(
                &b.pending,
                rune::TextStyle {
                    weight: 500.0,
                    ..t.ty.mono_small
                },
                t.c.kind.literal,
            )
            .ml(14.0),
        );
    }
    if let Some(echo) = b.echo {
        r = r.child(
            text(echo.to_string())
                .font(t.ty.ui_small)
                .color(t.c.fg.default)
                .truncate()
                .shrink()
                .min_w(0.0)
                .ml(14.0),
        );
    }
    r = r.child(rune::spacer());
    if let Some(save) = b.save {
        r = r.child(save.ml(18.0));
    }
    if let Some((counts, server)) = b.right {
        r = r.child(tags(t, counts, on_row).mr(16.0));
        if let ServerPhase::Trouble { mark, words } = server {
            r = r.child(
                row()
                    .center_y()
                    .gap(5.0)
                    .mr(14.0)
                    .on_press_tx(move |s: &mut S, tx| on_row(s, &Row::Server, tx))
                    .child(trouble_mark(t, *mark))
                    .child(
                        text(words.clone())
                            .font(t.ty.ui_small)
                            .color(t.c.fg.default),
                    ),
            );
        }
    }
    r
}
