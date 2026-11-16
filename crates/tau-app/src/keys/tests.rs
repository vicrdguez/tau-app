//! How the keymap engine could fail, one test (or a few) each:
//! - the embedded keymap doesn't parse, so the app panics at launch;
//! - a malformed table parses anyway: an unknown context, mode or key, a context missing from the
//!   tree, a cycle, two commands with one id;
//! - a key sequence reads or prints wrong (`<leader>`, `<D-k>`, `<C-CR>`, `<S-Tab>`);
//! - a rune event maps to the wrong key (Control letters, Option letters that type other
//!   characters, Command, shifted characters, Space as the leader);
//! - a binding acts outside its context's subtree, or an inner context loses its parents' keys;
//! - `shadows` is ignored, so the outer binding wins;
//! - a float that takes the keys lets `global` through (leader, `:`, `⌘K`);
//! - a mode leaks into another (the leader in Insert);
//! - counts are lost, misread (`0`, two digits) or kept for commands that may not count;
//! - `Esc` fails to cancel a pending prefix, or an unbound sequence loses its keys;
//! - builtin keys (`gg`) don't take part in prefix resolution;
//! - two command bindings live in one mode and context where one is a prefix of the other;
//! - which-key lists the wrong entries, labels or order, or shows before its delay;
//! - a printed chip is lit where its key doesn't run (Insert, under a float, a command that
//!   doesn't apply);
//! - `:` names, completions and the palette miss or duplicate commands;
//! - an auto-repeat fires a write, an answer or a readback's `y`;
//! - a moved binding hands its handler the keys typed, not the design's, so `[b` stops reading
//!   as backwards once `]b` and `[b` move;
//! - a chip prints the design's key where this device types another, or a moved key for a
//!   binding that didn't move, or is lit by the key that moved away;
//! - a key written for settings.toml doesn't read back as the same keys.

use super::*;

fn km() -> Keymap {
    Keymap::design()
}

fn k(seq: &str) -> Vec<Key> {
    parse_seq(seq).expect("a key sequence")
}

/// `km` with `command`'s key `design` typed as `to`.
fn moved(km: Keymap, command: &str, design: &str, to: &str) -> Keymap {
    km.rebind(command, &k(design), &k(to))
        .expect("the move is allowed")
}

/// Presses every key of `seq` and returns the last outcome.
fn press(keys: &mut Keys, km: Keymap, mode: Mode, ctx: Ctx, seq: &str) -> Outcome<'static> {
    let mut out = Outcome::Pending;
    for key in k(seq) {
        out = keys.press(km, mode, ctx, key, false, 0.0);
    }
    out
}

fn command_id<'a>(out: &'a Outcome<'a>) -> Option<&'a str> {
    match out {
        Outcome::Command { command, .. } => Some(&command.id),
        _ => None,
    }
}

fn resolve(mode: Mode, ctx: Ctx, seq: &str) -> Option<String> {
    resolve_in(km(), mode, ctx, seq)
}

fn resolve_in(km: Keymap, mode: Mode, ctx: Ctx, seq: &str) -> Option<String> {
    let mut keys = Keys::default();
    command_id(&press(&mut keys, km, mode, ctx, seq)).map(str::to_string)
}

const MINIMAL: &str = r#"
[meta]
schema = 1
leader = "<Space>"
whichkey_delay_ms = 300
hint_alphabet = "asdfghjkl"

[contexts]
global = ""
list = "global"
sidebar = "list"
reader = "global"
conversation = "reader"
decision = "reader"
project = "reader"
editor = "global"
composer = "editor"
note = "editor"
field = "editor"
connect = "editor"
start = "editor"
hint = "global"
picker = ""
bind = ""
readback = ""
reply = ""
confirm = ""
outbox = ""
server = ""
peek = ""

[groups]
"<leader>" = "leader"

[builtin]
list_motions = ["j"]

