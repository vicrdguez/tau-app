use rune::{Color, TextStyle};
use rune_kit::MarkdownStyle;
use rune_kit::syntax::{Syntax, SyntaxTheme};

use crate::theme::Theme;

pub(super) fn column(t: Theme, peek: bool) -> f32 {
    if peek {
        t.l.peek.width - 2.0 * t.l.peek.padding - 14.0
    } else {
        t.read.measure
    }
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
    let m = &t.l.markdown;
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
    for (kind, color) in [
        (Syntax::Keyword, t.c.syntax.keyword.0),
        (Syntax::String, t.c.syntax.string.0),
        (Syntax::Number, t.c.syntax.constant.0),
        (Syntax::Type, t.c.syntax.r#type.0),
        (Syntax::Function, t.c.syntax.function.0),
    ] {
        s = s.with(kind, color);
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
