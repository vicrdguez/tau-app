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

// An empty place could fail by reading the same whether the server has nothing, the filter
// excludes everything, or every row is hidden on this device; by printing `␣th` where it does
// nothing; by a pointer path that doesn't reset or reveal; by counting hidden rows outside the
// filter; or by "1 faulted tasks".

fn press_all(rt: &mut rune::Runtime<App>, seq: &str) {
    rt.update(|a, _| press(a, seq));
    for _ in 0..3 {
        rt.frame(1.0);
    }
}

#[test]
fn hiding_every_fault_says_how_many_and_shows_them_again() {
    use crate::screens::home::drawn::{click, drawn, shown};
    for (project, sentence) in [
        (None, "2 faulted tasks hidden on this device"),
        (Some("render-core"), "1 faulted task hidden on this device"),
    ] {
        let mut a = fixture(Place::Faults);
        filter_project(&mut a, Place::Faults, project.map(Into::into));
        let n = rows(&a).len();
        for _ in 0..n {
            press(&mut a, "x");
        }
        assert!(rows(&a).is_empty());
        let s = a.scope();
        assert!(
            a.theme().keymap.chip("<leader>th", s.mode, s.ctx).lit,
            "the printed ␣th works here"
        );
        let mut rt = drawn(a, 1512.0, 982.0);
        let text = shown(&rt);
        assert!(text.iter().any(|s| s == sentence), "{project:?}: {text:?}");
        assert!(!text.iter().any(|s| s == "No matching rows."));
        click(&mut rt, "show hidden");
        assert_eq!(rows(&rt.state).len(), n, "show hidden reveals them");
        assert!(rows(&rt.state).iter().all(|r| r.hidden));
        press_all(&mut rt, "x");
        press_all(&mut rt, "<leader>th");
        assert_eq!(rows(&rt.state).len(), 1, "one fault restored");
        assert!(!rows(&rt.state)[0].hidden);
    }
}

#[test]
fn hidden_failed_runs_are_counted_for_the_filter() {
    use crate::screens::home::drawn::{drawn, shown};
    let mut a = fixture(Place::Conversations);
    a.screens.places.conversations.state = Some("failed".into());
    filter_project(&mut a, Place::Conversations, Some("notes-cli".into()));
    press(&mut a, "x");
    assert!(rows(&a).is_empty());
    let text = shown(&drawn(a, 1512.0, 982.0));
    assert!(
        text.iter()
            .any(|s| s == "1 failed run hidden on this device"),
        "{text:?}"
    );
}

#[test]
fn a_filter_that_matches_nothing_names_itself_and_clears() {
    use crate::screens::home::drawn::{click, drawn, shown};
    for (place, state, sentence, control) in [
        (
            Place::Faults,
            None,
            "No faulted tasks in notes-cli.",
            "show all projects",
        ),
        (
            Place::Timeline,
            None,
            "No events in notes-cli yet.",
            "show all projects",
        ),
        (
            Place::Conversations,
            Some("failed"),
            "No failed conversations in forecasts.",
            "show all",
        ),
    ] {
        let mut a = fixture(place);
        let project = if place == Place::Conversations {
            "forecasts"
        } else {
            "notes-cli"
        };
        filter_project(&mut a, place, Some(project.into()));
        a.screens.places.conversations.state = state.map(Into::into);
        if place != Place::Timeline {
            filter_project(&mut a, Place::Timeline, Some("render-core".into()));
        }
        assert!(rows(&a).is_empty());
        let mut rt = drawn(a, 1512.0, 982.0);
        let text = shown(&rt);
        assert!(text.iter().any(|s| s == sentence), "{text:?}");
        click(&mut rt, control);
        let a = &rt.state;
        assert!(settings(a).project.is_none() && settings(a).state.is_none());
        assert!(!rows(a).is_empty(), "{place:?} shows every project again");
        if place != Place::Timeline {
            assert_eq!(
                a.screens.places.timeline.project.as_deref(),
                Some("render-core"),
                "another place keeps its filter"
            );
        }
    }
}

#[test]
fn a_place_with_nothing_on_the_server_says_so() {
    use crate::screens::home::drawn::{drawn, shown};
    let mut f = tau_client::Fixture::design().value().clone();
    for (k, v) in f["calls"].as_object_mut().unwrap() {
        if k.starts_with("faults ") || k.starts_with("conversations ") {
            v["result"]["items"] = serde_json::json!([]);
        }
    }
    for (place, sentence) in [
        (Place::Faults, "No faulted tasks."),
        (Place::Conversations, "No conversations yet."),
    ] {
        let mut a = App::default();
        a.link = tau_client::Link::fixture(tau_client::Fixture::parse(&f.to_string()).unwrap());
        link::settle(&mut a);
        a.show(Stage::Place(place));
        link::settle(&mut a);
        assert!(rows(&a).is_empty());
        let text = shown(&drawn(a, 1512.0, 982.0));
        assert!(text.iter().any(|s| s == sentence), "{text:?}");
        assert!(!text.iter().any(|s| s.starts_with("show all")));
    }
}

// A filter could fail to clear: no pointer path from the breadcrumb, a clear that resets
// another place's filter, a `show all` drawn while nothing is filtered. Conversations' group
// heads could read whole-project totals as result counts.