[[command]]
id = "open.home"
ex = "home"
desc = "home"
tier = "local"
kind = "command"
ctx = ["global"]
keys = [{ mode = "n", ctx = "global", seq = "<leader>oh" }]
"#;

// ───────────── parsing ─────────────

#[test]
fn the_embedded_keymap_parses() {
    let km = km();
    assert!(km.commands().len() > 60, "{} commands", km.commands().len());
    assert_eq!(km.whichkey_delay_ms(), 300.0);
}

#[test]
fn a_minimal_table_parses() {
    let km = Keymap::parse(MINIMAL).expect("parses");
    assert_eq!(km.commands().len(), 1);
}

#[test]
fn an_unknown_context_is_an_error() {
    let toml = MINIMAL.replace(
        "keys = [{ mode = \"n\", ctx = \"global\"",
        "keys = [{ mode = \"n\", ctx = \"lobby\"",
    );
    let err = Keymap::parse(&toml).expect_err("an unknown context");
    assert!(err.0.contains("lobby"), "{err}");
}

#[test]
fn a_context_missing_from_the_tree_is_an_error() {
    let toml = MINIMAL.replace("peek = \"\"\n", "");
    let err = Keymap::parse(&toml).expect_err("peek is missing");
    assert!(err.0.contains("peek"), "{err}");
}

#[test]
fn a_cycle_in_the_context_tree_is_an_error() {
    let toml = MINIMAL.replace("global = \"\"", "global = \"sidebar\"");
    assert!(Keymap::parse(&toml).is_err());
}

#[test]
fn an_unknown_mode_is_an_error() {
    let toml = MINIMAL.replace("mode = \"n\"", "mode = \"x\"");
    let err = Keymap::parse(&toml).expect_err("an unknown mode");
    assert!(err.0.contains("\"x\"") || err.0.contains("`x`"), "{err}");
}

#[test]
fn a_bad_key_is_an_error() {
    let toml = MINIMAL.replace("<leader>oh", "<leader><Q-x>");
    assert!(Keymap::parse(&toml).is_err());
}

#[test]
fn two_commands_with_one_id_are_an_error() {
    let dup = MINIMAL.to_string()
        + &MINIMAL[MINIMAL.find("[[command]]").expect("a command")..]
            .replace("<leader>oh", "<leader>oq");
    let err = Keymap::parse(&dup).expect_err("a duplicate id");
    assert!(err.0.contains("open.home"), "{err}");
}

// ───────────── keys ─────────────

#[test]
fn sequences_read_as_the_keymap_writes_them() {
    assert_eq!(
        k("<leader>cn"),
        vec![Key::char(' '), Key::char('c'), Key::char('n')]
    );
    assert_eq!(k("]b"), vec![Key::char(']'), Key::char('b')]);
    assert_eq!(k("<C-h>"), vec![Key::ctrl('h')]);
    let cmd_k = Key {
        code: Code::Char('k'),
        mods: Mods {
            cmd: true,
            ..Mods::default()
        },
    };
    assert_eq!(k("<D-k>"), vec![cmd_k]);
    let ctrl_cr = Key {
        code: Code::Enter,
        mods: Mods {
            ctrl: true,
            ..Mods::default()
        },
    };
    assert_eq!(k("<C-CR>"), vec![ctrl_cr]);
    let s_tab = Key {
        code: Code::Tab,
        mods: Mods {
            shift: true,
            ..Mods::default()
        },
    };
    assert_eq!(k("<S-Tab>"), vec![s_tab]);
    assert_eq!(k("<Esc>"), vec![Key::plain(Code::Esc)]);
    assert_eq!(k("G"), vec![Key::char('G')]);
}

#[test]
fn malformed_sequences_are_errors() {
    assert!(parse_seq("").is_err());
    assert!(parse_seq("<Q-x>").is_err());
    assert!(parse_seq("<Nope>").is_err());
}

