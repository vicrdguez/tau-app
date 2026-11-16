use super::*;
use crate::app::{commands, input, link};
use crate::keys::parse_seq;
use serde_json::json;
use tau_client::outbox::WriteCall;

fn ready() -> App {
    let mut a = App::default();
    crate::screens::start::scenes::base(&mut a);
    a.live = true;
    a.show(Stage::Item(Item::Project("render-core".into())));
    crate::app::testing::settle(&mut a.link);
    a
}
fn run(a: &mut App, id: &str) {
    commands::run_id(a, id, &mut Tx::default());
}
fn keys(a: &mut App, seq: &str) {
    for k in parse_seq(seq).unwrap() {
        input::press(a, k, false, &mut Tx::default());
    }
}
fn field(a: &mut App, extension: &str, key: &str) {
    a.screens.project.cursor = Some(Target::Field(FieldKey {
        project: "render-core".into(),
        extension: extension.into(),
        key: key.into(),
    }));
}

#[test]
fn schema_order_and_secret_display_follow_the_schema() {
    let a = ready();
    let fields = fields(&a, "render-core");
    assert_eq!(
        fields
            .iter()
            .filter(|f| f.id.extension == "w1")
            .map(|f| f.id.key.as_str())
            .collect::<Vec<_>>(),
        vec!["designer", "implementer", "reviewer", "base"]
    );
    let secret = fields.iter().find(|f| f.id.key == "apiKey").unwrap();
    assert_eq!(secret.display(), "set · hidden");
}

#[test]
fn edits_and_resets_are_drafts_and_save_groups_by_extension() {
    let mut a = ready();
    field(&mut a, "w1", "base");
    run(&mut a, "field.reset");
    let id = FieldKey {
        project: "render-core".into(),
        extension: "w1".into(),
        key: "base".into(),
    };
    assert_eq!(a.screens.project.drafts.get(&id), Some(&json!(null)));
    a.screens.project.drafts.insert(
        FieldKey {
            key: "designer".into(),
            ..id.clone()
        },
        json!("openai/gpt-5.1"),
    );
    a.screens.project.drafts.insert(
        FieldKey {
            extension: "websearch".into(),
            key: "provider".into(),
            ..id
        },
        json!("tavily"),
    );
    assert!(
        a.link
            .outbox()
            .entries()
            .iter()
            .all(|e| e.call != WriteCall::SetOptions)
    );
    run(&mut a, "options.save");
    run(&mut a, "options.save");
    let writes: Vec<_> = a
        .link
        .outbox()
        .entries()
        .iter()
        .filter(|e| e.call == WriteCall::SetOptions)
        .collect();
    assert_eq!(writes.len(), 2);
    assert_eq!(
        writes
            .iter()
            .find(|e| e.body["extension"] == "w1")
            .unwrap()
            .body["patch"],
        json!({"base":null,"designer":"openai/gpt-5.1"})
    );
}

#[test]
fn a_successful_save_keeps_a_newer_edit() {
    let mut a = ready();
    field(&mut a, "w1", "base");
    run(&mut a, "field.reset");
    run(&mut a, "options.save");
    let id = FieldKey {
        project: "render-core".into(),
        extension: "w1".into(),
        key: "base".into(),
    };
    a.screens.project.drafts.insert(id.clone(), json!("newer"));
    a.live = true;
    for _ in 0..5 {
        link::pump(&mut a, &mut Tx::default());
    }
    assert_eq!(a.screens.project.drafts.get(&id), Some(&json!("newer")));
}

#[test]
fn secret_editor_is_empty_and_cancel_does_not_save() {
    let mut a = ready();
    field(&mut a, "websearch", "apiKey");
    run(&mut a, "field.edit");
    assert!(
        a.screens
            .project
            .editing
            .as_ref()
            .unwrap()
            .editor
            .text()
            .is_empty()
    );
    keys(&mut a, "<Esc><Esc>");
    assert!(a.screens.project.drafts.is_empty());
}

#[test]
fn bind_fields_escape_to_checklist_and_confirm_before_writing() {
    let mut a = ready();
    run(&mut a, "project.bind");
    assert_eq!(a.scope().mode, Mode::Insert);
    a.float_edit().replace_text("scratch");
    keys(&mut a, "<Tab>");
    a.float_edit().replace_text("/home/vrod/src/scratch");
    keys(&mut a, "<Esc>");
    assert_eq!(a.scope().mode, Mode::Command);
    keys(&mut a, "<CR>");
    assert!(matches!(a.floats.last(), Some((Float::Confirm(_), _))));
    assert!(
        a.link
            .outbox()
            .entries()
            .iter()
            .all(|e| e.call != WriteCall::Bind)
    );
    keys(&mut a, "n");
    assert!(
        a.link
            .outbox()
            .entries()
            .iter()
            .all(|e| e.call != WriteCall::Bind)
    );
    keys(&mut a, "<CR>y");
    assert!(
        a.link
            .outbox()
            .entries()
            .iter()
            .any(|e| e.call == WriteCall::Bind && e.body["project"] == "scratch")
    );
}

#[test]
fn invalid_bind_does_not_open_confirm() {
    let mut a = ready();
    run(&mut a, "project.bind");
    a.float_edit().replace_text("Bad Name");
    keys(&mut a, "<Tab>");
    a.float_edit().replace_text("relative/path");
    keys(&mut a, "<Esc><CR>");
    assert!(!matches!(a.floats.last(), Some((Float::Confirm(_), _))));
}

#[test]
fn add_extensions_excludes_bound_extensions_and_requires_confirm() {
    let mut a = ready();
    run(&mut a, "project.add");
    let f = super::super::top_float::<crate::floats::project_bind::Bind>(&mut a).unwrap();
    assert_eq!(
        f.extensions
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        vec!["w2"]
    );
    keys(&mut a, "<Tab><CR>");
    assert!(matches!(a.floats.last(), Some((Float::Confirm(_), _))));
    keys(&mut a, "y");
    assert!(
        a.link
            .outbox()
            .entries()
            .iter()
            .any(|e| e.call == WriteCall::AddExtensions && e.body["extensions"] == json!(["w2"]))
    );
}

#[test]
fn project_list_opens_by_identity_and_counts_use_the_polls() {
    let mut a = ready();
    let counts = counts(&a, "render-core");
    assert_eq!(counts.conversations, Some((7, 1, 1)));
    assert_eq!(counts.decisions, Some(1));
    assert_eq!(counts.faults, Some(1));
    a.show(Stage::Place(crate::frame::Place::Projects));
    a.screens.project.list_cursor = Some("notes-cli".into());
    run(&mut a, "list.open");
    assert_eq!(a.stage, Stage::Item(Item::Project("notes-cli".into())));
}
