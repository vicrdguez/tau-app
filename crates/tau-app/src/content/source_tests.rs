// Failure modes:
// - The cap hides or invents a line, or adds the omission notice inside a fence.
// - Escapes or inserted hard breaks move a note away from its original source bytes.
// - Empty, reversed, out-of-bounds, or partial UTF-8 ranges panic or tint unrelated text.
// - A newline inside code changes, while a prose newline silently becomes a space.
// - Character entities smuggle invisible characters past source sanitization.
// - Heading, strong, and code colours leak into repeated text elsewhere.
// - Code padding, empty images, nested lists, or tables shift passage indexing.
// - Raw HTML (a workflow's `<design>` tag) is dropped, and an HTML block swallows the prose after it.
// - An HTML block's lines run together, or a `#` or `-` inside it turns into a heading or a list,
//   though the block is literal text; a line break before a closing tag is lost.

use super::{Ink, prepare};
use rune_kit::markdown::{Case, find};

#[test]
fn preserves_short_source_and_counts_terminal_newlines() {
    for src in ["", "text", "text\n", "text\r\n"] {
        let p = prepare(src);
        assert_eq!(p.text, src);
        assert_eq!(p.omitted, 0);
    }
    let src = "line\n".repeat(2000);
    assert_eq!(prepare(&src).text.lines().count(), 2000);
    assert_eq!(prepare(&src).omitted, 0);
}

#[test]
fn clips_source_at_two_thousand_lines_without_a_synthetic_fence() {
    let src = format!("```rust\n{}last", "let x = 1;\n".repeat(2000));
    let p = prepare(&src);
    assert_eq!(p.text.lines().count(), 2000);
    assert_eq!(p.omitted, 2);
    assert!(!p.text.contains("more lines"));
    assert_eq!(p.map(src.len() - 4..src.len()), None);
}

#[test]
fn hard_breaks_prose_but_not_code_or_structure() {
    let src = "first\nsecond\n\n`code\nspan`\n\n```text\na\nb\n```\n\n- one\n- two";
    let p = prepare(src);
    assert!(p.text.starts_with("first\\\nsecond\n\n"));
    assert!(p.text.contains("`code\nspan`"));
    assert!(p.text.contains("```text\na\nb\n```"));
    assert!(p.text.ends_with("- one\n- two"));
    assert!(!find(&p.text, "first\nsecond", Case::Sensitive).is_empty());
}

#[test]
fn escapes_controls_and_maps_source_bytes_after_growth() {
    let src = "é\u{202e}z\nnext\tend";
    let p = prepare(src);
    assert_eq!(p.text, "éU+202Ez\\\nnextU+0009end");
    assert_eq!(&p.text[p.map(2..5).unwrap()], "U+202E");
    let at = src.find("next").unwrap();
    assert_eq!(&p.text[p.map(at..at + 4).unwrap()], "next");
    assert_eq!(&p.text[p.map(1..2).unwrap()], "é");
    assert_eq!(&p.text[p.map(3..4).unwrap()], "U+202E");
    assert_eq!(p.map(5..5), None);
    assert_eq!(p.map(std::ops::Range { start: 8, end: 3 }), None);
    assert_eq!(p.map(usize::MAX..usize::MAX), None);
    assert_eq!(&p.text[p.map(src.len() - 3..usize::MAX).unwrap()], "end");
}

#[test]
fn escapes_invisible_entities_too() {
    let p = prepare("safe &#x202E;unsafe&#8203; &#10; &#13; &#9;x");
    assert_eq!(p.text, "safe U+202EunsafeU+200B U+000A U+000D U+0009x");
}

#[test]
fn colours_only_the_structured_occurrence_and_maps_code_padding() {
    let src = "# same\n\nsame **same** `é` **last**\n\n![ ](https://example.org/a)\n\n- ## nested\n\n| a | b |\n| - | - |\n| **steel** | `gold` |";
    let p = prepare(src);
    let passages = p
        .inks
        .iter()
        .map(|(rs, ink)| {
            (
                rs.iter().map(|r| &p.text[r.clone()]).collect::<String>(),
                *ink,
            )
        })
        .collect::<Vec<_>>();
    assert!(passages.contains(&("same".into(), Ink::Heading(1))));
    assert!(passages.contains(&("same".into(), Ink::Strong)));
    assert!(passages.contains(&("é".into(), Ink::Code)));
    assert!(passages.contains(&("last".into(), Ink::Strong)));
    assert!(passages.contains(&("nested".into(), Ink::Heading(2))));
    assert!(passages.contains(&("steel".into(), Ink::Strong)));
    assert!(passages.contains(&("gold".into(), Ink::Code)));
    assert_eq!(passages.iter().filter(|(s, _)| s == "same").count(), 2);
}

#[test]
fn an_html_block_keeps_its_lines_as_typed() {
    let src = "<design>\n# Last call per device\n- not a list\n</design>\n\nA column.\n</design>";
    let p = prepare(src);
    assert!(
        !p.inks.iter().any(|(_, ink)| matches!(ink, Ink::Heading(_))),
        "no heading: {:?}",
        p.text
    );
    let shown = rune_kit::markdown::Rendered::new(&p.text)
        .texts()
        .collect::<Vec<_>>()
        .join("|");
    assert!(
        shown.contains("<design>\n# Last call per device\n- not a list\n</design>"),
        "{shown:?}"
    );
    assert!(shown.contains("A column.\n</design>"), "{shown:?}");
    let at = src.find("Last").unwrap();
    assert_eq!(&p.text[p.map(at..at + 4).unwrap()], "Last");
    assert!(
        !p.inks
            .iter()
            .any(|(rs, ink)| *ink == Ink::Escape
                && rs.iter().any(|r| p.text[r.clone()].contains('<'))),
        "an escaped `<` draws as itself, not as an invisible character"
    );
}

#[test]
fn raw_html_shows_as_typed_and_keeps_the_text_after_it() {
    let src = "<design>\nKeep this line.\n</design>\n\nSee <b>bold</b> and `<code>`.";
    let p = prepare(src);
    let shown = rune_kit::markdown::Rendered::new(&p.text)
        .texts()
        .collect::<Vec<_>>()
        .join("\n");
    for typed in [
        "<design>",
        "Keep this line.",
        "</design>",
        "<b>bold</b>",
        "<code>",
    ] {
        assert!(shown.contains(typed), "{typed:?} is drawn: {shown:?}");
    }
    let at = src.find("Keep").unwrap();
    let mapped = p.map(at..at + 4).unwrap();
    assert_eq!(
        &p.text[mapped], "Keep",
        "a note on text after the tag stays on its bytes"
    );
}
