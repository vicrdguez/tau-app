//! How the frame's own keys could fail, driven through the app's key handler as a window would:
//! - `␣tb` fails to hide or show the sidebar, or under 1000 px fails to do what `C-h` does;
//! - `␣ht` fails to switch the theme;
//! - `C-h` leaves the keys in the stage or puts the cursor elsewhere than the current row, and
//!   `C-l Esc q` fail to give them back;
//! - the sidebar's `j k gg G ⏎ x` move, open or close the wrong row;
//! - `␣oh` and the place keys fail to switch the stage, or leave the keys in the sidebar;
//! - `]b [b` skip open items, or ignore the item shown last;
//! - `:q` closes the wrong item or shows the wrong one next; an unknown `:` name passes silently;
//! - the `:` line forgets its history;
//! - which-key shows before its delay, or `Esc` leaves it up;
//! - a global key acts under a float that takes the keys;
//! - the palette runs another command than the one chosen;
//! - Confirm's `y` fails to run its step, or `n` runs it;
//! - `␣td` acts where it doesn't apply;
//! - a command handler names an id the keymap doesn't have, so it can never run.

use rune::{KeyEvent, Tx};

use super::*;
use crate::keys::{Code, Key, parse_seq};
use crate::theme::Variant;

/// The fixture's open items.
fn app() -> App {
    let mut a = App::default();
    for item in [
        Item::Conversation {
            project: "notes-cli".into(),
            id: 4,
        },
        Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into(),
        },
        Item::Conversation {
            project: "render-core".into(),
            id: 5,
        },
        Item::Project("render-core".into()),
        Item::Decision {
            project: "forecasts".into(),
            id: "w2:2:pi-durable-2#1".into(),
        },
    ] {
        a.open.open(item);
    }
    a
}

/// Presses keys as the window does; a key the handler passes on is typed into the float's
/// input, as its field would.
fn keys(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        let mut tx = Tx::default();
        if !input::press(a, key, false, &mut tx) {
            let typed = match (key.typed(), key.code) {
                (Some(c), _) => rune::Key::Text(c.to_string()),
                (None, Code::Backspace) => rune::Key::Backspace,
                _ => continue,
            };
            a.float_edit().key(
                &KeyEvent {
                    key: typed,
                    mods: rune::Mods::default(),
                },
                None,
            );
        }
    }
}

fn conv(p: &str, id: u64) -> Item {
    Item::Conversation {
        project: p.into(),
        id,
    }
}

#[test]
fn space_t_b_hides_and_shows_the_sidebar() {
    let mut a = app();
    keys(&mut a, "<leader>tb");
    assert_eq!((a.sidebar_visible, a.sidebar_place()), (false, None));
    keys(&mut a, "<leader>tb");
    assert_eq!(a.sidebar_place(), Some(SidebarPlace::Beside));
}

#[test]
fn under_1000_px_space_t_b_does_what_ctrl_h_does() {
    let mut a = app();
    a.window = [800.0, 982.0];
    assert_eq!(
        a.sidebar_place(),
        None,
        "no room: not drawn, whatever the choice"
    );
    keys(&mut a, "<leader>tb");
    assert_eq!(
        (a.focus, a.sidebar_visible, a.sidebar_place()),
        (Focus::Sidebar, true, Some(SidebarPlace::Over))
    );
    keys(&mut a, "<Esc>");
    assert_eq!((a.focus, a.sidebar_place()), (Focus::Stage, None));
}

#[test]
fn space_h_t_switches_the_theme() {
    let mut a = app();
    keys(&mut a, "<leader>ht");
    assert_eq!(a.settings.theme, Variant::EmberSoft);
    keys(&mut a, "<leader>ht<leader>ht");
    assert_eq!(a.settings.theme, Variant::Ember);
}

