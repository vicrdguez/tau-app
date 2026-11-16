//! How the palette's and which-key's reasons could fail (`spec.md` §6, `grammar.md` §5):
//! - a command that can't run where the keys are is greyed with no reason;
//! - a command that runs where the keys are carries a reason anyway;
//! - an answer command reads something other than `open a decision first`;
//! - a command bound in several views names only one of them;
//! - some context has no words, so a reason comes out empty.

use super::unavailable;
use crate::keys::{Ctx, Keymap};

fn command(id: &str) -> &'static crate::keys::Command {
    Keymap::design().command(id).expect("in keymap.toml")
}

#[test]
fn a_command_that_runs_here_has_no_reason() {
    assert_eq!(
        unavailable(Keymap::design(), command("palette"), Ctx::List),
        None
    );
    assert_eq!(
        unavailable(Keymap::design(), command("group.next"), Ctx::Sidebar),
        None
    );
}

#[test]
fn an_answer_command_outside_its_decision_says_to_open_one() {
    assert_eq!(
        unavailable(Keymap::design(), command("auth.answer"), Ctx::List),
        Some("open a decision first".to_string())
    );
}

#[test]
fn a_command_of_several_views_names_each() {
    assert_eq!(
        unavailable(Keymap::design(), command("toggle.details"), Ctx::List),
        Some("open a conversation or a decision first".to_string())
    );
}

#[test]
fn a_command_of_a_region_names_where_it_runs() {
    assert_eq!(
        unavailable(Keymap::design(), command("sidebar.leave"), Ctx::List),
        Some("only in the sidebar".to_string())
    );
}

#[test]
fn every_palette_command_runs_on_home_or_says_why_not() {
    for c in Keymap::design().palette() {
        if let Some(reason) = unavailable(Keymap::design(), c, Ctx::List) {
            assert!(!reason.trim().is_empty(), "{} has an empty reason", c.id);
            assert!(!reason.contains("  "), "{}: {reason:?}", c.id);
        }
    }
}
