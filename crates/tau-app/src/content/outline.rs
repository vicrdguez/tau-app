//! The prepared source as the blocks this module draws itself: headings, lists, quotes, code
//! and images, around the prose rune-kit draws. A list item's or a quote's body is its source
//! with the markers and indentation cut out, so it parses as a document of its own, and a
//! [`Cut`] maps any range of the prepared source into it.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// rune-kit's parser options.
const OPTIONS: Options = Options::ENABLE_STRIKETHROUGH
    .union(Options::ENABLE_TABLES)
    .union(Options::ENABLE_TASKLISTS);

/// A block's text: the pieces of the prepared source it shows, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Cut(pub Vec<Range<usize>>);

#[derive(Debug)]
pub(super) enum Node {
    /// rune-kit draws it from its source.
    Heading(u8, Cut),
    /// A paragraph, table, HTML block or rule: rune-kit draws it from its source.
    Prose(Cut),
    Image {
        alt: String,
        host: String,
    },
    /// Every code block, fenced or indented, `diff` and `mermaid` included.
    Code {
        lang: Option<String>,
        code: Cut,
    },
    List {
        start: Option<u64>,
        items: Vec<Item>,
    },
    Quote {
        /// The GFM alert's kind word.
        alert: Option<&'static str>,
        body: Vec<Node>,
    },
}

#[derive(Debug)]
pub(super) struct Item {
    pub task: Option<bool>,
    pub body: Vec<Node>,
}

impl Cut {
    pub fn text(&self, src: &str) -> String {
        self.0.iter().map(|r| &src[r.clone()]).collect()
    }

    /// `range` of the prepared source in this cut's text: one range per piece it touches.
    pub fn map(&self, range: Range<usize>) -> Vec<Range<usize>> {
        let mut out = Vec::new();
        let mut at = 0;
        for piece in &self.0 {
            let (a, b) = (range.start.max(piece.start), range.end.min(piece.end));
            if a < b {
                out.push(at + a - piece.start..at + b - piece.start);
            }
            at += piece.len();
        }
        out
    }

    /// The prepared source behind `local`, ranges of this cut's text.
    fn sub(&self, local: &[Range<usize>]) -> Cut {
        let mut out = Vec::new();
        for r in local {
            let mut at = 0;
            for piece in &self.0 {
                let (a, b) = (r.start.max(at), r.end.min(at + piece.len()));
                if a < b {
                    out.push(piece.start + a - at..piece.start + b - at);
                }
                at += piece.len();
            }
        }
        Cut(out)
    }
}

pub(super) fn outline(src: &str) -> Vec<Node> {
    nodes(src, &Cut(std::iter::once(0..src.len()).collect()))
}

fn nodes(src: &str, cut: &Cut) -> Vec<Node> {
    let text = cut.text(src);
    let events = Parser::new_ext(&text, OPTIONS)
        .into_offset_iter()
        .collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut i = 0;
    while i < events.len() {
        let end = close(&events, i);
        let inner = &events[(i + 1).min(end)..end];
        let (event, range) = &events[i];
        let trimmed = || cut.sub(&[trim(&text, range.clone())]);
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                out.push(Node::Heading(*level as u8, trimmed()));
            }
            Event::Start(Tag::Paragraph) => paragraph(&text, cut, range.clone(), inner, &mut out),
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next(),
                    CodeBlockKind::Indented => None,
                };
                let lines = inner
                    .iter()
                    .filter(|(e, _)| matches!(e, Event::Text(_)))
                    .map(|(_, r)| r.clone())
                    .collect::<Vec<_>>();
                out.push(Node::Code {
                    lang: lang.map(str::to_string),
                    code: cut.sub(&lines),
                });
            }
            Event::Start(Tag::List(start)) => {
                let mut items = Vec::new();
                let mut j = 0;
                while j < inner.len() {
                    let item_end = close(inner, j);
                    let task = inner[j + 1..item_end]
                        .iter()
                        .take_while(|(e, _)| !matches!(e, Event::Text(_)))
                        .find_map(|(e, r)| match e {
                            Event::TaskListMarker(done) => Some((*done, r.clone())),
                            _ => None,
                        });
                    let body = item_body(&text, inner[j].1.clone(), task.as_ref().map(|t| t.1.end));
                    items.push(Item {
                        task: task.map(|t| t.0),
                        body: nodes(src, &cut.sub(&body)),
                    });
                    j = item_end + 1;
                }
                out.push(Node::List {
                    start: *start,
                    items,
                });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let mut body = quote_body(&text, range.clone());
                let alert = body.first().and_then(|r| alert(&text[r.clone()]));
                if alert.is_some() {
                    body.remove(0);
                }
                out.push(Node::Quote {
                    alert,
                    body: nodes(src, &cut.sub(&body)),
                });
            }
            Event::Start(_) | Event::Rule => out.push(Node::Prose(trimmed())),
            _ => {}
        }
        i = end + 1;
    }
    out
}

