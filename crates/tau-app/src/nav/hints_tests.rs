//! How hint mode could fail (`grammar.md` §2, §13):
//! - labels repeat, leave the home row's order, mix one and two letters, or one is the start of
//!   another, so a typed letter is ambiguous;
//! - `gs` with nothing to label leaves Normal anyway;
//! - in HINT a key does its Normal work (`j` moves, `i` opens the composer) instead of picking a
//!   label;
//! - a label typed before the labels were drawn picks something;
//! - a label opens or acts, or moves the cursor elsewhere than its target, or leaves no jump;
//! - `Esc` moves the cursor;
//! - a two-letter label acts on its first letter, or `BS` can't take that letter back;
//! - the labels change when targets arrive after `gs`;
//! - in the sidebar, the labels are the stage's, or a label opens the row;
//! - a click that shows another page leaves the old page's labels up.

use rune::Tx;

use super::hints::{self, labels};
use super::*;
use crate::app::scenes::on_fixture;
use crate::app::{Focus, input};
use crate::frame::{Place, Row};
use crate::keys::{Ctx, Mode, parse_seq};
use crate::ui::Badge;

fn keys(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        input::press(a, key, false, &mut Tx::default());
    }
}

fn at(stage: Stage) -> App {
    let mut a = App::default();
    on_fixture(&mut a, stage);
    a
}

fn notes_cli() -> Stage {
    Stage::Item(Item::Conversation {
        project: "notes-cli".into(),
        id: 4,
    })
}

fn anchor(a: &App) -> Option<Anchor> {
    screens::page(a).anchor(a)
}

#[test]
fn labels_are_one_home_row_letter_up_to_nine_then_two_letters_each() {
    assert_eq!(labels(3, "asdfghjkl"), ["a", "s", "d"]);
    assert_eq!(labels(9, "asdfghjkl").last().map(String::as_str), Some("l"));
    let ten = labels(10, "asdfghjkl");
    assert_eq!(&ten[..3], ["aa", "as", "ad"]);
    assert!(ten.iter().all(|l| l.chars().count() == 2));
    let many = labels(81, "asdfghjkl");
    for (i, l) in many.iter().enumerate() {
        assert!(
            many.iter()
                .enumerate()
                .all(|(j, m)| i == j || !m.starts_with(l.as_str())),
            "{l} is unambiguous"
        );
    }
}

#[test]
fn gs_with_nothing_to_label_stays_in_normal() {
    let mut a = at(Stage::Start);
    keys(&mut a, "<Esc>gs");
    assert_ne!(a.scope().mode, Mode::Hint);
}

#[test]
fn in_hint_mode_every_key_picks_a_label_and_nothing_else_runs() {
    let mut a = at(notes_cli());
    let before = anchor(&a);
    keys(&mut a, "gs");
    assert_eq!((a.scope().mode, a.badge()), (Mode::Hint, Badge::Hint));
    hints::draw(&mut a);
    keys(&mut a, "jiq");
    assert_eq!(anchor(&a), before);
    assert_eq!(
        a.scope().ctx,
        Ctx::Hint,
        "i opened nothing and q left nothing"
    );
}

#[test]
fn a_label_typed_before_the_labels_were_drawn_picks_nothing() {
    let mut a = at(notes_cli());
    let before = anchor(&a);
    keys(&mut a, "gsa");
    assert_eq!(anchor(&a), before);
    assert_eq!(a.scope().mode, Mode::Hint);
}

#[test]
fn a_label_moves_the_cursor_to_its_target_records_a_jump_and_opens_nothing() {
    let mut a = at(notes_cli());
    let before = anchor(&a);
    keys(&mut a, "gs");
    let first = hints::draw(&mut a)[0].clone();
    keys(&mut a, &first.0);
    assert_eq!(a.scope().mode, Mode::Normal);
    assert_eq!(anchor(&a), Some(first.1));
    assert_eq!(a.stage, notes_cli());
    assert!(a.floats.is_empty());
    keys(&mut a, "<C-o>");
    assert_eq!(anchor(&a), before);
}

#[test]
fn esc_leaves_hint_mode_with_the_cursor_where_it_was() {
    let mut a = at(notes_cli());
    let before = anchor(&a);
    keys(&mut a, "gs");
    hints::draw(&mut a);
    keys(&mut a, "<Esc>");
    assert_eq!(a.scope().mode, Mode::Normal);
    assert_eq!(anchor(&a), before);
}

#[test]
fn a_two_letter_label_waits_for_its_second_letter_and_bs_takes_one_back() {
    let mut a = at(notes_cli());
    keys(&mut a, "gs");
    let drawn = hints::draw(&mut a);
    assert!(drawn.len() > 10, "the transcript has more than nine stops");
    let before = anchor(&a);
    // Not the last stop, whose anchor is following the run.
    let (label, target) = drawn[9].clone();
    let mut chars = label.chars();
    let (first, second) = (chars.next().unwrap(), chars.next().unwrap());
    keys(&mut a, &first.to_string());
    assert_eq!((a.scope().mode, anchor(&a)), (Mode::Hint, before));
    keys(&mut a, &format!("<BS>{first}{second}"));
    assert_eq!(anchor(&a), Some(target));
}

#[test]
fn the_labels_stay_as_first_drawn() {
    let mut a = at(notes_cli());
    keys(&mut a, "zRgs");
    let drawn = hints::draw(&mut a);
    a.screens.conversation = Default::default();
    assert_ne!(
        screens::page(&a).targets(&a).len(),
        drawn.len(),
        "folding the rows again changed the targets"
    );
    assert_eq!(hints::draw(&mut a), drawn);
}

#[test]
fn in_the_sidebar_the_labels_are_its_rows_and_move_its_cursor_only() {
    let mut a = at(Stage::Place(Place::Home));
    keys(&mut a, "<C-h>gs");
    let drawn = hints::draw(&mut a);
    let (rows, _) = a.sidebar_rows();
    assert_eq!(drawn.len(), rows.len());
    let (label, _) = drawn
        .iter()
        .zip(&rows)
        .find(|(_, r)| **r == Row::Place(Place::Timeline))
        .map(|(d, _)| d.clone())
        .unwrap();
    keys(&mut a, &label);
    assert_eq!(a.sidebar_cursor, Row::Place(Place::Timeline));
    assert_eq!(a.focus, Focus::Sidebar);
    assert_eq!(a.stage, Stage::Place(Place::Home));
}

#[test]
fn showing_another_page_ends_hint_mode() {
    let mut a = at(notes_cli());
    keys(&mut a, "gs");
    a.show(Stage::Place(Place::Home));
    assert_eq!(a.scope().mode, Mode::Normal);
}
