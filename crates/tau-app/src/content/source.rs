use std::ops::Range;

use rune_kit::markdown::{Block, Inline, Rendered, Style, parse_cached};

use super::line::{escape, invisible};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ink {
    Heading(u8),
    Strong,
    Code,
    Escape,
}

type Inks = Vec<(Vec<Range<usize>>, Ink)>;
type Edits = Vec<(Range<usize>, String)>;

type Changes = Vec<(Range<usize>, Range<usize>)>;

pub(super) struct Prepared<'a> {
    pub text: String,
    pub omitted: usize,
    pub inks: Inks,
    source: &'a str,
    /// The raw-HTML escapes (source to `shown`), then the rest (`shown` to `text`).
    changes: [Changes; 2],
}

impl Prepared<'_> {
    pub fn map(&self, range: Range<usize>) -> Option<Range<usize>> {
        if range.start >= range.end || range.start >= self.source.len() {
            return None;
        }
        let mut start = range.start;
        let mut end = range.end.min(self.source.len());
        while !self.source.is_char_boundary(start) {
            start -= 1;
        }
        while !self.source.is_char_boundary(end) {
            end += 1;
        }
        let position = |changes: &Changes, at, end: bool| {
            let mut from = 0;
            let mut to = 0;
            for (old, new) in changes {
                if at <= old.start {
                    break;
                }
                if at < old.end {
                    return if end { new.end } else { new.start };
                }
                from = old.end;
                to = new.end;
            }
            to + at - from
        };
        let [html, rest] = &self.changes;
        let start = position(rest, position(html, start, false), false);
        let end = position(rest, position(html, end, true), true);
        Some(start..end)
    }
}

pub(super) fn prepare(src: &str) -> Prepared<'_> {
    let omitted = src.lines().count().saturating_sub(2000);
    let end = if omitted > 0 {
        src.split_inclusive('\n').take(2000).map(str::len).sum()
    } else {
        src.len()
    };
    let source = &src[..end];
    // Raw HTML first: once it is literal text, its soft breaks read as any paragraph's do.
    let (shown, html) = apply(source, raw_html(source));
    let (_, mut edits) = inspect(&shown);
    for (at, c) in shown.char_indices() {
        if invisible(c) && c != '\n' && !(c == '\r' && shown[at..].starts_with("\r\n")) {
            edits.push((at..at + c.len_utf8(), escape(c)));
        }
    }
    let (text, rest) = apply(&shown, edits);
    let (mut inks, _) = inspect(&text);
    for (old, new) in &rest {
        if !shown[old.clone()].contains('\n') {
            inks.push((vec![new.clone()], Ink::Escape));
        }
    }
    Prepared {
        text,
        omitted,
        inks,
        source,
        changes: [html, rest],
    }
}

/// `src` with each edit's range replaced, and where each went.
fn apply(src: &str, mut edits: Edits) -> (String, Changes) {
    edits.sort_by_key(|(r, _)| (r.start, r.end));
    edits.dedup_by(|a, b| a.0 == b.0);
    let mut text = String::with_capacity(src.len());
    let mut changes = Vec::new();
    let mut from = 0;
    for (range, replacement) in edits {
        text.push_str(&src[from..range.start]);
        let start = text.len();
        text.push_str(&replacement);
        changes.push((range.clone(), start..text.len()));
        from = range.end;
    }
    text.push_str(&src[from..]);
    (text, changes)
}

// Both public rune-kit views walk the same parsed blocks. Its source map, not a second
// Markdown parser, tells us which spaces came from soft breaks and which bytes own a style.
fn inspect(src: &str) -> (Inks, Edits) {
    let rendered = Rendered::new(src);
    let texts = rendered.texts().collect::<Vec<_>>();
    let mut scan = Scan {
        src,
        rendered: &rendered,
        texts: &texts,
        block: 0,
        inks: Vec::new(),
        edits: Vec::new(),
    };
    scan.blocks(&parse_cached(src));
    (scan.inks, scan.edits)
}

struct Scan<'a> {
    src: &'a str,
    rendered: &'a Rendered,
    texts: &'a [&'a str],
    block: usize,
    inks: Inks,
    edits: Edits,
}

