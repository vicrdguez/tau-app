//! A short Failed timestamp must end at the column's right edge, not start in the
//! 84 px slot and leave about 50 px of blank space after it.
//!
//! Activity rows could fail by showing only the date of another day (two events that day look
//! the same), by cutting a long time, or by starting their text at a different x per row.
//!
//! Failed and Running could fail to line up: Running without Failed's time slot shifts every
//! column, or each group drops a different slot at the same width. The label could fall under
//! 200 px. An overflow line could start off the text column of the rows above it.
use super::drawn::{find, row_texts, scene};
use super::*;
use rune::{Cx, Env, Runtime};

fn view(a: &App, _: &mut Cx<App>) -> El {
    let rs = rows(a);
    let fault = rs
        .iter()
        .find(|r| r.group == "Failed" && r.at.is_some())
        .unwrap();
    let grid = list::Grid::home(a, &rs);
    super::rows::row_view(a.theme(), a, fault, false, &grid)
}

#[test]
fn failed_timestamp_ends_at_the_right_edge_of_its_fixed_slot() {
    let mut a = App::default();
    crate::app::link::settle(&mut a);
    let at = rows(&a)
        .into_iter()
        .find(|r| r.group == "Failed" && r.at.is_some())
        .unwrap()
        .at
        .unwrap();
    let stamp = crate::time::clock(Wall(at));
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(a, view, (744.0, 28.0), Env::new(true, 1.0), fonts);
    rt.frame(0.0);
    let time = rt
        .elements()
        .into_iter()
        .find(|e| e.text.as_deref() == Some(stamp.as_str()))
        .unwrap();
    assert!(
        time.rect[2] < 50.0,
        "the five-character timestamp has intrinsic width: {:?}",
        time.rect
    );
    assert!(
        (time.rect[0] + time.rect[2] - 732.0).abs() <= 1.0,
        "time ends at the column edge inside its 12 px row padding: {:?}",
        time.rect
    );
}

#[test]
fn activity_rows_show_the_timeline_time_and_start_their_text_on_one_line() {
    for (w, h) in [(1512.0, 1400.0), (1300.0, 1400.0), (800.0, 1400.0)] {
        let rt = scene("home", w, h);
        let a = &rt.state;
        let now = super::now(a);
        let events: Vec<_> = rows(a)
            .into_iter()
            .filter(|r| matches!(r.id, RowId::Event(..)))
            .collect();
        let stamps: Vec<_> = events
            .iter()
            .map(|r| crate::time::stamp(Wall(r.at.unwrap()), now))
            .collect();
        assert!(
            stamps.iter().any(|s| s.contains(' ')) && stamps.iter().any(|s| !s.contains(' ')),
            "the fixture has events from today and another day: {stamps:?}"
        );
        let els = rt.elements();
        let mut text_x = Vec::new();
        for (r, stamp) in events.iter().zip(&stamps) {
            let texts = row_texts(&els, &r.id);
            let time = find(&texts, stamp);
            assert!(
                time.rect[2] >= stamp.chars().count() as f32 * 6.0 - 0.5,
                "{stamp:?} is drawn whole at {w}: {:?}",
                time.rect
            );
            text_x.push(find(&texts, &r.label).rect[0]);
        }
        assert!(
            text_x.windows(2).all(|p| (p[0] - p[1]).abs() < 0.5),
            "event text starts on one line at {w}: {text_x:?}"
        );
    }
}

fn same(xs: &[f32]) -> bool {
    xs.windows(2).all(|p| (p[0] - p[1]).abs() < 0.5)
}