/// The index of the event that closes the one at `i` (itself when it opens nothing).
fn close<T>(events: &[(Event, T)], i: usize) -> usize {
    if !matches!(events[i].0, Event::Start(_)) {
        return i;
    }
    let mut depth = 0;
    for (j, (e, _)) in events.iter().enumerate().skip(i) {
        match e {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    return j;
                }
            }
            _ => {}
        }
    }
    events.len() - 1
}

/// A paragraph, split around the images that stand directly in it.
fn paragraph(
    text: &str,
    cut: &Cut,
    range: Range<usize>,
    inner: &[(Event, Range<usize>)],
    out: &mut Vec<Node>,
) {
    let mut from = range.start;
    let mut depth = 0;
    let mut image: Option<(String, String)> = None;
    for (e, r) in inner {
        match e {
            Event::Start(Tag::Image { dest_url, .. }) if depth == 0 => {
                prose(text, cut, from..r.start, out);
                image = Some((String::new(), host(dest_url)));
            }
            Event::End(TagEnd::Image) if depth == 1 => {
                if let Some((alt, host)) = image.take() {
                    out.push(Node::Image { alt, host });
                }
                from = r.end;
            }
            Event::Text(s) | Event::Code(s) => {
                if let Some((alt, _)) = image.as_mut() {
                    alt.push_str(s);
                }
            }
            _ => {}
        }
        match e {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            _ => {}
        }
    }
    prose(text, cut, from..range.end, out);
}

fn prose(text: &str, cut: &Cut, range: Range<usize>, out: &mut Vec<Node>) {
    let r = trim(text, range);
    if !r.is_empty() {
        out.push(Node::Prose(cut.sub(&[r])));
    }
}

fn trim(text: &str, r: Range<usize>) -> Range<usize> {
    let s = &text[r.clone()];
    let start = r.start + s.len() - s.trim_start().len();
    start..r.start + s.trim_end().len()
}

/// An image source's host (`example.org:8080`), or nothing for a relative one.
fn host(url: &str) -> String {
    let Some((_, rest)) = url.split_once("://") else {
        return String::new();
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    authority.rsplit('@').next().unwrap_or_default().to_string()
}

/// A GFM alert's kind word, from a quote's first line (`[!NOTE]`, any case). The line may end
/// in the hard break `source` inserts for its newline.
fn alert(line: &str) -> Option<&'static str> {
    let tag = line.trim_end().trim_end_matches('\\').trim_end();
    let kind = tag.strip_prefix("[!")?.strip_suffix(']')?;
    ["Note", "Tip", "Important", "Warning", "Caution"]
        .into_iter()
        .find(|k| k.eq_ignore_ascii_case(kind))
}

/// The lines of `range`, each with its newline.
fn lines(text: &str, range: Range<usize>) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut at = range.start;
    std::iter::from_fn(move || {
        if at >= range.end {
            return None;
        }
        let end = text[at..range.end]
            .find('\n')
            .map_or(range.end, |i| at + i + 1);
        let line = at..end;
        at = end;
        Some(line)
    })
}

fn spaces(s: &str) -> usize {
    s.len() - s.trim_start_matches(' ').len()
}

/// A list item's content: its first line after the marker (and task box), its later lines
/// without the indentation the marker sets (CommonMark §5.2).
fn item_body(text: &str, item: Range<usize>, task_end: Option<usize>) -> Vec<Range<usize>> {
    let line_start = text[..item.start].rfind('\n').map_or(0, |i| i + 1);
    let marker = line_start + spaces(&text[line_start..]);
    let rest = &text[marker..item.end];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let after = marker + digits + 1;
    let first_end = text[after..item.end]
        .find('\n')
        .map_or(item.end, |i| after + i + 1);
    let gap = spaces(&text[after..first_end]);
    let blank = text[after + gap..first_end].trim().is_empty();
    let gap = if blank || gap >= 5 { gap.min(1) } else { gap };
    let width = after + gap - line_start;
    let mut start = after + gap;
    if let Some(end) = task_end {
        start = end + spaces(&text[end..first_end]);
    }
    let mut out = Vec::from_iter(std::iter::once(start..first_end));
    for line in lines(text, first_end..item.end) {
        let cut = spaces(&text[line.clone()]).min(width);
        out.push(line.start + cut..line.end);
    }
    out
}

/// A quote's lines without their `>` and the space after it; a lazy line stays whole.
fn quote_body(text: &str, range: Range<usize>) -> Vec<Range<usize>> {
    lines(text, range)
        .map(|line| {
            let s = &text[line.clone()];
            let indent = spaces(s).min(3);
            if s[indent..].starts_with('>') {
                let at = indent + 1;
                let at = at + usize::from(s[at..].starts_with(' '));
                line.start + at..line.end
            } else {
                line
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "outline_tests.rs"]
mod tests;
