//! How finding a link could fail:
//! - a link's range or destination is wrong, or an autolink or a reference link is missed;
//! - text that only looks like a link (code, plain words) counts;
//! - the link under the cursor loses to an earlier one in its block, or a block without a link
//!   takes one from another block.

use super::*;

const SOURCE: &str = "Read [the spec](https://example.com/spec?a=1) or <https://tau.dev>.\n\n\
                      `[no](https://code.example)` and [ref][r].\n\n\
                      Plain words.\n\n[r]: https://ref.example/x";

#[test]
fn links_are_found_with_their_ranges_and_destinations() {
    let all = links(SOURCE);
    let found: Vec<(&str, &str)> = all
        .iter()
        .map(|(r, url)| (&SOURCE[r.clone()], url.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            (
                "[the spec](https://example.com/spec?a=1)",
                "https://example.com/spec?a=1"
            ),
            ("<https://tau.dev>", "https://tau.dev"),
            ("[ref][r]", "https://ref.example/x"),
        ]
    );
}

#[test]
fn the_link_under_the_cursor_wins_else_the_first_of_its_block() {
    let first = 0..SOURCE.find("\n\n").unwrap();
    let on = SOURCE.find("tau.dev").unwrap();
    assert_eq!(
        link_at(SOURCE, on, first.clone()).as_deref(),
        Some("https://tau.dev")
    );
    assert_eq!(
        link_at(SOURCE, 0, first).as_deref(),
        Some("https://example.com/spec?a=1")
    );
    let plain = SOURCE.find("Plain").unwrap();
    assert_eq!(link_at(SOURCE, plain, plain..plain + 12), None);
}
