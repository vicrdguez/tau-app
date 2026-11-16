//! How settings.toml could fail its user:
//! - a value the user wrote is ignored, or read into the wrong setting;
//! - text that isn't TOML applies half of itself, or says nothing about where it broke;
//! - one bad value takes the others down with it, or falls back silently;
//! - a width out of range is refused instead of held at the nearest bound, or held silently;
//! - an unknown key is dropped from the file, or not reported;
//! - a `[keys]` entry of the wrong shape, an unknown command, a key the command lacks, a key that
//!   doesn't parse, or a move the keymap refuses applies anyway, or is not reported;
//! - a swap of two keys written in the file fails because its lines are checked one at a time;
//! - a save rewrites what it didn't change: comments, unknown keys, values it refused, the order;
//! - a save writes over text that isn't TOML;
//! - a key put back to its design key leaves its line, or an empty command table, behind;
//! - a key the user spelled another way gains a second line instead of changing theirs;
//! - the same save twice writes different bytes;
//! - what is saved doesn't load back as what was set.

use super::file::{HEADER, read, write};
use super::*;
use crate::keys::{Keymap, parse_seq};

fn k(seq: &str) -> Vec<Key> {
    parse_seq(seq).expect("a key sequence")
}

fn moved(km: Keymap, command: &str, from: &str, to: &str) -> Keymap {
    km.rebind(command, &k(from), &k(to))
        .expect("an allowed move")
}

fn width(px: f32) -> Measure {
    Measure::new(px).expect("a width in range")
}

fn clean(text: &str) -> Settings {
    let (s, problems) = read(text);
    assert_eq!(problems, Vec::<String>::new(), "{text}");
    s
}

fn problems(text: &str) -> Vec<String> {
    read(text).1
}

fn key_edit(command: &'static str, design: &str) -> Edit {
    Edit::Key {
        command,
        design: k(design),
    }
}

// ───────────── reading ─────────────

#[test]
fn no_text_is_every_default() {
    assert_eq!(clean(""), Settings::default());
    assert_eq!(
        Settings::default(),
        Settings {
            theme: Variant::Ember,
            reading: width(840.0),
            motion: true,
            keymap: Keymap::design(),
        }
    );
}

#[test]
fn every_setting_reads() {
    let s = clean(
        r#"
theme = "ember-soft"
reading_width = 760
motion = false

[keys."start.workflow"]
"<M-w>" = "<M-m>"
"#,
    );
    assert_eq!(
        s,
        Settings {
            theme: Variant::EmberSoft,
            reading: width(760.0),
            motion: false,
            keymap: moved(Keymap::design(), "start.workflow", "<M-w>", "<M-m>"),
        }
    );
}

#[test]
fn text_that_is_not_toml_is_every_default_and_names_its_line() {
    let (s, p) = read("theme = \"ember-soft\"\nreading_width 760\n");
    assert_eq!(s, Settings::default());
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(p[0].starts_with("settings.toml line 2: "), "{p:?}");
}

