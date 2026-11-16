// Failure modes:
// - A nested list item keeps its parent's indentation, so its body reparses as code or as
//   the wrong list, and its marker level is lost.
// - A continuation line or a lazy line of an item drops out of the item, or keeps the indent.
// - A task item shows its `[x]` as text, or loses whether it is done.
// - An ordered list forgets its start number.
// - A quote keeps its `>` markers, so its body reparses as another quote; a lazy line is lost.
// - An alert shows `[!NOTE]` as text (also once its line ends in an inserted hard break), or an
//   unknown kind turns a plain quote into an alert.
// - A fence keeps its fence lines, loses its language, or an unclosed fence (streaming) loses
//   its last lines; `diff` and `mermaid` fences take another path than other code.
// - An image on its own line is shown as text; an image inside a sentence splits the
//   sentence wrongly; an image inside a link turns into a placeholder.
// - A note's range over removed markers or indentation maps into the wrong local bytes, or
//   a range outside a block paints inside it.

use super::{Cut, Node, outline};

fn show(text: &str, nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(|n| match n {
            Node::Heading(level, cut) => format!("h{level} {:?}", cut.text(text)),
            Node::Prose(cut) => format!("p {:?}", cut.text(text)),
            Node::Image { alt, host } => format!("img {alt:?} {host:?}"),
            Node::Code { lang, code } => format!("code {lang:?} {:?}", code.text(text)),
            Node::List { start, items } => {
                let items = items
                    .iter()
                    .map(|i| {
                        let task = match i.task {
                            Some(true) => "[x] ",
                            Some(false) => "[ ] ",
                            None => "",
                        };
                        format!("{task}{}", show(text, &i.body))
                    })
                    .collect::<Vec<_>>();
                format!("list {start:?} [{}]", items.join(", "))
            }
            Node::Quote { alert, body } => format!("quote {alert:?} [{}]", show(text, body)),
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn outlined(text: &str) -> String {
    show(text, &outline(text))
}

#[test]
fn top_level_blocks_keep_their_source() {
    assert_eq!(outlined(""), "");
    assert_eq!(
        outlined("# Title\n\nSome *text*\n\n---\n\n| a | b |\n| - | - |\n| 1 | 2 |\n"),
        "h1 \"# Title\" | p \"Some *text*\" | p \"---\" | p \"| a | b |\\n| - | - |\\n| 1 | 2 |\""
    );
    assert_eq!(outlined("Title\n==="), "h1 \"Title\\n===\"");
}

#[test]
fn nested_items_lose_their_parents_indentation() {
    assert_eq!(
        outlined("- a\n  - b\n    - c\n- d"),
        "list None [p \"a\" | list None [p \"b\" | list None [p \"c\"]], p \"d\"]"
    );
    assert_eq!(
        outlined("1. one\n   continued\nlazy\n\n   second para"),
        "list Some(1) [p \"one\\ncontinued\\nlazy\" | p \"second para\"]"
    );
    assert_eq!(outlined("3. c\n4. d"), "list Some(3) [p \"c\", p \"d\"]");
    assert_eq!(
        outlined("-     indented code"),
        "list None [code None \"indented code\"]"
    );
}

#[test]
fn task_items_drop_their_box_from_the_text() {
    assert_eq!(
        outlined("- [x] done\n- [ ] open\n- plain"),
        "list None [[x] p \"done\", [ ] p \"open\", p \"plain\"]"
    );
}

#[test]
fn quotes_lose_their_markers_and_keep_lazy_lines() {
    assert_eq!(
        outlined("> a\n> b\nlazy\n>\n> > inner"),
        "quote None [p \"a\\nb\\nlazy\" | quote None [p \"inner\"]]"
    );
    assert_eq!(
        outlined(">- item\n>  more"),
        "quote None [list None [p \"item\\nmore\"]]"
    );
}

#[test]
fn alerts_take_their_kind_and_drop_the_marker_line() {
    assert_eq!(
        outlined("> [!WARNING]\n> Mind the gap."),
        "quote Some(\"Warning\") [p \"Mind the gap.\"]"
    );
    assert_eq!(
        outlined("> [!note]\\\n> After a hard break."),
        "quote Some(\"Note\") [p \"After a hard break.\"]"
    );
    assert_eq!(
        outlined("> [!SHOUT]\n> Not an alert."),
        "quote None [p \"[!SHOUT]\\nNot an alert.\"]"
    );
}

#[test]
fn every_fence_is_code_with_its_language_and_no_fence_lines() {
    assert_eq!(
        outlined("```rust title\nfn a() {}\n```"),
        "code Some(\"rust\") \"fn a() {}\\n\""
    );
    assert_eq!(
        outlined("```diff\n+a\n-b\n```\n\n```mermaid\ngraph TD\n```"),
        "code Some(\"diff\") \"+a\\n-b\\n\" | code Some(\"mermaid\") \"graph TD\\n\""
    );
    assert_eq!(
        outlined("```json\n{\"a\": 1,\n  \"b\""),
        "code Some(\"json\") \"{\\\"a\\\": 1,\\n  \\\"b\\\"\""
    );
    assert_eq!(
        outlined("- item\n\n  ```sh\n  ls -la\n  ```"),
        "list None [p \"item\" | code Some(\"sh\") \"ls -la\\n\"]"
    );
}

#[test]
fn an_image_standing_alone_is_a_placeholder_and_inline_ones_split_the_line() {
    assert_eq!(
        outlined("![Diagram alt](https://user@example.org:8080/d.png \"t\")"),
        "img \"Diagram alt\" \"example.org:8080\""
    );
    assert_eq!(
        outlined("before ![x](pic.png) after"),
        "p \"before\" | img \"x\" \"\" | p \"after\""
    );
    assert_eq!(
        outlined("[![badge](https://ci.example/b.svg)](https://ci.example)"),
        "p \"[![badge](https://ci.example/b.svg)](https://ci.example)\""
    );
}

#[test]
fn a_cut_maps_source_ranges_through_removed_markers() {
    let text = "> one\n> two";
    let nodes = outline(text);
    let Node::Quote { body, .. } = &nodes[0] else {
        panic!("a quote: {}", show(text, &nodes));
    };
    let Node::Prose(cut) = &body[0] else {
        panic!("a paragraph")
    };
    assert_eq!(cut.text(text), "one\ntwo");
    assert_eq!(cut.map(2..5), vec![0..3]);
    assert_eq!(cut.map(4..10), vec![2..4, 4..6]);
    assert_eq!(cut.map(6..8), Vec::<std::ops::Range<usize>>::new());
    assert_eq!(cut.map(20..30), Vec::<std::ops::Range<usize>>::new());
    assert_eq!(
        Cut(std::iter::once(0..3).collect()).map(0..0),
        Vec::<std::ops::Range<usize>>::new()
    );
}

// Added after the code: half-written or hostile Markdown must never panic the outline or cut a
// piece inside a character.
#[test]
fn any_source_outlines_into_pieces_on_character_boundaries() {
    fn check(src: &str, nodes: &[Node]) {
        let cut = |c: &Cut| {
            for r in &c.0 {
                assert!(r.start <= r.end && r.end <= src.len(), "{src:?}: {r:?}");
                assert!(src.is_char_boundary(r.start) && src.is_char_boundary(r.end));
            }
        };
        for n in nodes {
            match n {
                Node::Heading(_, c) | Node::Prose(c) | Node::Code { code: c, .. } => cut(c),
                Node::Image { .. } => {}
                Node::List { items, .. } => items.iter().for_each(|i| check(src, &i.body)),
                Node::Quote { body, .. } => check(src, body),
            }
        }
    }
    let alphabet = [
        "-", " ", "  ", "\n", "\r\n", ">", "1.", "2)", "```", "~~~", "[x]", "[ ]", "![a](b)",
        "[!NOTE]", "é", "\\", "|", "#", "*", "\t", "<b>", "    ",
    ];
    let mut seed = 0x2545_f491_u64;
    for _ in 0..20_000 {
        let mut src = String::new();
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        for k in 0..(seed >> 60) as usize + 1 {
            let pick = (seed >> (k * 5 % 55)) as usize % alphabet.len();
            src.push_str(alphabet[pick]);
        }
        check(&src, &outline(&src));
    }
}