#[test]
fn ctrl_h_gives_the_sidebar_the_keys_on_the_current_row() {
    let mut a = app();
    keys(&mut a, "<C-h>");
    assert_eq!(
        (a.focus, a.sidebar_cursor.clone()),
        (Focus::Sidebar, Row::Place(Place::Home))
    );
    for back in ["<C-l>", "<Esc>", "q"] {
        keys(&mut a, back);
        assert_eq!(a.focus, Focus::Stage, "{back}");
        keys(&mut a, "<C-h>");
    }
}

#[test]
fn the_sidebar_moves_over_every_row() {
    let mut a = app();
    keys(&mut a, "<C-h>gg");
    assert_eq!(a.sidebar_cursor, Row::New);
    keys(&mut a, "j");
    assert_eq!(a.sidebar_cursor, Row::Find);
    keys(&mut a, "G");
    assert_eq!(a.sidebar_cursor, Row::Server);
    keys(&mut a, "k");
    assert_eq!(
        a.sidebar_cursor,
        Row::Item(Item::Decision {
            project: "forecasts".into(),
            id: "w2:2:pi-durable-2#1".into()
        })
    );
    keys(&mut a, "gg]g]g");
    assert_eq!(
        a.sidebar_cursor,
        Row::Item(conv("notes-cli", 4)),
        "]g moves to the next section"
    );
}

#[test]
fn enter_in_the_sidebar_opens_the_row_and_gives_the_stage_the_keys() {
    let mut a = app();
    keys(&mut a, "<C-h>jjj");
    assert_eq!(a.sidebar_cursor, Row::Place(Place::Projects));
    keys(&mut a, "<CR>");
    assert_eq!(
        (a.stage.clone(), a.focus),
        (Stage::Place(Place::Projects), Focus::Stage)
    );
    keys(&mut a, "<C-h>jl");
    assert_eq!(a.stage, Stage::Item(conv("notes-cli", 4)), "l opens too");
}

#[test]
fn x_in_the_sidebar_closes_the_item_under_its_cursor() {
    let mut a = app();
    keys(&mut a, "<C-h>Gk");
    keys(&mut a, "x");
    assert!(!a.open.contains(&Item::Decision {
        project: "forecasts".into(),
        id: "w2:2:pi-durable-2#1".into()
    }));
    assert_eq!(a.focus, Focus::Sidebar);
}

#[test]
fn the_place_keys_switch_the_stage_and_take_the_keys_back() {
    let mut a = app();
    keys(&mut a, "<C-h><leader>ot");
    assert_eq!(
        (a.stage.clone(), a.focus),
        (Stage::Place(Place::Timeline), Focus::Stage)
    );
    keys(&mut a, "<leader>oc");
    assert_eq!(a.stage, Stage::Place(Place::Conversations));
    keys(&mut a, "<leader>of");
    assert_eq!(
        (a.stage.clone(), a.current_row()),
        (Stage::Place(Place::Faults), Row::Place(Place::Home))
    );
    keys(&mut a, "<leader>op<leader>oh");
    assert_eq!(a.stage, Stage::Place(Place::Home));
}

#[test]
fn bracket_b_cycles_open_items_from_the_one_shown_last() {
    let mut a = app();
    keys(&mut a, "]b");
    assert_eq!(
        a.stage,
        Stage::Item(conv("notes-cli", 4)),
        "nothing shown yet: the first"
    );
    keys(&mut a, "]b]b");
    assert_eq!(a.stage, Stage::Item(conv("render-core", 5)));
    keys(&mut a, "<leader>oh]b");
    assert_eq!(
        a.stage,
        Stage::Item(Item::Project("render-core".into())),
        "from Home, after the one shown last"
    );
    keys(&mut a, "2[b");
    assert_eq!(
        a.stage,
        Stage::Item(Item::Decision {
            project: "render-core".into(),
            id: "w1:3:design#2".into()
        })
    );
}

#[test]
fn colon_q_closes_the_shown_item_and_shows_the_next() {
    let mut a = app();
    a.show(Stage::Item(conv("render-core", 5)));
    keys(&mut a, ":q<CR>");
    assert!(!a.open.contains(&conv("render-core", 5)));
    assert_eq!(
        (a.stage.clone(), a.floats.len()),
        (Stage::Item(Item::Project("render-core".into())), 0)
    );
}

