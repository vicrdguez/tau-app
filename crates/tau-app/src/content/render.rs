//! The outline as elements: the blocks this module draws itself (headings' frames, lists,
//! quotes, code, images) and the gaps between blocks; rune-kit draws the prose inside them.

use std::ops::Range;

use rune::{Align, Color, El, TextHighlight, TextStyle, col, el, icon, rich, row, text};
use rune_kit::markdown::Highlight;

use crate::assets::carbon;
use crate::theme::Theme;

use super::line;
use super::outline::{Cut, Item, Node};
use super::style::{Fonts, METRICS, column, fonts, prose, syntax};

/// A paint over ranges of the prepared source.
pub(super) struct Mark {
    pub ranges: Vec<Range<usize>>,
    pub fg: Option<Color>,
    pub bg: Color,
    pub key: Option<String>,
}

pub(super) struct Render<'a> {
    pub t: Theme,
    pub src: &'a str,
    pub marks: Vec<Mark>,
    pub peek: bool,
    pub streaming: bool,
}

/// Where blocks sit: how many lists deep, whether in a quote, and whether the last of them
/// ends the source (the streaming tail).
#[derive(Clone, Copy, Default)]
pub(super) struct Flow {
    lists: usize,
    quote: bool,
    tail: bool,
    /// How much narrower than the column the blocks are: list indents and quote bars.
    inset: f32,
}

impl Flow {
    pub fn top() -> Flow {
        Flow {
            tail: true,
            ..Flow::default()
        }
    }
}

/// Marks in a cut's own coordinates.
type Local<'m> = Vec<(Vec<Range<usize>>, &'m Mark)>;

const BULLETS: [&str; 3] = ["•", "◦", "▪"];
const ITEM_GAP: f32 = 4.0;
const QUOTE_PAD: f32 = 12.0;