#[test]
fn sequences_print_as_the_design_prints_them() {
    assert_eq!(display(&k("<leader>cn")), "␣cn");
    assert_eq!(display(&k("<leader>.")), "␣.");
    assert_eq!(display(&k("<D-k>")), "⌘K");
    assert_eq!(display(&k("<D-CR>")), "⌘⏎");
    assert_eq!(display(&k("<M-CR>")), "M-⏎");
    assert_eq!(display(&k("<C-h>")), "C-h");
    assert_eq!(display(&k("<CR>")), "⏎");
    assert_eq!(display(&k("<Esc>")), "Esc");
    assert_eq!(display(&k("<S-Tab>")), "S-Tab");
}

fn ev(key: rune::Key, mods: rune::Mods) -> rune::KeyEvent {
    rune::KeyEvent { key, mods }
}

#[test]
fn rune_events_map_to_keymap_keys() {
    let none = rune::Mods::default();
    let ctrl = rune::Mods { ctrl: true, ..none };
    let alt = rune::Mods { alt: true, ..none };
    let cmd = rune::Mods {
        cmd: true,
        logo: true,
        ..none
    };
    let shift = rune::Mods {
        shift: true,
        ..none
    };
    assert_eq!(
        from_rune(&ev(rune::Key::Text(" ".into()), none), Some(' ')),
        Some(Key::char(' '))
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Text("h".into()), ctrl), Some('h')),
        Some(Key::ctrl('h'))
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Text("\u{8}".into()), ctrl), None),
        Some(Key::ctrl('h'))
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Text("π".into()), alt), Some('p')),
        k("<M-p>").first().copied()
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Command('k'), cmd), Some('k')),
        k("<D-k>").first().copied()
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Enter, cmd), None),
        k("<D-CR>").first().copied()
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Enter, alt), None),
        k("<M-CR>").first().copied()
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Text("G".into()), shift), Some('g')),
        Some(Key::char('G'))
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Text(":".into()), shift), Some(';')),
        Some(Key::char(':'))
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Tab, shift), None),
        k("<S-Tab>").first().copied()
    );
    assert_eq!(
        from_rune(&ev(rune::Key::Escape, none), None),
        Some(Key::plain(Code::Esc))
    );
}

// ───────────── resolution ─────────────

#[test]
fn a_leader_sequence_waits_then_resolves() {
    let mut keys = Keys::default();
    assert_eq!(
        press(&mut keys, km(), Mode::Normal, Ctx::List, "<leader>"),
        Outcome::Pending
    );
    assert_eq!(
        press(&mut keys, km(), Mode::Normal, Ctx::List, "o"),
        Outcome::Pending
    );
    let out = press(&mut keys, km(), Mode::Normal, Ctx::List, "h");
    assert_eq!(command_id(&out), Some("open.home"));
    assert!(keys.is_idle());
}

#[test]
fn a_binding_acts_only_in_its_contexts_subtree() {
    assert_eq!(
        resolve(Mode::Normal, Ctx::Conversation, "]m").as_deref(),
        Some("message.next")
    );
    assert_eq!(resolve(Mode::Normal, Ctx::List, "]m"), None);
    assert_eq!(
        resolve(Mode::Normal, Ctx::Decision, "]n").as_deref(),
        Some("note.next")
    );
    assert_eq!(resolve(Mode::Normal, Ctx::Conversation, "]n"), None);
}

#[test]
fn an_inner_context_keeps_its_parents_keys() {
    assert_eq!(
        resolve(Mode::Normal, Ctx::Sidebar, "<leader>oh").as_deref(),
        Some("open.home")
    );
    assert_eq!(
        resolve(Mode::Normal, Ctx::Sidebar, "]g").as_deref(),
        Some("group.next")
    );
    assert_eq!(
        resolve(Mode::Normal, Ctx::Start, "<leader>cn").as_deref(),
        Some("conv.new")
    );
}

#[test]
fn an_inner_binding_that_shadows_wins() {
    assert_eq!(
        resolve(Mode::Normal, Ctx::Sidebar, "x").as_deref(),
        Some("view.close")
    );
    assert_eq!(
        resolve(Mode::Normal, Ctx::List, "x").as_deref(),
        Some("row.hide")
    );
}

