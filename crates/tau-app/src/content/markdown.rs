use std::ops::Range;

use rune::{Color, El, col, text};

use crate::theme::Theme;

use super::outline::outline;
use super::render::{Flow, Mark, Render};
use super::source::{Ink, prepare};

/// A note's passage, in UTF-8 bytes of the original source. Active notes use the darker tint.
#[derive(Clone, Debug)]
pub struct Tint {
    pub range: Range<usize>,
    pub active: bool,
}

/// A caller's own paint over a source range, in UTF-8 bytes of the original source: a reader's
/// caret or its Visual selection. It is chrome over content, so the caller names the colours.
#[derive(Clone, Debug)]
pub struct Paint {
    pub range: Range<usize>,
    pub fg: Option<Color>,
    pub bg: Color,
}

/// Inert Markdown. No image loader, task handler, or link opener is installed.
pub struct Markdown<'a> {
    theme: Theme,
    source: &'a str,
    peek: bool,
    streaming: bool,
    tints: Vec<Tint>,
    paints: Vec<Paint>,
}

pub fn markdown(t: Theme, source: &str) -> Markdown<'_> {
    Markdown {
        theme: t,
        source,
        peek: false,
        streaming: false,
        tints: Vec::new(),
        paints: Vec::new(),
    }
}

impl Markdown<'_> {
    pub fn peek(mut self) -> Self {
        self.peek = true;
        self
    }

    pub fn streaming(mut self, on: bool) -> Self {
        self.streaming = on;
        self
    }

    /// Replaces the note tints. Invalid ranges paint nothing, partial characters expand.
    pub fn tint(mut self, ranges: impl IntoIterator<Item = Tint>) -> Self {
        self.tints = ranges.into_iter().collect();
        self
    }

    /// Replaces the caller's paints. They draw over the note tints, keyed `paint-0`, `paint-1`, ….
    pub fn paint(mut self, paints: impl IntoIterator<Item = Paint>) -> Self {
        self.paints = paints.into_iter().collect();
        self
    }

    pub fn build(self) -> El {
        let t = self.theme;
        let prepared = prepare(self.source);
        let mut marks = prepared
            .inks
            .iter()
            .map(|(ranges, ink)| {
                let color = match ink {
                    Ink::Heading(1) => t.c.content.h1,
                    Ink::Heading(2) => t.c.content.h2,
                    Ink::Heading(5..=6) if !self.peek => t.c.fg.alt,
                    Ink::Heading(_) => t.c.fg.default,
                    Ink::Strong => t.c.content.strong,
                    Ink::Code => t.c.kind.code,
                    Ink::Escape => t.c.fg.alt,
                };
                let bg = if *ink == Ink::Escape {
                    t.c.surface.bar
                } else {
                    Color::TRANSPARENT
                };
                Mark {
                    ranges: ranges.clone(),
                    fg: Some(color),
                    bg,
                    key: None,
                }
            })
            .collect::<Vec<_>>();
        if self.streaming && !self.source.ends_with('\n') {
            let start = self.source.rfind('\n').map_or(0, |at| at + 1);
            if let Some(range) = prepared.map(start..self.source.len()) {
                marks.push(Mark {
                    ranges: vec![range],
                    fg: Some(t.c.fg.muted),
                    bg: Color::TRANSPARENT,
                    key: None,
                });
            }
        }
        for (i, tint) in self.tints.into_iter().enumerate() {
            if let Some(range) = prepared.map(tint.range) {
                marks.push(Mark {
                    ranges: vec![range],
                    fg: None,
                    bg: if tint.active {
                        t.c.tint.note_active
                    } else {
                        t.c.tint.note_range
                    },
                    key: Some(format!("note-{i}")),
                });
            }
        }
        for (i, p) in self.paints.into_iter().enumerate() {
            if let Some(range) = prepared.map(p.range) {
                marks.push(Mark {
                    ranges: vec![range],
                    fg: p.fg,
                    bg: p.bg,
                    key: Some(format!("paint-{i}")),
                });
            }
        }
        let nodes = outline(&prepared.text);
        let body = Render {
            t,
            src: &prepared.text,
            marks,
            peek: self.peek,
            streaming: self.streaming,
        }
        .blocks(&nodes, Flow::top());
        let mut result = col().w_full().min_w(0.0).child(body);
        if prepared.omitted > 0 {
            result = result.child(
                text(format!("… {} more lines", prepared.omitted))
                    .font(t.ty.ui_small)
                    .color(t.c.fg.muted)
                    .mt(12.0),
            );
        }
        result
    }
}
