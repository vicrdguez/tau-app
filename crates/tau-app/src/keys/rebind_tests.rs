//! How moving a key could fail:
//! - a command or key that doesn't exist moves anyway, or names the wrong command (`NoCommand`,
//!   `NotItsKey`), including a settings.toml line naming a design key an update took away;
//! - a fixed key moves: `Esc`, a hint label, an answer key, the readback's, reply's or Confirm's
//!   keys, or the settings float's own (`Fixed`);
//! - a key moves to `Esc` or `<label>`;
//! - Insert or a picker takes a key that types (`Typing`), a Normal key starts with a count digit
//!   (`Count`), a key that changes the server takes a reserved or trap key (`Reserved`), a leader
//!   key goes under a prefix which-key can't label (`NoGroup`);
//! - two bindings live together where one equals or starts the other (`Clash`), in one context
//!   or across a parent and child, against a command or a builtin key; or contexts that never
//!   meet are reported as clashing;
//! - an inner binding that shadows loses the exception when both keys move together, or a move
//!   creates a new shadow;
//! - putting back the design's key leaves a move behind, or skips the check against keys moved
//!   since;
//! - the same change twice gives another map;
//! - a swap or a cycle written in one settings.toml fails because each line is checked alone,
//!   or a line dropped for one problem leaves another line clashing with the key it restored;
//! - the key that moved away still runs, or the moved key fails in one of its modes;
//! - what is saved doesn't load back as the same map; the float's rows miss a key or a reason.

use super::*;

fn k(seq: &str) -> Vec<Key> {
    parse_seq(seq).expect("a key sequence")
}

fn design() -> Keymap {
    Keymap::design()
}

fn rebind(km: Keymap, command: &str, from: &str, to: &str) -> Result<Keymap, Refusal> {
    km.rebind(command, &k(from), &k(to))
}

fn moved(km: Keymap, command: &str, from: &str, to: &str) -> Keymap {
    rebind(km, command, from, to).expect("the move is allowed")
}

fn line<'a>(command: &'a str, from: &str, to: &str) -> Rebind<'a> {
    Rebind {
        command,
        design: k(from),
        to: k(to),
    }
}

fn runs(km: Keymap, mode: Mode, ctx: Ctx, seq: &str) -> Outcome<'static> {
    let mut keys = Keys::default();
    let mut out = Outcome::Pending;
    for key in k(seq) {
        out = keys.press(km, mode, ctx, key, false, 0.0);
    }
    out
}

fn ran(km: Keymap, mode: Mode, ctx: Ctx, seq: &str) -> Option<(&'static str, Vec<Key>)> {
    match runs(km, mode, ctx, seq) {
        Outcome::Command { command, keys, .. } => Some((command.id.as_str(), keys)),
        _ => None,
    }
}

fn clash_with(r: Result<Keymap, Refusal>) -> (Overlap, Option<&'static str>, Mode, Ctx) {
    match r {
        Err(Refusal::Clash {
            other,
            overlap,
            mode,
            ctx,
            ..
        }) => (
            overlap,
            match other {
                Other::Command(c) => Some(c.id.as_str()),
                Other::Builtin => None,
            },
            mode,
            ctx,
        ),
        other => panic!("not a clash: {other:?}"),
    }
}

// ───────────── refusals, one kind each ─────────────

#[test]
fn an_unknown_command_is_refused() {
    assert_eq!(
        rebind(design(), "no.such", "x", "X"),
        Err(Refusal::NoCommand("no.such".into()))
    );
}

#[test]
fn a_key_the_command_does_not_have_is_refused() {
    assert!(matches!(
        rebind(design(), "item.next", "]q", "]o"),
        Err(Refusal::NotItsKey { command, .. }) if command.id == "item.next"
    ));
}

#[test]
fn fixed_keys_stay_where_they_are() {
    let fixed = |command, from, to| match rebind(design(), command, from, to) {
        Err(Refusal::Fixed(f)) => f,
        other => panic!("{command} {from}: {other:?}"),
    };
    assert_eq!(fixed("float.close", "<Esc>", "<C-g>"), Fixed::Leave);
    assert_eq!(fixed("float.close", "q", "<Esc>"), Fixed::Leave);
    assert_eq!(fixed("hint.select", "<label>", "z"), Fixed::Labels);
    assert_eq!(
        fixed("auth.answer", "<leader>ad", "<leader>az"),
        Fixed::Answer
    );
    assert_eq!(fixed("confirm.keys", "y", "Y"), Fixed::Answer);
    assert_eq!(fixed("readback.pick", "j", "J"), Fixed::Answer);
    assert_eq!(fixed("reply.keys", "q", "Q"), Fixed::Answer);
    assert_eq!(fixed("settings.keys", "<C-j>", "<C-n>"), Fixed::Settings);
    assert_eq!(fixed("settings.keys", "<C-x>", "<C-x>"), Fixed::Settings);
}

