use super::*;
use crate::app::{input, link};
use crate::keys::parse_seq;

fn fixture(place: Place) -> App {
    let mut a = App::default();
    link::settle(&mut a);
    a.show(Stage::Place(place));
    a
}
fn press(a: &mut App, seq: &str) {
    for key in parse_seq(seq).unwrap() {
        input::press(a, key, false, &mut rune::Tx::default());
    }
}

#[test]
fn filters_belong_to_places_and_never_home() {
    let mut a = fixture(Place::Conversations);
    filter_project(&mut a, Place::Conversations, Some("notes-cli".into()));
    a.screens.places.conversations.state = Some("failed".into());
    let rows = rows(&a);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].id,
        RowId::Conversation(ConvRef::new("notes-cli", 5))
    );
    a.show(Stage::Place(Place::Home));
    assert_eq!(
        home::rows(&a)
            .iter()
            .filter(|r| r.group == "Needs you")
            .count(),
        3
    );
    a.show(Stage::Place(Place::Timeline));
    assert!(settings(&a).project.is_none());
    a.show(Stage::Place(Place::Conversations));
    assert_eq!(settings(&a).project.as_deref(), Some("notes-cli"));
}

#[test]
fn conversations_are_grouped_and_newest_first() {
    let a = fixture(Place::Conversations);
    let r = rows(&a);
    assert_eq!(r.len(), 13);
    assert_eq!(r[0].id, RowId::Conversation(ConvRef::new("forecasts", 3)));
    let notes: Vec<_> = r.iter().filter(|r| r.project == "notes-cli").collect();
    assert_eq!(
        notes[0].id,
        RowId::Conversation(ConvRef::new("notes-cli", 5))
    );
}

#[test]
fn timeline_orders_by_time_and_faults_toggle_hidden_locally() {
    let mut a = fixture(Place::Timeline);
    let r = rows(&a);
    assert!(r.windows(2).all(|w| w[0].at >= w[1].at));
    a.show(Stage::Place(Place::Faults));
    assert_eq!(rows(&a).len(), 2);
    press(&mut a, "x");
    assert_eq!(rows(&a).len(), 1);
    press(&mut a, "<leader>th");
    assert_eq!(rows(&a).len(), 2);
    assert_eq!(rows(&a).iter().filter(|r| r.hidden).count(), 1);
}

#[test]
fn place_settings_roundtrip_with_object_cursor() {
    let mut a = fixture(Place::Conversations);
    press(&mut a, "jj");
    let before = settings(&a).clone();
    let saved = serde_json::to_string(&before).unwrap();
    let restored: ListState = serde_json::from_str(&saved).unwrap();
    assert_eq!(restored.cursor.id, before.cursor.id);
    a.show(Stage::Place(Place::Faults));
    a.show(Stage::Place(Place::Conversations));
    assert_eq!(settings(&a).cursor.id, before.cursor.id);
}

#[test]
fn find_prefixes_and_filter_picker_choose_without_writes() {
    let mut a = fixture(Place::Conversations);
    assert!(
        floats::find_rows(&a, "p:render")
            .iter()
            .all(|r| matches!(r.target, floats::Choice::Object(RowId::Project(_))))
    );
    let n = a.link.outbox().entries().len();
    press(&mut a, "<leader>pf");
    assert_eq!(a.scope().ctx, Ctx::Picker);
    press(&mut a, "<C-j><CR>");
    assert!(settings(&a).project.is_some());
    assert_eq!(a.link.outbox().entries().len(), n);
}

// Replacing a search must not search only the previous search's matches.
#[test]
fn a_new_search_can_find_a_row_the_previous_search_hid() {
    let mut a = fixture(Place::Conversations);
    for r in list::conversations(&a) {
        if let RowId::Conversation(c) = r.id {
            a.link.want_label(c);
        }
    }
    link::settle(&mut a);
    settings_mut(&mut a).search = "migration".into();
    let found = matching_rows(&a, "config loader");
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].id,
        RowId::Conversation(ConvRef::new("notes-cli", 4))
    );
}

#[test]
fn server_and_outbox_floats_keep_global_keys_out() {
    let mut a = fixture(Place::Conversations);
    press(&mut a, "<leader>os");
    assert_eq!(a.scope().ctx, Ctx::Server);
    press(&mut a, "<leader>oh");
    assert_eq!(a.stage, Stage::Place(Place::Conversations));
    press(&mut a, "<Esc><leader>ru");
    assert_eq!(a.scope().ctx, Ctx::Outbox);
    press(&mut a, "<Esc>");
    assert!(a.floats.is_empty());
}