#[test]
fn colon_q_on_the_last_item_shows_home() {
    let mut a = App::default();
    a.show(Stage::Item(conv("x", 1)));
    keys(&mut a, ":q<CR>");
    assert_eq!(a.stage, Stage::Place(Place::Home));
}

#[test]
fn an_unknown_colon_name_says_so() {
    let mut a = app();
    keys(&mut a, ":nope<CR>");
    assert!(
        a.echo.as_deref().is_some_and(|e| e.contains("nope")),
        "{:?}",
        a.echo
    );
    assert!(a.floats.is_empty());
}

#[test]
fn the_command_line_keeps_its_history() {
    let mut a = app();
    keys(&mut a, ":theme<CR>");
    assert_eq!(a.settings.theme, Variant::EmberSoft);
    keys(&mut a, ":<Up>");
    let Some((Float::CmdLine(c), _)) = a.floats.last() else {
        panic!("the : line")
    };
    assert_eq!(c.line.text, "theme");
}

#[test]
fn which_key_waits_its_delay_and_esc_closes_it() {
    let mut a = app();
    let mut tx = Tx::default();
    a.now = 10.0;
    keys(&mut a, "<leader>");
    assert!(!poll(&mut a, 10.2, &mut tx) && !a.which_key_shown);
    assert!(poll(&mut a, 10.31, &mut tx) && a.which_key_shown);
    keys(&mut a, "<Esc>");
    assert!(poll(&mut a, 10.4, &mut tx) && !a.which_key_shown);
}

#[test]
fn no_global_key_acts_under_a_float() {
    let mut a = app();
    keys(&mut a, "<leader>;");
    assert_eq!(a.badge(), Badge::Command);
    keys(&mut a, "<C-h>");
    assert_eq!((a.focus, a.floats.len()), (Focus::Stage, 1));
    keys(&mut a, "<Esc>");
    assert!(a.floats.is_empty());
}

#[test]
fn the_palette_runs_the_chosen_command() {
    let mut a = app();
    let cmd_k = Key {
        code: Code::Char('k'),
        mods: crate::keys::Mods {
            cmd: true,
            ..Default::default()
        },
    };
    input::press(&mut a, cmd_k, false, &mut Tx::default());
    keys(&mut a, "sidebar");
    let rows: Vec<&str> = floats::palette_rows(a.settings.keymap, "sidebar")
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(rows[..2], ["sidebar.focus", "toggle.sidebar"]);
    keys(&mut a, "<C-j><CR>");
    assert_eq!((a.floats.len(), a.sidebar_visible), (0, false));
}

#[test]
fn confirm_y_runs_its_step_and_n_does_not() {
    let mut a = app();
    let step = || -> floats::OnYes { Box::new(|a: &mut App, _: &mut Tx| a.say("done")) };
    a.open_float(Float::Confirm(floats::Confirm {
        sentence: "Go?".into(),
        yes: "goes",
        on_yes: Some(step()),
    }));
    keys(&mut a, "n");
    assert_eq!((a.floats.len(), a.echo.clone()), (0, None));
    a.open_float(Float::Confirm(floats::Confirm {
        sentence: "Go?".into(),
        yes: "goes",
        on_yes: Some(step()),
    }));
    keys(&mut a, "y");
    assert_eq!((a.floats.len(), a.echo.as_deref()), (0, Some("done")));
}

#[test]
fn space_t_d_toggles_details_only_where_it_applies() {
    let mut a = app();
    keys(&mut a, "<leader>td");
    assert!(!a.details_visible, "Home has no details panel");
    a.show(Stage::Item(conv("notes-cli", 4)));
    keys(&mut a, "<leader>td");
    assert!(a.details_visible);
}

#[test]
fn every_handler_names_a_command_in_the_keymap() {
    for id in commands::handled() {
        assert!(
            Keymap::design().command(id).is_some(),
            "{id} is not in keymap.toml"
        );
    }
}
