//! Printed keys: chips and hints. Each one names its key in the keymap's syntax, so what it prints
//! and whether it is lit both come from `keymap.toml` (`spec.md` §4.6, `grammar.md` §9).

use rune::{El, TextStyle, icon, row, text};

use crate::assets::marks;
use crate::keys::{Ctx, Mode, display, keymap, parse_seq};
use crate::theme::Theme;

/// Where the keys are: the mode and context of the region that has them. A printed key is lit
/// only when it runs a command there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    pub mode: Mode,
    pub ctx: Ctx,
}

impl Scope {
    pub fn lit(self, seq: &str) -> bool {
        keymap().lit(seq, self.mode, self.ctx)
    }
}

/// `seq` in the design's printed form (`<leader>cn` → `␣cn`).
pub fn printed(seq: &str) -> String {
    parse_seq(seq)
        .map(|k| display(&k))
        .unwrap_or_else(|_| seq.to_string())
}

/// A printed key's characters, with `␣` and `⏎` drawn as the client's own glyphs so no font
/// fallback is involved (`visual-language.md` §6).
pub fn glyphs(printed: &str, style: TextStyle, color: rune::Color) -> El {
    let mut out = row().center_y().gap(1.0);
    let mut run = String::new();
    let flush = |out: El, run: &mut String| {
        if run.is_empty() {
            out
        } else {
            out.child(text(std::mem::take(run)).font(style).color(color))
        }
    };
    for c in printed.chars() {
        let glyph = match c {
            '␣' => Some(&*marks::KEY_SPACE),
            '⏎' => Some(&*marks::KEY_RETURN),
            _ => None,
        };
        match glyph {
            Some(g) => {
                out = flush(out, &mut run);
                out = out.child(icon(g, 9.0).color(color));
            }
            None => run.push(c),
        }
    }
    flush(out, &mut run)
}

/// A key chip: `key` orange on `surface.bar` (`surface.cursor.side` inside a float), or muted
/// and inert where its key doesn't run.
pub fn chip(t: Theme, scope: Scope, seq: &str, in_float: bool) -> El {
    let lit = scope.lit(seq);
    let fg = if lit { t.c.kind.literal } else { t.c.fg.muted };
    let bg = if in_float {
        t.c.surface.cursor_side
    } else {
        t.c.surface.bar
    };
    glyphs(&printed(seq), t.ty.key, fg)
        .h(18.0)
        .px(5.0)
        .radius(t.l.radius.pill)
        .fill(bg)
}

/// A muted key hint inside a container: no fill, `fg.alt` while lit, `fg.muted` while inert.
pub fn hint(t: Theme, scope: Scope, seq: &str) -> El {
    let fg = if scope.lit(seq) {
        t.c.fg.alt
    } else {
        t.c.fg.muted
    };
    glyphs(&printed(seq), t.ty.hint, fg)
}

/// A float's foot hint: the chip, then a muted word (`Esc closes`).
pub fn float_hint(t: Theme, scope: Scope, seqs: &[&str], word: &str) -> El {
    let mut chips = row().center_y().gap(4.0);
    for seq in seqs {
        chips = chips.child(chip(t, scope, seq, true));
    }
    chips.child(
        text(word.to_string())
            .font(t.ty.ui_small)
            .color(t.c.fg.muted)
            .ml(3.0),
    )
}
