//! The basis as the reader moves over it: its blocks with their source ranges, the visible text
//! the caret may sit on, the reader's motions, and how a selection becomes a note's range
//! (`spec.md` §5.6 "Notes and ranges"). Offsets are UTF-8 bytes of `text`; the server counts
//! UTF-16 code units, converted only at the edge ([`utf16_of`], [`byte_of`]).

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Heading(u8),
    Paragraph,
    /// One item of a top-level list, with its marker.
    Item,
    Code,
    Table,
    Quote,
    Rule,
    Html,
    /// A line of the source view.
    Line,
}

/// One block the reader's cursor sits on: a top-level block, or one item of a top-level list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The block's source, trimmed of trailing white space.
    pub range: Range<usize>,
    pub kind: Kind,
    /// The source of the text the reader sees, in order: plain text and code, never markup.
    pub runs: Vec<Range<usize>>,
}

/// A note's range as a selection maps to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteRange {
    pub range: Range<usize>,
    /// The selection crossed markup or blocks, so the note takes the blocks it touched.
    pub widened: bool,
}

/// The basis as blocks: rendered (Markdown) or as source lines (`␣ts`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Doc {
    pub blocks: Vec<Block>,
    text_len: usize,
    chars: Vec<(usize, char)>,
}

/// UTF-16 code units of `text` before `byte`.
pub fn utf16_of(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].encode_utf16().count()
}

/// The byte offset of `units` UTF-16 code units into `text`; `None` past its end or inside a
/// surrogate pair.
pub fn byte_of(text: &str, units: usize) -> Option<usize> {
    let mut n = 0;
    for (i, c) in text.char_indices() {
        if n == units {
            return Some(i);
        }
        if n > units {
            return None;
        }
        n += c.len_utf16();
    }
    (n == units).then_some(text.len())
}

impl Doc {
    /// The Markdown's blocks, each list item its own.
    pub fn rendered(text: &str) -> Doc {
        let opts =
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
        let mut blocks = Vec::new();
        let mut depth = 0usize;
        let mut list = false;
        let mut open: Option<Block> = None;
        for (event, r) in Parser::new_ext(text, opts).into_offset_iter() {
            match event {
                Event::Start(tag) => {
                    let starts = match (&tag, depth, list) {
                        (Tag::List(_), 0, _) => {
                            list = true;
                            None
                        }
                        (Tag::Item, 1, true) => Some(Kind::Item),
                        (_, 0, _) => Some(kind(&tag)),
                        _ => None,
                    };
                    if let Some(kind) = starts {
                        open = Some(Block {
                            range: r.clone(),
                            kind,
                            runs: Vec::new(),
                        });
                    }
                    depth += 1;
                }
                Event::End(end) => {
                    depth -= 1;
                    let closes = match (end, depth, list) {
                        (TagEnd::List(_), 0, _) => {
                            list = false;
                            false
                        }
                        (TagEnd::Item, 1, true) => true,
                        (_, 0, _) => true,
                        _ => false,
                    };
                    if closes && let Some(mut b) = open.take() {
                        b.range = trimmed(text, b.range.start..r.end);
                        blocks.push(b);
                    }
                }
                Event::Text(_) => {
                    if let Some(b) = open.as_mut() {
                        b.runs.push(r);
                    }
                }
                Event::Code(_) => {
                    if let Some(b) = open.as_mut() {
                        b.runs.push(code_inner(text, r));
                    }
                }
                Event::Rule if depth == 0 => blocks.push(Block {
                    range: trimmed(text, r),
                    kind: Kind::Rule,
                    runs: Vec::new(),
                }),
                _ => {}
            }
        }
        Doc::new(text, blocks)
    }

    /// The source view: one block per line, every character visible.
    pub fn source(text: &str) -> Doc {
        let mut blocks = Vec::new();
        let mut at = 0;
        for line in text.split_inclusive('\n') {
            let r = at..at + line.trim_end_matches(['\n', '\r']).len();
            at += line.len();
            if !r.is_empty() {
                blocks.push(Block {
                    range: r.clone(),
                    kind: Kind::Line,
                    runs: vec![r],
                });
            }
        }
        Doc::new(text, blocks)
    }