#[test]
fn where_keys_type_only_a_chord_with_control_option_or_command_moves_there() {
    assert!(matches!(
        rebind(design(), "start.send", "<D-CR>", "w"),
        Err(Refusal::Typing {
            mode: Mode::Insert,
            ctx: Ctx::Start,
            ..
        })
    ));
    assert!(matches!(
        rebind(design(), "picker.keys", "<C-j>", "j"),
        Err(Refusal::Typing {
            mode: Mode::Command,
            ctx: Ctx::Picker,
            ..
        })
    ));
    assert!(matches!(
        rebind(design(), "start.send", "<D-CR>", "<C-s><C-s>"),
        Err(Refusal::Typing { .. })
    ));
}

#[test]
fn a_normal_key_never_starts_with_a_count() {
    assert_eq!(
        rebind(design(), "item.next", "]b", "2b"),
        Err(Refusal::Count)
    );
}

#[test]
fn reserved_and_trap_keys_stay_with_keys_that_change_nothing_on_the_server() {
    assert!(matches!(
        rebind(design(), "project.bind", "<leader>pb", "q"),
        Err(Refusal::Reserved { .. })
    ));
    assert!(matches!(
        rebind(design(), "conv.stop", "<leader>cx", "x"),
        Err(Refusal::Reserved { .. })
    ));
}

#[test]
fn a_leader_key_needs_a_labelled_group_for_every_prefix() {
    assert!(matches!(
        rebind(design(), "open.home", "<leader>oh", "<leader>zh"),
        Err(Refusal::NoGroup { prefix }) if prefix == "␣z"
    ));
    assert!(rebind(design(), "open.home", "<leader>oh", "<leader>ox").is_ok());
}

#[test]
fn a_key_another_binding_has_or_starts_is_refused() {
    assert_eq!(
        clash_with(rebind(design(), "open.home", "<leader>oh", "<leader>oc")),
        (
            Overlap::Same,
            Some("open.conversations"),
            Mode::Normal,
            Ctx::Global
        )
    );
    assert_eq!(
        clash_with(rebind(design(), "open.home", "<leader>oh", "<leader>o")).0,
        Overlap::Starts
    );
    assert_eq!(
        clash_with(rebind(design(), "peek", "K", "gdx")),
        (
            Overlap::Extends,
            Some("goto.linked"),
            Mode::Normal,
            Ctx::Global
        )
    );
    assert_eq!(
        clash_with(rebind(design(), "composer.send", "<D-CR>", "<C-w>")),
        (Overlap::Same, None, Mode::Insert, Ctx::Composer),
        "C-w is the editor's own Insert key"
    );
}

#[test]
fn a_clash_across_a_parent_and_its_child_is_refused_either_way() {
    assert_eq!(
        clash_with(rebind(design(), "message.next", "]m", "gdx")),
        (
            Overlap::Extends,
            Some("goto.linked"),
            Mode::Normal,
            Ctx::Conversation
        )
    );
    assert_eq!(
        clash_with(rebind(design(), "goto.linked", "gd", "]m")),
        (
            Overlap::Same,
            Some("message.next"),
            Mode::Normal,
            Ctx::Global
        )
    );
}

#[test]
fn contexts_that_never_meet_share_a_key() {
    let km = moved(design(), "note.next", "]n", "]t");
    assert_eq!(
        ran(km, Mode::Normal, Ctx::Decision, "]t"),
        Some(("note.next", k("]n")))
    );
    assert_eq!(
        ran(km, Mode::Normal, Ctx::Conversation, "]t"),
        Some(("toolcall.next", k("]t")))
    );
}

// ───────────── shadows ─────────────

#[test]
fn a_shadowing_key_stays_fixed_so_its_parent_is_never_uncovered() {
    for to in ["X", "l"] {
        let refusal = rebind(design(), "view.close", "x", to).unwrap_err();
        assert!(matches!(refusal, Refusal::Fixed(_)));
        assert!(refusal.to_string().contains("parent"), "{refusal}");
    }
    let row = design()
        .key_rows()
        .into_iter()
        .find(|r| r.command.id == "view.close" && r.design == k("x"))
        .unwrap();
    let why = Refusal::Fixed(row.fixed.expect("the float explains why it stays")).to_string();
    assert!(why.contains("parent"), "{why}");
    assert_eq!(
        ran(design(), Mode::Normal, Ctx::Sidebar, "x"),
        Some(("view.close", k("x")))
    );
}