#[test]
fn an_unknown_theme_is_ember_and_says_so_and_the_rest_stand() {
    let (s, p) = read("theme = \"dark\"\nmotion = false\n");
    assert_eq!((s.theme, s.motion), (Variant::Ember, false));
    assert_eq!(
        p,
        vec![r#"theme "dark" isn't ember, ember-soft or ember-light"#]
    );
    assert_eq!(
        problems("theme = 3\n"),
        vec!["theme 3 isn't ember, ember-soft or ember-light"]
    );
}

#[test]
fn a_width_out_of_range_is_held_at_the_nearest_bound_and_says_so() {
    let (s, p) = read("reading_width = 2000\nmotion = false\n");
    assert_eq!((s.reading, s.motion), (width(1200.0), false));
    assert_eq!(p, vec!["reading_width 2000 is past 1200 px, so it is 1200"]);
    let (s, p) = read("reading_width = 300\n");
    assert_eq!(s.reading, width(600.0));
    assert_eq!(p, vec!["reading_width 300 is under 600 px, so it is 600"]);
}

#[test]
fn a_width_that_is_not_a_whole_number_is_the_default_and_says_so() {
    for text in ["reading_width = \"wide\"\n", "reading_width = 760.5\n"] {
        let (s, p) = read(&format!("{text}theme = \"ember-light\"\n"));
        assert_eq!(
            (s.reading, s.theme),
            (Measure::default(), Variant::EmberLight)
        );
        assert_eq!(
            p,
            vec!["reading_width takes a whole number of px from 600 to 1200"]
        );
    }
}

#[test]
fn motion_that_is_not_true_or_false_is_on_and_says_so() {
    let (s, p) = read("motion = \"no\"\nreading_width = 880\n");
    assert_eq!((s.motion, s.reading), (true, width(880.0)));
    assert_eq!(p, vec!["motion takes true or false"]);
}

#[test]
fn an_unknown_key_is_reported_and_the_rest_stand() {
    let (s, p) = read("font = 3\ntheme = \"ember-soft\"\n");
    assert_eq!(s.theme, Variant::EmberSoft);
    assert_eq!(p, vec!["font is not a setting"]);
}

#[test]
fn keys_of_the_wrong_shape_are_reported_and_the_rest_load() {
    let good = moved(Keymap::design(), "start.workflow", "<M-w>", "<M-m>");
    let (s, p) = read("keys = 3\n");
    assert_eq!(s.keymap, Keymap::design());
    assert_eq!(p, vec!["keys takes one table per command"]);
    let (s, p) = read(
        r#"
[keys]
"item.next" = "]o"
"start.project" = { "<M-p>" = 3 }
"start.workflow" = { "<M-w>" = "<M-m>" }
"#,
    );
    assert_eq!(s.keymap, good);
    assert_eq!(
        p,
        vec![
            r#"item.next takes lines of "<its key>" = "<your key>""#,
            "start.project <M-p> takes a key in quotes",
        ]
    );
}

#[test]
fn a_key_that_does_not_parse_is_reported() {
    let p = problems(
        r#"
[keys."start.workflow"]
"<Q-w>" = "<M-m>"
[keys."start.project"]
"<M-p>" = "<Q-m>"
"#,
    );
    assert_eq!(
        p,
        vec![
            r#"start.workflow: "<Q-w>" isn't a key"#,
            r#"start.project <M-p>: "<Q-m>" isn't a key"#,
        ]
    );
}

#[test]
fn an_unknown_command_or_a_key_the_command_lacks_is_reported() {
    let (s, p) = read(
        r#"
[keys."no.such"]
"x" = "X"
[keys."start.workflow"]
"<M-q>" = "<M-m>"
[keys."item.next"]
"]b" = "]o"
"#,
    );
    assert_eq!(s.keymap, moved(Keymap::design(), "item.next", "]b", "]o"));
    assert_eq!(
        p,
        vec![
            "no.such is not a command",
            "<M-q> is not one of start.workflow's keys (M-w)",
        ]
    );
}

#[test]
fn a_move_the_keymap_refuses_is_reported_with_its_reason() {
    let (s, p) = read("[keys.\"start.project\"]\n\"<M-p>\" = \"p\"\n");
    assert_eq!(s.keymap, Keymap::design());
    assert_eq!(
        p,
        vec![
            "start.project <M-p>: in Insert on the start page a key needs Control, Option or Command"
        ]
    );
}

#[test]
fn a_swap_of_two_keys_loads() {
    let s = clean(
        r#"
[keys."start.project"]
"<M-p>" = "<M-w>"
[keys."start.workflow"]
"<M-w>" = "<M-p>"
"#,
    );
    let rows = s.keymap.rebinds();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|r| r.command == "start.project" && r.to == k("<M-w>"))
    );
    assert!(
        rows.iter()
            .any(|r| r.command == "start.workflow" && r.to == k("<M-p>"))
    );
}

// ───────────── writing ─────────────

fn with(s: Settings, f: impl FnOnce(&mut Settings)) -> Settings {
    let mut s = s;
    f(&mut s);
    s
}

