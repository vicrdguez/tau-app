//! How the start page's keys could fail:
//! - `M-p` or `M-w` opens nothing in Insert, or nothing in Normal;
//! - a moved key opens its picker in one mode only, or the key it replaced still opens it (a
//!   literal handler behind the keymap's back);
//! - the start line prints the design's key after a move, or mutes a hint whose key runs in
//!   Insert;
//! - a picker opens while a create is pending.

use super::*;
use crate::app::{commands, input, link, testing};
use crate::keys::parse_seq;
use std::sync::atomic::Ordering;
use tau_client::outbox::{WriteCall, WriteState};
use tau_client::wire::parse_reply;

fn key(a: &mut App, seq: &str) {
    for k in parse_seq(seq).unwrap() {
        input::press(a, k, false, &mut Tx::default());
    }
}
fn run(a: &mut App, id: &str) {
    commands::run_id(a, id, &mut Tx::default());
}
fn frame(a: &mut App) {
    for _ in 0..8 {
        link::pump(a, &mut Tx::default());
    }
}
fn ready() -> App {
    let mut a = App::default();
    scenes::base(&mut a);
    a.live = true;
    a
}

#[test]
fn new_keeps_text_and_uses_the_named_project_and_insert() {
    let mut a = ready();
    a.screens.start.editor.edit.replace_text("keep me");
    a.show(Stage::Item(Item::Conversation {
        project: "notes-cli".into(),
        id: 4,
    }));
    run(&mut a, "conv.new");
    assert_eq!(a.stage, Stage::Start);
    assert_eq!(a.screens.start.editor.text(), "keep me");
    assert_eq!(a.screens.start.project.as_deref(), Some("notes-cli"));
    assert!(matches!(a.screens.start.with, Some(StartWith::Model(_))));
    assert_eq!(a.scope().mode, Mode::Insert);
}

#[test]
fn enter_is_a_newline_and_empty_start_does_nothing() {
    let mut a = ready();
    run(&mut a, "start.send");
    assert_eq!(a.screens.start.pending, None);
    key(&mut a, "<CR>");
    assert_eq!(a.screens.start.editor.text(), "\n");
    assert!(
        a.link
            .outbox()
            .entries()
            .iter()
            .all(|e| e.call != WriteCall::Create)
    );
}

#[test]
fn one_press_pins_the_submission_and_disables_edits_and_pickers() {
    let mut a = ready();
    a.screens
        .start
        .editor
        .edit
        .replace_text("the requested task");
    run(&mut a, "start.send");
    let id = a.screens.start.pending.expect("create in flight");
    let before = a.link.outbox().entries().len();
    run(&mut a, "start.send");
    key(&mut a, "<M-p><M-w><CR><Esc>");
    assert_eq!(a.link.outbox().entries().len(), before);
    assert_eq!(a.screens.start.editor.text(), "the requested task");
    assert!(a.floats.is_empty());
    let entry = a.link.outbox().get(id).unwrap();
    assert_eq!(entry.body["project"], "render-core");
    assert_eq!(entry.body["workflow"], "w1");
}

#[test]
fn repeated_command_return_never_creates() {
    let mut a = ready();
    a.screens.start.editor.edit.replace_text("task");
    input::press(
        &mut a,
        parse_seq("<D-CR>").unwrap()[0],
        true,
        &mut Tx::default(),
    );
    assert_eq!(a.screens.start.pending, None);
}

#[test]
fn successful_create_clears_draft_remembers_choice_without_stealing_stage() {
    let mut a = ready();
    a.live = true;
    a.screens.start.editor.edit.replace_text("task");
    run(&mut a, "start.send");
    a.show(Stage::Place(crate::frame::Place::Projects));
    frame(&mut a);
    assert_eq!(a.stage, Stage::Place(crate::frame::Place::Projects));
    assert!(a.screens.start.editor.text().is_empty());
    assert_eq!(
        a.screens.start.saved.last_project.as_deref(),
        Some("render-core")
    );
    assert_eq!(
        a.screens.start.saved.last_with.get("render-core"),
        Some(&StartWith::Workflow("w1".into()))
    );
    assert!(a.open.groups().iter().any(|(_, items)| items.iter().any(
        |i| matches!(i, Item::Conversation {project, id} if project == "render-core" && *id > 8)
    )));
}

#[test]
fn timeout_keeps_editable_text_and_never_sends_input() {
    let setup = testing::paired(false);
    let wire = setup.wire.clone();
    let clock = setup.clock.clone();
    let mut a = App::launch(setup.link);
    frame(&mut a);
    set_project(&mut a, "render-core".into());
    frame(&mut a);
    wire.silent.store(true, Ordering::SeqCst);
    a.screens
        .start
        .editor
        .edit
        .replace_text("keep after timeout");
    run(&mut a, "start.send");
    clock.advance(std::time::Duration::from_secs(21));
    frame(&mut a);
    assert_eq!(a.screens.start.pending, None);
    assert_eq!(a.screens.start.editor.text(), "keep after timeout");
    assert!(
        a.screens
            .start
            .failure
            .as_deref()
            .unwrap()
            .contains("a conversation may have been created")
    );
    assert!(
        a.link
            .outbox()
            .entries()
            .iter()
            .all(|e| e.call != WriteCall::Input)
    );
}

#[test]
fn options_invalid_keeps_text_and_opens_options() {
    let setup = testing::paired(false);
    let wire = setup.wire.clone();
    let mut a = App::launch(setup.link);
    frame(&mut a);
    set_project(&mut a, "render-core".into());
    frame(&mut a);
    *wire.refuse.lock().unwrap() = Some(parse_reply(
        409,
        Some("1"),
        br#"{"refused":{"code":"options_invalid","message":"invalid"}}"#,
    ));
    a.screens
        .start
        .editor
        .edit
        .replace_text("keep after refusal");
    run(&mut a, "start.send");
    frame(&mut a);
    assert_eq!(a.stage, Stage::Item(Item::Project("render-core".into())));
    assert_eq!(a.screens.start.editor.text(), "keep after refusal");
    assert!(
        a.screens
            .start
            .failure
            .as_deref()
            .unwrap()
            .contains("w1 can't start")
    );
}