#[test]
fn a_float_that_takes_the_keys_lets_nothing_of_global_through() {
    assert_eq!(
        resolve(Mode::Command, Ctx::Picker, "<C-j>").as_deref(),
        Some("picker.keys")
    );
    let mut keys = Keys::default();
    for seq in ["<leader>", ":", "<D-k>", "K", "]b"] {
        let out = press(&mut keys, km(), Mode::Command, Ctx::Picker, seq);
        assert!(
            matches!(out, Outcome::Unbound { .. }),
            "{seq} under the picker: {out:?}"
        );
    }
    assert_eq!(
        resolve(Mode::Readback, Ctx::Confirm, "y").as_deref(),
        Some("confirm.keys")
    );
    assert_eq!(
        resolve(Mode::Readback, Ctx::Readback, "y").as_deref(),
        Some("readback.confirm")
    );
    assert_eq!(resolve(Mode::Readback, Ctx::Confirm, "<leader>oh"), None);
    assert_eq!(
        resolve(Mode::Server, Ctx::Server, "q").as_deref(),
        Some("server.keys")
    );
}

#[test]
fn modes_do_not_leak() {
    let mut keys = Keys::default();
    let out = press(&mut keys, km(), Mode::Insert, Ctx::Start, "<leader>");
    assert!(
        matches!(out, Outcome::Unbound { .. }),
        "the leader in Insert: {out:?}"
    );
    assert_eq!(
        resolve(Mode::Insert, Ctx::Start, "<D-CR>").as_deref(),
        Some("start.send")
    );
    assert_eq!(
        resolve(Mode::Insert, Ctx::Composer, "<D-CR>").as_deref(),
        Some("composer.send")
    );
    assert_eq!(
        resolve(Mode::Insert, Ctx::Composer, "<M-CR>").as_deref(),
        Some("composer.followup")
    );
    assert_eq!(
        resolve(Mode::Normal, Ctx::Start, "<M-p>").as_deref(),
        Some("start.project")
    );
    assert_eq!(resolve(Mode::Insert, Ctx::Composer, "<M-p>"), None);
}

#[test]
fn a_count_reaches_a_countable_command() {
    let mut keys = Keys::default();
    let out = press(&mut keys, km(), Mode::Normal, Ctx::List, "3]b");
    assert!(
        matches!(out, Outcome::Command { count: Some(3), .. }),
        "{out:?}"
    );
    let out = press(&mut keys, km(), Mode::Normal, Ctx::List, "12]b");
    assert!(
        matches!(
            out,
            Outcome::Command {
                count: Some(12),
                ..
            }
        ),
        "{out:?}"
    );
}

