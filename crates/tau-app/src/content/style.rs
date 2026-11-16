use std::sync::LazyLock;

use rune::{Color, TextStyle};
use rune_kit::MarkdownStyle;
use rune_kit::syntax::{Syntax, SyntaxTheme};
use serde::Deserialize;
use serde_json::Value;

use crate::theme::Theme;

// These tokens are not exposed by theme::Layout yet. Keep their values in tokens.json.
static TOKENS: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../../design/tokens.json")).expect("design tokens")
});

#[derive(Deserialize)]
pub(super) struct Metrics {
    pub paragraph_gap: f32,
    pub list_indent: f32,
    pub table_cell: [f32; 2],
    pub quote_bar: f32,
    pub code_rule: f32,
    pub code_padding: [f32; 2],
    pub image_placeholder: f32,
    pub task_box: f32,
}

pub(super) static METRICS: LazyLock<Metrics> = LazyLock::new(|| {
    serde_json::from_value(TOKENS["tau"]["layout"]["markdown"].clone())
        .expect("tau.layout.markdown")
});

/// The width a table fits before it scrolls: the reading measure (`visual-language.md` §3), or
/// the peek's column inside its padding and its content bar's 2 px and 12 px.
pub(super) fn column(peek: bool) -> f32 {
    if !peek {
        return TOKENS["tau"]["layout"]["decision"]["text"]
            .as_f64()
            .expect("prose measure") as f32;
    }
    let p = &TOKENS["tau"]["layout"]["peek"];
    let n = |k: &str| p[k].as_f64().expect("tau.layout.peek") as f32;
    n("width") - 2.0 * n("padding") - 14.0
}

/// The fonts a Markdown's blocks use: full, or stepped down in the peek (`spec.md` §5.4).
#[derive(Clone, Copy)]
pub(super) struct Fonts {
    pub body: TextStyle,
    pub code: TextStyle,
    pub inline_code: f32,
    pub table: TextStyle,
    pub headings: [TextStyle; 6],
}

pub(super) fn fonts(t: Theme, peek: bool) -> Fonts {
    let ty = &t.ty;
    if peek {
        Fonts {
            body: ty.prose_small,
            code: TextStyle {
                line_height: 18.0,
                ..ty.mono_small
            },
            inline_code: ty.mono_small.size,
            table: TextStyle {
                size: 12.0,
                line_height: 17.0,
                ..ty.prose_small
            },
            headings: [
                ty.peek_h1, ty.peek_h2, ty.peek_h3, ty.peek_h3, ty.peek_h3, ty.peek_h3,
            ],
        }
    } else {
        Fonts {
            body: ty.prose,
            code: TextStyle {
                line_height: 20.0,
                ..ty.mono
            },
            inline_code: ty.prose_code.size,
            table: ty.prose_table,
            headings: [
                ty.prose_h1,
                ty.prose_h2,
                ty.prose_h3,
                ty.prose_h4,
                ty.prose_h5,
                ty.prose_h6,
            ],
        }
    }
}

/// What rune-kit draws: paragraphs, headings and tables. A heading of `level` takes the fourth
/// heading slot, the deepest rune-kit has; a quote's prose is italic `fg.alt`.
pub(super) fn prose(
    t: Theme,
    f: &Fonts,
    peek: bool,
    level: u8,
    quote: bool,
    width: f32,
) -> MarkdownStyle {
    let m = &*METRICS;
    let (body, text) = if quote {
        (f.body.italic(), t.c.fg.alt)
    } else {
        (f.body, t.c.fg.default)
    };
    let cell = if peek { [3.0, 7.0] } else { m.table_cell };
    let mut s = MarkdownStyle::neutral(!t.variant.is_light())
        .text(text)
        .muted(t.c.fg.muted)
        .body_font(body)
        .strong_weight(700.0)
        .link(t.c.kind.link)
        .link_underline(t.c.kind.link)
        .link_hover(t.c.kind.link_active)
        .inline_code_bg(t.c.surface.code)
        .inline_code_font(f.inline_code / body.size, t.ty.prose_code.weight)
        .inline_code_pad(if peek { 4.0 } else { 5.0 })
        .table_outline(None)
        .table_rules(t.c.border.hairline)
        .table_header_fill(Color::TRANSPARENT)
        .table_header_text(t.c.fg.alt)
        .table_header_rule(t.c.border.strong)
        .table_header_weight(600.0)
        .table_font(f.table)
        .table_cell_padding(cell[1], cell[0])
        .table_flush_left(true)
        // Columns wrap down to their longest word, then the table scrolls.
        .table_min_column(10_000.0)
        .text_width(width)
        .selection(t.c.surface.selection)
        .highlight_current_ring(None);
    for (i, font) in f.headings[..3].iter().enumerate() {
        s = s.heading_font(i as u8 + 1, *font);
    }
    s.heading_font(4, f.headings[usize::from(level.clamp(4, 6)) - 1])
}

/// Code colours, never weight or slant (`visual-language.md` §4.7).
pub(super) fn syntax(t: Theme) -> SyntaxTheme {
    let mut s = SyntaxTheme::neutral(!t.variant.is_light());
    for (kind, key) in [
        (Syntax::Keyword, "keyword"),
        (Syntax::String, "string"),
        (Syntax::Number, "constant"),
        (Syntax::Type, "type"),
        (Syntax::Function, "function"),
    ] {
        let value = TOKENS["variants"][t.variant.name()]["syntax"][key]["color"]
            .as_str()
            .expect("syntax colour");
        s = s.with(kind, rune::color::hex(value).expect("syntax hex colour"));
    }
    for (kind, color) in [
        (Syntax::Comment, t.c.fg.muted),
        (Syntax::Property, t.c.fg.alt),
        (Syntax::Punctuation, t.c.fg.alt),
        (Syntax::Inserted, t.c.text.ok),
        (Syntax::Deleted, t.c.text.failed),
        (Syntax::Parameter, t.c.fg.default),
        (Syntax::Heading, t.c.fg.default),
        (Syntax::Link, t.c.kind.link),
        (Syntax::Operator, t.c.fg.alt),
        (Syntax::Regex, t.c.fg.default),
        (Syntax::Attribute, t.c.fg.alt),
        (Syntax::Variable, t.c.fg.default),
    ] {
        s = s.with(kind, color);
    }
    s
}
