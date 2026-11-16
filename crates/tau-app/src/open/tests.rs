//! How the sidebar's open items could fail (`spec.md` §4.1), one test each:
//! - opening an item twice lists it twice;
//! - a new item or a new project's group lands out of order, or reopening moves an item;
//! - closing leaves an empty group behind;
//! - closing the shown item shows the wrong neighbour (next, else previous, else Home);
//! - `]b [b` skip, fail to wrap, or ignore a count;
//! - from a place, `]b` ignores the item shown last, or forgets that it was closed;
//! - `]b` with nothing open picks something.

use super::*;

fn conv(p: &str, id: u64) -> Item {
    Item::Conversation {
        project: p.into(),
        id,
    }
}

fn dec(p: &str, id: &str) -> Item {
    Item::Decision {
        project: p.into(),
        id: id.into(),
    }
}

fn proj(p: &str) -> Item {
    Item::Project(p.into())
}

/// The fixture's open items: notes-cli #4; render-core w1:3:design#2, #5, the project; forecasts
/// w2:2:pi-durable-2#1.
fn fixture() -> OpenItems {
    let mut o = OpenItems::default();
    for item in [
        conv("notes-cli", 4),
        dec("render-core", "w1:3:design#2"),
        conv("render-core", 5),
        proj("render-core"),
        dec("forecasts", "w2:2:pi-durable-2#1"),
    ] {
        o.open(item);
    }
    o
}

fn names(o: &OpenItems) -> Vec<(String, Vec<Item>)> {
    o.groups().to_vec()
}

#[test]
fn opening_an_open_item_lists_it_once_and_moves_nothing() {
    let mut o = fixture();
    let before = names(&o);
    o.open(conv("render-core", 5));
    o.open(conv("notes-cli", 4));
    assert_eq!(names(&o), before);
}

#[test]
fn a_new_item_joins_the_end_of_its_group_and_a_new_project_the_end_of_the_list() {
    let mut o = fixture();
    o.open(conv("notes-cli", 2));
    o.open(conv("scratch", 1));
    let groups: Vec<&str> = o.groups().iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(groups, ["notes-cli", "render-core", "forecasts", "scratch"]);
    assert_eq!(
        o.groups()[0].1,
        vec![conv("notes-cli", 4), conv("notes-cli", 2)]
    );
}

#[test]
fn closing_the_last_item_of_a_group_removes_the_group() {
    let mut o = fixture();
    o.close(&conv("notes-cli", 4));
    let groups: Vec<&str> = o.groups().iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(groups, ["render-core", "forecasts"]);
}

#[test]
fn closing_shows_the_next_item_else_the_previous_else_nothing() {
    let mut o = fixture();
    assert_eq!(
        o.close(&conv("render-core", 5)),
        Some(proj("render-core")),
        "the next in the sidebar's order"
    );
    assert_eq!(
        o.close(&dec("forecasts", "w2:2:pi-durable-2#1")),
        Some(proj("render-core")),
        "the last closes to the previous"
    );
    let mut one = OpenItems::default();
    one.open(proj("x"));
    assert_eq!(one.close(&proj("x")), None, "the only one closes to Home");
    assert!(one.is_empty());
}

#[test]
fn cycling_wraps_both_ways_and_takes_a_count() {
    let o = fixture();
    let first = conv("notes-cli", 4);
    let last = dec("forecasts", "w2:2:pi-durable-2#1");
    assert_eq!(o.step(Some(&last), 1), Some(first.clone()));
    assert_eq!(o.step(Some(&first), -1), Some(last.clone()));
    assert_eq!(
        o.step(Some(&first), 1),
        Some(dec("render-core", "w1:3:design#2"))
    );
    assert_eq!(o.step(Some(&first), 3), Some(proj("render-core")));
    assert_eq!(
        o.step(Some(&first), 5),
        Some(first.clone()),
        "five steps round five items"
    );
}

#[test]
fn from_a_place_cycling_counts_from_the_item_shown_last() {
    let o = fixture();
    assert_eq!(
        o.step(Some(&conv("render-core", 5)), 1),
        Some(proj("render-core"))
    );
    assert_eq!(
        o.step(None, 1),
        Some(conv("notes-cli", 4)),
        "nothing shown yet: from the top"
    );
    assert_eq!(
        o.step(None, -1),
        Some(dec("forecasts", "w2:2:pi-durable-2#1")),
        "or from the bottom"
    );
    assert_eq!(
        o.step(Some(&conv("gone", 9)), 1),
        Some(conv("notes-cli", 4)),
        "a closed item counts as none"
    );
}

#[test]
fn cycling_with_nothing_open_finds_nothing() {
    assert_eq!(OpenItems::default().step(None, 1), None);
}