fn header_texts(rt: &rune::Runtime<App>, s: &str) -> usize {
    crate::screens::home::drawn::texts(&rt.elements(), s)
        .iter()
        .filter(|e| e.rect[1] < 44.0)
        .count()
}

#[test]
fn the_breadcrumb_clears_only_its_own_place_filter() {
    use crate::screens::home::drawn::{click, drawn};
    let mut a = fixture(Place::Timeline);
    let rt = drawn(fixture(Place::Timeline), 1512.0, 982.0);
    assert_eq!(header_texts(&rt, "show all"), 0, "nothing to clear");
    filter_project(&mut a, Place::Conversations, Some("forecasts".into()));
    a.screens.places.conversations.state = Some("idle".into());
    filter_project(&mut a, Place::Timeline, Some("render-core".into()));
    let mut rt = drawn(a, 1512.0, 982.0);
    assert_eq!(header_texts(&rt, "show all"), 1);
    let n = rows(&rt.state).len();
    click(&mut rt, "show all");
    let a = &rt.state;
    assert!(a.screens.places.timeline.project.is_none());
    assert!(rows(a).len() > n, "every project's events again");
    assert_eq!(
        a.screens.places.conversations.project.as_deref(),
        Some("forecasts")
    );
    assert_eq!(
        a.screens.places.conversations.state.as_deref(),
        Some("idle")
    );
    assert_eq!(header_texts(&rt, "show all"), 0);
}

#[test]
fn a_state_filter_counts_the_rows_shown_before_the_total() {
    use crate::screens::home::drawn::{drawn, shown};
    let text = shown(&drawn(fixture(Place::Conversations), 1512.0, 982.0));
    assert!(
        text.iter().any(|s| s == "1 running · 1 failed · 4 in all"),
        "{text:?}"
    );
    let mut a = fixture(Place::Conversations);
    a.screens.places.conversations.state = Some("running".into());
    let mut rt = drawn(a, 1512.0, 982.0);
    let text = shown(&rt);
    assert!(
        text.iter().any(|s| s == "1 running shown · 4 in all"),
        "{text:?}"
    );
    assert!(text.iter().any(|s| s == "1 running shown · 7 in all"));
    assert!(!text.iter().any(|s| s.contains("failed")));
    assert_eq!(header_texts(&rt, "show all"), 1);
    crate::screens::home::drawn::click(&mut rt, "show all");
    assert!(rt.state.screens.places.conversations.state.is_none());
}

/// Faults rows could fail to line up: fields flowing after the error put the project at a
/// different x per row; an `after` part cut mid-ID; the error pushing the slots off the row.
#[test]
fn fault_rows_stand_on_fixed_slots() {
    use crate::screens::home::drawn::{find, row_texts, scene};
    for w in [1512.0, 1300.0, 800.0] {
        let rt = scene("faults", w, 982.0);
        let a = &rt.state;
        let els = rt.elements();
        let rs = rows(a);
        assert_eq!(rs.len(), 2);
        let (mut project, mut id, mut time_end, mut after) = (vec![], vec![], vec![], vec![]);
        let mut error_end = vec![];
        for r in &rs {
            let texts = row_texts(&els, &r.id);
            let kind = find(&texts, &r.label);
            let p = find(&texts, &r.project).rect;
            project.push(p[0]);
            id.push(find(&texts, &format!("#{}", r.conversation.unwrap())).rect[0]);
            let stamp = crate::time::stamp(tau_client::Wall(r.at.unwrap()), home::now(a));
            if let Some(t) = texts
                .iter()
                .find(|e| e.text.as_deref() == Some(stamp.as_str()))
            {
                time_end.push(t.rect[0] + t.rect[2]);
            }
            let error = texts
                .iter()
                .find(|e| e.rect[0] > kind.rect[0] + kind.rect[2] && e.rect[0] < p[0])
                .expect("the error sits between the kind and the project");
            error_end.push(error.rect[0] + error.rect[2]);
            if let Some(d) = &r.after {
                let s = format!("after {d}");
                match texts.iter().find(|e| e.text.as_deref() == Some(s.as_str())) {
                    Some(e) => {
                        assert!(
                            e.rect[2] >= s.chars().count() as f32 * 6.0 - 0.5,
                            "{s} whole at {w}"
                        );
                        assert!(e.rect[0] > p[0], "after follows #id at {w}");
                        after.push(e.rect[0]);
                    }
                    None => assert!(
                        !texts
                            .iter()
                            .any(|e| e.text.as_deref().is_some_and(|t| t.starts_with("after"))),
                        "after is whole or absent at {w}"
                    ),
                }
            }
        }
        let same = |xs: &[f32]| xs.windows(2).all(|p| (p[0] - p[1]).abs() < 0.5);
        assert!(same(&project), "project at {w}: {project:?}");
        assert!(same(&id), "#id at {w}: {id:?}");
        assert!(
            same(&time_end) && [0, rs.len()].contains(&time_end.len()),
            "time ends on one line at {w}, in every row or none: {time_end:?}"
        );
        assert!(
            error_end.iter().all(|e| *e <= project[0] - 11.5),
            "the error ends before the project at {w}: {error_end:?} {project:?}"
        );
        if w != 1300.0 {
            assert_eq!(after.len(), 1, "after shows at {w}");
            assert_eq!(time_end.len(), rs.len(), "time shows at {w}");
        }
    }
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
