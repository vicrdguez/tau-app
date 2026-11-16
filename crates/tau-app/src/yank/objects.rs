//! Text objects over plain source text, in UTF-8 bytes: the sentence (`is as`) and the blanks an
//! `a` object adds. Words are the editor's (`crate::editor::text`).

use std::ops::Range;

pub use crate::editor::text::{a_word, inner_word};

/// What may close a sentence after its full stop: `(slowly.)`, `"done."`.
const CLOSERS: [char; 6] = [')', ']', '"', '\'', '’', '”'];

/// `is`: the sentence holding `at`, from its first letter through its full stop and closers;
/// `as` adds the blanks after it. A blank line ends a sentence too, and the text's end ends the
/// last one.
pub fn sentence(s: &str, at: usize, around: bool) -> Range<usize> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let n = chars.len();
    let byte = |k: usize| chars.get(k).map_or(s.len(), |c| c.0);
    let blank = |k: usize| chars.get(k).is_some_and(|c| c.1.is_whitespace());
    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    let mut i = 0;
    while blank(i) {
        i += 1;
    }
    let mut start = i;
    while i < n {
        let c = chars[i].1;
        let paragraph = c == '\n' && chars.get(i + 1).is_some_and(|x| x.1 == '\n');
        if matches!(c, '.' | '!' | '?') || paragraph {
            let mut j = i;
            if !paragraph {
                j += 1;
                while j < n && CLOSERS.contains(&chars[j].1) {
                    j += 1;
                }
            }
            if j >= n || blank(j) {
                let mut k = j;
                while blank(k) {
                    k += 1;
                }
                spans.push((byte(start), byte(j), byte(k)));
                start = k;
                i = k;
                continue;
            }
        }
        i += 1;
    }
    if start < n {
        let end = s.trim_end().len().max(byte(start));
        spans.push((byte(start), end, s.len()));
    }
    let span = spans
        .iter()
        .find(|(a, _, after)| *a <= at && at < *after)
        .or_else(|| spans.iter().rev().find(|(a, ..)| *a <= at))
        .or(spans.first());
    match span {
        Some(&(a, end, after)) => a..if around { after } else { end },
        None => at..at,
    }
}

/// `r` with the white space after it, up to the next text: what `ap` and `am` add.
pub fn spread(s: &str, r: Range<usize>) -> Range<usize> {
    let rest = &s[r.end..];
    let blanks = rest.len() - rest.trim_start().len();
    r.start..r.end + blanks
}

#[cfg(test)]
#[path = "objects_tests.rs"]
mod tests;
