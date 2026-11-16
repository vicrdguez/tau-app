use rune::{Color, El, Span, TextStyle, rich};

use crate::theme::Theme;

/// A visible run in a row or title. Escapes use the form `U+202E`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Run {
    Text(String),
    Escape(String),
}

/// A character that draws nothing or reorders what follows: shown as its `U+` escape.
pub fn invisible(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{061c}' | '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}'
            | '\u{2060}' | '\u{2066}'..='\u{2069}' | '\u{feff}')
}

pub(super) fn escape(c: char) -> String {
    format!("U+{:04X}", c as u32)
}

/// Removes paired, equal-length backtick delimiters and exposes invisible characters.
/// Other Markdown stays literal. Width truncation belongs to `line`, not this parser.
pub fn runs(src: &str) -> Vec<Run> {
    let mut ticks = Vec::new();
    let mut chars = src.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if c == '`' {
            let mut end = start + 1;
            while chars.peek().is_some_and(|(_, c)| *c == '`') {
                end = chars.next().unwrap().0 + 1;
            }
            ticks.push(start..end);
        }
    }
    let mut removed = Vec::new();
    let mut i = 0;
    while i < ticks.len() {
        if let Some(j) = (i + 1..ticks.len()).find(|&j| ticks[j].len() == ticks[i].len()) {
            removed.push(ticks[i].clone());
            removed.push(ticks[j].clone());
            i = j + 1;
        } else {
            i += 1;
        }
    }
    let mut out = Vec::new();
    let mut plain = String::new();
    let mut removed = removed.into_iter().peekable();
    for (at, c) in src.char_indices() {
        while removed.peek().is_some_and(|r| r.end <= at) {
            removed.next();
        }
        if removed.peek().is_some_and(|r| r.contains(&at)) {
            continue;
        }
        if invisible(c) {
            if !plain.is_empty() {
                out.push(Run::Text(std::mem::take(&mut plain)));
            }
            out.push(Run::Escape(escape(c)));
        } else {
            plain.push(c);
        }
    }
    if !plain.is_empty() {
        out.push(Run::Text(plain));
    }
    out
}

/// One line, cut at the end by rune. The caller owns its font and foreground colour.
pub fn line(t: Theme, src: &str, font: TextStyle, color: Color) -> El {
    let mut shown = String::new();
    let mut spans = Vec::new();
    for run in runs(src) {
        let start = shown.len();
        match run {
            Run::Text(s) => shown.push_str(&s),
            Run::Escape(s) => {
                shown.push_str(&s);
                spans.push(
                    Span::new(start, shown.len())
                        .mono()
                        .size(t.ty.mono_small.size)
                        .weight(t.ty.mono_small.weight)
                        .color(t.c.fg.alt)
                        .bg(t.c.surface.bar, 0.0),
                );
            }
        }
    }
    rich(shown, spans)
        .font(font)
        .color(color)
        .truncate()
        .min_w(0.0)
}

#[cfg(test)]
#[path = "line_tests.rs"]
mod tests;
