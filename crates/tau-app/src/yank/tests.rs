//! How yanking could fail, keys in and the register out (`grammar.md` §4, `visual-language.md` §3):
//! - the yanked text isn't the source bytes of the object under the cursor (markup dropped,
//!   blanks added, the next sentence or block run in);
//! - `iw aw is as ip ap yy` take another object than the one asked for;
//! - in Visual, `y` takes other than the selection, or leaves Visual on;
//! - once `y` waits for its object, the next key runs its own command (`i` opens the composer);
//! - `im am` in a conversation take other than the message's text; in a decision they take
//!   anything;
//! - `Esc` leaves the operator waiting, or a key that is no object is kept for later;
//! - `␣y` doesn't yank, or acts outside a reader.

use rune::Tx;
use serde_json::json;

use crate::app::scenes::on_fixture;
use crate::app::{App, Stage, input};
use crate::keys::parse_seq;
use crate::open::Item;
use crate::screens;

const BASIS: &str = "The timeline is the one record that every client reads the same way. Today it can only be paged through `history`. An export lets the owner keep it outside the store, diff two exports, and feed it to other tools.";

fn keys(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        input::press(a, key, false, &mut Tx::default());
    }
}

/// The decision with its cursor on the first letter of `word` in the paragraph under `## Why`.
fn review_at(word: &str) -> App {
    let mut a = App::default();
    on_fixture(
        &mut a,
        Stage::Item(Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into(),
        }),
    );
    let text = &a
        .link
        .server()
        .decisions
        .values()
        .find_map(|k| k.value().filter(|v| v.text.contains(BASIS)))
        .expect("the basis arrived")
        .text;
    let at = text.find(BASIS).unwrap() + BASIS.find(word).unwrap();
    screens::page(&a).land(&mut a, &json!({ "Text": at }));
    a
}

fn yanked(a: &App) -> Option<&str> {
    a.register.as_deref()
}

#[test]
fn pending_yank_shows_its_effective_key_and_object_prefix() {
    for (command, design, to, prefix) in [
        ("yank", "y", "Y", "Y"),
        ("yank.clipboard", "<leader>y", "<leader>hy", "␣hy"),
    ] {
        let mut a = review_at("timeline");
        a.settings.keymap = a
            .settings
            .keymap
            .rebind(
                command,
                &parse_seq(design).unwrap(),
                &parse_seq(to).unwrap(),
            )
            .unwrap();
        keys(&mut a, to);
        assert_eq!(super::shown(&a), prefix);
        keys(&mut a, "i");
        assert_eq!(super::shown(&a), format!("{prefix}i"));
        keys(&mut a, "w");
        assert_eq!(yanked(&a), Some("timeline"));
        assert_eq!(super::shown(&a), "");
    }
}

#[test]
fn accepted_rebinds_leave_every_yank_object_available_in_both_readers() {
    for conversation in [false, true] {
        for (object, decision, transcript) in [
            ("iw", Some("timeline"), Some("Port")),
            ("aw", Some("timeline "), Some("Port ")),
            (
                "is",
                Some("The timeline is the one record that every client reads the same way."),
                Some("Port the config loader from JSON to TOML."),
            ),
            (
                "as",
                Some("The timeline is the one record that every client reads the same way. "),
                Some("Port the config loader from JSON to TOML. "),
            ),
            ("ip", Some(BASIS), Some(INPUT)),
            (
                "ap",
                Some(
                    "The timeline is the one record that every client reads the same way. Today it can only be paged through `history`. An export lets the owner keep it outside the store, diff two exports, and feed it to other tools.\n\n",
                ),
                Some(INPUT),
            ),
            ("im", None, Some(INPUT)),
            ("am", None, Some(INPUT)),
            ("y", Some(BASIS), Some(INPUT)),
        ] {
            let mut a = if conversation {
                notes_cli()
            } else {
                review_at("timeline")
            };
            if conversation {
                keys(&mut a, "ggj");
            }
            a.settings.keymap = a
                .settings
                .keymap
                .rebind(
                    "toggle.details",
                    &parse_seq("<leader>td").unwrap(),
                    &parse_seq("s").unwrap(),
                )
                .unwrap();
            keys(&mut a, &format!("y{object}"));
            assert_eq!(
                yanked(&a),
                if conversation { transcript } else { decision },
                "{conversation} {object}"
            );
            assert!(!a.details_visible);
            assert!(a.yank.is_none());
        }
    }
}