impl Render<'_> {
    pub fn blocks(&self, nodes: &[Node], flow: Flow) -> El {
        let mut c = col().w_full().min_w(0.0);
        let mut below = 0.0;
        for (i, node) in nodes.iter().enumerate() {
            let (above, after) = self.margins(node);
            // A list nested in an item hangs right under the item's text, as items follow each other.
            let prev = if flow.lists > 0 && matches!(node, Node::List { .. }) {
                ITEM_GAP
            } else {
                below
            };
            let tail = flow.tail && i + 1 == nodes.len();
            let block = self.node(node, Flow { tail, ..flow }).key(format!("b{i}"));
            c = c.child(if i == 0 {
                block
            } else {
                block.mt(above.max(prev))
            });
            below = after;
        }
        c
    }

    /// A block's margins above and below, collapsed with its neighbour's as a page's are.
    fn margins(&self, node: &Node) -> (f32, f32) {
        let gap = if self.peek {
            8.0
        } else {
            METRICS.paragraph_gap
        };
        match (node, self.peek) {
            (Node::Heading(1, _), false) => (0.0, 12.0),
            (Node::Heading(2, _), false) => (26.0, 8.0),
            (Node::Heading(..), false) => (24.0, 6.0),
            (Node::Heading(1, _), true) => (0.0, 6.0),
            (Node::Heading(2, _), true) => (12.0, 4.0),
            (Node::Heading(..), true) => (10.0, 3.0),
            _ => (0.0, gap),
        }
    }

    fn node(&self, node: &Node, flow: Flow) -> El {
        let t = self.t;
        let f = fonts(t, self.peek);
        match node {
            Node::Heading(level, cut) => {
                let heading = self.prose(cut, &f, *level, flow);
                if *level > 1 {
                    return heading;
                }
                col()
                    .w_full()
                    .child(heading)
                    .pb(if self.peek { 4.0 } else { 6.0 })
                    .border_sides([0.0, 0.0, 1.0, 0.0], t.c.border.strong)
                    .border_layout()
            }
            Node::Prose(cut) => self.prose(cut, &f, 0, flow),
            Node::Image { alt, host } => self.image(&f, alt, host),
            Node::Code { lang, code } => self.code(&f, lang.as_deref(), code, flow.tail),
            Node::List { start, items } => self.list(&f, *start, items, flow),
            Node::Quote { alert, body } => self.quote(&f, *alert, body, flow),
        }
    }

    /// `cut`'s text with the marks it holds, in its own coordinates.
    fn local(&self, cut: &Cut) -> (String, Local<'_>) {
        let marks = self
            .marks
            .iter()
            .filter_map(|m| {
                let ranges = m
                    .ranges
                    .iter()
                    .flat_map(|r| cut.map(r.clone()))
                    .collect::<Vec<_>>();
                (!ranges.is_empty()).then_some((ranges, m))
            })
            .collect();
        (cut.text(self.src), marks)
    }

    fn prose(&self, cut: &Cut, f: &Fonts, level: u8, flow: Flow) -> El {
        let (mut text, marks) = self.local(cut);
        if level == 5 && !self.peek {
            text = upper(&text);
        }
        let highlights = marks.into_iter().map(|(ranges, m)| {
            let mut h = Highlight::new().bg(m.bg);
            if let Some(fg) = m.fg {
                h = h.fg(fg);
            }
            if let Some(key) = &m.key {
                h = h.key(key.clone());
            }
            (ranges, h)
        });
        let width = column(self.peek) - flow.inset;
        rune_kit::Markdown::new(text)
            .streaming(self.streaming && flow.tail)
            .scroll_wide(true)
            .tasks_read_only()
            .highlights(highlights)
            .render(&prose(self.t, f, self.peek, level, flow.quote, width))
    }

    /// Square, on `surface.code`, a gold rule at its left and the language at its top right;
    /// lines never wrap and scroll under a fade. While streaming, the line still arriving
    /// stays plain, so colour never moves under it.
    fn code(&self, f: &Fonts, lang: Option<&str>, cut: &Cut, tail: bool) -> El {
        let t = self.t;
        let m = &*METRICS;
        let (code, marks) = self.local(cut);
        let code = code.strip_suffix('\n').unwrap_or(&code).to_string();
        let coloured = if self.streaming && tail {
            code.rfind('\n').map_or(0, |at| at + 1)
        } else {
            code.len()
        };
        let spans = syntax(t).spans(&rune_kit::syntax::highlight(lang, &code[..coloured]), 0);
        let highlights = marks
            .into_iter()
            .flat_map(|(ranges, m)| {
                let len = code.len();
                ranges.into_iter().filter_map(move |r| {
                    let r = r.start.min(len)..r.end.min(len);
                    (!r.is_empty()).then(|| {
                        let mut h = TextHighlight::new(r, m.bg);
                        if let Some(fg) = m.fg {
                            h = h.color(fg);
                        }
                        match &m.key {
                            Some(key) => h.key(key.clone()),
                            None => h,
                        }
                    })
                })
            })
            .collect::<Vec<_>>();
        let (pad_y, pad_x) = if self.peek {
            ([5.0, 5.0], 10.0)
        } else {
            ([20.0, m.code_padding[0]], m.code_padding[1])
        };
        let body = rich(code, spans)
            .font(f.code)
            .color(t.c.fg.default)
            .text_highlights(highlights);
        let viewport = col()
            .key("scroll")
            .w_full()
            .scroll_x()
            .scroll_edge_fade(m.clip_fade)
            .child(
                col()
                    .pl(pad_x - m.code_rule)
                    .pr(pad_x)
                    .pt(pad_y[0])
                    .pb(pad_y[1])
                    .child(body),
            );
        let mut frame = row()
            .role("code")
            .w_full()
            .fill(t.c.surface.code)
            .child(el().w(m.code_rule).h_full().fill(t.c.kind.code))
            .child(viewport.grow(1.0).shrink().min_w(0.0));
        if let Some(lang) = lang {
            // The peek has no room above its code: the label sits over the first line's fade.
            let top = if self.peek { 6.0 } else { 3.0 };
            frame = frame.child(
                line(t, lang, t.ty.ui_small, t.c.kind.code)
                    .fill(t.c.surface.code)
                    .pl(6.0)
                    .pin(None, Some(top), Some(10.0), None),
            );
        }
        frame
    }

    /// Markers hang in a column per level: `• ◦ ▪` muted, numerals right-aligned and
    /// tabular, or a task's box.
    fn list(&self, f: &Fonts, start: Option<u64>, items: &[Item], flow: Flow) -> El {
        let t = self.t;
        let indent = if self.peek { 18.0 } else { METRICS.list_indent };
        let space = rune::text::measure("x x", &[], f.body, None).0
            - rune::text::measure("xx", &[], f.body, None).0;
        let inner = Flow {
            lists: flow.lists + 1,
            inset: flow.inset + indent + 2.0,
            ..flow
        };
        let mut c = col().role("list").w_full();
        for (k, item) in items.iter().enumerate() {
            let marker = match (item.task, start) {
                (Some(done), _) => self.task(f, done).w(indent).pr(space),
                (None, Some(n)) => text(format!("{}.", n + k as u64))
                    .font(TextStyle {
                        size: f.code.size,
                        line_height: f.body.line_height,
                        ..t.ty.mono
                    })
                    .color(t.c.fg.alt)
                    .w(indent)
                    .align(Align::End)
                    .pr(space),
                // A browser sets a disc a few points further from its text than a space.
                (None, None) => text(BULLETS[flow.lists % BULLETS.len()])
                    .font(f.body)
                    .color(t.c.fg.muted)
                    .w(indent)
                    .align(Align::End)
                    .pr(space + 4.0),
            };
            let tail = flow.tail && k + 1 == items.len();
            let body = self
                .blocks(&item.body, Flow { tail, ..inner })
                .grow(1.0)
                .shrink()
                .pl(2.0);
            c = c.child(
                row()
                    .key(format!("i{k}"))
                    .role("listitem")
                    .w_full()
                    .mt(if k == 0 { 0.0 } else { ITEM_GAP })
                    .child(marker.a11y_hidden())
                    .child(body),
            );
        }
        c
    }

    fn task(&self, f: &Fonts, done: bool) -> El {
        let t = self.t;
        let size = METRICS.task_box;
        let mut b = row()
            .size(size, size)
            .center()
            .border(1.0, t.c.border.strong)
            .fill(t.c.surface.code);
        if done {
            b = b.child(icon(&carbon::CHECKMARK, 10.0).color(t.c.fg.alt));
        }
        row()
            .justify_end()
            .child(b.mt(((f.body.line_height - size) / 2.0).max(0.0)))
    }

    /// The quote's bar, its prose italic `fg.alt`; an alert's kind word heads it.
    fn quote(&self, f: &Fonts, alert: Option<&str>, body: &[Node], flow: Flow) -> El {
        let t = self.t;
        let mut inner = col().w_full().min_w(0.0);
        if let Some(kind) = alert {
            inner = inner.child(
                text(kind)
                    .font(TextStyle {
                        weight: 600.0,
                        ..f.body
                    })
                    .color(t.c.fg.alt)
                    .mb(4.0),
            );
        }
        inner = inner.child(self.blocks(
            body,
            Flow {
                quote: true,
                inset: flow.inset + METRICS.quote_bar + QUOTE_PAD,
                ..flow
            },
        ));
        row()
            .role("blockquote")
            .w_full()
            .child(el().w(METRICS.quote_bar).h_full().fill(t.c.border.strong))
            .child(inner.grow(1.0).shrink().pl(QUOTE_PAD))
    }

    /// No image loads: its alt text and host in a placeholder.
    fn image(&self, f: &Fonts, alt: &str, host: &str) -> El {
        let t = self.t;
        let label = ["image", alt, host]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        row()
            .role("image")
            .w_full()
            .h(METRICS.image_placeholder)
            .center_y()
            .px(12.0)
            .border(1.0, t.c.border.hairline)
            .child(line(t, &label, f.body, t.c.fg.muted))
    }
}

/// `s` in capitals where a capital takes the same bytes, so marks keep their place.
fn upper(s: &str) -> String {
    s.chars()
        .map(|c| {
            let mut u = c.to_uppercase();
            match (u.next(), u.next()) {
                (Some(u), None) if u.len_utf8() == c.len_utf8() => u,
                _ => c,
            }
        })
        .collect()
}
