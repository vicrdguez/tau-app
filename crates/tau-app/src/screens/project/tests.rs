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
    keys(&mut a, "<Space><CR>");
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

// How Bind's keys could fail (spec §6):
// - Tab or S-Tab in the checklist ticks a row, or doesn't go on to a field, or doesn't wrap;
// - S-Tab from name doesn't wrap to the checklist;
// - Space doesn't tick the row under the cursor, or ticks another, or opens the leader;
// - j k, C-j C-k, Down Up or C-n C-p don't move the cursor, or move it past either end;
// - in Add extensions Tab still ticks, or Space doesn't;
// - the confirmation names other extensions than the ticked ones;
// - the checklist's foot prints a key that doesn't work there.

fn bind_float(a: &mut App) -> &mut crate::floats::project_bind::Bind {
    super::super::top_float::<crate::floats::project_bind::Bind>(a).expect("bind float")
}

fn ticked(a: &mut App) -> Vec<String> {
    bind_float(a).selected()
}

fn part(a: &mut App) -> crate::floats::project_bind::Part {
    bind_float(a).part
}

fn confirm_sentence(a: &App) -> String {
    match a.floats.last() {
        Some((Float::Confirm(c), _)) => c.sentence.clone(),
        _ => panic!("no Confirm float"),
    }
}

/// The Bind float with a valid name and directory, its checklist holding the keys.
fn bind_checklist() -> App {
    let mut a = ready();
    run(&mut a, "project.bind");
    a.float_edit().replace_text("scratch");
    keys(&mut a, "<Tab>");
    a.float_edit().replace_text("/home/vrod/src/scratch");
    keys(&mut a, "<Tab>");
    a
}

#[test]
fn bind_tab_walks_name_directory_extensions_and_wraps_without_ticking() {
    use crate::floats::project_bind::Part;
    let mut a = bind_checklist();
    assert!(part(&mut a) == Part::Extensions);
    keys(&mut a, "<Tab>");
    assert!(part(&mut a) == Part::Name, "Tab wraps to the name");
    keys(&mut a, "<S-Tab>");
    assert!(
        part(&mut a) == Part::Extensions,
        "S-Tab wraps to the checklist"
    );
    keys(&mut a, "<S-Tab>");
    assert!(part(&mut a) == Part::Directory);
    keys(&mut a, "<Tab><Tab><Tab>");
    assert!(part(&mut a) == Part::Directory);
    assert!(ticked(&mut a).is_empty(), "Tab never ticks");
    assert_eq!(bind_float(&mut a).name.text(), "scratch");
}

#[test]
fn bind_checklist_moves_with_j_k_ctrl_and_arrows_and_space_ticks_the_cursor_row() {
    let mut a = bind_checklist();
    keys(&mut a, "<Down><Down>");
    assert_eq!(bind_float(&mut a).cursor, 2);
    keys(&mut a, "<Down>");
    assert_eq!(bind_float(&mut a).cursor, 2, "the last row holds");
    keys(&mut a, "<Up>");
    assert_eq!(bind_float(&mut a).cursor, 1);
    keys(&mut a, "kk");
    assert_eq!(bind_float(&mut a).cursor, 0, "the first row holds");
    keys(&mut a, "j<C-j>");
    assert_eq!(bind_float(&mut a).cursor, 2);
    keys(&mut a, "<C-k><C-p>");
    assert_eq!(bind_float(&mut a).cursor, 0);
    keys(&mut a, "<C-n>");
    assert_eq!(bind_float(&mut a).cursor, 1);
    keys(&mut a, "<Space>");
    assert_eq!(ticked(&mut a), vec!["w2"]);
    assert!(a.keys.pending().is_empty(), "Space is no leader here");
    keys(&mut a, "<Space>");
    assert!(ticked(&mut a).is_empty(), "Space unticks");
}

#[test]
fn bind_confirmation_names_exactly_the_ticked_extensions() {
    let mut a = bind_checklist();
    keys(&mut a, "<CR>");
    assert!(
        confirm_sentence(&a).starts_with("Bind scratch with no extensions?"),
        "{}",
        confirm_sentence(&a)
    );
    keys(&mut a, "n<Space><CR>");
    assert!(
        confirm_sentence(&a).starts_with("Bind scratch with w1?"),
        "{}",
        confirm_sentence(&a)
    );
    keys(&mut a, "njj<Space><CR>");
    assert!(
        confirm_sentence(&a).starts_with("Bind scratch with w1, websearch?"),
        "{}",
        confirm_sentence(&a)
    );
    keys(&mut a, "y");
    assert!(
        a.link.outbox().entries().iter().any(
            |e| e.call == WriteCall::Bind && e.body["extensions"] == json!(["w1", "websearch"])
        )
    );
}

#[test]
fn add_extensions_ticks_with_space_and_never_with_tab() {
    let mut a = ready();
    run(&mut a, "project.add");
    keys(&mut a, "<Tab><S-Tab>");
    assert!(ticked(&mut a).is_empty(), "Tab never ticks");
    keys(&mut a, "<Space>");
    assert_eq!(ticked(&mut a), vec!["w2"]);
}

#[test]
fn the_checklist_foot_prints_space_enter_and_esc_lit() {
    let a = bind_checklist();
    let scope = a.scope();
    for seq in ["<Space>", "<CR>", "<Esc>"] {
        let chip = a.settings.keymap.chip(seq, scope.mode, scope.ctx);
        assert!(chip.lit, "{seq} works in the checklist");
    }
    assert_eq!(
        a.settings
            .keymap
            .chip("<Space>", scope.mode, scope.ctx)
            .printed,
        "␣"
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
