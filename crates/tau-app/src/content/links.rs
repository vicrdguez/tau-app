//! The links in Markdown source, for `gx` (`grammar.md` §4): where each sits in the source and
//! where it goes.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag};

/// Every link in `source`, in order: its source range (UTF-8 bytes) and its destination.
pub fn links(source: &str) -> Vec<(Range<usize>, String)> {
    Parser::new_ext(source, Options::all())
        .into_offset_iter()
        .filter_map(|(event, range)| match event {
            Event::Start(Tag::Link { dest_url, .. }) => Some((range, dest_url.to_string())),
            _ => None,
        })
        .collect()
}

/// The destination of the link holding byte `at`, else of the first link in `block`.
pub fn link_at(source: &str, at: usize, block: Range<usize>) -> Option<String> {
    let all = links(source);
    all.iter()
        .find(|(r, _)| r.contains(&at))
        .or_else(|| all.iter().find(|(r, _)| block.contains(&r.start)))
        .map(|(_, url)| url.clone())
}

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;
