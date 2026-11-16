//! Restart failures: visibility and item order reset; closed conversation drafts disappear;
//! item and place cursors reset; start choices or drafts disappear; hidden failures and the
//! away base reset; notes require opening a decision to load; servers share state; forgetting
//! leaves in-memory state that can be saved again; polling writes files.

use super::*;
use crate::screens::conversation::transcript::Stop;
use crate::screens::decision::{notes::Draft, state::Cursor};
use crate::screens::home::list::RowId;
use std::sync::Arc;
use tau_client::fixture::FixtureTransport;
use tau_client::store::{ClientConfig, Secrets, Store};
use tau_client::{ConvRef, DecisionRef, Fixture, LinkConfig};

fn launch(dir: &std::path::Path, url: &str) -> App {
    let store = Store::at(dir.to_path_buf());
    store
        .save_client(&ClientConfig {
            server: Some(url.into()),
            device: Some("laptop".into()),
        })
        .unwrap();
    let f = Fixture::design();
    let secrets = Secrets::memory();
    secrets.save(url, &f.token()).unwrap();
    App::launch(Link::with_transport(
        LinkConfig {
            dir: Some(dir.to_path_buf()),
            secrets,
            ..LinkConfig::standard()
        },
        Arc::new(FixtureTransport::new(f)),
    ))
}

#[test]
fn restart_restores_device_state_without_restoring_the_stage() {
    let dir = tempfile::tempdir().unwrap();
    let url = Fixture::design().url();
    let mut a = launch(dir.path(), &url);
    link::settle(&mut a);
    let c = ConvRef::new("notes-cli", 4);
    let d = DecisionRef::new("render-core", "w1:3:design#2");
    let items = [
        Item::Conversation {
            project: "notes-cli".into(),
            id: 4,
        },
        Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into(),
        },
        Item::Project("notes-cli".into()),
    ];
    for item in &items {
        a.show(Stage::Item(item.clone()));
    }
    a.sidebar_visible = false;
    a.details_visible = true;
    a.screens
        .conversation
        .conv_mut(&c)
        .composer
        .edit
        .replace_text("unsent draft");
    a.screens.conversation.conv_mut(&c).cursor = Some(Stop::Head(tau_client::EntryId(12)));
    a.screens.decision.page(&d).cursor = Some(Cursor::Text(20));
    a.screens.decision.notes.set(
        &d,
        vec![Draft {
            text: "keep this note".into(),
            range: None,
            widened: false,
        }],
        a.link.store(),
    );
    a.screens.start.editor.edit.replace_text("start draft");
    a.screens.start.project = Some("render-core".into());
    a.screens.start.with = Some(tau_client::outbox::StartWith::Workflow("w1".into()));
    a.screens.places.conversations.project = Some("render-core".into());
    a.screens.places.conversations.state = Some("failed".into());
    a.screens.places.conversations.cursor.id =
        Some(RowId::Conversation(ConvRef::new("render-core", 5)));
    a.screens.home.cursor.id = Some(RowId::Decision(d.clone()));
    a.hidden.runs.insert(ConvRef::new("render-core", 5));
    a.hidden
        .faults
        .insert(("forecasts".into(), tau_client::TaskId(1)));
    let expected_open = a.open.clone();
    link::on_focus(&mut a, false, &mut Tx::default());
    let base = a.screens.home.away.base;
    drop(a);
    let b = launch(dir.path(), &url);
    assert_eq!(b.stage, Stage::Start);
    assert!(!b.sidebar_visible);
    assert!(b.details_visible);
    assert_eq!(b.open, expected_open);
    assert_eq!(
        b.screens.conversation.conv(&c).unwrap().composer.text(),
        "unsent draft"
    );
    assert_eq!(
        b.screens.conversation.conv(&c).unwrap().cursor,
        Some(Stop::Head(tau_client::EntryId(12)))
    );
    assert_eq!(
        b.screens.decision.page_of(&d).unwrap().cursor,
        Some(Cursor::Text(20))
    );
    assert_eq!(b.screens.decision.notes.of(&d)[0].text, "keep this note");
    assert_eq!(b.screens.start.editor.text(), "start draft");
    assert_eq!(b.screens.start.project.as_deref(), Some("render-core"));
    assert_eq!(
        b.screens.start.with,
        Some(tau_client::outbox::StartWith::Workflow("w1".into()))
    );
    assert_eq!(
        b.screens.places.conversations.project.as_deref(),
        Some("render-core")
    );
    assert_eq!(
        b.screens.places.conversations.state.as_deref(),
        Some("failed")
    );
    assert_eq!(
        b.screens.places.conversations.cursor.id,
        Some(RowId::Conversation(ConvRef::new("render-core", 5)))
    );
    assert_eq!(b.screens.home.cursor.id, Some(RowId::Decision(d)));
    assert!(b.hidden.runs.contains(&ConvRef::new("render-core", 5)));
    assert!(
        b.hidden
            .faults
            .contains(&("forecasts".into(), tau_client::TaskId(1)))
    );
    assert_eq!(b.screens.home.away.base, base);
}