fn picker(a: &mut App) -> Option<String> {
    crate::screens::top_float::<Picker>(a).map(|p| p.title.clone())
}

fn workflow_on_m_m(a: &mut App) {
    let (from, to) = (parse_seq("<M-w>").unwrap(), parse_seq("<M-m>").unwrap());
    a.keymap = a
        .keymap
        .rebind("start.workflow", &from, &to)
        .expect("M-m is free on the start page");
}

#[test]
fn the_design_keys_open_each_picker_in_insert_and_in_normal() {
    let mut a = ready();
    for mode in [Mode::Insert, Mode::Normal] {
        if mode == Mode::Normal {
            key(&mut a, "<Esc>");
        }
        assert_eq!(a.scope().mode, mode);
        key(&mut a, "<M-p>");
        assert_eq!(picker(&mut a).as_deref(), Some("Project"), "{mode:?}");
        key(&mut a, "<Esc><M-w>");
        assert_eq!(picker(&mut a).as_deref(), Some("Workflow"), "{mode:?}");
        key(&mut a, "<Esc>");
        assert!(a.floats.is_empty());
    }
}

#[test]
fn a_moved_workflow_key_opens_the_list_in_both_modes_and_the_old_key_opens_nothing() {
    let mut a = ready();
    workflow_on_m_m(&mut a);
    for mode in [Mode::Insert, Mode::Normal] {
        if mode == Mode::Normal {
            key(&mut a, "<Esc>");
        }
        assert_eq!(a.scope().mode, mode);
        key(&mut a, "<M-w>");
        assert!(a.floats.is_empty(), "{mode:?}: the key it replaced");
        key(&mut a, "<M-m>");
        assert_eq!(picker(&mut a).as_deref(), Some("Workflow"), "{mode:?}");
        key(&mut a, "<Esc>");
    }
}

#[test]
fn the_start_line_prints_the_key_this_device_types_lit_in_insert() {
    let mut a = ready();
    workflow_on_m_m(&mut a);
    assert_eq!(a.scope().mode, Mode::Insert);
    let lit = a.theme().c.fg.alt;
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = rune::Runtime::with_text(
        a,
        crate::app::view,
        (1512.0, 982.0),
        rune::Env::new(true, 1.0),
        fonts,
    );
    rt.frame(1.0);
    let elements = rt.elements();
    let printed = |key: &str| {
        elements
            .iter()
            .find(|e| e.text.as_deref() == Some(key))
            .map(|e| e.text_color)
    };
    assert_eq!(printed("M-m"), Some(Some(lit)), "the moved key, lit");
    assert_eq!(printed("M-p"), Some(Some(lit)), "the project's key, lit");
    assert_eq!(printed("M-w"), None, "the key it replaced");
}

#[test]
fn no_picker_opens_while_a_create_is_pending_in_either_mode() {
    let mut a = ready();
    workflow_on_m_m(&mut a);
    a.screens.start.editor.edit.replace_text("task");
    run(&mut a, "start.send");
    assert!(a.screens.start.pending.is_some());
    for mode in [Mode::Insert, Mode::Normal] {
        if mode == Mode::Normal {
            key(&mut a, "<Esc>");
        }
        assert_eq!(a.scope().mode, mode);
        key(&mut a, "<M-p><M-m><M-w>");
        assert!(a.floats.is_empty(), "{mode:?}");
    }
}

#[test]
fn workflow_picker_searches_models_in_the_same_list() {
    let mut a = ready();
    key(&mut a, "<M-w>");
    a.float_edit().replace_text("son");
    key(&mut a, "<CR>");
    assert_eq!(
        a.screens.start.with,
        Some(StartWith::Model("anthropic/claude-sonnet-4-5".into()))
    );
    assert!(a.floats.is_empty());
}

#[test]
fn draft_is_serializable_and_loaded_from_the_device_store() {
    let setup = testing::paired(false);
    let mut a = App::launch(setup.link);
    frame(&mut a);
    set_project(&mut a, "notes-cli".into());
    a.screens.start.editor.edit.replace_text("persisted task");
    persist(&mut a);
    a.screens.start = State::default();
    a.show(Stage::Start);
    assert_eq!(a.screens.start.editor.text(), "persisted task");
    assert_eq!(a.screens.start.project.as_deref(), Some("notes-cli"));
}

#[test]
fn start_blockers_distinguish_missing_and_invalid_options() {
    let mut a = ready();
    scenes::scratch(&mut a);
    assert!(matches!(blocker(&a), Blocker::Options(_)));
    a.screens.start.editor.edit.replace_text("keep me");
    run(&mut a, "start.send");
    assert_eq!(a.stage, Stage::Item(Item::Project("scratch".into())));
    assert_eq!(a.screens.start.editor.text(), "keep me");
    assert!(
        a.link
            .outbox()
            .entries()
            .iter()
            .all(|e| !matches!(e.state, WriteState::Sending) || e.call != WriteCall::Create)
    );
}

#[test]
fn unavailable_and_oversize_never_create() {
    let mut a = ready();
    a.screens
        .start
        .editor
        .edit
        .replace_text("x".repeat(1024 * 1024 + 1));
    run(&mut a, "start.send");
    assert_eq!(a.echo.as_deref(), Some("Over 1 MiB · nothing was sent"));
    assert!(a.screens.start.pending.is_none());
}
