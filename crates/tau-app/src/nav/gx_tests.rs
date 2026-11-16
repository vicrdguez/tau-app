//! How `gx` could fail (`grammar.md` §4, `visual-language.md` §3, §8):
//! - it opens without asking, or the Confirm shows less than the whole URL;
//! - a character that draws nothing or reorders the text hides in the URL it shows;
//! - it takes another link than the one under the cursor, or one from another block;
//! - a link that isn't to the web or mail (a relative path, `file:`) is offered;
//! - in a conversation it finds no link in the message text under the cursor.

use rune::Tx;
use serde_json::{Value, json};
use tau_client::{Fixture, Link};

use crate::app::scenes::on_fixture;
use crate::app::{App, Stage, input};
use crate::floats::Float;
use crate::keys::parse_seq;
use crate::open::Item;
use crate::screens;

fn keys(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        input::press(a, key, false, &mut Tx::default());
    }
}

/// The fixture with `text` at `pointer`.
fn app_with(pointer: &str, text: &str, stage: Stage) -> App {
    let mut f: Value =
        serde_json::from_str(include_str!("../../../../design/fixture.json")).unwrap();
    *f.pointer_mut(pointer).expect("the fixture has it") = json!(text);
    let mut a = App::launch(Link::fixture(Fixture::parse(&f.to_string()).unwrap()));
    on_fixture(&mut a, stage);
    a
}

/// The decision whose basis is `basis`, its cursor on the first `word`.
fn decision(basis: &str, word: &str) -> App {
    let mut a = app_with(
        "/calls/decision/render-core~1w1:3:design#2/result/text",
        basis,
        Stage::Item(Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into(),
        }),
    );
    let at = basis.find(word).unwrap();
    screens::page(&a).land(&mut a, &json!({ "Text": at }));
    a
}

fn asked(a: &App) -> Option<&str> {
    match a.floats.last() {
        Some((Float::Confirm(c), _)) => Some(&c.sentence),
        _ => None,
    }
}

const LONG: &str = "https://example.com/a/very/long/path/that/keeps/going?x=1&y=2#frag";

#[test]
fn gx_asks_first_and_shows_the_whole_url() {
    let mut a = decision(
        &format!("Read [the spec]({LONG}) now. And [this](https://other.example).\n\nNo link."),
        "spec",
    );
    keys(&mut a, "gx");
    let s = asked(&a).expect("a Confirm");
    assert!(s.contains(LONG), "{s}");
    assert!(!s.contains("other.example"));
}

#[test]
fn a_block_without_a_link_offers_none() {
    let mut a = decision(
        "Read [the spec](https://example.com).\n\nNo link here.",
        "here",
    );
    keys(&mut a, "gx");
    assert_eq!(asked(&a), None);
    assert_eq!(a.echo.as_deref(), Some("no link here"));
}

#[test]
fn what_draws_nothing_in_the_url_is_shown_as_its_escape() {
    let mut a = decision("[file](https://example.com/\u{202e}gpj.exe) here", "file");
    keys(&mut a, "gx");
    let s = asked(&a).expect("a Confirm");
    assert!(s.contains("https://example.com/U+202Egpj.exe"), "{s}");
    assert!(!s.contains('\u{202e}'));
}

#[test]
fn only_web_and_mail_links_are_offered() {
    let mut a = decision(
        "[notes](./notes.md) and [disk](file:///etc/passwd)",
        "notes",
    );
    keys(&mut a, "gx");
    assert_eq!(asked(&a), None);
    assert_eq!(a.echo.as_deref(), Some("not a web or mail link"));
    let mut a = decision("[write](mailto:owner@example.com)", "write");
    keys(&mut a, "gx");
    assert!(asked(&a).is_some());
}

#[test]
fn in_a_conversation_the_link_is_the_one_in_the_block_under_the_cursor() {
    let mut a = app_with(
        "/streams/open notes-cli #4/first/ops/0/1/entries/0/model/0/content",
        "Port it as [the guide](https://toml.io/en/v1.0.0) says.",
        Stage::Item(Item::Conversation {
            project: "notes-cli".into(),
            id: 4,
        }),
    );
    keys(&mut a, "ggjgx");
    assert!(asked(&a).is_some_and(|s| s.contains("https://toml.io/en/v1.0.0")));
}