    fn new(text: &str, blocks: Vec<Block>) -> Doc {
        Doc {
            blocks,
            text_len: text.len(),
            chars: text.char_indices().collect(),
        }
    }

    /// Every caret position in text order: the start of each visible character.
    pub fn positions(&self) -> Vec<usize> {
        self.blocks
            .iter()
            .flat_map(|b| b.runs.iter())
            .flat_map(|r| self.chars_in(r.clone()))
            .collect()
    }

    fn chars_in(&self, r: Range<usize>) -> impl Iterator<Item = usize> + '_ {
        let from = self.chars.partition_point(|(i, _)| *i < r.start);
        self.chars[from..]
            .iter()
            .take_while(move |(i, _)| *i < r.end)
            .filter(|(_, c)| *c != '\n' && *c != '\r')
            .map(|(i, _)| *i)
    }

    fn block_positions(&self, block: usize) -> Vec<usize> {
        self.blocks[block]
            .runs
            .iter()
            .flat_map(|r| self.chars_in(r.clone()))
            .collect()
    }

    fn char_at(&self, at: usize) -> char {
        let i = self.chars.partition_point(|(i, _)| *i < at);
        self.chars.get(i).map_or(' ', |(_, c)| *c)
    }

    /// The end of the character at `at`.
    pub fn after(&self, at: usize) -> usize {
        let i = self.chars.partition_point(|(i, _)| *i <= at);
        self.chars.get(i).map_or(self.text_len, |(i, _)| *i)
    }

    /// The block holding `at`, else the last block starting before it.
    pub fn block_of(&self, at: usize) -> Option<usize> {
        let i = self.blocks.partition_point(|b| b.range.start <= at);
        i.checked_sub(1)
    }

    /// The first caret position of `block`, or of the next block that has one.
    pub fn first(&self, block: usize) -> Option<usize> {
        (block..self.blocks.len()).find_map(|b| self.block_positions(b).first().copied())
    }

    /// The caret clamped to a visible position: itself, else the next one, else the last.
    pub fn snap(&self, at: usize) -> Option<usize> {
        let p = self.positions();
        p.iter().copied().find(|&q| q >= at).or(p.last().copied())
    }

    /// `j` and `k`: the first position of the block `by` blocks on; `None` past either end.
    pub fn block_step(&self, at: usize, by: i64) -> Option<usize> {
        let from = self.block_of(at)? as i64;
        let mut b = from;
        let step = by.signum();
        let mut left = by.abs();
        while left > 0 {
            b += step;
            if b < 0 || b >= self.blocks.len() as i64 {
                return None;
            }
            if !self.block_positions(b as usize).is_empty() {
                left -= 1;
            }
        }
        self.block_positions(b as usize).first().copied()
    }

    /// `h` and `l`: within the block.
    pub fn char_step(&self, at: usize, by: i64) -> usize {
        let Some(b) = self.block_of(at) else {
            return at;
        };
        let p = self.block_positions(b);
        let i = p.iter().position(|&q| q >= at).unwrap_or(0) as i64;
        p.get((i + by).clamp(0, p.len() as i64 - 1) as usize)
            .copied()
            .unwrap_or(at)
    }

    /// `0`.
    pub fn line_start(&self, at: usize) -> usize {
        self.block_of(at)
            .and_then(|b| self.block_positions(b).first().copied())
            .unwrap_or(at)
    }

    /// `$`.
    pub fn line_end(&self, at: usize) -> usize {
        self.block_of(at)
            .and_then(|b| self.block_positions(b).last().copied())
            .unwrap_or(at)
    }

    fn class(&self, at: usize) -> u8 {
        let c = self.char_at(at);
        if c.is_whitespace() {
            0
        } else if c.is_alphanumeric() || c == '_' {
            1
        } else {
            2
        }
    }

    /// Word starts: a visible, non-blank character after a blank, another class or a block edge.
    fn word_starts(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for b in 0..self.blocks.len() {
            let mut prev: Option<(usize, u8)> = None;
            for p in self.block_positions(b) {
                let c = self.class(p);
                let contiguous = prev.is_some_and(|(q, _)| self.after(q) == p);
                if c != 0 && (!contiguous || prev.is_none_or(|(_, pc)| pc != c)) {
                    out.push(p);
                }
                prev = Some((p, c));
            }
        }
        out
    }

    /// `w`.
    pub fn word_forward(&self, at: usize) -> usize {
        self.word_starts()
            .into_iter()
            .find(|&p| p > at)
            .unwrap_or(at)
    }

    /// `b`.
    pub fn word_back(&self, at: usize) -> usize {
        self.word_starts()
            .into_iter()
            .rev()
            .find(|&p| p < at)
            .unwrap_or(at)
    }

    /// `e`: the last character of this word, or of the next.
    pub fn word_end(&self, at: usize) -> usize {
        let p = self.positions();
        let Some(mut i) = p.iter().position(|&q| q > at) else {
            return at;
        };
        while i < p.len() && self.class(p[i]) == 0 {
            i += 1;
        }
        let Some(&first) = p.get(i) else { return at };
        let c = self.class(first);
        while i + 1 < p.len()
            && self.after(p[i]) == p[i + 1]
            && self.class(p[i + 1]) == c
            && self.block_of(p[i]) == self.block_of(p[i + 1])
        {
            i += 1;
        }
        p[i]
    }

    /// `iw`, and `aw` with the blanks after it.
    pub fn word_around(&self, at: usize, with_blank: bool) -> Range<usize> {
        let b = self.block_of(at).unwrap_or(0);
        let p = self.block_positions(b);
        let Some(i) = p.iter().position(|&q| q == at) else {
            return at..self.after(at);
        };
        let c = self.class(at);
        let joined = |a: usize, b: usize| {
            self.after(p[a]) == p[b] && self.class(p[a]) == c && self.class(p[b]) == c
        };
        let mut s = i;
        while s > 0 && joined(s - 1, s) {
            s -= 1;
        }
        let mut e = i;
        while e + 1 < p.len() && joined(e, e + 1) {
            e += 1;
        }
        if with_blank {
            while e + 1 < p.len() && self.class(p[e + 1]) == 0 {
                e += 1;
            }
        }
        p[s]..self.after(p[e])
    }

    /// How a selection becomes a note's range: exact inside one run of plain text (or anywhere in
    /// the source view), else the whole blocks it touches.
    pub fn note_range(&self, _text: &str, sel: Range<usize>) -> NoteRange {
        let exact = self
            .blocks
            .iter()
            .flat_map(|b| b.runs.iter())
            .any(|r| r.start <= sel.start && sel.end <= r.end);
        if exact {
            return NoteRange {
                range: sel,
                widened: false,
            };
        }
        let first = self.block_of(sel.start).unwrap_or(0);
        let last = self
            .block_of(sel.end.saturating_sub(1).max(sel.start))
            .unwrap_or(first);
        NoteRange {
            range: self.blocks[first].range.start..self.blocks[last].range.end,
            widened: true,
        }
    }
}

fn kind(tag: &Tag) -> Kind {
    match tag {
        Tag::Heading { level, .. } => Kind::Heading(*level as u8),
        Tag::CodeBlock(_) => Kind::Code,
        Tag::Table(_) => Kind::Table,
        Tag::BlockQuote(_) => Kind::Quote,
        Tag::HtmlBlock => Kind::Html,
        _ => Kind::Paragraph,
    }
}

fn trimmed(text: &str, r: Range<usize>) -> Range<usize> {
    r.start..r.start + text[r].trim_end().len()
}

/// A code span's text without its backticks and the one space each side may add.
fn code_inner(text: &str, r: Range<usize>) -> Range<usize> {
    let s = &text[r.clone()];
    let ticks = s.len() - s.trim_start_matches('`').len();
    let mut a = r.start + ticks;
    let mut b = r.end.saturating_sub(ticks).max(a);
    let inner = &text[a..b];
    if inner.len() >= 2
        && inner.starts_with(' ')
        && inner.ends_with(' ')
        && !inner.trim().is_empty()
    {
        a += 1;
        b -= 1;
    }
    a..b
}