#[test]
fn a_new_file_starts_with_the_header() {
    let s = with(Settings::default(), |s| s.theme = Variant::EmberSoft);
    let out = write("", &s, &Edit::Field(Field::Theme)).unwrap();
    assert_eq!(out, format!("{HEADER}\ntheme = \"ember-soft\"\n"));
}

#[test]
fn a_save_changes_one_key_and_keeps_the_rest_as_written() {
    let text = "# mine\ntheme = \"ember\"  # the dark one\nfont = 3\nmotion = \"no\"\n\n# moved for vim\n[keys.\"item.next\"]\n\"]b\" = \"]o\"  # habit\n";
    let s = with(clean_or_default(text), |s| s.theme = Variant::EmberLight);
    assert_eq!(
        write(text, &s, &Edit::Field(Field::Theme)).unwrap(),
        text.replace("\"ember\"", "\"ember-light\"")
    );
    let s = with(s, |s| s.reading = width(880.0));
    assert_eq!(
        write(text, &s, &Edit::Field(Field::Reading)).unwrap(),
        text.replace(
            "motion = \"no\"\n",
            "motion = \"no\"\nreading_width = 880\n"
        )
    );
}

fn clean_or_default(text: &str) -> Settings {
    read(text).0
}

#[test]
fn a_save_never_writes_over_text_that_is_not_toml() {
    let text = "theme = \"ember\"\nreading_width 760\n";
    let err = write(text, &Settings::default(), &Edit::Field(Field::Theme)).unwrap_err();
    assert!(err.starts_with("settings.toml line 2: "), "{err}");
}

#[test]
fn a_moved_key_is_saved_under_its_command() {
    let s = with(Settings::default(), |s| {
        s.keymap = moved(s.keymap, "start.workflow", "<M-w>", "<M-m>")
    });
    let edit = key_edit("start.workflow", "<M-w>");
    let out = write("motion = false\n", &s, &edit).unwrap();
    assert_eq!(
        out,
        "motion = false\n\n[keys.\"start.workflow\"]\n\"<M-w>\" = \"<M-m>\"\n"
    );
    assert_eq!(clean(&out).keymap, s.keymap);
}

#[test]
fn a_key_put_back_removes_its_line_and_an_empty_command_table() {
    let text = "[keys.\"start.workflow\"]\n\"<M-w>\" = \"<M-m>\"\n\n[keys.\"item.next\"]\n\"]b\" = \"]o\"\n\"[b\" = \"[o\"\n";
    let s = clean(text);
    let back = with(s, |s| {
        s.keymap = moved(s.keymap, "start.workflow", "<M-w>", "<M-w>")
    });
    let out = write(text, &back, &key_edit("start.workflow", "<M-w>")).unwrap();
    assert!(!out.contains("start.workflow"), "{out}");
    assert_eq!(clean(&out), back);
    let back = with(back, |s| {
        s.keymap = moved(s.keymap, "item.next", "]b", "]b")
    });
    let out = write(&out, &back, &key_edit("item.next", "]b")).unwrap();
    assert!(
        out.contains("[keys.\"item.next\"]\n\"[b\" = \"[o\"\n"),
        "{out}"
    );
    assert!(!out.contains("]o"), "{out}");
}

#[test]
fn duplicate_design_key_spellings_are_reported_and_the_first_wins() {
    let text =
        "[keys.\"help.theme\"]\n\"<leader>ht\" = \"<leader>hy\"\n\"<Space>ht\" = \"<leader>hz\"\n";
    let (s, p) = read(text);
    assert_eq!(
        s.keymap,
        moved(Keymap::design(), "help.theme", "<leader>ht", "<leader>hy")
    );
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(
        p[0].contains("help.theme") && p[0].contains("already"),
        "{p:?}"
    );
    let new = with(s, |s| {
        s.keymap = moved(s.keymap, "help.theme", "<leader>ht", "<leader>hq")
    });
    let out = write(text, &new, &key_edit("help.theme", "<leader>ht")).unwrap();
    assert_eq!(read(&out).0, new);
    let back = with(s, |s| {
        s.keymap = moved(s.keymap, "help.theme", "<leader>ht", "<leader>ht")
    });
    let out = write(text, &back, &key_edit("help.theme", "<leader>ht")).unwrap();
    assert_eq!(clean(&out), back);
}

