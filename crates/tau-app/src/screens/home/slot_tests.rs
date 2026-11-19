//! A short Failed timestamp must end at the column's right edge, not start in the
//! 84 px slot and leave about 50 px of blank space after it.
//!
//! Activity rows could fail by showing only the date of another day (two events that day look
//! the same), by cutting a long time, or by starting their text at a different x per row.
use super::*;
use rune::{Cx, Env, Runtime, runtime::ElementInfo};

fn view(a: &App, _: &mut Cx<App>) -> El {
    let fault = rows(a)
        .into_iter()
        .find(|r| r.group == "Failed" && r.at.is_some())
        .unwrap();
    let grid = list::Grid::home(a, &rows(a));
    super::rows::row_view(a.theme(), a, &fault, false, &grid)
}

/// The scene drawn by the whole app at `w × h`, settled.
fn scene(name: &str, w: f32, h: f32) -> Runtime<App> {
    let mut a = App::default();
    crate::app::scenes::all()
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap()
        .1(&mut a);
    a.window = [w, h];
    let fonts =
        rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(a, crate::app::view, (w, h), Env::new(true, 1.0), fonts);
    for _ in 0..3 {
        rt.frame(1.0);
    }
    rt
}

/// The text elements drawn inside the row `id`.
fn row_texts(els: &[ElementInfo], id: &RowId) -> Vec<ElementInfo> {
    let key = format!("list-{id:?}");
    let Some(row) = els.iter().position(|e| e.key.as_deref() == Some(&key)) else {
        panic!("no row {key}");
    };
    let inside = |mut i: usize| {
        while let Some(p) = els[i].parent {
            if p == row {
                return true;
            }
            i = p;
        }
        false
    };
    (0..els.len())
        .filter(|&i| inside(i) && els[i].text.is_some())
        .map(|i| els[i].clone())
        .collect()
}

fn find<'a>(texts: &'a [ElementInfo], s: &str) -> &'a ElementInfo {
    texts
        .iter()
        .find(|e| e.text.as_deref() == Some(s))
        .unwrap_or_else(|| {
            panic!(
                "no {s:?} in {:?}",
                texts.iter().map(|e| &e.text).collect::<Vec<_>>()
            )
        })
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
