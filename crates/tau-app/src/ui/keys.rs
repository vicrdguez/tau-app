//! Printed keys: chips and hints. Each one names its key as `keymap.toml` writes it; the theme's
//! keymap (`t.keymap`) prints the key this device types for it and says whether it is lit
//! (`spec.md` §4.6, `grammar.md` §9).

use rune::{El, TextStyle, Tx, icon, row, text};

use crate::assets::marks;
use crate::keys::{Chip, Ctx, Keymap, Mode};
use crate::theme::Theme;

/// Where the keys are: the mode and context of the region that has them. A printed key is lit
/// only when it runs a command there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    pub mode: Mode,
    pub ctx: Ctx,
}

impl Scope {
    pub fn lit(self, km: Keymap, seq: &str) -> bool {
        km.chip(seq, self.mode, self.ctx).lit
    }
}

/// A printed key's characters, with `␣` and `⏎` drawn as the client's own glyphs so no font
/// fallback is involved (`visual-language.md` §6). Assistive technology reads it as one piece,
/// `printed`, until a caller names it in words (`chip`, `hint`).
pub fn glyphs(printed: &str, style: TextStyle, color: rune::Color) -> El {
    let mut out = row().center_y().gap(1.0);
    let mut run = String::new();
    let flush = |out: El, run: &mut String| {
        if run.is_empty() {
            out
        } else {
            out.child(
                text(std::mem::take(run))
                    .font(style)
                    .color(color)
                    .a11y_hidden(),
            )
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
                out = out.child(icon(g, 9.0).color(color).a11y_hidden());
            }
            None => run.push(c),
        }
    }
    flush(out, &mut run).role("text").a11y_value(printed)
}

/// A key read in words where it is printed: `Space c n`, `Command-Return`.
fn spoken(el: El, key: &Chip) -> El {
    el.label(key.spoken.clone()).a11y_value(key.spoken.clone())
}

/// The one name of words and the keys printed with them: `start, Command-Return`.
pub fn with_keys(words: &str, keys: &[&Chip]) -> String {
    let keys: Vec<&str> = keys.iter().map(|k| k.spoken.as_str()).collect();
    format!("{words}, {}", keys.join(" or "))
}

/// `body` read as `name` alone: what it draws is hidden from assistive technology.
pub fn named(body: El, name: String) -> El {
    row()
        .role("text")
        .label(name.clone())
        .a11y_value(name)
        .child(body.a11y_hidden())
}

/// A key chip: `key` orange on `surface.bar` (`surface.cursor.side` inside a float), or muted
/// and inert where its key doesn't run.
pub fn chip(t: Theme, scope: Scope, seq: &str, in_float: bool) -> El {
    chip_if(t, scope, seq, in_float, true)
}

fn chip_if(t: Theme, scope: Scope, seq: &str, in_float: bool, acts: bool) -> El {
    let key = t.keymap.chip(seq, scope.mode, scope.ctx);
    let fg = if key.lit && acts {
        t.c.kind.literal
    } else {
        t.c.fg.muted
    };
    let bg = if in_float {
        t.c.surface.cursor_side
    } else {
        t.c.surface.bar
    };
    spoken(glyphs(&key.printed, t.ty.key, fg), &key)
        .h(18.0)
        .px(5.0)
        .radius(t.l.radius.pill)
        .fill(bg)
}

/// A muted key hint inside a container: no fill, `fg.alt` while lit, `fg.muted` while inert.
pub fn hint(t: Theme, scope: Scope, seq: &str) -> El {
    let key = t.keymap.chip(seq, scope.mode, scope.ctx);
    let fg = if key.lit { t.c.fg.alt } else { t.c.fg.muted };
    spoken(glyphs(&key.printed, t.ty.hint, fg), &key)
}

/// A control made of printed keys and words (`⌘⏎ start`, the start line's project): one
/// accessible name for the whole, and a press that does what its key does, only while `lit`.
pub fn control<S: 'static>(
    body: El,
    name: String,
    lit: bool,
    press: impl Fn(&mut S, &mut Tx) + 'static,
) -> El {
    let c = row()
        .role("button")
        .label(name)
        .disabled(!lit)
        .child(body.a11y_hidden());
    if lit { c.on_press_tx(press) } else { c }
}

/// A float's foot hint: the chip, then a muted word (`Esc closes`).
pub fn float_hint(t: Theme, scope: Scope, seqs: &[&str], word: &str) -> El {
    float_hint_if(t, scope, seqs, word, true)
}

/// A foot hint whose key does something only when `acts`: `⏎ choose` with no row to choose
/// draws muted and inert, and stays in its place.
pub fn float_hint_if(t: Theme, scope: Scope, seqs: &[&str], word: &str, acts: bool) -> El {
    let mut chips = row().center_y().gap(4.0);
    for seq in seqs {
        chips = chips.child(chip_if(t, scope, seq, true, acts));
    }
    let keys: Vec<Chip> = seqs
        .iter()
        .map(|seq| t.keymap.chip(seq, scope.mode, scope.ctx))
        .collect();
    named(
        chips.child(
            text(word.to_string())
                .font(t.ty.ui_small)
                .color(t.c.fg.muted)
                .ml(3.0),
        ),
        with_keys(word, &keys.iter().collect::<Vec<_>>()),
    )
}
