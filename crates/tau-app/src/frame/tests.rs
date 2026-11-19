//! How the frame's printed keys and counts could read to assistive technology:
//! - a chip or hint is read in pieces (`⌘` and `start`, `cn` without its leader);
//! - a row made of a word and a chip reads its key's glyphs, not its key;
//! - a count tag reads as a bare number, or with the wrong plural;
//! - the bottom bar's tags, Home's title tag or a header segment drop any of that.

use crate::app::scenes;
use crate::app::testing::{Spoken, spoken, window};
use crate::app::{App, Stage};
use crate::open::Item;

fn names(a: App) -> Vec<Spoken> {
    spoken(&window(a))
}

fn has(nodes: &[Spoken], name: &str) -> usize {
    nodes.iter().filter(|n| n.name == name).count()
}

#[test]
fn sidebar_rows_read_their_word_and_their_key() {
    let mut a = App::default();
    scenes::home(&mut a);
    let nodes = names(a);
    assert_eq!(has(&nodes, "New, Space c n"), 1, "{nodes:#?}");
    assert_eq!(has(&nodes, "Find, Space Period"), 1);
    assert!(
        nodes.iter().all(|n| n.name != "cn" && n.name != "New cn"),
        "{nodes:#?}"
    );
}

#[test]
fn count_tags_read_what_they_count_with_plurals_by_count() {
    let mut a = App::default();
    scenes::home(&mut a);
    let nodes = names(a);
    assert_eq!(
        has(&nodes, "3 open decisions"),
        2,
        "the sidebar's tag and Home's title tag"
    );
    assert_eq!(has(&nodes, "4 failures"), 1);
    let bare: Vec<_> = nodes
        .iter()
        .filter(|n| ["button", "group"].contains(&n.role.as_str()))
        .filter(|n| n.name.chars().all(|c| c.is_ascii_digit()) && !n.name.is_empty())
        .collect();
    assert!(bare.is_empty(), "{bare:#?}");
    let mut a = App::default();
    scenes::home(&mut a);
    a.sidebar_visible = false;
    a.server.counts.needs = Some(1);
    a.server.counts.failed = Some(1);
    a.server.counts.unknown = Some(2);
    let nodes: Vec<Spoken> = names(a)
        .into_iter()
        .filter(|n| n.bounds[1] >= 956.0)
        .collect();
    assert_eq!(has(&nodes, "1 open decision"), 1, "the bottom bar's tags");
    assert_eq!(has(&nodes, "1 failure"), 1);
    assert_eq!(has(&nodes, "2 writes without a reply"), 1);
    let mut a = App::default();
    scenes::home(&mut a);
    a.server.counts.unknown = Some(1);
    assert_eq!(has(&names(a), "1 write without a reply"), 1);
}

#[test]
fn header_segments_and_float_hints_read_as_one_name() {
    let mut a = App::default();
    scenes::on_fixture(
        &mut a,
        Stage::Item(Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into(),
        }),
    );
    let nodes = names(a);
    assert_eq!(has(&nodes, "Answer, Space a d"), 1, "{nodes:#?}");
    let mut a = App::default();
    scenes::on_fixture(
        &mut a,
        Stage::Item(Item::Conversation {
            project: "notes-cli".into(),
            id: 4,
        }),
    );
    assert_eq!(has(&names(a), "stop, Space c x"), 1);
    let mut a = App::default();
    scenes::home(&mut a);
    a.open_float(crate::floats::Float::palette());
    let nodes = names(a);
    assert!(
        nodes.iter().any(|n| n.name.ends_with(", Return")),
        "{nodes:#?}"
    );
    assert!(nodes.iter().any(|n| n.name.ends_with(", Escape")));
}

/// What is not drawn is not exposed: a sidebar hidden by `␣tb` or by a window under 1000 px
/// leaves no row in the accessibility tree, and one the keys bring back over the stage does.
#[test]
fn a_hidden_sidebar_exposes_no_row() {
    let home = |width: f32, visible: bool, focus: crate::app::Focus| {
        let mut a = App::default();
        scenes::home(&mut a);
        a.sidebar_visible = visible;
        a.focus = focus;
        a.window[0] = width;
        has(
            &spoken(&crate::app::testing::window_at(a, [width, 982.0])),
            "New, Space c n",
        )
    };
    use crate::app::Focus::{Sidebar, Stage};
    assert_eq!(home(1512.0, true, Stage), 1, "drawn beside the stage");
    assert_eq!(home(1512.0, false, Stage), 0, "hidden by ␣tb");
    assert_eq!(home(900.0, true, Stage), 0, "hidden under 1000 px");
    assert_eq!(home(900.0, true, Sidebar), 1, "drawn over the stage");
}