#[test]
fn closed_conversation_drafts_survive_and_other_servers_start_clean() {
    let dir = tempfile::tempdir().unwrap();
    let url = Fixture::design().url();
    let mut a = launch(dir.path(), &url);
    let c = ConvRef::new("notes-cli", 4);
    a.screens
        .conversation
        .conv_mut(&c)
        .composer
        .edit
        .replace_text("closed draft");
    a.sidebar_visible = false;
    link::on_focus(&mut a, false, &mut Tx::default());
    let b = launch(dir.path(), "http://other.example:6283");
    assert!(b.sidebar_visible);
    assert!(b.screens.conversation.conv(&c).is_none());
    let b = launch(dir.path(), &url);
    assert!(b.open.is_empty());
    assert_eq!(
        b.screens.conversation.conv(&c).unwrap().composer.text(),
        "closed draft"
    );
}

#[test]
fn restart_keeps_each_project_cursor_and_an_unfinished_note() {
    use crate::screens::{
        decision::state::{Anchor, Editing},
        project::Target,
    };
    let dir = tempfile::tempdir().unwrap();
    let url = Fixture::design().url();
    let mut a = launch(dir.path(), &url);
    link::settle(&mut a);
    a.show(Stage::Item(Item::Project("render-core".into())));
    a.screens.project.cursor = Some(Target::Work(Place::Faults));
    a.show(Stage::Item(Item::Project("notes-cli".into())));
    a.screens.project.cursor = Some(Target::Work(Place::Home));
    let d = DecisionRef::new("render-core", "w1:3:design#2");
    a.screens.decision.editing = Some(Editing::new(
        d.clone(),
        Anchor::Whole,
        None,
        "half a thought",
    ));
    link::on_close(&mut a, &mut Tx::default());
    let mut b = launch(dir.path(), &url);
    link::settle(&mut b);
    assert_eq!(
        b.screens.decision.editing.as_ref().unwrap().editor.text(),
        "half a thought"
    );
    assert_eq!(b.screens.decision.editing.as_ref().unwrap().decision, d);
    b.show(Stage::Item(Item::Project("render-core".into())));
    assert_eq!(b.screens.project.cursor, Some(Target::Work(Place::Faults)));
    b.show(Stage::Item(Item::Project("notes-cli".into())));
    assert_eq!(b.screens.project.cursor, Some(Target::Work(Place::Home)));
}

// A restored ID must survive frames before its list or options have arrived.
#[test]
fn restored_cursors_wait_for_their_server_data() {
    use crate::screens::project::{FieldKey, Target};
    let dir = tempfile::tempdir().unwrap();
    let url = Fixture::design().url();
    let mut a = launch(dir.path(), &url);
    link::settle(&mut a);
    let c = RowId::Conversation(ConvRef::new("render-core", 5));
    a.screens.home.cursor.id = Some(c.clone());
    a.screens.places.conversations.cursor.id = Some(c.clone());
    a.show(Stage::Item(Item::Project("render-core".into())));
    let field = Target::Field(FieldKey {
        project: "render-core".into(),
        extension: "w1".into(),
        key: "reviewer".into(),
    });
    a.screens.project.cursor = Some(field.clone());
    link::on_close(&mut a, &mut Tx::default());
    let mut b = launch(dir.path(), &url);
    b.live = false;
    poll(&mut b, 0.0, &mut Tx::default());
    assert_eq!(b.screens.home.cursor.id, Some(c.clone()));
    b.show(Stage::Place(Place::Conversations));
    poll(&mut b, 0.0, &mut Tx::default());
    assert_eq!(b.screens.places.conversations.cursor.id, Some(c));
    b.show(Stage::Item(Item::Project("render-core".into())));
    assert_eq!(b.screens.project.cursor, Some(field.clone()));
    link::settle(&mut b);
    poll(&mut b, 1.0, &mut Tx::default());
    assert_eq!(b.screens.project.cursor, Some(field));
}

#[test]
fn polling_does_not_save_a_changed_start_draft() {
    let dir = tempfile::tempdir().unwrap();
    let url = Fixture::design().url();
    let mut a = launch(dir.path(), &url);
    link::settle(&mut a);
    a.live = false;
    a.screens.start.editor.edit.replace_text("not flushed yet");
    poll(&mut a, 1.0, &mut Tx::default());
    assert!(
        a.link
            .store()
            .load::<serde_json::Value>("start.json")
            .is_none()
    );
    link::on_close(&mut a, &mut Tx::default());
    assert_eq!(
        launch(dir.path(), &url).screens.start.editor.text(),
        "not flushed yet"
    );
}

#[test]
fn forget_clears_memory_and_restart_state() {
    let dir = tempfile::tempdir().unwrap();
    let url = Fixture::design().url();
    let mut a = launch(dir.path(), &url);
    a.screens.start.editor.edit.replace_text("private draft");
    a.show(Stage::Item(Item::Project("render-core".into())));
    link::on_focus(&mut a, false, &mut Tx::default());
    commands::run_id(&mut a, "server.forget", &mut Tx::default());
    input::press(
        &mut a,
        crate::keys::Key::char('y'),
        false,
        &mut Tx::default(),
    );
    assert!(a.open.is_empty());
    assert_eq!(a.screens.start.editor.text(), "");
    link::on_focus(&mut a, false, &mut Tx::default());
    let b = launch(dir.path(), &url);
    assert!(b.open.is_empty());
    assert_eq!(b.screens.start.editor.text(), "");
}