#[test]
fn inline_key_tables_keep_moves_additions_and_resets() {
    for text in [
        "keys = {}\n",
        "keys = { \"help.theme\" = { \"<leader>ht\" = \"<leader>hy\" } }\n",
        "[keys]\n\"help.theme\" = { \"<leader>ht\" = \"<leader>hy\" }\n",
    ] {
        let mut s = clean(text);
        let mut text = text.to_string();
        for (command, from, to) in [
            ("start.workflow", "<M-w>", "<M-m>"),
            ("start.workflow", "<M-w>", "<M-n>"),
            ("help.theme", "<leader>ht", "<leader>hz"),
            ("start.workflow", "<M-w>", "<M-w>"),
            ("help.theme", "<leader>ht", "<leader>ht"),
        ] {
            s.keymap = moved(s.keymap, command, from, to);
            text = write(&text, &s, &key_edit(command, from)).unwrap();
            assert_eq!(clean(&text), s, "{text}");
        }
    }
}

#[test]
fn a_key_spelled_another_way_is_changed_in_place() {
    let text = "[keys.\"help.theme\"]\n\"<Space>ht\" = \"<leader>hy\"\n";
    let s = with(clean(text), |s| {
        s.keymap = moved(s.keymap, "help.theme", "<leader>ht", "<leader>hz")
    });
    let out = write(text, &s, &key_edit("help.theme", "<leader>ht")).unwrap();
    assert_eq!(
        out,
        "[keys.\"help.theme\"]\n\"<Space>ht\" = \"<leader>hz\"\n"
    );
}

#[test]
fn the_same_save_twice_writes_the_same_bytes() {
    let s = Settings {
        theme: Variant::EmberSoft,
        reading: width(920.0),
        motion: false,
        keymap: moved(Keymap::design(), "start.workflow", "<M-w>", "<M-m>"),
    };
    let edits = [
        Edit::Field(Field::Theme),
        Edit::Field(Field::Reading),
        Edit::Field(Field::Motion),
        key_edit("start.workflow", "<M-w>"),
    ];
    for start in ["", "# mine\nfont = 3\n"] {
        for edit in &edits {
            let once = write(start, &s, edit).unwrap();
            assert_eq!(write(&once, &s, edit).unwrap(), once, "{edit:?}");
        }
    }
}

#[test]
fn what_is_saved_loads_back_as_what_was_set() {
    let s = Settings {
        theme: Variant::EmberLight,
        reading: width(1000.0),
        motion: false,
        keymap: moved(
            moved(Keymap::design(), "start.workflow", "<M-w>", "<M-m>"),
            "item.next",
            "]b",
            "]o",
        ),
    };
    let mut text = String::new();
    for edit in [
        key_edit("item.next", "]b"),
        Edit::Field(Field::Motion),
        Edit::Field(Field::Theme),
        key_edit("start.workflow", "<M-w>"),
        Edit::Field(Field::Reading),
    ] {
        text = write(&text, &s, &edit).unwrap();
    }
    assert!(text.starts_with(HEADER), "{text}");
    assert_eq!(clean(&text), s);
}

// ───────────── the values the float steps ─────────────

#[test]
fn theme_and_motion_wrap_and_the_width_is_held_at_its_ends() {
    let s = Settings::default();
    assert_eq!(Field::Theme.step(&s, -1).theme, Variant::EmberLight);
    assert_eq!(Field::Theme.step(&s, 1).theme, Variant::EmberSoft);
    assert!(!Field::Motion.step(&s, 1).motion);
    assert!(!Field::Motion.step(&s, -1).motion);
    assert_eq!(Field::Reading.step(&s, 1).reading, width(880.0));
    let wide = with(s, |s| s.reading = width(1200.0));
    assert_eq!(Field::Reading.step(&wide, 1).reading, width(1200.0));
    let narrow = with(s, |s| s.reading = width(600.0));
    assert_eq!(Field::Reading.step(&narrow, -1).reading, width(600.0));
}
