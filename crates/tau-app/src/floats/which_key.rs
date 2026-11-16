//! which-key (`grammar.md` §5): 300 ms after a prefix, the keys that can follow it. It takes no
//! keys and leaves every cursor as it is.

use rune::{El, col, el, icon, row, text};

use crate::assets::marks;
use crate::keys::{Ctx, Mode, WhichEntry, WhichKey, display};
use crate::theme::Theme;
use crate::ui::{Scope, Shell, float_hint, glyphs, shell};

/// The `␣a` head's subtitle (`spec.md` §6).
const ANSWER_SUBTITLE: &str = "every command reads back what it sends before y";

/// A label or description, its leading `◇` or `◆` drawn as the outline diamond in `fg.alt`: an
/// answer command is never coral here.
fn words(t: Theme, s: &str, style: rune::TextStyle, color: rune::Color) -> El {
    let (diamond, rest) = match s.strip_prefix("◇ ").or_else(|| s.strip_prefix("◆ ")) {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let mut r = row().center_y().gap(6.0);
    if diamond {
        r = r.child(icon(&marks::DIAMOND_OUTLINE, 10.0).color(t.c.fg.alt));
    }
    r.child(text(rest.to_string()).font(style).color(color))
}

fn entry(t: Theme, e: &WhichEntry, ctx: Ctx, why: Why, width: f32) -> El {
    let reason = e
        .command
        .and_then(|c| super::unavailable(c, ctx).or_else(|| why(&c.id)));
    let dim = !e.applies || reason.is_some();
    let key_color = if dim { t.c.fg.muted } else { t.c.kind.literal };
    let (style, color) = match (e.group, dim) {
        (_, true) => (t.ty.ui, t.c.fg.muted),
        (true, false) => (t.ty.ui_strong, t.c.fg.default),
        (false, false) => (t.ty.ui, t.c.fg.alt),
    };
    let key_style = rune::TextStyle {
        size: 12.5,
        line_height: 18.0,
        weight: 500.0,
        ..t.ty.mono
    };
    let mut out = row()
        .center_y()
        .gap(10.0)
        .min_w(width)
        .pr(16.0)
        .h(t.l.float.keygrid_row)
        .child(
            row()
                .w(14.0)
                .center()
                .child(glyphs(&display(&[e.key]), key_style, key_color)),
        )
        .child(words(t, &e.desc, style, color));
    if let Some(reason) = reason {
        out = out.child(
            text(format!("· {reason}"))
                .font(t.ty.ui_small)
                .color(t.c.fg.muted)
                .ml(-4.0),
        );
    }
    out
}

fn grid(t: Theme, entries: &[&WhichEntry], ctx: Ctx, why: Why, columns: usize) -> El {
    let w = t.l.float.keygrid_column;
    let mut out = col().px(t.l.float.padding_x).py(4.0);
    for chunk in entries.chunks(columns.max(1)) {
        let mut line = row();
        for e in chunk {
            line = line.child(entry(t, e, ctx, why, w));
        }
        out = out.child(line);
    }
    out
}

/// Why a command can't run now though the keys reach it (`not answered`), by its id.
pub type Why<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The which-key float for `wk`, as wide as `width` allows (four 200 px columns at most). `ctx`
/// is where the keys are, for the reason a leaf can't run there; `why` gives the page's reason
/// for one that can't run now (`grammar.md` §5).
pub fn view(t: Theme, wk: &WhichKey, ctx: Ctx, why: Why, width: f32) -> El {
    let f = t.l.float;
    let columns = (((width - 2.0 * f.padding_x) / f.keygrid_column).floor() as usize).clamp(1, 4);
    let groups: Vec<&WhichEntry> = wk.entries.iter().filter(|e| e.group).collect();
    let leaves: Vec<&WhichEntry> = wk.entries.iter().filter(|e| !e.group).collect();
    let mut body = col().w_full();
    if !groups.is_empty() {
        body = body.child(grid(t, &groups, ctx, why, columns));
    }
    if !groups.is_empty() && !leaves.is_empty() {
        body = body.child(
            el().h(1.0)
                .mx(f.padding_x)
                .my(6.0)
                .fill(t.c.border.hairline),
        );
    }
    if !leaves.is_empty() {
        body = body.child(grid(t, &leaves, ctx, why, columns));
    }
    let answer = wk.label.starts_with('◇');
    let label = wk.label.trim_start_matches("◇ ").to_string();
    let title = if answer {
        text(label).font(t.ty.float_title).color(t.c.fg.default)
    } else {
        let key = if wk.prefix == [crate::keys::Key::char(' ')] {
            icon(&marks::KEY_SPACE, 12.0).color(t.c.kind.literal)
        } else {
            glyphs(
                &display(&wk.prefix),
                rune::TextStyle {
                    size: 14.0,
                    line_height: 20.0,
                    ..t.ty.mono
                },
                t.c.kind.literal,
            )
        };
        row()
            .center_y()
            .gap(6.0)
            .child(key)
            .child(text(label).font(t.ty.float_title).color(t.c.fg.default))
    };
    // which-key takes no keys: its one hint is the pending prefix's `Esc`, lit in Normal.
    let scope = Scope {
        mode: Mode::Normal,
        ctx: Ctx::Global,
    };
    shell(
        t,
        Shell {
            tag: None,
            title,
            subtitle: answer.then(|| ANSWER_SUBTITLE.to_string()),
            meta: None,
            body,
            input: None,
            hints: vec![float_hint(t, scope, &["<Esc>"], "closes")],
        },
    )
}
