//! How the plain-text objects could fail:
//! - `is` runs into the next sentence, or `as` drops the blanks after it;
//! - a sentence without a full stop at the end of its text is cut short;
//! - a sentence's closing quote or bracket is left off, or one before it starts it;
//! - a range falls inside a multi-byte character;
//! - the blanks `ap` and `am` add run into the next block's text.

use super::*;

const TEXT: &str = "The timeline is one record. Today it is paged (slowly.)  An export helps";

fn at(word: &str) -> usize {
    TEXT.find(word).unwrap()
}

#[test]
fn a_sentence_is_its_text_and_around_it_the_blanks_after() {
    assert_eq!(
        &TEXT[sentence(TEXT, at("record"), false)],
        "The timeline is one record."
    );
    assert_eq!(
        &TEXT[sentence(TEXT, at("record"), true)],
        "The timeline is one record. "
    );
}

#[test]
fn a_sentence_keeps_its_closing_bracket_and_starts_after_the_one_before() {
    assert_eq!(
        &TEXT[sentence(TEXT, at("paged"), false)],
        "Today it is paged (slowly.)"
    );
    assert_eq!(
        &TEXT[sentence(TEXT, at("paged"), true)],
        "Today it is paged (slowly.)  "
    );
}

#[test]
fn the_last_sentence_runs_to_the_end_without_a_full_stop() {
    assert_eq!(&TEXT[sentence(TEXT, at("helps"), false)], "An export helps");
}

#[test]
fn ranges_stay_on_character_boundaries() {
    let s = "Ça va. Très bien.";
    let r = sentence(s, s.find("Très").unwrap(), false);
    assert_eq!(&s[r], "Très bien.");
}

#[test]
fn the_blanks_after_stop_at_the_next_text() {
    let s = "one\n\n\ntwo";
    assert_eq!(&s[spread(s, 0..3)], "one\n\n\n");
}