#[test]
fn a_count_is_dropped_for_a_command_that_may_not_count() {
    let mut keys = Keys::default();
    let out = press(&mut keys, km(), Mode::Normal, Ctx::Decision, "2<leader>ad");
    match out {
        Outcome::Command { command, count, .. } => {
            assert_eq!(command.id, "auth.answer");
            assert_eq!(count, None);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn zero_starts_no_count() {
    let mut keys = Keys::default();
    let out = press(&mut keys, km(), Mode::Normal, Ctx::Reader, "0");
    assert!(
        matches!(out, Outcome::Builtin { count: None, .. }),
        "0 is a reader motion: {out:?}"
    );
    let out = press(&mut keys, km(), Mode::Normal, Ctx::List, "10j");
    assert!(
        matches!(
            out,
            Outcome::Builtin {
                count: Some(10),
                ..
            }
        ),
        "{out:?}"
    );
}

#[test]
fn esc_cancels_a_pending_prefix_and_a_count() {
    let mut keys = Keys::default();
    press(&mut keys, km(), Mode::Normal, Ctx::List, "2<leader>o");
    assert_eq!(
        press(&mut keys, km(), Mode::Normal, Ctx::List, "<Esc>"),
        Outcome::Cancelled
    );
    assert!(keys.is_idle());
    let out = press(&mut keys, km(), Mode::Normal, Ctx::List, "]b");
    assert!(
        matches!(out, Outcome::Command { count: None, .. }),
        "{out:?}"
    );
}

#[test]
fn an_unbound_sequence_hands_back_its_keys() {
    let mut keys = Keys::default();
    let out = press(&mut keys, km(), Mode::Normal, Ctx::List, "gx");
    assert_eq!(
        out,
        Outcome::Unbound {
            keys: k("gx"),
            count: None
        }
    );
    assert!(keys.is_idle());
    let out = press(&mut keys, km(), Mode::Normal, Ctx::Editor, "3w");
    assert_eq!(
        out,
        Outcome::Builtin {
            keys: k("w"),
            count: Some(3)
        }
    );
}

#[test]
fn builtin_keys_take_part_in_prefix_resolution() {
    let mut keys = Keys::default();
    assert_eq!(
        press(&mut keys, km(), Mode::Normal, Ctx::List, "g"),
        Outcome::Pending
    );
    assert_eq!(
        press(&mut keys, km(), Mode::Normal, Ctx::List, "g"),
        Outcome::Builtin {
            keys: k("gg"),
            count: None
        }
    );
    assert_eq!(
        resolve(Mode::Normal, Ctx::List, "gd").as_deref(),
        Some("goto.linked")
    );
    let out = press(&mut keys, km(), Mode::Normal, Ctx::Editor, "dw");
    assert_eq!(
        out,
        Outcome::Unbound {
            keys: k("dw"),
            count: None
        },
        "d waits for dd, then hands both keys on"
    );
}

#[test]
fn a_bound_key_that_does_not_apply_says_so() {
    let mut keys = Keys::default();
    match press(&mut keys, km(), Mode::Normal, Ctx::List, "<leader>td") {
        Outcome::Command {
            command, applies, ..
        } => {
            assert_eq!(command.id, "toggle.details");
            assert!(!applies, "␣td has nothing to act on in a list");
        }
        other => panic!("{other:?}"),
    }
    match press(
        &mut keys,
        km(),
        Mode::Normal,
        Ctx::Conversation,
        "<leader>td",
    ) {
        Outcome::Command { applies, .. } => assert!(applies),
        other => panic!("{other:?}"),
    }
}

#[test]
fn no_two_command_bindings_live_where_one_is_a_prefix_of_the_other() {
    let km = km();
    let modes = [
        Mode::Normal,
        Mode::Visual,
        Mode::Insert,
        Mode::Command,
        Mode::Hint,
        Mode::Readback,
        Mode::Peek,
        Mode::Outbox,
        Mode::Server,
    ];
    for mode in modes {
        for ctx in Ctx::ALL {
            let all: Vec<Bound> = km.bindings().collect();
            let live: Vec<usize> = km
                .active(mode, ctx)
                .into_iter()
                .filter(|&i| matches!(all[i].target, Target::Command(_)))
                .collect();
            for &i in &live {
                for &j in &live {
                    let (a, b) = (&all[i], &all[j]);
                    let clash = a.seq.len() < b.seq.len() && b.seq.starts_with(a.seq);
                    assert!(
                        !clash,
                        "{mode:?} {ctx:?}: {} is a prefix of {}",
                        display(a.seq),
                        display(b.seq)
                    );
                    let twice = a.seq == b.seq && i != j && a.target != b.target;
                    assert!(
                        !twice,
                        "{mode:?} {ctx:?}: {} runs two commands",
                        display(a.seq)
                    );
                }
            }
        }
    }
}

// ───────────── which-key ─────────────

#[test]
fn which_key_for_the_leader_lists_groups_then_commands_in_the_grammars_order() {
    let wk = km()
        .which_key(&k("<leader>"), Mode::Normal, Ctx::List)
        .expect("the leader is a prefix");
    assert_eq!(wk.label, "leader");
    let shown: Vec<(String, String, bool)> = wk
        .entries
        .iter()
        .map(|e| (display(&[e.key]), e.desc.clone(), e.group))
        .collect();
    let expect = [
        ("a", "◇ answer", true),
        ("c", "conversation", true),
        ("o", "open", true),
        ("p", "project", true),
        ("m", "this view", true),
        ("r", "recover", true),
        ("t", "toggle", true),
        ("h", "help", true),
        ("n", "next decision", false),
        (".", "find anything", false),
        (";", "commands", false),
        ("y", "yank to clipboard", false),
    ];
    let expect: Vec<(String, String, bool)> = expect
        .iter()
        .map(|(a, b, c)| (a.to_string(), b.to_string(), *c))
        .collect();
    assert_eq!(shown, expect);
    let yank = wk
        .entries
        .iter()
        .find(|e| e.key == Key::char('y'))
        .expect("y");
    assert!(
        !yank.applies,
        "yank applies in readers and editors, not in a list"
    );
    assert!(
        wk.entries
            .iter()
            .find(|e| e.key == Key::char('n'))
            .expect("n")
            .applies
    );
}

#[test]
fn which_key_for_a_group_names_it_and_its_leaves() {
    let wk = km()
        .which_key(&k("<leader>a"), Mode::Normal, Ctx::List)
        .expect("␣a is a prefix");
    assert_eq!(wk.label, "◇ answer");
    let leaves: Vec<(String, bool)> = wk
        .entries
        .iter()
        .map(|e| (display(&[e.key]), e.applies))
        .collect();
    assert_eq!(
        leaves,
        vec![("d".to_string(), false), ("r".to_string(), false)]
    );
    let wk = km()
        .which_key(&k("<leader>a"), Mode::Normal, Ctx::Decision)
        .expect("␣a is a prefix");
    assert!(wk.entries.iter().all(|e| e.applies));
}

#[test]
fn which_key_is_none_for_a_complete_or_unknown_sequence() {
    assert!(
        km().which_key(&k("<leader>oh"), Mode::Normal, Ctx::List)
            .is_none()
    );
    assert!(km().which_key(&k("Q"), Mode::Normal, Ctx::List).is_none());
    assert!(
        km().which_key(&k("<leader>"), Mode::Insert, Ctx::Start)
            .is_none()
    );
}

#[test]
fn which_key_is_none_where_only_builtin_keys_extend_the_prefix() {
    assert!(
        km().which_key(&k("d"), Mode::Normal, Ctx::Editor).is_none(),
        "d is the editor's operator"
    );
    let wk = km()
        .which_key(&k("d"), Mode::Normal, Ctx::Decision)
        .expect("dd discards a note draft");
    assert_eq!(wk.label, "draft");
}

#[test]
fn which_key_lists_a_groups_leaves_from_every_view_greying_those_that_cannot_run() {
    let wk = km()
        .which_key(&k("<leader>m"), Mode::Normal, Ctx::Decision)
        .expect("␣m is a prefix in a decision");
    let leaves: Vec<(String, bool)> = wk
        .entries
        .iter()
        .map(|e| (display(&[e.key]), e.applies))
        .collect();
    assert_eq!(
        leaves,
        vec![
            ("n".into(), true),
            ("w".into(), true),
            ("s".into(), false),
            ("r".into(), false)
        ]
    );
}

#[test]
fn which_key_waits_its_delay_after_the_last_key() {
    let mut keys = Keys::default();
    keys.press(km(), Mode::Normal, Ctx::List, Key::char(' '), false, 10.0);
    assert!(!keys.which_key_due(10.29, 300.0));
    assert!(keys.which_key_due(10.3, 300.0));
    keys.press(km(), Mode::Normal, Ctx::List, Key::char('o'), false, 10.5);
    assert!(
        !keys.which_key_due(10.6, 300.0),
        "a new prefix key restarts the wait"
    );
    assert_eq!(keys.shown(), "␣o");
    keys.press(km(), Mode::Normal, Ctx::List, Key::char('h'), false, 10.7);
    assert!(!keys.which_key_due(20.0, 300.0), "nothing pending");
}

#[test]
fn the_shown_keys_lead_with_the_count() {
    let mut keys = Keys::default();
    press(&mut keys, km(), Mode::Normal, Ctx::List, "2<leader>a");
    assert_eq!(keys.shown(), "2␣a");
}

// ───────────── printed chips ─────────────

#[test]
fn a_chip_is_lit_only_where_its_key_runs() {
    let km = km();
    assert!(km.chip("<leader>cn", Mode::Normal, Ctx::List).lit);
    assert!(km.chip("<leader>cn", Mode::Normal, Ctx::Sidebar).lit);
    assert!(
        !km.chip("<leader>cn", Mode::Insert, Ctx::Start).lit,
        "every leader chip is muted in Insert"
    );
    assert!(
        !km.chip("<leader>cn", Mode::Command, Ctx::Picker).lit,
        "everything outside a float is muted"
    );
    assert!(km.chip("<leader>cx", Mode::Normal, Ctx::Conversation).lit);
    assert!(
        !km.chip("<leader>cx", Mode::Normal, Ctx::Sidebar).lit,
        "␣cx doesn't apply in the sidebar"
    );
    assert!(km.chip("<Esc>", Mode::Command, Ctx::Picker).lit);
    assert!(
        !km.chip("<leader>", Mode::Normal, Ctx::List).lit,
        "a prefix runs nothing"
    );
}

#[test]
fn a_chip_prints_the_key_this_device_types() {
    let design = km();
    let mine = moved(design, "item.next", "]b", "]o");
    let chip = |km: Keymap, seq: &str, mode: Mode, ctx: Ctx| {
        let c = km.chip(seq, mode, ctx);
        (c.printed, c.lit)
    };
    assert_eq!(
        chip(design, "]b", Mode::Normal, Ctx::List),
        ("]b".into(), true)
    );
    assert_eq!(
        chip(mine, "]b", Mode::Normal, Ctx::List),
        ("]o".into(), true)
    );
    assert_eq!(
        chip(mine, "[b", Mode::Normal, Ctx::List),
        ("[b".into(), true),
        "its other key didn't move"
    );
    assert_eq!(
        chip(mine, "<leader>cn", Mode::Normal, Ctx::List),
        ("␣cn".into(), true)
    );
    assert_eq!(
        chip(mine, "]b", Mode::Command, Ctx::Picker),
        ("]o".into(), false),
        "under a float it prints the moved key, muted"
    );
}

#[test]
fn the_key_that_moved_away_runs_nothing_there() {
    let mine = moved(km(), "item.next", "]b", "]o");
    assert_eq!(resolve_in(mine, Mode::Normal, Ctx::List, "]b"), None);
    assert_eq!(
        resolve_in(mine, Mode::Normal, Ctx::List, "]o").as_deref(),
        Some("item.next")
    );
    let moved_away = parse_seq("]b").expect("keys");
    let look = mine.lookup(Mode::Normal, Ctx::List, &moved_away);
    assert_eq!(look.exact, None);
}

#[test]
fn a_moved_binding_hands_its_handler_the_design_keys() {
    let mine = moved(
        moved(km(), "item.next", "]b", "]o"),
        "item.next",
        "[b",
        "[o",
    );
    let mut keys = Keys::default();
    for (typed, design) in [("]o", "]b"), ("[o", "[b"), ("3[o", "[b")] {
        match press(&mut keys, mine, Mode::Normal, Ctx::List, typed) {
            Outcome::Command { command, keys, .. } => {
                assert_eq!(command.id, "item.next");
                assert_eq!(keys, k(design), "{typed}");
            }
            other => panic!("{typed}: {other:?}"),
        }
    }
    match press(&mut keys, mine, Mode::Normal, Ctx::List, "]g") {
        Outcome::Command { keys, .. } => assert_eq!(keys, k("]g"), "nothing moved"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_key_the_keymap_writes_reads_back_from_its_written_form() {
    let toml: toml::Value =
        toml::from_str(include_str!("../../../../design/keymap.toml")).expect("toml");
    let mut seqs: Vec<String> = Vec::new();
    for c in toml["command"].as_array().expect("commands") {
        for b in c["keys"].as_array().into_iter().flatten() {
            seqs.push(b["seq"].as_str().expect("seq").to_string());
        }
    }
    for set in toml["builtin"].as_table().expect("builtin").values() {
        for s in set.as_array().expect("a set") {
            seqs.push(s.as_str().expect("a key").to_string());
        }
    }
    seqs.extend(toml["groups"].as_table().expect("groups").keys().cloned());
    assert!(seqs.len() > 200, "{} sequences", seqs.len());
    for s in &seqs {
        let keys = k(s);
        assert_eq!(parse_seq(&written(&keys)), Ok(keys), "{s}");
    }
    for s in [
        "<leader>cm",
        "<M-m>",
        "]o",
        "<D-CR>",
        "<S-Tab>",
        "<C-h>",
        "<Esc>",
        "G",
    ] {
        assert_eq!(written(&k(s)), s);
    }
}

// ───────────── ex names and the palette ─────────────

#[test]
fn ex_names_find_their_commands() {
    let km = km();
    assert_eq!(km.by_ex("q").map(|c| c.id.as_str()), Some("view.close"));
    assert_eq!(km.by_ex("qa").map(|c| c.id.as_str()), Some("app.quit"));
    assert_eq!(km.by_ex("w").map(|c| c.id.as_str()), Some("options.save"));
    assert_eq!(
        km.by_ex("messages").map(|c| c.id.as_str()),
        Some("open.messages")
    );
    assert!(km.by_ex("-newline").is_none() && km.by_ex("-").is_none());
}

#[test]
fn ex_completion_offers_names_by_prefix() {
    let names: Vec<&str> = km()
        .ex_completions("th")
        .iter()
        .map(|c| c.ex.as_str())
        .collect();
    assert_eq!(names, vec!["theme", "thinking"]);
    assert!(km().ex_completions("-").is_empty());
}

#[test]
fn the_palette_lists_every_named_command_once_with_its_keys() {
    let km = km();
    let ids: Vec<&str> = km.palette().map(|c| c.id.as_str()).collect();
    let unique: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), unique.len());
    assert!(ids.contains(&"palette") && ids.contains(&"app.quit") && !ids.contains(&"picker.keys"));
    let palette = km.command("palette").expect("palette");
    assert_eq!(
        km.keys_of(palette, Mode::Normal),
        vec!["␣;".to_string(), "⌘K".to_string()]
    );
    let quit = km.command("app.quit").expect("quit");
    assert!(km.keys_of(quit, Mode::Normal).is_empty());
}

// ───────────── auto-repeat ─────────────

#[test]
fn an_auto_repeat_never_fires_a_write_an_answer_or_a_readback_y() {
    let km = km();
    let mut keys = Keys::default();
    let held = |keys: &mut Keys, mode, ctx, seq: &str| {
        let mut out = Outcome::Pending;
        for key in k(seq) {
            out = keys.press(km, mode, ctx, key, true, 0.0);
        }
        out
    };
    assert_eq!(
        held(&mut keys, Mode::Insert, Ctx::Start, "<D-CR>"),
        Outcome::Ignored
    );
    assert_eq!(
        held(&mut keys, Mode::Readback, Ctx::Readback, "y"),
        Outcome::Ignored
    );
    assert_eq!(
        held(&mut keys, Mode::Readback, Ctx::Confirm, "y"),
        Outcome::Ignored
    );
    assert!(matches!(
        held(&mut keys, Mode::Normal, Ctx::List, "j"),
        Outcome::Builtin { .. }
    ));
    assert_eq!(
        command_id(&held(&mut keys, Mode::Normal, Ctx::List, "]b")),
        Some("item.next")
    );
}
