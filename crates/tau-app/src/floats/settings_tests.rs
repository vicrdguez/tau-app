//! How the settings float could fail its user, driven through the keys as a window gets them, on
//! a device whose files are in a temp dir:
//! - `␣hs`, `⌘,` or `:settings` opens nothing;
//! - typing doesn't narrow the rows, or narrows on the wrong text;
//! - a new key is kept that the keymap refuses, or a refusal ends the capture or says nothing;
//! - a kept key runs in one mode only, the key it replaced still runs, the start line prints the
//!   old key, or the file never gains the line;
//! - keys typed while capturing reach the keymap (`M-p` opens the project picker);
//! - Backspace doesn't take a key back; an auto-repeat or a fifth key is taken;
//! - `Esc` in capture closes the float or loses the filter and the row; a second `Esc` doesn't
//!   close it;
//! - `C-x` leaves the key moved, or its line in the file;
//! - a command's second key is missing, or rebinding and resetting it loses the query or the row;
//! - the width steps past its bounds, or the page behind doesn't follow;
//! - the motion row doesn't flip motion;
//! - a fixed row takes a capture.

use rune::Tx;

use super::Float;
use super::settings::{Row, SettingsFloat, rows};
use crate::app::{App, input, testing};
use crate::keys::{Keymap, Mode, display, parse_seq};
use crate::screens::start::scenes;
use crate::settings::{FILE, Field};

/// The start page with its project and workflow drawn, on a device whose files are in `dir`.
fn start(dir: &tempfile::TempDir) -> App {
    let mut a = testing::launch(dir.path(), &testing::fixture().url());
    scenes::base(&mut a);
    a.live = true;
    a
}

fn key(a: &mut App, seq: &str) {
    for k in parse_seq(seq).unwrap() {
        input::press(a, k, false, &mut Tx::default());
    }
}

fn float(a: &App) -> Option<&SettingsFloat> {
    match a.floats.last() {
        Some((Float::Settings(f), _)) => Some(f),
        _ => None,
    }
}

fn shown(a: &App) -> Vec<Row> {
    let f = float(a).expect("the settings float");
    rows(a.settings.keymap, &f.query.text)
}

/// The settings float over the start page, from Normal, filtered to `query`.
fn open(a: &mut App, query: &str) {
    key(a, "<Esc><leader>hs");
    a.float_edit().replace_text(query);
}

fn picker_title(a: &mut App) -> Option<String> {
    crate::screens::top_float::<super::project_picker::Picker>(a).map(|p| p.title.clone())
}

fn on_disk(dir: &tempfile::TempDir) -> String {
    std::fs::read_to_string(dir.path().join(FILE)).unwrap_or_default()
}

#[test]
fn the_leader_command_key_and_the_ex_name_each_open_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    key(&mut a, "<Esc><leader>hs");
    assert!(float(&a).is_some(), "␣hs");
    key(&mut a, "<Esc><D-,>");
    assert!(float(&a).is_some(), "⌘,");
    key(&mut a, "<Esc>:");
    a.float_edit().replace_text("settings");
    key(&mut a, "<CR>");
    assert!(float(&a).is_some(), ":settings");
    assert_eq!(a.scope().ctx, crate::keys::Ctx::Settings);
}

#[test]
fn typing_narrows_the_rows_to_the_values_and_keys_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "");
    assert!(matches!(
        shown(&a)[..3],
        [
            Row::Field(Field::Theme),
            Row::Field(Field::Reading),
            Row::Field(Field::Motion)
        ]
    ));
    a.float_edit().replace_text("width");
    assert!(matches!(shown(&a)[..], [Row::Field(Field::Reading), ..]));
    a.float_edit().replace_text("workflow");
    match &shown(&a)[0] {
        Row::Key(r) => assert_eq!(
            (r.command.id.as_str(), display(&r.design)),
            ("start.workflow", "M-w".to_string())
        ),
        other => panic!("{other:?}"),
    }
    a.float_edit().replace_text("M-w");
    assert!(
        shown(&a)
            .iter()
            .all(|r| matches!(r, Row::Key(k) if display(&k.now).contains("M-w")))
    );
    a.float_edit().replace_text("composer");
    assert!(shown(&a).iter().all(
        |r| matches!(r, Row::Key(k) if k.ctxs.contains(&crate::keys::Ctx::Composer)
                || k.command.desc.contains("composer"))
    ));
}

#[test]
fn a_new_workflow_key_runs_in_both_modes_prints_on_the_start_line_and_is_saved() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "workflow");
    key(&mut a, "<CR>");
    assert_eq!(float(&a).and_then(|f| f.capture.clone()), Some(vec![]));
    key(&mut a, "<M-m><CR>");
    assert_eq!(float(&a).and_then(|f| f.capture.clone()), None);
    assert_eq!(
        a.echo.as_deref(),
        Some("start page: workflow or model is now M-m")
    );
    assert!(
        on_disk(&dir).contains("[keys.\"start.workflow\"]\n\"<M-w>\" = \"<M-m>\"\n"),
        "{}",
        on_disk(&dir)
    );
    key(&mut a, "<Esc>");
    assert!(a.floats.is_empty());
    for mode in [Mode::Normal, Mode::Insert] {
        if mode == Mode::Insert {
            key(&mut a, "i");
        }
        assert_eq!(a.scope().mode, mode);
        key(&mut a, "<M-w>");
        assert_eq!(picker_title(&mut a), None, "{mode:?}: the key it replaced");
        key(&mut a, "<M-m>");
        assert_eq!(
            picker_title(&mut a).as_deref(),
            Some("Workflow"),
            "{mode:?}"
        );
        key(&mut a, "<Esc>");
    }
    assert_eq!(
        a.theme()
            .keymap
            .chip("<M-w>", Mode::Insert, crate::keys::Ctx::Start)
            .printed,
        "M-m"
    );
}

