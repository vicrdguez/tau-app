//! A float can shrink the viewport while leaving its selected row below the clipped edge.
//! The same failure applies to which-key, which opens without App::open_float. Conversation
//! queue and composer must shrink with the page, never remain behind the float.
//!
//! A float's own selection can leave its body (`spec.md` §6), at 800 × 600 above all:
//! - moving down puts the selected row behind the foot, or moving back up puts it under the head;
//! - the highlighted row is not the one the next key acts on;
//! - a float that opens with its cursor far down shows rows above it;
//! - typing a filter, or a window resize, leaves the body scrolled off the cursor;
//! - a short list scrolls at all, or the head or the foot moves with the rows.
use super::*;
use crate::keys::{Code, parse_seq};
use crate::screens::{self, home::list::RowId};
use crate::ui::{FLOAT_BODY, FLOAT_CURSOR};
use rune::{Env, Runtime};

fn scene(name: &str) -> Runtime<App> {
    scene_at(name, (1512.0, 700.0))
}
fn scene_at(name: &str, size: (f32, f32)) -> Runtime<App> {
    let mut a = App::default();
    scenes::all().iter().find(|(n, _)| *n == name).unwrap().1(&mut a);
    a.window = [size.0, size.1];
    let text = rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt = Runtime::with_text(a, view, size, Env::new(true, 1.0), text);
    rt.on_key = Some(input::on_key);
    advance(&mut rt);
    rt
}
/// Keys as the window sends them: through `on_key`, then into the focused field.
fn press(rt: &mut Runtime<App>, seq: &str) {
    for k in parse_seq(seq).unwrap() {
        let mods = rune::Mods {
            ctrl: k.mods.ctrl,
            alt: k.mods.alt,
            cmd: k.mods.cmd,
            shift: k.mods.shift,
            ..rune::Mods::default()
        };
        let key = match k.code {
            Code::Char(c) => rune::Key::Text(c.to_string()),
            Code::Enter => rune::Key::Enter,
            Code::Esc => rune::Key::Escape,
            Code::Tab => rune::Key::Tab,
            Code::Backspace => rune::Key::Backspace,
            Code::Up => rune::Key::Up,
            Code::Down => rune::Key::Down,
            other => panic!("{other:?}"),
        };
        rt.key(rune::KeyEvent { key, mods });
        rt.frame(1.0);
    }
    rt.frame(1.0);
}
/// The selected row lies wholly inside the float's body, above its foot, and reads `label`.
fn shows(rt: &Runtime<App>, label: &str) {
    let els = rt.elements();
    let body = rect(rt, FLOAT_BODY);
    let foot = rect(rt, "float-foot");
    let cursor = els
        .iter()
        .find(|e| e.key.as_deref() == Some(FLOAT_CURSOR) && !e.exiting)
        .unwrap_or_else(|| panic!("no selected row for {label}"));
    let r = cursor.rect;
    assert!(
        r[1] >= body[1] - 0.5 && r[1] + r[3] <= body[1] + body[3] + 0.5 && r[1] + r[3] <= foot[1],
        "{label}: row {r:?}, body {body:?}, foot {foot:?}"
    );
    let inside = format!("{}/", cursor.path);
    assert!(
        els.iter()
            .any(|e| e.path.starts_with(&inside) && e.text.as_deref() == Some(label)),
        "the highlighted row doesn't read {label}"
    );
}
fn settings_label(a: &App, i: usize) -> String {
    let Some((Float::Settings(f), _)) = a.floats.last() else {
        panic!("settings")
    };
    match &crate::floats::settings::rows(a.settings.keymap, &f.query.text)[i] {
        crate::floats::settings::Row::Field(field) => field.label().to_string(),
        crate::floats::settings::Row::Key(k) => k.command.desc.clone(),
    }
}
fn settings_cursor(a: &App) -> usize {
    match a.floats.last() {
        Some((Float::Settings(f), _)) => f.cursor,
        _ => panic!("settings"),
    }
}
fn bind(a: &mut App) -> &mut crate::floats::project_bind::Bind {
    screens::top_float::<crate::floats::project_bind::Bind>(a).expect("bind")
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

#[test]
fn settings_keeps_its_selected_row_in_view_past_the_first_page_and_back() {
    let mut rt = scene_at("settings", (800.0, 600.0));
    shows(&rt, &settings_label(&rt.state, 0));
    let top = rect(&rt, FLOAT_CURSOR);
    for (down, up) in [("<C-j>", "<C-k>"), ("<Down>", "<Up>")] {
        for _ in 0..14 {
            press(&mut rt, down);
            shows(&rt, &settings_label(&rt.state, settings_cursor(&rt.state)));
        }
        assert_eq!(settings_cursor(&rt.state), 14);
        for _ in 0..14 {
            press(&mut rt, up);
            shows(&rt, &settings_label(&rt.state, settings_cursor(&rt.state)));
        }
        assert_eq!(settings_cursor(&rt.state), 0);
        assert_eq!(rect(&rt, FLOAT_CURSOR), top, "back where it opened");
    }
}

#[test]
fn bind_keeps_every_extension_in_view_and_the_highlight_is_what_toggles() {
    let mut rt = scene_at("bind-float", (800.0, 600.0));
    let names: Vec<String> = bind(&mut rt.state)
        .extensions
        .iter()
        .map(|e| e.name.clone())
        .collect();
    assert!(names.len() >= 3, "{names:?}");
    shows(&rt, &names[0]);
    for name in names.iter().skip(1) {
        press(&mut rt, "<C-j>");
        shows(&rt, name);
    }
    for name in names.iter().rev().skip(1) {
        press(&mut rt, "<C-k>");
        shows(&rt, name);
    }
    let last = names.len() - 1;
    rt.update(|a, _| bind(a).cursor = last);
    for _ in 0..3 {
        rt.frame(1.0);
    }
    press(&mut rt, "<C-k><C-j>");
    shows(&rt, &names[last]);
}

#[test]
fn a_float_opened_far_down_shows_its_row_and_keeps_it_through_a_resize() {
    let mut rt = scene_at("home", (1512.0, 982.0));
    rt.update(|a, _| {
        let mut sheet = crate::floats::Sheet::keys(a.settings.keymap);
        sheet.cursor = 40;
        a.open_float(Float::Sheet(sheet));
    });
    advance(&mut rt);
    let label = |a: &App| match a.floats.last() {
        Some((Float::Sheet(s), _)) => s.rows[s.shown()[s.cursor]].text.clone(),
        _ => panic!("sheet"),
    };
    shows(&rt, &label(&rt.state));
    rt.set_viewport((800.0, 600.0), 1.0);
    advance(&mut rt);
    shows(&rt, &label(&rt.state));
}

#[test]
fn typing_a_filter_shows_the_first_match_and_clearing_it_the_top() {
    let mut rt = scene_at("settings", (800.0, 600.0));
    press(&mut rt, &"<C-j>".repeat(12));
    press(&mut rt, "motion");
    assert_eq!(settings_cursor(&rt.state), 0);
    shows(&rt, "motion");
    press(&mut rt, &"<BS>".repeat(6));
    shows(&rt, &settings_label(&rt.state, 0));
}

#[test]
fn every_picker_keeps_its_selection_in_view_at_800_by_600() {
    for (name, down, up) in [
        ("palette", "<C-j>", "<C-k>"),
        ("find", "<C-j>", "<C-k>"),
        ("keys-sheet", "<C-j>", "<C-k>"),
        ("start-project-picker", "<C-j>", "<C-k>"),
        ("start-workflow-picker", "<C-j>", "<C-k>"),
        ("conversation-model-picker", "<C-j>", "<C-k>"),
        ("filter", "<C-j>", "<C-k>"),
        ("unknown-outcome", "j", "k"),
    ] {
        let mut rt = scene_at(name, (800.0, 600.0));
        for step in 0..24 {
            press(&mut rt, if step < 12 { down } else { up });
            let body = rect(&rt, FLOAT_BODY);
            let r = rt
                .elements()
                .into_iter()
                .find(|e| e.key.as_deref() == Some(FLOAT_CURSOR))
                .unwrap_or_else(|| panic!("{name}: no selected row"))
                .rect;
            assert!(
                r[1] >= body[1] - 0.5 && r[1] + r[3] <= body[1] + body[3] + 0.5,
                "{name} step {step}: row {r:?}, body {body:?}"
            );
        }
    }
}

#[test]
fn the_extensions_label_sits_beside_the_first_extension() {
    let rt = scene_at("bind-float", (1512.0, 982.0));
    let text = |s: &str| {
        rt.elements()
            .into_iter()
            .find(|e| e.text.as_deref() == Some(s))
            .unwrap_or_else(|| panic!("no {s}"))
            .rect
    };
    let (label, first) = (text("extensions"), text("w1"));
    let middle = |r: [f32; 4]| r[1] + r[3] / 2.0;
    assert!(
        (middle(label) - middle(first)).abs() <= 1.0,
        "label {label:?}, first row {first:?}"
    );
}

#[test]
fn a_bind_field_that_takes_the_caret_back_is_shown() {
    let mut rt = scene_at("bind-float", (800.0, 600.0));
    press(&mut rt, "<C-j><C-j>");
    rt.update(|a, _| bind(a).name.edit.replace_text("Not A Name"));
    press(&mut rt, "<CR>");
    assert!(bind(&mut rt.state).part == crate::floats::project_bind::Part::Name);
    let body = rect(&rt, FLOAT_BODY);
    let field = rect(&rt, "bind-name");
    assert!(
        field[1] >= body[1] - 0.5 && field[1] + field[3] <= body[1] + body[3] + 0.5,
        "name field {field:?}, body {body:?}"
    );
}

#[test]
fn messages_open_on_the_newest() {
    let mut rt = scene_at("home", (800.0, 600.0));
    rt.update(|a, _| {
        for i in 0..30 {
            a.say(format!("message {i}"));
        }
        a.open_float(Float::messages(a.messages.len()));
    });
    advance(&mut rt);
    shows(&rt, "message 29");
}