// ───────────── putting back, repeating ─────────────

#[test]
fn putting_back_the_design_key_leaves_the_design() {
    let km = moved(design(), "item.next", "]b", "]o");
    assert_eq!(moved(km, "item.next", "]b", "]b"), design());
    assert!(design().rebinds().is_empty());
    let km = moved(design(), "editor.newline", "<CR>", "<C-y>");
    assert_eq!(
        moved(km, "editor.newline", "<CR>", "<CR>"),
        design(),
        "Insert's <CR> is the design's, though a move couldn't choose it"
    );
}

/// A table of its own: every context, one group, `builtin` and `commands` as given.
fn table(builtin: &str, commands: &str) -> Keymap {
    let contexts = Ctx::ALL
        .iter()
        .map(|c| {
            let parent = Keymap::design().parent(*c).map_or("", |p| p.name());
            format!("{} = \"{parent}\"\n", c.name())
        })
        .collect::<String>();
    Keymap::parse(&format!(
        "[meta]\nschema = 1\nleader = \"<Space>\"\nwhichkey_delay_ms = 300\nhint_alphabet = \"asdfghjkl\"\n\
         [contexts]\n{contexts}[groups]\n\"<leader>\" = \"leader\"\n[builtin]\n{builtin}\n{commands}"
    ))
    .expect("parses")
}

const HOME: &str = r#"
[[command]]
id = "open.home"
ex = "home"
desc = "home"
tier = "local"
kind = "command"
ctx = ["global"]
"#;

#[test]
fn one_key_a_command_binds_in_a_parent_and_a_child_moves_as_one() {
    let km = table(
        "",
        &format!(
            "{HOME}keys = [{{ mode = \"n\", ctx = \"global\", seq = \"H\" }}, {{ mode = \"n\", ctx = \"list\", seq = \"H\" }}]"
        ),
    );
    let km = moved(km, "open.home", "H", "J");
    assert_eq!(
        ran(km, Mode::Normal, Ctx::List, "J"),
        Some(("open.home", k("H")))
    );
}

#[test]
fn a_pair_the_design_itself_binds_is_the_design_lints() {
    let km = table(
        "list_motions = [\"j\"]",
        &format!("{HOME}keys = [{{ mode = \"n\", ctx = \"list\", seq = \"j\" }}]"),
    );
    let away = moved(km, "open.home", "j", "J");
    assert_eq!(moved(away, "open.home", "j", "j"), km);
}

#[test]
fn the_same_move_twice_gives_the_same_map() {
    let once = moved(design(), "item.next", "]b", "]o");
    assert_eq!(moved(design(), "item.next", "]b", "]o"), once);
    assert_eq!(moved(once, "item.next", "]b", "]o"), once);
}

// ───────────── settings.toml: every line at once ─────────────

#[test]
fn a_swap_written_in_one_file_loads() {
    assert!(rebind(design(), "start.project", "<M-p>", "<M-w>").is_err());
    let (km, refused) = design().rebind_all(&[
        line("start.project", "<M-p>", "<M-w>"),
        line("start.workflow", "<M-w>", "<M-p>"),
    ]);
    assert_eq!(refused, vec![]);
    assert_eq!(
        ran(km, Mode::Normal, Ctx::Start, "<M-w>"),
        Some(("start.project", k("<M-p>")))
    );
    assert_eq!(
        ran(km, Mode::Normal, Ctx::Start, "<M-p>"),
        Some(("start.workflow", k("<M-w>")))
    );
}

#[test]
fn a_three_way_cycle_written_in_one_file_loads() {
    let (km, refused) = design().rebind_all(&[
        line("open.home", "<leader>oh", "<leader>oc"),
        line("open.conversations", "<leader>oc", "<leader>ot"),
        line("open.timeline", "<leader>ot", "<leader>oh"),
    ]);
    assert_eq!(refused, vec![]);
    let at = |seq| ran(km, Mode::Normal, Ctx::List, seq).map(|(id, _)| id);
    assert_eq!(at("<leader>oc"), Some("open.home"));
    assert_eq!(at("<leader>ot"), Some("open.conversations"));
    assert_eq!(at("<leader>oh"), Some("open.timeline"));
}

#[test]
fn a_line_naming_a_key_the_command_no_longer_has_is_dropped_and_the_rest_load() {
    let (km, refused) = design().rebind_all(&[
        line("item.next", "]q", "]o"),
        line("start.workflow", "<M-w>", "<M-m>"),
    ]);
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].0.command, "item.next");
    assert!(matches!(refused[0].1, Refusal::NotItsKey { .. }));
    assert_eq!(km, moved(design(), "start.workflow", "<M-w>", "<M-m>"));
}