impl Scan<'_> {
    fn blocks(&mut self, blocks: &[Block]) {
        for block in blocks {
            match block {
                Block::Para(inline) => self.inline(inline, None),
                Block::Heading(level, inline) => self.inline(inline, Some(*level)),
                Block::Code(s) | Block::Diff(s) | Block::Mermaid(s) => self.plain(s),
                Block::Image { alt, .. } => self.plain(alt),
                Block::Quote(blocks) => self.blocks(blocks),
                Block::List { items, .. } => {
                    for item in items {
                        self.blocks(item);
                    }
                }
                Block::Table(table) => {
                    for cell in table.head.iter().chain(table.rows.iter().flatten()) {
                        self.inline(cell, None);
                    }
                }
                Block::Rule => {}
            }
        }
    }

    fn ranges(&self, range: Range<usize>) -> Vec<Range<usize>> {
        self.rendered.source_ranges(self.src, self.block, range)
    }

    fn plain(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.characters(false, &[]);
        self.block += 1;
    }

    fn inline(&mut self, inline: &Inline, heading: Option<u8>) {
        let pads = inline
            .spans
            .iter()
            .filter(|s| s.2 == Style::CodePad)
            .collect::<Vec<_>>();
        let visible = |at: usize| {
            at - pads
                .iter()
                .map(|s| at.min(s.1).saturating_sub(s.0))
                .sum::<usize>()
        };
        let len = visible(inline.text.len());
        if len == 0 {
            return;
        }
        if let Some(level) = heading {
            self.inks.push((self.ranges(0..len), Ink::Heading(level)));
        }
        let mut code = Vec::new();
        for &(a, b, style) in &inline.spans {
            let range = visible(a)..visible(b);
            let ink = match style {
                Style::Strong => Ink::Strong,
                Style::Code => {
                    code.push(range.clone());
                    Ink::Code
                }
                _ => continue,
            };
            self.inks.push((self.ranges(range), ink));
        }
        self.characters(true, &code);
        self.block += 1;
    }

    fn characters(&mut self, prose: bool, code: &[Range<usize>]) {
        let Some(text) = self.texts.get(self.block) else {
            return;
        };
        for (at, c) in text.char_indices() {
            let control = invisible(c);
            let soft = prose && c == ' ' && !code.iter().any(|r| r.contains(&at));
            if !control && !soft {
                continue;
            }
            let ranges = self.ranges(at..at + c.len_utf8());
            if let [range] = ranges.as_slice() {
                let original = &self.src[range.clone()];
                if control {
                    if !matches!(c, '\n' | '\r') || !original.contains(['\n', '\r']) {
                        self.edits.push((range.clone(), escape(c)));
                    }
                } else if original == "\n" || original == "\r\n" {
                    self.edits.push((range.clone(), format!("\\{original}")));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;

// rune-kit drops raw HTML, and an HTML block would take the prose after it down to the next blank
// line. The design shows tags as typed (`<design>` in a workflow's input), so every `<` that
// opens HTML is escaped before rendering. An HTML block is literal to its blank line, so a line
// of it that would open a block once its tag is text (`# Last call`, `- item`) has that mark
// escaped too.
fn raw_html(src: &str) -> Edits {
    use pulldown_cmark::{Event, Options, Parser};
    let mut edits = Edits::new();
    for (event, range) in Parser::new_ext(src, Options::all()).into_offset_iter() {
        let block = match event {
            Event::Html(_) => true,
            Event::InlineHtml(_) => false,
            _ => continue,
        };
        let text = &src[range.clone()];
        edits.extend(
            text.match_indices('<')
                .map(|(at, _)| (range.start + at..range.start + at + 1, "\\<".to_string())),
        );
        if let Some((at, c)) = text.char_indices().find(|(_, c)| !c.is_whitespace())
            && block
            && c != '<'
            && c.is_ascii_punctuation()
        {
            let at = range.start + at;
            edits.push((at..at + 1, format!("\\{c}")));
        }
    }
    edits
}
