//! How the screens seam could fail:
//! - a stage is drawn by the wrong family (Faults by Home, the Projects place by the places);
//! - the keys go to the page while the sidebar or a frame float has them, or to the page while a
//!   float a family opened has them;
//! - a global command a family owns can't run from another page (`␣cn` from Home);
//! - the peek column shows on a view, or a details panel on a list.

use rune::{El, Tx};

use super::*;
use crate::app::{Invocation, commands};
use crate::keys::Keymap;

fn conversation() -> Item {
    Item::Conversation {
        project: "notes-cli".into(),
        id: 4,
    }
}

#[test]
fn each_stage_is_drawn_by_its_family() {
    let cases = [
        (Stage::Connect, ScreenId::Connect),
        (Stage::Start, ScreenId::Start),
        (Stage::Place(Place::Home), ScreenId::Home),
        (Stage::Place(Place::Faults), ScreenId::Places),
        (Stage::Place(Place::Conversations), ScreenId::Places),
        (Stage::Place(Place::Timeline), ScreenId::Places),
        (Stage::Place(Place::Projects), ScreenId::Project),
        (
            Stage::Item(Item::Project("render-core".into())),
            ScreenId::Project,
        ),
        (Stage::Item(conversation()), ScreenId::Conversation),
        (
            Stage::Item(Item::Decision {
                project: "render-core".into(),
                id: "w1:3:design#2".into(),
            }),
            ScreenId::Decision,
        ),
    ];
    for (stage, id) in cases {
        assert_eq!(of(&stage), id, "{stage:?}");
    }
}

struct Probe;

impl ScreenFloat for Probe {
    fn owner(&self) -> ScreenId {
        ScreenId::Peek
    }
    fn scope(&self) -> Scope {
        Scope {
            mode: Mode::Normal,
            ctx: Ctx::Peek,
        }
    }
    fn badge(&self) -> Badge {
        Badge::Normal
    }
    fn view(&self, _: Theme, _: &App) -> El {
        rune::el()
    }
}

#[test]
fn the_keys_go_to_the_float_owner_else_the_page_else_nobody() {
    let mut a = App::default();
    a.show(Stage::Item(conversation()));
    assert_eq!(holder(&a), Some(ScreenId::Conversation));
    a.focus = Focus::Sidebar;
    assert_eq!(holder(&a), None, "the sidebar has the keys");
    a.focus = Focus::Stage;
    a.open_float(Float::palette());
    assert_eq!(holder(&a), None, "a frame float has the keys");
    a.close_float();
    a.open_float(Float::Screen(Box::new(Probe)));
    assert_eq!(holder(&a), Some(ScreenId::Peek));
    assert!(top_float::<Probe>(&mut a).is_some());
    assert_eq!(a.scope().ctx, Ctx::Peek);
}

fn run(a: &mut App, id: &str) {
    let command = Keymap::design().command(id).expect("a keymap command");
    let inv = Invocation {
        command,
        count: None,
        applies: true,
        keys: Vec::new(),
    };
    commands::run(a, &inv, &mut Tx::default());
}

#[test]
fn a_family_global_runs_from_any_page() {
    let mut a = App::default();
    a.show(Stage::Place(Place::Home));
    run(&mut a, "conv.new");
    assert_eq!(a.stage, Stage::Start);
}

#[test]
fn lists_have_the_peek_and_views_the_details() {
    let mut a = App::default();
    for p in [Place::Home, Place::Conversations, Place::Projects] {
        a.show(Stage::Place(p));
        assert!(page(&a).has_peek(&a) && !page(&a).has_details(&a), "{p:?}");
    }
    a.show(Stage::Item(conversation()));
    assert!(!page(&a).has_peek(&a) && page(&a).has_details(&a));
    a.show(Stage::Item(Item::Project("render-core".into())));
    assert!(!page(&a).has_peek(&a));
}

#[test]
fn the_frame_ui_floats_and_content_never_name_the_client() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for dir in ["frame", "ui", "floats", "content"] {
        for entry in std::fs::read_dir(src.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            assert!(
                !text.contains("tau_client"),
                "{} names tau_client: they take plain data",
                path.display()
            );
        }
    }
}

#[test]
#[ignore = "inventory: prints every palette command with no handler"]
fn list_unbuilt_commands() {
    for c in Keymap::design().palette() {
        if !commands::handled().any(|id| id == c.id) {
            println!("UNBUILT {} {}", c.id, c.desc);
        }
    }
}
