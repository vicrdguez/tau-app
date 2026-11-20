use super::*;
use crate::app::{input, link};
use crate::keys::parse_seq;

fn press(a: &mut App, seq: &str) {
    for key in parse_seq(seq).unwrap() {
        input::press(a, key, false, &mut rune::Tx::default());
    }
}

#[test]
fn peek_waits_three_hundred_ms_and_never_opens_an_item() {
    let mut a = App::default();
    link::settle(&mut a);
    home::reconcile(&mut a);
    let mut tx = rune::Tx::default();
    Peek.poll(&mut a, 0.0, &mut tx);
    assert!(a.screens.peek.shown.is_none());
    Peek.poll(&mut a, 0.299, &mut tx);
    assert!(a.screens.peek.shown.is_none());
    Peek.poll(&mut a, 0.300, &mut tx);
    assert!(matches!(a.screens.peek.shown, Some(RowId::Decision(_))));
    assert!(a.open.is_empty());
}

#[test]
fn wide_k_does_nothing_and_narrow_float_has_its_own_keys() {
    let mut a = App::default();
    link::settle(&mut a);
    home::reconcile(&mut a);
    press(&mut a, "K");
    assert!(a.floats.is_empty());
    a.window[0] = 800.0;
    press(&mut a, "K");
    assert_eq!(a.scope().ctx, Ctx::Peek);
    press(&mut a, "j");
    assert_eq!(home::selected(&a).unwrap().project, "forecasts");
    press(&mut a, "<CR>");
    assert!(a.floats.is_empty());
    assert!(matches!(a.stage, Stage::Item(Item::Decision { .. })));
}

// A final tool-call-only message must not erase the last agent text from the preview.
#[test]
fn tool_call_tail_keeps_the_last_agent_text() {
    let mut a = App::default();
    link::settle(&mut a);
    let c = ConvRef::new("notes-cli", 4);
    a.link.show(Slot::Peek, Some(c.clone()));
    link::settle(&mut a);
    let tr = &a.link.server().transcripts[&c];
    assert!(
        last_text(tr)
            .unwrap()
            .contains("The network is closed here")
    );
}

#[test]
fn sidebar_peek_targets_the_open_item_not_the_home_cursor() {
    let mut a = App::default();
    link::settle(&mut a);
    a.focus = Focus::Sidebar;
    let item = Item::Conversation {
        project: "notes-cli".into(),
        id: 4,
    };
    a.open.open(item.clone());
    a.sidebar_cursor = crate::frame::Row::Item(item);
    press(&mut a, "K");
    assert_eq!(
        target(&a),
        Some(RowId::Conversation(tau_client::ConvRef::new(
            "notes-cli",
            4
        )))
    );
    press(&mut a, "q");
    assert_eq!(a.focus, Focus::Sidebar);
}

/// The peek after `change`, once its delay has run: it must name the row under the cursor.
fn settled_after(a: &mut App, at: f64, change: impl FnOnce(&mut App)) -> Option<RowId> {
    change(a);
    let mut tx = rune::Tx::default();
    Peek.poll(a, at, &mut tx);
    Peek.poll(a, at + 0.300, &mut tx);
    a.screens.peek.shown.clone()
}

#[test]
fn the_peek_follows_the_cursor_through_a_filter_a_clear_and_hidden_rows() {
    let mut a = App::default();
    link::settle(&mut a);
    a.show(Stage::Place(Place::Conversations));
    places::reconcile(&mut a);
    let cursor = |a: &App| places::selected(a).map(|r| r.id);
    let first = settled_after(&mut a, 0.0, |_| {});
    assert_eq!(
        first,
        Some(RowId::Conversation(ConvRef::new("forecasts", 3)))
    );
    let filtered = settled_after(&mut a, 1.0, |a| {
        places::settings_mut(a).state = Some("running".into());
        places::reconcile(a);
    });
    assert_ne!(filtered, first, "the filter removed the first row");
    assert_eq!(filtered, cursor(&a));
    assert_eq!(settled_after(&mut a, 2.0, places::show_all), cursor(&a));
    a.show(Stage::Place(Place::Faults));
    places::reconcile(&mut a);
    let top = settled_after(&mut a, 3.0, |_| {});
    assert_eq!(top, cursor(&a));
    let after_hide = settled_after(&mut a, 4.0, |a| press(a, "x"));
    assert_ne!(after_hide, top, "x hid the row the peek showed");
    assert_eq!(after_hide, cursor(&a));
    press(&mut a, "<leader>thk");
    assert_eq!(
        settled_after(&mut a, 5.0, |_| {}),
        top,
        "the hidden row, shown"
    );
    let rehidden = settled_after(&mut a, 6.0, |a| press(a, "<leader>th"));
    assert_ne!(rehidden, top, "hiding hidden rows left the cursor's row");
    assert_eq!(rehidden, cursor(&a));
}
