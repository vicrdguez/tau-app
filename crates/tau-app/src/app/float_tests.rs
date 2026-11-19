//! A float can cover the page's cursor row or target and leave it under its edge. The same failure
//! applies to which-key, which opens without App::open_float. A conversation's queue and composer
//! must rise with the float's edge, never stay behind the float. The answer readback, which grows
//! with its notes, must not hide the page twice: once by the frame and once more by a page that
//! also makes room for it.
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

/// Bind's errors (`spec.md` §6) could fail by standing under the checklist, away from their
/// field, or under the other field, or out of the float body while their field has the caret.
#[test]
fn a_bind_error_stands_under_its_field_in_view_with_the_caret() {
    let cases = [
        (
            "Not A Name",
            "/home/vrod/src/scratch",
            "Use 1–63 lowercase letters",
            "bind-name",
            "bind-working directory",
        ),
        (
            "scratch",
            "relative/path",
            "Use an absolute path on the server.",
            "bind-working directory",
            "extensions",
        ),
    ];
    for (name, dir, words, field, next) in cases {
        let mut rt = scene_at("bind-float", (800.0, 600.0));
        press(&mut rt, "<C-j><C-j>");
        rt.update(|a, _| {
            bind(a).name.edit.replace_text(name);
            bind(a).directory.edit.replace_text(dir);
        });
        press(&mut rt, "<CR>");
        advance(&mut rt);
        let error = rt
            .elements()
            .into_iter()
            .find(|e| e.text.as_deref().is_some_and(|t| t.starts_with(words)))
            .unwrap_or_else(|| panic!("no {words:?}"))
            .rect;
        let below = |key: &str| {
            rt.elements()
                .into_iter()
                .find(|e| e.key.as_deref() == Some(key) || e.text.as_deref() == Some(key))
                .unwrap_or_else(|| panic!("no {key}"))
                .rect
        };
        let (f, n) = (below(field), below(next));
        assert!(
            f[1] + f[3] <= error[1] && error[1] + error[3] <= n[1],
            "{words}: field {f:?}, error {error:?}, next {n:?}"
        );
        let body = rect(&rt, FLOAT_BODY);
        assert!(
            error[1] >= body[1] - 0.5 && error[1] + error[3] <= body[1] + body[3] + 0.5,
            "{words}: error {error:?}, body {body:?}"
        );
    }
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

/// The picker's input: the name a screen reader says, and the words drawn after its caret, which
/// end before the count starts.
fn input_says(rt: &Runtime<App>) -> (String, String) {
    fn find(v: &serde_json::Value) -> Option<&serde_json::Value> {
        if v["key"] == "float-input" {
            return Some(v);
        }
        v["children"].as_array()?.iter().find_map(find)
    }
    let tree = rt.accessibility_json();
    let name = find(&tree).expect("the picker's input")["name"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let els = rt.elements();
    let field = els
        .iter()
        .position(|e| e.key.as_deref() == Some("float-input"))
        .unwrap();
    let after = |i: usize| {
        els.iter()
            .skip(i + 1)
            .find(|e| e.text.is_some() && !e.path.starts_with(&els[i].path))
            .unwrap()
    };
    let placeholder = after(field);
    let count = after(els.iter().position(|e| e.path == placeholder.path).unwrap());
    assert!(
        placeholder.rect[0] + placeholder.rect[2] <= count.rect[0],
        "{:?} runs into {:?}",
        placeholder.text,
        count.text
    );
    (name, placeholder.text.clone().unwrap())
}
fn has_text(rt: &Runtime<App>, s: &str) -> bool {
    rt.elements().iter().any(|e| e.text.as_deref() == Some(s))
}
/// Whether the chip before `word` in the foot is lit.
fn lit(rt: &Runtime<App>, word: &str) -> bool {
    let els = rt.elements();
    let hint = els
        .iter()
        .find(|e| e.text.as_deref() == Some(word))
        .and_then(|e| e.parent)
        .unwrap_or_else(|| panic!("no hint {word}"));
    let inside = format!("{}/", els[hint].path);
    let chip = els
        .iter()
        .filter(|e| e.path.starts_with(&inside) && e.text.as_deref() != Some(word))
        .find_map(|e| e.text_color)
        .expect("a chip");
    let t = rt.state.theme();
    assert!(chip == t.c.kind.literal || chip == t.c.fg.muted, "{chip:?}");
    chip == t.c.kind.literal
}
fn rows_shown(rt: &Runtime<App>) -> usize {
    let els = rt.elements();
    let body = els
        .iter()
        .position(|e| e.key.as_deref() == Some(FLOAT_BODY))
        .unwrap();
    let col = els.iter().position(|e| e.parent == Some(body)).unwrap();
    els.iter().filter(|e| e.parent == Some(col)).count()
}

#[test]
fn the_project_picker_says_how_to_filter_and_what_its_query_kept() {
    let mut rt = scene_at("start-project-picker", (1512.0, 982.0));
    assert_eq!(
        input_says(&rt),
        ("Filter projects".into(), "type to filter".into())
    );
    assert!(has_text(&rt, "3 projects"));
    assert!(lit(&rt, "choose"));
    press(&mut rt, "render");
    assert!(has_text(&rt, "1 of 3 projects"));
    assert!(!has_text(&rt, "type to filter"));
    assert_eq!(rows_shown(&rt), 1);
    press(&mut rt, &"<BS>".repeat(6));
    press(&mut rt, "websearch");
    assert!(has_text(&rt, "2 of 3 projects"));
    assert_eq!(rows_shown(&rt), 2);
    press(&mut rt, &"<BS>".repeat(9));
    press(&mut rt, "zzzz");
    assert!(has_text(&rt, "0 of 3 projects"));
    assert!(has_text(&rt, "No project matches \"zzzz\""));
    assert_eq!(rows_shown(&rt), 1, "one line where the rows were");
    assert!(
        rt.elements()
            .iter()
            .all(|e| e.key.as_deref() != Some(FLOAT_CURSOR))
    );
    assert!(!lit(&rt, "choose"), "⏎ resolves to nothing");
    press(&mut rt, "<CR>");
    assert_eq!(rt.state.floats.len(), 1, "⏎ chose nothing");
    press(&mut rt, &"<BS>".repeat(4));
    assert!(has_text(&rt, "3 projects"));
    assert_eq!(rows_shown(&rt), 3);
    shows(&rt, "render-core");
    assert!(lit(&rt, "choose"));
}

#[test]
fn find_says_what_it_searches_and_what_its_query_kept() {
    let mut rt = scene_at("find", (1512.0, 982.0));
    assert_eq!(
        input_says(&rt),
        (
            "Search projects, conversations, decisions and events".into(),
            "type to search".into()
        )
    );
    let total = crate::screens::places::floats::find_rows(&rt.state, "").len();
    assert!(total > 8, "{total}");
    assert!(has_text(&rt, &format!("{total} results")));
    press(&mut rt, "pi-durable");
    assert!(has_text(&rt, &format!("1 of {total} results")));
    assert_eq!(rows_shown(&rt), 1);
    press(&mut rt, &"<BS>".repeat(10));
    press(&mut rt, "notes-cli");
    let several = crate::screens::places::floats::find_rows(&rt.state, "notes-cli").len();
    assert!(several > 1);
    assert!(has_text(&rt, &format!("{several} of {total} results")));
    press(&mut rt, &"<BS>".repeat(9));
    press(&mut rt, "zzzz");
    assert!(has_text(&rt, &format!("0 of {total} results")));
    assert!(has_text(&rt, "Nothing matches \"zzzz\""));
    assert!(!lit(&rt, "choose"));
    press(&mut rt, "<CR>");
    assert_eq!(rt.state.floats.len(), 1, "⏎ opened nothing");
    press(&mut rt, &"<BS>".repeat(4));
    assert!(has_text(&rt, &format!("{total} results")));
    assert!(lit(&rt, "choose"));
    shows(&rt, "forecasts");
}

#[test]
fn a_palette_query_that_keeps_nothing_says_so_and_enter_keeps_it_open() {
    let mut rt = scene_at("palette", (800.0, 600.0));
    assert_eq!(input_says(&rt).0, "Filter commands");
    press(&mut rt, "zzzz");
    assert!(has_text(&rt, "No command matches \"zzzz\""));
    assert!(!lit(&rt, "choose"));
    press(&mut rt, "<CR>");
    assert_eq!(rt.state.floats.len(), 1);
}

/// The group heads the settings body shows, top to bottom, each with its count.
fn settings_heads(rt: &Runtime<App>) -> Vec<(String, String)> {
    let els = rt.elements();
    let body = els
        .iter()
        .find(|e| e.key.as_deref() == Some(FLOAT_BODY))
        .unwrap();
    let inside = format!("{}/", body.path);
    els.iter()
        .filter(|e| {
            e.path.starts_with(&inside) && matches!(e.text.as_deref(), Some("Preferences" | "Keys"))
        })
        .map(|head| {
            let count = els
                .iter()
                .find(|e| e.parent == head.parent && e.path != head.path && e.text.is_some())
                .and_then(|e| e.text.clone())
                .unwrap_or_default();
            (head.text.clone().unwrap(), count)
        })
        .collect()
}

#[test]
fn settings_groups_preferences_and_keys_under_counted_heads_the_cursor_skips() {
    let mut rt = scene_at("settings", (1512.0, 982.0));
    let keys = crate::floats::settings::rows(rt.state.settings.keymap, "").len() - 3;
    assert_eq!(
        settings_heads(&rt),
        vec![
            ("Preferences".into(), "3".into()),
            ("Keys".into(), keys.to_string())
        ]
    );
    assert!(has_text(&rt, &format!("3 preferences · {keys} keys")));
    press(&mut rt, "<C-j><C-j><C-j>");
    shows(&rt, &settings_label(&rt.state, 3));
    press(&mut rt, "<C-k>");
    shows(&rt, "motion");
    press(&mut rt, "motion");
    assert_eq!(
        settings_heads(&rt),
        vec![("Preferences".into(), "1".into())]
    );
    assert!(has_text(&rt, &format!("1 of {} settings", keys + 3)));
    shows(&rt, "motion");
    press(&mut rt, &"<BS>".repeat(6));
    press(&mut rt, "commands");
    assert_eq!(settings_heads(&rt), vec![("Keys".into(), "2".into())]);
    shows(&rt, "commands");
}

#[test]
fn find_rows_lead_with_their_kind_and_line_up_their_projects() {
    for size in [(1512.0, 982.0), (800.0, 600.0)] {
        let rt = scene_at("find", size);
        let t = rt.state.theme();
        let els = rt.elements();
        let body = els
            .iter()
            .position(|e| e.key.as_deref() == Some(FLOAT_BODY))
            .unwrap();
        let list = els.iter().position(|e| e.parent == Some(body)).unwrap();
        let rows: Vec<&rune::ElementInfo> = els
            .iter()
            .filter(|e| e.parent == Some(list) && e.rect[1] < rect(&rt, "float-foot")[1])
            .collect();
        assert!(rows.len() >= 5, "{}", rows.len());
        let mut icon_x = None;
        let mut slot_x = None;
        for row in rows {
            let inside = format!("{}/", row.path);
            let parts: Vec<&rune::ElementInfo> = els
                .iter()
                .filter(|e| e.path.starts_with(&inside) && e.text_color.is_some())
                .collect();
            let (icons, texts): (Vec<&rune::ElementInfo>, Vec<&rune::ElementInfo>) =
                parts.iter().partition(|e| e.text.is_none());
            assert_eq!(icons.len(), 1, "one kind icon: {:?}", row.path);
            assert_eq!(icons[0].text_color, Some(t.c.fg.muted));
            assert_eq!(icons[0].rect[2], 14.0);
            assert_eq!(*icon_x.get_or_insert(icons[0].rect[0]), icons[0].rect[0]);
            match texts.as_slice() {
                [name] => assert_eq!(name.text_color, Some(t.c.kind.worker), "a project row"),
                [label, project] => {
                    assert_eq!(label.text_color, Some(t.c.fg.default));
                    assert_eq!(project.text_color, Some(t.c.kind.worker));
                    let p = project.text.as_deref().unwrap();
                    assert!(!label.text.as_deref().unwrap().ends_with(p), "{p} twice");
                    assert_eq!(*slot_x.get_or_insert(project.rect[0]), project.rect[0]);
                    assert!(label.rect[0] + label.rect[2] <= project.rect[0]);
                }
                other => panic!("{:?}", other.iter().map(|e| &e.text).collect::<Vec<_>>()),
            }
        }
        assert!(slot_x.is_some(), "an object row at {size:?}");
    }
}

#[test]
fn the_answer_readback_keeps_the_cursor_above_it_and_hides_no_more_than_itself() {
    for size in [[1512.0, 700.0], [800.0, 600.0]] {
        let mut rt = scene("decision-long");
        rt.update(|a, _| a.window = size);
        rt.set_viewport((size[0], size[1]), 1.0);
        advance(&mut rt);
        rt.update(|a, tx| commands::run_id(a, "auth.answer", tx));
        advance(&mut rt);
        advance(&mut rt);
        above(&rt, "decision-cursor");
        let page = rect(&rt, "decision-page");
        let float = rect(&rt, "bottom-float");
        assert!(
            page[1] + page[3] >= float[1] - 1.0,
            "{size:?}: the page reaches the float: {page:?}, {float:?}"
        );
    }
}

/// How a float's printed keys could fail a screen reader (`spec.md` §4.6): which-key, the
/// Settings key cells, the keys sheet or the palette's key column name a key by its glyphs (`␣;`,
/// `⌘K`, `.`) instead of in words, or name it nowhere. The bottom bar's keys typed so far are
/// not the keymap's and are left out.
#[test]
fn every_key_a_float_prints_is_named_in_words() {
    for (scene, said) in [
        ("which-key-root", "Period"),
        ("palette", "Space Semicolon"),
        ("palette", "Command-K"),
        ("settings", "Space Semicolon"),
        ("keys-sheet", "Space Semicolon or Command-K"),
    ] {
        let rt = scene_at(scene, (1512.0, 982.0));
        let bar = 982.0 - f64::from(crate::theme::tokens().layout.height.bottombar);
        let nodes: Vec<_> = crate::app::testing::spoken(&rt)
            .into_iter()
            .filter(|n| n.bounds[1] < bar)
            .collect();
        assert!(
            nodes.iter().any(|n| n.name == said),
            "{scene}: no key named {said:?}"
        );
        let glyphs: Vec<&str> = nodes
            .iter()
            .map(|n| n.name.as_str())
            .filter(|n| n.contains(['␣', '⏎', '⌘', '⌥']) || *n == "." || *n == ";")
            .collect();
        assert!(glyphs.is_empty(), "{scene}: named by glyphs {glyphs:?}");
    }
}