#[test]
fn words_in_a_decision_are_their_source_bytes() {
    let mut a = review_at("timeline");
    keys(&mut a, "yiw");
    assert_eq!(yanked(&a), Some("timeline"));
    keys(&mut a, "yaw");
    assert_eq!(yanked(&a), Some("timeline "));
}

#[test]
fn sentences_stop_at_their_full_stop_and_keep_their_markup() {
    let mut a = review_at("paged");
    keys(&mut a, "yis");
    assert_eq!(
        yanked(&a),
        Some("Today it can only be paged through `history`.")
    );
    keys(&mut a, "yas");
    assert_eq!(
        yanked(&a),
        Some("Today it can only be paged through `history`. ")
    );
}

#[test]
fn a_paragraph_and_the_line_are_the_block_under_the_cursor() {
    let mut a = review_at("export");
    keys(&mut a, "yip");
    assert_eq!(yanked(&a), Some(BASIS));
    a.register = None;
    keys(&mut a, "yy");
    assert_eq!(yanked(&a), Some(BASIS));
    keys(&mut a, "yap");
    assert_eq!(yanked(&a), Some(format!("{BASIS}\n\n").as_str()));
}

#[test]
fn y_in_visual_takes_the_selection_and_leaves_visual() {
    let mut a = review_at("timeline");
    keys(&mut a, "vey");
    assert_eq!(yanked(&a), Some("timeline"));
    assert_eq!(a.scope().mode, crate::keys::Mode::Normal);
}

#[test]
fn a_message_object_is_only_the_conversations() {
    let mut a = review_at("timeline");
    keys(&mut a, "yim");
    assert_eq!(yanked(&a), None);
}

fn notes_cli() -> App {
    let mut a = App::default();
    on_fixture(
        &mut a,
        Stage::Item(Item::Conversation {
            project: "notes-cli".into(),
            id: 4,
        }),
    );
    a
}

const INPUT: &str = "Port the config loader from JSON to TOML. Keep reading `notes.json` for one release and warn when it is used.";

#[test]
fn in_a_conversation_the_operator_takes_the_keys_until_its_object() {
    let mut a = notes_cli();
    keys(&mut a, "ggyim");
    assert_eq!(yanked(&a), Some(INPUT));
    assert_eq!(
        a.scope().ctx,
        crate::keys::Ctx::Conversation,
        "i didn't open the composer"
    );
}

#[test]
fn a_turn_is_one_message_of_its_texts_each_as_sent() {
    let mut a = notes_cli();
    keys(&mut a, "gg]myim");
    let turn = yanked(&a).expect("the turn's text");
    assert!(turn.starts_with(
        "I'll start with the loader and see who calls it.\n\nOnly `src/cli.ts` and one test \
         call it. I'll add `smol-toml` for parsing.\n\nThe network is closed here"
    ));
    assert!(!turn.contains("Port the config loader"), "the turn only");
}

#[test]
fn sentences_and_words_in_a_conversation_start_at_the_cursors_block() {
    let mut a = notes_cli();
    keys(&mut a, "ggjyis");
    assert_eq!(
        yanked(&a),
        Some("Port the config loader from JSON to TOML.")
    );
    keys(&mut a, "yiw");
    assert_eq!(yanked(&a), Some("Port"));
}

#[test]
fn esc_drops_the_operator_and_the_next_key_acts_as_itself() {
    let mut a = notes_cli();
    keys(&mut a, "gg");
    keys(&mut a, "y<Esc>i");
    assert_eq!(yanked(&a), None);
    assert_eq!(a.scope().ctx, crate::keys::Ctx::Composer);
}

#[test]
fn a_key_that_is_no_object_drops_the_operator() {
    let mut a = notes_cli();
    keys(&mut a, "ggyxim");
    assert_eq!(yanked(&a), None);
}

#[test]
fn space_y_yanks_in_a_reader_and_nowhere_else() {
    let mut a = review_at("timeline");
    keys(&mut a, "<leader>yiw");
    assert_eq!(yanked(&a), Some("timeline"));
    let mut home = App::default();
    on_fixture(&mut home, Stage::Place(crate::frame::Place::Home));
    keys(&mut home, "<leader>yy");
    assert_eq!(yanked(&home), None);
}
