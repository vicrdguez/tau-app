//! How the keys cheat sheet could fail (`grammar.md` §5, §12 KL10):
//! - `␣hk` opens nothing, or opens it outside the float shell's picker mode;
//! - the sheet misses a key the table binds, or lists one it doesn't (it must match the table);
//! - typing filters out a row that matches, or keeps one that doesn't;
//! - `Esc` leaves it open.

use rune::Tx;

use super::*;
use crate::app::{App, input};
use crate::floats::Float;
use crate::keys::{Keymap, Target, display, parse_seq};
use crate::ui::Badge;

fn keys(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        input::press(a, key, false, &mut Tx::default());
    }
}

fn sheet(a: &App) -> &Sheet {
    match a.floats.last() {
        Some((Float::Sheet(s), _)) => s,
        _ => panic!("the cheat sheet is open"),
    }
}

#[test]
fn space_h_k_opens_the_sheet_as_a_picker_and_esc_closes_it() {
    let mut a = App::default();
    keys(&mut a, "<leader>hk");
    assert_eq!(sheet(&a).title, "Keys");
    assert_eq!(a.badge(), Badge::Command);
    keys(&mut a, "<Esc>");
    assert!(a.floats.is_empty());
}

#[test]
fn the_sheet_lists_every_key_the_table_binds_and_no_other() {
    let km = Keymap::design();
    let rows = keys_rows(km);
    for b in km.bindings() {
        let Target::Command(i) = b.target else {
            continue;
        };
        let desc = &km.commands()[i].desc;
        let shown = display(b.seq);
        assert!(
            rows.iter().any(|r| &r.text == desc
                && r.meta.starts_with(b.ctx.name())
                && r.keys.contains(&b.seq)),
            "{shown} {desc} in {}",
            b.ctx.name()
        );
    }
    for r in &rows {
        assert!(
            km.commands().iter().any(|c| c.desc == r.text),
            "{} is no command's",
            r.text
        );
    }
    assert!(
        rows.iter().any(|r| r.printed() == "gs"
            && r.text == "hint: targets in view"
            && r.meta == "global")
    );
}

#[test]
fn typing_keeps_the_rows_that_match() {
    let mut a = App::default();
    keys(&mut a, "<leader>hk");
    a.float_edit().replace_text("jump");
    let s = sheet(&a);
    let kept: Vec<&str> = s.shown().iter().map(|&i| s.rows[i].text.as_str()).collect();
    assert!(kept.contains(&"jump back") && kept.contains(&"jump forward"));
    assert!(kept.iter().all(|t| t.contains("jump")));
}
