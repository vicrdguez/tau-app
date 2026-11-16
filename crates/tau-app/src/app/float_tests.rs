//! A float can shrink the viewport while leaving its selected row below the clipped edge.
//! The same failure applies to which-key, which opens without App::open_float. Conversation
//! queue and composer must shrink with the page, never remain behind the float.
use super::*;
use crate::screens::{self, home::list::RowId};
use rune::{Env, Runtime};

fn scene(name: &str) -> Runtime<App> {
    let mut a = App::default();
    scenes::all().iter().find(|(n, _)| *n == name).unwrap().1(&mut a);
    a.window = [1512.0, 700.0];
    let text = rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(a, view, (1512.0, 700.0), Env::new(true, 1.0), text);
    advance(&mut rt);
    rt
}
fn advance(rt: &mut Runtime<App>) {
    rt.update(|a, tx| {
        poll(a, a.now + 1.0, tx);
    });
    for _ in 0..3 {
        rt.frame(1.0);
    }
}
fn rect(rt: &Runtime<App>, key: &str) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("missing {key}"))
        .rect
}
fn above(rt: &Runtime<App>, key: &str) {
    let r = rect(rt, key);
    let f = rect(rt, "bottom-float");
    assert!(
        r[1] >= 44.0 && r[1] + r[3] <= f[1] + 1.0,
        "{key}: {r:?}, float: {f:?}"
    );
}

#[test]
fn floats_reveal_the_home_place_and_decision_cursor() {
    for name in ["home", "timeline", "decision-long"] {
        let mut rt = scene(name);
        rt.update(|a, _| {
            screens::page(a).keys(a, &[crate::keys::Key::char('G')], None);
        });
        advance(&mut rt);
        let key = match &rt.state.stage {
            Stage::Place(Place::Home) => format!(
                "list-{:?}",
                rt.state.screens.home.cursor.id.as_ref().unwrap()
            ),
            Stage::Place(_) => format!(
                "list-{:?}",
                screens::places::settings(&rt.state)
                    .cursor
                    .id
                    .as_ref()
                    .unwrap()
            ),
            _ => "decision-cursor".into(),
        };
        rt.update(|a, _| a.open_float(Float::palette()));
        advance(&mut rt);
        above(&rt, &key);
    }
}

#[test]
fn which_key_reveals_the_selected_home_row() {
    let mut rt = scene("home");
    rt.update(|a, _| {
        let id = screens::home::rows(a)
            .into_iter()
            .rev()
            .find(|r| matches!(r.id, RowId::Event(..)))
            .unwrap()
            .id;
        screens::home::select(a, id);
    });
    advance(&mut rt);
    let key = format!(
        "list-{:?}",
        rt.state.screens.home.cursor.id.as_ref().unwrap()
    );
    rt.update(|a, tx| {
        input::press(a, crate::keys::Key::char(' '), false, tx);
    });
    advance(&mut rt);
    above(&rt, &key);
}

#[test]
fn conversation_composer_and_queue_lift_above_a_float() {
    let mut rt = scene("conversation-steer-insert");
    rt.update(|a, _| a.open_float(Float::palette()));
    advance(&mut rt);
    above(&rt, "composer-container");
    let float = rect(&rt, "bottom-float");
    let queue = rt
        .elements()
        .into_iter()
        .find(|e| {
            e.text
                .as_deref()
                .is_some_and(|s| s.contains("queued inputs can't be taken back"))
        })
        .expect("queue stays visible");
    assert!(queue.rect[1] + queue.rect[3] < float[1]);
    let output = rt
        .elements()
        .into_iter()
        .find(|e| e.text.as_deref().is_some_and(|s| s.contains("fail 0")))
        .expect("running output");
    assert!(
        output.rect[1] + output.rect[3] < queue.rect[1],
        "follow keeps the running output above the queue: {:?}",
        output.rect
    );
}
