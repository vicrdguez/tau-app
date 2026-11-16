//! How the basis document could fail (`spec.md` §5.6 "Notes and ranges", FA7):
//! - a list is one block, so the cursor can't sit on one bullet, or an item loses its marker;
//! - a block's range keeps its trailing newline or blank line;
//! - UTF-16 offsets drift on a multi-byte or astral character, or land inside a pair;
//! - a selection inside one plain run widens, or one crossing a code span stays exact;
//! - a selection across two blocks takes only the first;
//! - the source view widens a range;
//! - the caret lands on markup (`` ` ``, `#`, `- `) that the reader never sees;
//! - `w b e 0 $` and `j k` leave the block or skip one;
//! - the fixture's draft at 882..951 is not the sentence it quotes.

use super::doc::*;

static TEXT: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    let f: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../design/fixture.json")).unwrap();
    f["calls"]["decision"]["render-core/w1:3:design#2"]["result"]["text"]
        .as_str()
        .unwrap()
        .to_owned()
});

/// The basis of `w1:3:design#2`.
fn design() -> &'static str {
    &TEXT
}

fn doc() -> Doc {
    Doc::rendered(design())
}

fn slice(r: std::ops::Range<usize>) -> &'static str {
    &design()[r]
}

#[test]
fn each_list_item_is_its_own_block_with_its_marker() {
    let d = doc();
    let items: Vec<_> = d
        .blocks
        .iter()
        .filter(|b| b.kind == Kind::Item)
        .map(|b| slice(b.range.clone()))
        .collect();
    assert!(items[0].starts_with("- Lines end with"), "{items:?}");
    assert!(items[2].starts_with("- A line is at most 1 MiB"));
    assert!(items.iter().any(|i| i.starts_with("2. 500 entries")));
}

#[test]
fn blocks_are_trimmed_of_trailing_newlines() {
    for b in &doc().blocks {
        let s = slice(b.range.clone());
        assert!(!s.ends_with('\n') && !s.is_empty(), "{s:?}");
    }
}

#[test]
fn the_first_block_is_the_h1_and_the_code_and_table_are_whole() {
    let d = doc();
    assert_eq!(d.blocks[0].kind, Kind::Heading(1));
    assert_eq!(
        slice(d.blocks[0].range.clone()),
        "# NDJSON export of the event log"
    );
    let code = d.blocks.iter().find(|b| b.kind == Kind::Code).unwrap();
    assert!(slice(code.range.clone()).starts_with("```json"));
    assert!(slice(code.range.clone()).ends_with("```"));
    let table = d.blocks.iter().find(|b| b.kind == Kind::Table).unwrap();
    assert!(slice(table.range.clone()).ends_with("`{\"done\":true,\"events\":412}`. |"));
}

#[test]
fn utf16_round_trips_over_multibyte_and_astral_text() {
    let s = "aé😀b";
    assert_eq!(utf16_of(s, 0), 0);
    assert_eq!(utf16_of(s, 1), 1);
    assert_eq!(utf16_of(s, 3), 2, "é is one unit");
    assert_eq!(utf16_of(s, 7), 4, "😀 is two units");
    assert_eq!(byte_of(s, 4), Some(7));
    assert_eq!(byte_of(s, 3), None, "inside a surrogate pair");
    assert_eq!(byte_of(s, 5), Some(8));
    assert_eq!(byte_of(s, 6), None, "past the end");
}

#[test]
fn the_fixture_draft_quotes_its_sentence() {
    let a = byte_of(design(), 882).unwrap();
    let b = byte_of(design(), 951).unwrap();
    assert_eq!(
        &design()[a..b],
        "A longer event is written whole; the limit is reported, not enforced."
    );
}

#[test]
fn a_selection_inside_one_run_is_exact() {
    let d = doc();
    let at = design().find("two exports of the same").unwrap();
    let end = design().find("byte-equal.").unwrap() + "byte-equal.".len();
    let n = d.note_range(design(), at..end);
    assert_eq!((n.range, n.widened), (at..end, false));
}

#[test]
fn a_selection_across_a_code_span_takes_its_block() {
    let d = doc();
    let at = design().find("Keys keep the order").unwrap();
    let end = design().find("gives them").unwrap();
    let n = d.note_range(design(), at..end);
    assert!(n.widened);
    assert!(slice(n.range).starts_with("- Keys keep the order"));
}

#[test]
fn a_selection_across_blocks_takes_every_block_it_touches() {
    let d = doc();
    let at = design().find("no trailing comma").unwrap();
    let end = design().find("Keys keep the order").unwrap() + 4;
    let n = d.note_range(design(), at..end);
    assert!(n.widened);
    let s = slice(n.range);
    assert!(
        s.starts_with("- Lines end with") && s.ends_with("byte-equal."),
        "{s}"
    );
}

#[test]
fn a_selection_inside_code_is_exact() {
    let d = doc();
    let at = design().find("\"kind\":\"w1.event\"").unwrap();
    let n = d.note_range(design(), at..at + 6);
    assert!(!n.widened);
}

#[test]
fn the_source_view_keeps_any_range() {
    let d = Doc::source(design());
    let at = design().find("`history`").unwrap();
    let n = d.note_range(design(), at..at + 5);
    assert_eq!((n.range, n.widened), (at..at + 5, false));
    assert!(
        d.blocks
            .iter()
            .all(|b| !slice(b.range.clone()).contains('\n'))
    );
}

#[test]
fn the_caret_never_sits_on_markup() {
    let d = doc();
    let positions = d.positions();
    let span = design().find("`history`").unwrap();
    assert!(!positions.contains(&span), "the code span's backtick");
    assert!(
        positions.contains(&(span + 1)),
        "the code span's first letter"
    );
    assert!(!positions.contains(&0), "the heading's #");
    let item = design().find("- Lines end").unwrap();
    assert!(!positions.contains(&item), "the list marker");
    assert!(positions.contains(&(item + 2)));
    let first = d.first(0).unwrap();
    assert_eq!(&design()[first..first + 6], "NDJSON");
}

#[test]
fn word_and_line_motions_stay_in_text_order() {
    let d = doc();
    let start = d.first(0).unwrap();
    let w = d.word_forward(start);
    assert_eq!(&design()[w..w + 6], "export");
    assert_eq!(d.word_back(w), start);
    let e = d.word_end(start);
    assert_eq!(&design()[e..e + 1], "N", "the last letter of NDJSON");
    let end = d.line_end(start);
    assert_eq!(&design()[end..end + 1], "g");
    let next = d.block_step(start, 1).unwrap();
    assert_eq!(&design()[next..next + 4], "tau ");
    assert_eq!(d.block_step(next, -1), Some(start));
    assert_eq!(d.block_step(start, -1), None);
}
