//! Motions and objects over plain text, by byte offset on character boundaries. Lines end at
//! `\n`; a line's end is the offset of its `\n` (or of the text's end).

use std::ops::Range;

pub fn ch(s: &str, i: usize) -> Option<char> {
    s.get(i..).and_then(|t| t.chars().next())
}

pub fn next(s: &str, i: usize) -> usize {
    ch(s, i).map_or(s.len(), |c| i + c.len_utf8())
}

pub fn prev(s: &str, i: usize) -> usize {
    s[..i].char_indices().next_back().map_or(0, |(j, _)| j)
}

pub fn line_start(s: &str, i: usize) -> usize {
    s[..i].rfind('\n').map_or(0, |j| j + 1)
}

pub fn line_end(s: &str, i: usize) -> usize {
    s[i..].find('\n').map_or(s.len(), |j| i + j)
}

pub fn line_index(s: &str, i: usize) -> usize {
    s[..i].matches('\n').count()
}

pub fn line_count(s: &str) -> usize {
    s.matches('\n').count() + 1
}

/// The start of 0-based line `n`, or of the last line past the end.
pub fn line_at(s: &str, n: usize) -> usize {
    let mut start = 0;
    for _ in 0..n {
        match s[start..].find('\n') {
            Some(j) => start += j + 1,
            None => break,
        }
    }
    start
}

pub fn first_non_blank(s: &str, i: usize) -> usize {
    let start = line_start(s, i);
    let end = line_end(s, i);
    s[start..end]
        .find(|c: char| c != ' ' && c != '\t')
        .map_or(end, |j| start + j)
}

/// The last character of `i`'s line, where Normal mode's cursor stops (the line's start when
/// it is empty).
pub fn last_char(s: &str, i: usize) -> usize {
    let (start, end) = (line_start(s, i), line_end(s, i));
    if end > start { prev(s, end) } else { start }
}

/// Characters from the line's start to `i`.
pub fn column(s: &str, i: usize) -> usize {
    s[line_start(s, i)..i].chars().count()
}

/// The offset `col` characters into the line starting at `start`, stopping at its last
/// character (`normal`) or at its end.
pub fn at_column(s: &str, start: usize, col: usize, normal: bool) -> usize {
    let end = line_end(s, start);
    let at = s[start..end]
        .char_indices()
        .nth(col)
        .map_or(end, |(j, _)| start + j);
    if normal && at == end && end > start {
        prev(s, end)
    } else {
        at
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Space,
    Word,
    Punct,
}

fn class(c: char) -> Class {
    if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Punct
    }
}

/// `w`: the next word's start. An empty line counts as a word.
pub fn word_forward(s: &str, i: usize) -> usize {
    let n = s.len();
    let mut j = i;
    if let Some(c0) = ch(s, i).map(class).filter(|c| *c != Class::Space) {
        while ch(s, j).is_some_and(|c| class(c) == c0) {
            j = next(s, j);
        }
    }
    while let Some(c) = ch(s, j) {
        if c == '\n' {
            j = next(s, j);
            if ch(s, j) == Some('\n') {
                return j;
            }
        } else if c.is_whitespace() {
            j = next(s, j);
        } else {
            break;
        }
    }
    j.min(n)
}

/// `b`: the start of this word, or of the one before.
pub fn word_back(s: &str, i: usize) -> usize {
    if i == 0 {
        return 0;
    }
    let mut j = prev(s, i);
    while j > 0 && ch(s, j).is_some_and(char::is_whitespace) {
        if ch(s, j) == Some('\n') && ch(s, prev(s, j)) == Some('\n') {
            return j;
        }
        j = prev(s, j);
    }
    let Some(c0) = ch(s, j).map(class) else {
        return j;
    };
    if c0 == Class::Space {
        return j;
    }
    while j > 0 && ch(s, prev(s, j)).is_some_and(|c| class(c) == c0) {
        j = prev(s, j);
    }
    j
}

/// `e`: the end of this word, or of the next.
pub fn word_end(s: &str, i: usize) -> usize {
    let mut j = next(s, i);
    while ch(s, j).is_some_and(char::is_whitespace) {
        j = next(s, j);
    }
    let Some(c0) = ch(s, j).map(class) else {
        return i;
    };
    while ch(s, next(s, j)).is_some_and(|c| class(c) == c0) {
        j = next(s, j);
    }
    j
}

/// `iw`: the run of one class under `i`, within its line.
pub fn inner_word(s: &str, i: usize) -> Range<usize> {
    let Some(c0) = ch(s, i).filter(|c| *c != '\n').map(class) else {
        return i..i;
    };
    let same = |c: char| c != '\n' && class(c) == c0;
    let mut a = i;
    while a > 0 && ch(s, prev(s, a)).is_some_and(same) {
        a = prev(s, a);
    }
    let mut b = i;
    while ch(s, b).is_some_and(same) {
        b = next(s, b);
    }
    a..b
}

/// `aw`: the word and the blanks after it, or before it when none follow.
pub fn a_word(s: &str, i: usize) -> Range<usize> {
    let w = inner_word(s, i);
    let blank = |c: char| c == ' ' || c == '\t';
    if ch(s, i).is_some_and(blank) {
        let mut b = w.end;
        if ch(s, b).is_some_and(|c| c != '\n') {
            b = inner_word(s, b).end;
        }
        return w.start..b;
    }
    let mut b = w.end;
    while ch(s, b).is_some_and(blank) {
        b = next(s, b);
    }
    if b > w.end {
        return w.start..b;
    }
    let mut a = w.start;
    while a > line_start(s, w.start) && ch(s, prev(s, a)).is_some_and(blank) {
        a = prev(s, a);
    }
    a..w.end
}

/// The text of lines `first..=last` as a linewise register holds it, ending in `\n`.
pub fn lines_text(s: &str, first: usize, last: usize) -> String {
    let a = line_at(s, first);
    let b = line_end(s, line_at(s, last));
    format!("{}\n", &s[a..b])
}

/// The span that deleting lines `first..=last` removes, line break included.
pub fn lines_span(s: &str, first: usize, last: usize) -> Range<usize> {
    let a = line_at(s, first);
    let b = line_end(s, line_at(s, last));
    if b < s.len() {
        a..b + 1
    } else if a > 0 {
        a - 1..b
    } else {
        a..b
    }
}

/// The Markdown list marker a new line continues: `- `, `* `, `+ ` or the next `N. `, with the
/// line's indent. `None` when the line holds no list item.
pub fn list_marker(line: &str) -> Option<(String, bool)> {
    let body = line.trim_start_matches([' ', '\t']);
    let indent = &line[..line.len() - body.len()];
    for bullet in ["- ", "* ", "+ "] {
        if let Some(rest) = body.strip_prefix(bullet) {
            return Some((format!("{indent}{bullet}"), rest.trim().is_empty()));
        }
    }
    let digits = body.chars().take_while(char::is_ascii_digit).count();
    let rest = body[digits..].strip_prefix(". ")?;
    let n: u64 = body[..digits].parse().ok()?;
    Some((format!("{indent}{}. ", n + 1), rest.trim().is_empty()))
}
