//! How jumps could fail in the app, keys in and the stage out (`grammar.md` §4, §13):
//! - a jump (`gd`, `gg`, `G`, an open) doesn't keep where it left, so `C-o` can't return there, or
//!   returns to the object without its cursor;
//! - `C-i` after `C-o` doesn't come back;
//! - a motion that isn't a jump (`j`) is kept, or a jump that lands where it was;
//! - `C-o` to an item closed since doesn't open it;
//! - `:jumps` lists other than the entries, or `⏎` goes to another one than chosen;
//! - the list isn't kept on this device, so a relaunch loses it.

use rune::Tx;

use super::*;
use crate::app::scenes::on_fixture;
use crate::app::testing::paired;
use crate::app::{input, link};
use crate::floats::Float;
use crate::frame::Place;
use crate::keys::parse_seq;
use crate::open::Item;
use crate::screens;

fn review() -> Stage {
    Stage::Item(Item::Decision {
        project: "render-core".into(),
        id: "w1:3:design#2".into(),
    })
}

fn notes_cli() -> Stage {
    Stage::Item(Item::Conversation {
        project: "notes-cli".into(),
        id: 4,
    })
}

fn at(stage: Stage) -> App {
    let mut a = App::default();
    on_fixture(&mut a, stage);
    a
}

fn keys(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        input::press(a, key, false, &mut Tx::default());
        link::settle(a);
    }
}

fn anchor(a: &App) -> Option<screens::Anchor> {
    screens::page(a).anchor(a)
}

#[test]
fn ctrl_o_returns_to_the_object_and_cursor_a_jump_left_and_ctrl_i_comes_back() {
    let mut a = at(review());
    keys(&mut a, "jj");
    let left = anchor(&a);
    assert!(left.is_some(), "the decision has a cursor");
    keys(&mut a, "gd");
    let linked = a.stage.clone();
    assert_ne!(linked, review(), "gd showed the decision's conversation");
    keys(&mut a, "<C-o>");
    assert_eq!(a.stage, review());
    assert_eq!(anchor(&a), left);
    keys(&mut a, "<C-i>");
    assert_eq!(a.stage, linked);
}

#[test]
fn a_motion_that_is_no_jump_is_not_kept() {
    let mut a = at(review());
    keys(&mut a, "jjk");
    assert!(a.jumps.entries().is_empty());
}

#[test]
fn a_jump_that_lands_where_it_was_is_not_kept() {
    let mut a = at(Stage::Place(Place::Home));
    keys(&mut a, "<leader>oh");
    assert!(a.jumps.entries().is_empty());
    keys(&mut a, "<leader>oc");
    assert_eq!(a.jumps.entries().len(), 1);
}

#[test]
fn gg_and_g_are_jumps_inside_a_view() {
    let mut a = at(notes_cli());
    let following = anchor(&a);
    keys(&mut a, "gg");
    assert_ne!(anchor(&a), following, "gg moved the cursor");
    let top = anchor(&a);
    keys(&mut a, "<C-o>");
    assert_eq!((a.stage.clone(), anchor(&a)), (notes_cli(), following));
    keys(&mut a, "<C-i>");
    assert_eq!(anchor(&a), top);
}

#[test]
fn ctrl_o_after_g_in_a_decision_puts_its_cursor_back() {
    let mut a = at(review());
    keys(&mut a, "jj");
    let read = anchor(&a);
    keys(&mut a, "G");
    assert_ne!(anchor(&a), read, "G moved the cursor");
    keys(&mut a, "<C-o>");
    assert_eq!(anchor(&a), read);
}

#[test]
fn ctrl_o_to_a_closed_item_opens_it() {
    let mut a = at(notes_cli());
    keys(&mut a, "<leader>oc");
    let item = match notes_cli() {
        Stage::Item(i) => i,
        _ => unreachable!(),
    };
    a.open.close(&item);
    keys(&mut a, "<C-o>");
    assert_eq!(a.stage, notes_cli());
    assert!(a.open.contains(&item));
}

#[test]
fn jumps_lists_the_entries_newest_last_and_enter_goes_to_the_one_chosen() {
    let mut a = at(Stage::Place(Place::Home));
    keys(&mut a, "<leader>oc<leader>ot<leader>of");
    run_ex(&mut a, "jumps");
    let Some((Float::Sheet(s), _)) = a.floats.last() else {
        panic!("a float lists the jumps");
    };
    let rows: Vec<&str> = s.rows.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(rows, ["Home", "Conversations", "Timeline"]);
    keys(&mut a, "<C-k><CR>");
    assert_eq!(a.stage, Stage::Place(Place::Conversations));
    keys(&mut a, "<C-i>");
    assert_eq!(a.stage, Stage::Place(Place::Timeline));
}

fn run_ex(a: &mut App, name: &str) {
    let command = crate::keys::Keymap::design()
        .by_ex(name)
        .expect("an ex name");
    crate::app::commands::run(
        a,
        &crate::app::Invocation {
            command,
            count: None,
            applies: true,
            keys: Vec::new(),
        },
        &mut Tx::default(),
    );
}

#[test]
fn the_list_survives_a_relaunch_on_this_device() {
    let mut setup = paired(false);
    let link = std::mem::replace(&mut setup.link, link::demo());
    let mut a = App::launch(link);
    on_fixture(&mut a, Stage::Place(Place::Home));
    keys(&mut a, "<leader>oc");
    let link = std::mem::replace(&mut a.link, link::demo());
    let mut b = App::launch(link);
    link::settle(&mut b);
    b.show(Stage::Place(Place::Timeline));
    keys(&mut b, "<C-o>");
    assert_eq!(b.stage, Stage::Place(Place::Home));
}
