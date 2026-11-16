// Failure modes:
// - Markdown or unmatched backticks disappear from a row.
// - Delimiters of different lengths pair, or an inner backtick is lost.
// - Controls, bidi overrides, or invisible separators reach the shaper unchanged.
// - Escaping destroys ordinary Unicode or leaves empty text runs.
// - A long label is shortened before rune knows its available width.

use super::{Run, runs};

#[test]
fn leaves_markdown_and_unmatched_backticks_literal() {
    for s in [
        "",
        "# Needs you",
        "**ok** [open](command:run)",
        "`unfinished",
        "a``b",
    ] {
        let expected = if s.is_empty() {
            vec![]
        } else {
            vec![Run::Text(s.into())]
        };
        assert_eq!(runs(s), expected);
    }
}

#[test]
fn drops_only_matching_code_delimiters() {
    assert_eq!(
        runs("open `notes.json` and ``a`b`` then `c`"),
        vec![Run::Text("open notes.json and a`b then c".into())]
    );
    assert_eq!(runs("`a``b`"), vec![Run::Text("a``b".into())]);
}

#[test]
fn escapes_all_inert_controls() {
    let chars = [
        '\n', '\r', '\t', '\0', '\u{7f}', '\u{85}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}',
        '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}', '\u{200e}', '\u{200f}',
        '\u{61c}', '\u{200b}', '\u{200c}', '\u{200d}', '\u{2060}', '\u{feff}', '\u{2028}',
        '\u{2029}',
    ];
    for c in chars {
        assert_eq!(
            runs(&format!("a{c}b")),
            vec![
                Run::Text("a".into()),
                Run::Escape(format!("U+{:04X}", c as u32)),
                Run::Text("b".into())
            ]
        );
    }
}

#[test]
fn preserves_unicode_and_escapes_inside_code() {
    assert_eq!(
        runs("`é\u{202e}界`"),
        vec![
            Run::Text("é".into()),
            Run::Escape("U+202E".into()),
            Run::Text("界".into())
        ]
    );
    assert_eq!(
        runs("\t\n"),
        vec![Run::Escape("U+0009".into()), Run::Escape("U+000A".into())]
    );
}

#[test]
fn leaves_width_truncation_to_the_renderer() {
    let s = "x".repeat(262);
    assert_eq!(runs(&s), vec![Run::Text(s)]);
}
