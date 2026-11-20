use super::*;
use crate::app::{commands, input, link};
use crate::keys::parse_seq;
use tau_client::{Fixture, Link};

fn fixture() -> App {
    let mut a = App::default();
    link::settle(&mut a);
    reconcile(&mut a);
    a
}

fn press(a: &mut App, seq: &str) {
    for k in parse_seq(seq).unwrap() {
        input::press(a, k, false, &mut rune::Tx::default());
    }
}

#[test]
fn nothing_received_is_not_calm() {
    let a = App::default();
    assert_eq!(title(&a), "Home");
    assert!(rows(&a).is_empty());
    let a = fixture();
    assert_eq!(title(&a), "Needs you");
}

#[test]
fn home_orders_decisions_and_failures_and_matches_the_count() {
    let a = fixture();
    let r = rows(&a);
    assert_eq!(
        r[0].id,
        RowId::Decision(DecisionRef::new("render-core", "w1:3:design#2"))
    );
    assert_eq!(
        r[1].id,
        RowId::Decision(DecisionRef::new("forecasts", "w2:2:pi-durable-2#1"))
    );
    let failed: Vec<_> = r.iter().filter(|r| r.group == "Failed").collect();
    assert_eq!(failed.len(), 4);
    assert!(matches!(failed[0].id, RowId::Fault(..)));
    assert_eq!(
        failed[1].id,
        RowId::Conversation(ConvRef::new("notes-cli", 5))
    );
    assert_eq!(
        failed[2].id,
        RowId::Conversation(ConvRef::new("render-core", 5))
    );
    assert_eq!(a.server.counts.failed, Some(4));
}

#[test]
fn motions_open_objects_and_hide_only_the_selected_failure() {
    let mut a = fixture();
    press(&mut a, "]g");
    assert_eq!(selected(&a).unwrap().group, "Failed");
    let id = selected(&a).unwrap().id;
    press(&mut a, "x");
    assert!(!rows(&a).iter().any(|r| r.id == id));
    assert_eq!(a.server.counts.failed, Some(3));
    press(&mut a, "<leader>n");
    assert_eq!(
        a.stage,
        Stage::Item(Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into()
        })
    );
    assert_eq!(a.open.iter().count(), 1);
}

#[test]
fn cursor_identity_survives_insertion_and_advances_on_removal() {
    let mut c = Cursor::default();
    let a = RowId::Conversation(ConvRef::new("notes-cli", 4));
    let b = RowId::Conversation(ConvRef::new("notes-cli", 5));
    let x = RowId::Conversation(ConvRef::new("notes-cli", 6));
    c.reconcile(&[a.clone(), b.clone()]);
    assert_eq!(c.id, Some(a.clone()));
    c.reconcile(&[x.clone(), a.clone(), b.clone()]);
    assert_eq!(c.id, Some(a));
    c.reconcile(&[x, b.clone()]);
    assert_eq!(c.id, Some(b));
}

#[test]
fn failed_run_hiding_expires_after_a_nonfailed_poll() {
    let mut a = fixture();
    let conv = ConvRef::new("notes-cli", 5);
    a.hidden.runs.insert(conv.clone());
    let mut f = Fixture::design().value().clone();
    for r in f["calls"]["conversations notes-cli"]["result"]["items"]
        .as_array_mut()
        .unwrap()
    {
        if r["conversation"] == 5 {
            r["state"] = "idle".into();
        }
    }
    a.link = Link::fixture(Fixture::parse(&f.to_string()).unwrap());
    link::settle(&mut a);
    reconcile(&mut a);
    assert!(!a.hidden.runs.contains(&conv));
}

#[test]
fn since_counts_only_timestamped_objects_and_needs_five_minutes() {
    let mut a = fixture();
    let now = now(&a);
    a.screens.home.away.base = Some(now.0 - 299_999);
    a.screens.home.away.resume(now);
    assert!(!a.screens.home.away.visible);
    a.screens.home.away.base = Some(now.0 - 15_000_000);
    a.screens.home.away.resume(now);
    assert!(a.screens.home.away.visible);
    assert_eq!(since_counts(&a), (3, 1, 5));
}

#[test]
fn slot_dropping_preserves_label_and_fixed_identity_columns() {
    let all = Slots {
        extension: true,
        detail: true,
        time: true,
    };
    assert_eq!(Slots::home(744.0, true, true), all);
    assert_eq!(Slots::home(744.0, true, false), all);
    assert_eq!(
        Slots::home(744.0, false, true),
        Slots { time: false, ..all },
        "Running alone keeps no time column"
    );
    let failed_at_1300 = Slots {
        extension: false,
        detail: true,
        time: false,
    };
    assert_eq!(Slots::home(532.0, true, false), failed_at_1300);
    assert_eq!(
        Slots::home(532.0, true, true),
        failed_at_1300,
        "both groups drop the same slots, and the reason keeps the shared column"
    );
    assert_eq!(
        Slots::home(532.0, false, true),
        Slots {
            extension: true,
            detail: false,
            time: false,
        },
        "Running alone drops its model before its extension"
    );
    assert_eq!(Slots::running(532.0), Slots::home(532.0, false, true));
}

#[test]
fn failed_tag_destination_is_home_at_failed() {
    let mut a = fixture();
    a.show(Stage::Place(Place::Timeline));
    open_failed(&mut a);
    assert_eq!(a.stage, Stage::Place(Place::Home));
    assert_eq!(selected(&a).unwrap().group, "Failed");
    commands::run_id(&mut a, "decision.next", &mut rune::Tx::default());
    assert!(matches!(a.stage, Stage::Item(Item::Decision { .. })));
}
