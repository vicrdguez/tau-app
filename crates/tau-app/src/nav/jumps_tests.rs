//! How the jumplist itself could fail (`grammar.md` §13):
//! - `C-o` from the newest place forgets the place it left, so `C-i` can't come back;
//! - `C-o` at the oldest entry, or `C-i` at the newest, moves anyway;
//! - a count is ignored, or one past the end moves part of the way;
//! - the same place twice fills the list with copies;
//! - past 100 entries the newest go instead of the oldest;
//! - a list read back from this device starts anywhere but its newest end.

use super::*;
use crate::app::Stage;
use crate::frame::Place;
use crate::open::Item;

fn spot(n: u64) -> Spot {
    Spot {
        stage: Stage::Item(Item::Conversation {
            project: "notes-cli".into(),
            id: n,
        }),
        anchor: None,
    }
}

#[test]
fn back_returns_where_a_jump_left_and_forward_comes_back() {
    let mut j = Jumps::default();
    j.push(spot(1));
    assert_eq!(j.back(spot(2), 1), Some(spot(1)));
    assert_eq!(j.forward(1), Some(spot(2)));
    assert_eq!(j.back(spot(2), 1), Some(spot(1)));
}

#[test]
fn the_ends_go_nowhere() {
    let mut j = Jumps::default();
    assert_eq!(j.back(spot(1), 1), None);
    j.push(spot(1));
    assert_eq!(j.forward(1), None);
    assert_eq!(j.back(spot(2), 1), Some(spot(1)));
    assert_eq!(j.back(spot(1), 1), None);
    assert_eq!(j.forward(1), Some(spot(2)));
}

#[test]
fn a_count_moves_that_many_or_not_at_all() {
    let mut j = Jumps::default();
    for n in 1..=3 {
        j.push(spot(n));
    }
    assert_eq!(j.back(spot(4), 5), None);
    assert_eq!(j.back(spot(4), 3), Some(spot(1)));
    assert_eq!(j.forward(2), Some(spot(3)));
}

#[test]
fn a_place_already_listed_moves_to_the_newest_end() {
    let mut j = Jumps::default();
    j.push(spot(1));
    j.push(spot(2));
    j.push(spot(1));
    assert_eq!(j.entries(), &[spot(2), spot(1)]);
}

#[test]
fn past_a_hundred_the_oldest_go() {
    let mut j = Jumps::default();
    for n in 0..105 {
        j.push(spot(n));
    }
    assert_eq!(j.entries().len(), 100);
    assert_eq!(j.entries()[0], spot(5));
    assert_eq!(j.entries()[99], spot(104));
}

#[test]
fn a_list_read_back_starts_at_its_newest_end() {
    let mut j = Jumps::default();
    j.push(spot(1));
    j.push(Spot {
        stage: Stage::Place(Place::Timeline),
        anchor: Some(serde_json::json!({"Decision": ["render-core", "w1:3:design#2"]})),
    });
    let mut back: Jumps = serde_json::from_str(&serde_json::to_string(&j).unwrap()).unwrap();
    assert_eq!(back.entries(), j.entries());
    assert_eq!(back.forward(1), None);
    assert_eq!(back.back(spot(9), 2), Some(spot(1)));
}