#[test]
fn a_refused_key_changes_nothing_and_keeps_the_capture_with_its_reason() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "workflow");
    key(&mut a, "<CR><M-p><CR>");
    let f = float(&a).expect("still open");
    assert_eq!(
        f.capture,
        Some(parse_seq("<M-p>").unwrap()),
        "still capturing"
    );
    let reason = f.note.clone().expect("a reason on the row");
    assert!(reason.contains("start page: project"), "{reason}");
    assert_eq!(a.echo.as_deref(), Some(reason.as_str()));
    assert_eq!(a.settings.keymap, Keymap::design());
    assert!(!dir.path().join(FILE).exists());
    assert_eq!(picker_title(&mut a), None, "M-p never reached the keymap");
    key(&mut a, "<BS>");
    assert_eq!(float(&a).and_then(|f| f.capture.clone()), Some(vec![]));
    key(&mut a, "<M-m><CR>");
    assert!(float(&a).is_some_and(|f| f.capture.is_none()));
    assert_ne!(a.settings.keymap, Keymap::design());
}

#[test]
fn a_capture_ignores_auto_repeat_and_a_fifth_key() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "workflow");
    key(&mut a, "<CR>");
    input::press(
        &mut a,
        parse_seq("<M-m>").unwrap()[0],
        true,
        &mut Tx::default(),
    );
    assert_eq!(float(&a).and_then(|f| f.capture.clone()), Some(vec![]));
    key(&mut a, "<C-a><C-b><C-d><C-e><C-f>");
    assert_eq!(
        float(&a).and_then(|f| f.capture.clone()),
        Some(parse_seq("<C-a><C-b><C-d><C-e>").unwrap())
    );
}

#[test]
fn esc_leaves_the_capture_keeping_the_filter_and_row_and_a_second_esc_closes() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "start page");
    key(&mut a, "<C-j><CR><M-m>");
    key(&mut a, "<Esc>");
    let f = float(&a).expect("still open");
    assert_eq!(
        (f.capture.clone(), f.query.text.as_str(), f.cursor),
        (None, "start page", 1)
    );
    assert_eq!(a.settings.keymap, Keymap::design());
    key(&mut a, "<CR><C-c>");
    assert!(
        float(&a).is_some_and(|f| f.capture.is_none()),
        "C-c cancels too"
    );
    key(&mut a, "<Esc>");
    assert!(a.floats.is_empty());
}

#[test]
fn ctrl_x_puts_the_design_key_back_and_removes_the_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "workflow");
    key(&mut a, "<CR><M-m><CR>");
    assert!(on_disk(&dir).contains("start.workflow"));
    key(&mut a, "<C-x>");
    assert_eq!(a.settings.keymap, Keymap::design());
    assert!(
        !on_disk(&dir).contains("start.workflow"),
        "{}",
        on_disk(&dir)
    );
    assert_eq!(
        a.echo.as_deref(),
        Some("start page: workflow or model is M-w again")
    );
}

#[test]
fn the_width_steps_by_forty_held_at_both_ends_and_the_page_follows() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "");
    key(&mut a, "<C-j><C-l>");
    assert_eq!(a.theme().read.measure, 880.0);
    for _ in 0..12 {
        key(&mut a, "<C-l>");
    }
    assert_eq!(a.theme().read.measure, 1200.0);
    for _ in 0..20 {
        key(&mut a, "<C-h>");
    }
    assert_eq!(a.theme().read.measure, 600.0);
    assert!(on_disk(&dir).contains("reading_width = 600\n"));
    input::press(
        &mut a,
        crate::keys::Key::plain(crate::keys::Code::Right),
        false,
        &mut Tx::default(),
    );
    assert_eq!(a.theme().read.measure, 640.0, "Right steps as C-l does");
}

#[test]
fn the_motion_row_flips_motion() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "motion");
    key(&mut a, "<CR>");
    assert!(!a.settings.motion);
    assert!(on_disk(&dir).contains("motion = false\n"));
    key(&mut a, "<C-l>");
    assert!(a.settings.motion);
}

#[test]
fn a_fixed_row_says_why_and_takes_no_capture() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "settings");
    let at = shown(&a)
        .iter()
        .position(|r| matches!(r, Row::Key(k) if k.fixed.is_some()))
        .expect("a fixed row");
    for _ in 0..at {
        key(&mut a, "<C-j>");
    }
    key(&mut a, "<CR>");
    let f = float(&a).expect("open");
    assert_eq!(f.capture, None);
    assert_eq!(
        f.note.as_deref(),
        Some("the settings float keeps its own keys")
    );
    key(&mut a, "<C-x>");
    assert_eq!(a.settings.keymap, Keymap::design());
}

#[test]
fn both_keys_of_commands_show_and_one_rebound_puts_back_keeping_the_query() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = start(&dir);
    open(&mut a, "commands");
    let keys: Vec<String> = shown(&a)
        .iter()
        .filter_map(|r| match r {
            Row::Key(k) if k.command.id == "palette" => Some(display(&k.now)),
            _ => None,
        })
        .collect();
    assert_eq!(keys.len(), 2, "{keys:?}");
    key(&mut a, "<C-j><CR><M-y><CR>");
    assert_ne!(a.settings.keymap, Keymap::design());
    let f = float(&a).expect("open");
    assert_eq!((f.query.text.as_str(), f.cursor), ("commands", 1));
    key(&mut a, "<C-x>");
    assert_eq!(a.settings.keymap, Keymap::design());
    let f = float(&a).expect("open");
    assert_eq!((f.query.text.as_str(), f.cursor), ("commands", 1));
}