#[test]
fn lines_that_clash_with_each_other_are_all_dropped() {
    let (km, refused) = design().rebind_all(&[
        line("start.project", "<M-p>", "<M-m>"),
        line("start.workflow", "<M-w>", "<M-m>"),
        line("item.next", "]b", "]o"),
    ]);
    let dropped: Vec<&str> = refused.iter().map(|(l, _)| l.command).collect();
    assert_eq!(dropped, vec!["start.project", "start.workflow"]);
    assert!(
        refused
            .iter()
            .all(|(_, r)| matches!(r, Refusal::Clash { .. }))
    );
    assert_eq!(km, moved(design(), "item.next", "]b", "]o"));
}

#[test]
fn a_line_left_clashing_by_a_dropped_one_is_dropped_on_the_next_pass() {
    let (km, refused) = design().rebind_all(&[
        line("start.workflow", "<M-w>", "<M-p>"),
        line("start.project", "<M-p>", "p"),
    ]);
    let dropped: Vec<&str> = refused.iter().map(|(l, _)| l.command).collect();
    assert_eq!(dropped, vec!["start.project", "start.workflow"]);
    assert_eq!(km, design());
}

#[test]
fn what_is_saved_loads_back_as_the_same_map() {
    let km = moved(design(), "item.next", "]b", "]o");
    let km = moved(km, "item.next", "[b", "[o");
    let km = moved(km, "start.workflow", "<M-w>", "<M-m>");
    let saved = km.rebinds();
    assert_eq!(
        saved,
        vec![
            line("item.next", "]b", "]o"),
            line("item.next", "[b", "[o"),
            line("start.workflow", "<M-w>", "<M-m>"),
        ]
    );
    assert_eq!(design().rebind_all(&saved), (km, vec![]));
}

// ───────────── dispatch after a move ─────────────

#[test]
fn the_key_that_moved_away_is_unbound_afterwards() {
    let km = moved(design(), "start.workflow", "<M-w>", "<M-m>");
    assert!(matches!(
        runs(km, Mode::Normal, Ctx::Start, "<M-w>"),
        Outcome::Unbound { .. }
    ));
    assert_eq!(
        ran(km, Mode::Normal, Ctx::Start, "<M-m>"),
        Some(("start.workflow", k("<M-w>")))
    );
}

#[test]
fn a_moved_key_runs_in_every_mode_its_binding_had() {
    let km = moved(design(), "composer.send", "<D-CR>", "<C-s>");
    for mode in [Mode::Normal, Mode::Insert] {
        assert_eq!(
            ran(km, mode, Ctx::Composer, "<C-s>"),
            Some(("composer.send", k("<D-CR>"))),
            "{mode:?}"
        );
        assert_eq!(ran(km, mode, Ctx::Composer, "<D-CR>"), None, "{mode:?}");
    }
    let km = moved(design(), "cmdline.open", ":", "!");
    for mode in [Mode::Normal, Mode::Visual] {
        assert_eq!(
            ran(km, mode, Ctx::Global, "!").map(|(id, _)| id),
            Some("cmdline.open"),
            "{mode:?}"
        );
    }
}

// ───────────── the float's rows ─────────────

#[test]
fn the_rows_list_each_key_of_each_command_once_with_why_it_is_fixed() {
    let km = moved(design(), "item.next", "]b", "]o");
    let rows = km.key_rows();
    let row = |id: &str, seq: &str| {
        rows.iter()
            .find(|r| r.command.id == id && r.design == k(seq))
            .unwrap_or_else(|| panic!("{id} {seq}"))
    };
    assert_eq!(row("item.next", "]b").now, k("]o"));
    assert_eq!(row("item.next", "[b").now, k("[b"));
    let send = row("composer.send", "<D-CR>");
    assert_eq!(send.modes, vec![Mode::Normal, Mode::Insert]);
    assert_eq!(send.ctxs, vec![Ctx::Composer]);
    assert_eq!(send.fixed, None);
    assert_eq!(row("float.close", "<Esc>").fixed, Some(Fixed::Leave));
    assert_eq!(row("auth.answer", "<leader>ad").fixed, Some(Fixed::Answer));
    assert_eq!(row("hint.select", "<label>").fixed, Some(Fixed::Labels));
    assert_eq!(row("settings.keys", "<C-x>").fixed, Some(Fixed::Settings));
    let per_key: std::collections::BTreeSet<(&str, String)> = rows
        .iter()
        .map(|r| (r.command.id.as_str(), written(&r.design)))
        .collect();
    assert_eq!(per_key.len(), rows.len(), "one row per command and key");
}