#[test]
fn failed_and_running_stand_on_one_grid_at_every_width() {
    for w in [1512.0, 1300.0, 800.0] {
        let rt = scene("home", w, 1400.0);
        let a = &rt.state;
        let els = rt.elements();
        let (mut project, mut id, mut extension, mut detail) = (vec![], vec![], vec![], vec![]);
        let rs: Vec<_> = rows(a)
            .into_iter()
            .filter(|r| {
                matches!(r.id, RowId::Conversation(_) | RowId::Fault(..))
                    && matches!(r.group.as_str(), "Failed" | "Running")
            })
            .collect();
        assert!(rs.iter().any(|r| r.group == "Running") && rs.iter().any(|r| r.group == "Failed"));
        for r in &rs {
            let texts = row_texts(&els, &r.id);
            project.push(find(&texts, &r.project).rect[0]);
            id.push(find(&texts, &format!("#{}", r.conversation.unwrap())).rect[0]);
            if let Some(e) = texts
                .iter()
                .find(|e| !r.extension.is_empty() && e.text.as_deref() == Some(&r.extension))
            {
                extension.push(e.rect[0]);
            }
            let value = if r.group == "Running" {
                &r.model
            } else {
                &r.reason
            };
            if let Some(e) = texts.iter().find(|e| e.text.as_deref() == Some(value)) {
                detail.push(e.rect[0]);
            }
            if matches!(r.id, RowId::Conversation(_)) {
                let head: String = r.label.chars().take(8).collect();
                let label = texts
                    .iter()
                    .find(|e| e.text.as_deref().is_some_and(|s| s.starts_with(&head)))
                    .unwrap()
                    .rect[2];
                assert!(label >= 200.0, "{} keeps 200 px at {w}: {label}", r.label);
            }
        }
        assert!(same(&project), "project at {w}: {project:?}");
        assert!(same(&id), "#id at {w}: {id:?}");
        assert!(same(&extension), "extension at {w}: {extension:?}");
        assert!(same(&detail), "reason / model at {w}: {detail:?}");
        assert_eq!(
            detail.len(),
            rs.len(),
            "the shared column shows each group's value at {w}"
        );
    }
}

#[test]
fn overflow_lines_start_on_the_text_column_above_them() {
    let rt = scene("home", 1512.0, 1400.0);
    let els = rt.elements();
    let rs = rows(&rt.state);
    let more = rs
        .iter()
        .find(|r| r.id == RowId::MoreEvents)
        .expect("the fixture's Activity overflows");
    let event = rs
        .iter()
        .find(|r| matches!(r.id, RowId::Event(..)))
        .unwrap();
    let more_x = find(&row_texts(&els, &more.id), &more.label).rect[0];
    let event_x = find(&row_texts(&els, &event.id), &event.label).rect[0];
    assert!(
        (more_x - event_x).abs() < 0.5,
        "older line {more_x}, event text {event_x}"
    );

    fn failed(a: &App, _: &mut Cx<App>) -> El {
        let rs = rows(a);
        let grid = list::Grid::home(a, &rs);
        let run = rs
            .iter()
            .find(|r| r.group == "Failed" && matches!(r.id, RowId::Conversation(_)))
            .unwrap();
        let more = ListRow::new(
            RowId::MoreRuns,
            "Failed",
            "",
            "2 more failed runs · in Conversations".into(),
        );
        rune::col()
            .w_full()
            .child(super::rows::row_view(a.theme(), a, run, false, &grid))
            .child(super::rows::row_view(a.theme(), a, &more, false, &grid))
    }
    let mut a = App::default();
    crate::app::link::settle(&mut a);
    let label = rows(&a)
        .into_iter()
        .find(|r| r.group == "Failed" && matches!(r.id, RowId::Conversation(_)))
        .unwrap()
        .label;
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(a, failed, (744.0, 56.0), Env::new(true, 1.0), fonts);
    rt.frame(0.0);
    let els = rt.elements();
    let run_x = super::drawn::texts(&els, &label)[0].rect[0];
    let more_x = super::drawn::texts(&els, "2 more failed runs · in Conversations")[0].rect[0];
    assert!(
        (more_x - run_x).abs() < 0.5,
        "more line {more_x}, failed label {run_x}"
    );
}
