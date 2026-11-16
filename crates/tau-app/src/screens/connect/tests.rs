//! How Connect could fail (`spec.md` §5.1, `client.md` §7, §6):
//! - a link with no pairing opens anywhere but Connect, or Connect draws the sidebar, or opens in
//!   Normal, or without the URL prefilled;
//! - `Tab` and `S-Tab` leave the caret where it was;
//! - `⏎` with no code sends a `pair`, or leaves the caret away from the code;
//! - `⏎` to a local or `https://` server asks first, or to plain HTTP on another machine pairs
//!   without the Confirm float, or `n` there pairs anyway;
//! - a `y` typed in a field confirms instead of typing;
//! - a paired link stays on Connect instead of going Home with the sidebar;
//! - a wrong code, no server at the URL, a lost reply each show other words than the copy's, or
//!   lose what was typed;
//! - `:forget-server` forgets without a confirm, or leaves the stage off Connect, or drops the URL.

use rune::{KeyEvent, Tx};
use tau_client::LinkPhase;
use tau_client::store::Secrets;
use tau_client::wire::{Outcome, Unknown, parse_reply};

use super::*;
use crate::app::testing::{Setup, fixture, linked, paired};
use crate::app::{input, link};
use crate::floats::Float;
use crate::frame::Place;
use crate::keys::parse_seq;
use crate::screens::{self, page};
use crate::ui::Badge;

fn unpaired() -> (App, std::sync::Arc<crate::app::testing::Wire>) {
    let Setup { link, wire, .. } = linked(tempfile::tempdir().unwrap(), Secrets::memory(), false);
    (App::launch(link), wire)
}

/// Presses keys as the window does; a key the handler passes on is typed into the focused field.
fn keys(a: &mut App, seq: &str) {
    for key in parse_seq(seq).expect("keys") {
        let mut tx = Tx::default();
        if !input::press(a, key, false, &mut tx)
            && let Some(c) = key.typed()
            && let Some(edit) = if a.floats.is_empty() {
                a.screens.connect.focused_edit()
            } else {
                Some(a.float_edit())
            }
        {
            edit.key(
                &KeyEvent {
                    key: rune::Key::Text(c.to_string()),
                    mods: rune::Mods::default(),
                },
                None,
            );
        }
    }
}

fn frame(a: &mut App) {
    link::pump(a, &mut Tx::default());
}

fn with(a: &mut App, url: &str, code: &str) {
    let s = &mut a.screens.connect;
    s.server.edit.replace_text(url);
    s.code.edit.replace_text(code);
}

#[test]
fn an_unpaired_link_opens_connect_in_insert_on_the_server_field() {
    let (a, _) = unpaired();
    assert_eq!(a.stage, Stage::Connect);
    assert!(!page(&a).framed(), "no sidebar, toggle or breadcrumb");
    assert_eq!(a.badge(), Badge::Insert);
    assert_eq!(a.screens.connect.field, Field::Server);
    assert_eq!(a.screens.connect.server.text(), "http://127.0.0.1:6283");
}

#[test]
fn tab_and_shift_tab_move_between_the_fields() {
    let (mut a, _) = unpaired();
    keys(&mut a, "<Tab>");
    assert_eq!(a.screens.connect.field, Field::Code);
    keys(&mut a, "<S-Tab>");
    assert_eq!(a.screens.connect.field, Field::Server);
}

#[test]
fn enter_without_a_code_sends_nothing_and_goes_to_the_code() {
    let (mut a, _) = unpaired();
    keys(&mut a, "<CR>");
    assert_eq!(a.link.status().phase, LinkPhase::NotPaired);
    assert_eq!(a.screens.connect.field, Field::Code);
}

#[test]
fn a_local_server_pairs_at_once_and_goes_home() {
    let (mut a, _) = unpaired();
    keys(&mut a, "<Tab>K7QM-3XRT<CR>");
    assert!(a.floats.is_empty(), "no confirm for a local server");
    assert_eq!(a.link.status().phase, LinkPhase::Pairing);
    for _ in 0..5 {
        frame(&mut a);
    }
    assert_eq!(a.stage, Stage::Place(Place::Home));
    assert!(screens::page(&a).framed());
}

#[test]
fn only_plain_http_to_another_machine_warns() {
    for url in [
        "http://127.0.0.1:6283",
        "http://localhost:6283",
        "http://[::1]:6283",
        "https://tau.tail4e2f.ts.net",
        "https://192.168.1.20",
    ] {
        assert!(!plain_http_elsewhere(url), "{url}");
    }
    for url in [
        "http://192.168.1.20:6283",
        "http://tau.example",
        "HTTP://10.0.0.2",
    ] {
        assert!(plain_http_elsewhere(url), "{url}");
    }
}

#[test]
fn plain_http_elsewhere_asks_first_and_n_cancels() {
    let (mut a, _) = unpaired();
    with(&mut a, "http://192.168.1.20:6283", "K7QM-3XRT");
    keys(&mut a, "<CR>");
    assert!(matches!(a.floats.last(), Some((Float::Confirm(_), _))));
    assert_eq!(a.link.status().phase, LinkPhase::NotPaired);
    keys(&mut a, "n");
    assert!(a.floats.is_empty());
    assert_eq!(a.link.status().phase, LinkPhase::NotPaired);
    keys(&mut a, "<CR>y");
    assert_eq!(a.link.status().phase, LinkPhase::Pairing);
}

#[test]
fn a_y_typed_in_a_field_is_a_letter() {
    let (mut a, _) = unpaired();
    with(&mut a, "http://192.168.1.20:6283", "");
    keys(&mut a, "<Tab>y");
    assert_eq!(a.screens.connect.code.text(), "y");
    assert_eq!(a.link.status().phase, LinkPhase::NotPaired);
}

fn refused(a: &mut App, wire: &crate::app::testing::Wire, outcome: Outcome) {
    *wire.refuse.lock().unwrap() = Some(outcome);
    with(a, "http://127.0.0.1:6283", "K7QM-3XRT");
    keys(a, "<CR>");
    for _ in 0..3 {
        frame(a);
    }
}

#[test]
fn a_wrong_code_says_the_server_s_sentence_and_how_codes_work() {
    let (mut a, wire) = unpaired();
    let reply =
        br#"{"refused":{"code":"unauthorized","message":"The pairing code is wrong or expired."}}"#;
    refused(&mut a, &wire, parse_reply(401, Some("1"), reply));
    assert_eq!(a.stage, Stage::Connect);
    assert_eq!(
        a.screens.connect.failure.as_deref(),
        Some(
            "The pairing code is wrong or expired. A code works once, for 10 minutes, and five \
             wrong tries cancel every pending code. Run tau device pair <name> again."
        )
    );
    assert_eq!(a.screens.connect.code.text(), "K7QM-3XRT", "the code stays");
}

#[test]
fn no_server_at_the_url_says_so() {
    let (mut a, wire) = unpaired();
    refused(
        &mut a,
        &wire,
        Outcome::Unknown(Unknown::Unreachable(
            "Connection refused (os error 61)".into(),
        )),
    );
    assert_eq!(
        a.screens.connect.failure.as_deref(),
        Some(
            "No tau server answered at http://127.0.0.1:6283: Connection refused (os error 61). \
             Check the address, and that tau is running there."
        )
    );
}

#[test]
fn a_lost_reply_says_the_name_may_be_used() {
    let (mut a, wire) = unpaired();
    refused(&mut a, &wire, Outcome::Unknown(Unknown::Timeout));
    let words = a.screens.connect.failure.clone().unwrap_or_default();
    assert!(words.starts_with("No reply from the server."), "{words}");
}

#[test]
fn forget_server_asks_then_returns_to_connect_with_the_url() {
    let mut s = paired(false);
    s.link.set_focused(true);
    let mut a = App::launch(s.link);
    for _ in 0..5 {
        frame(&mut a);
    }
    assert_ne!(a.stage, Stage::Connect);
    keys(&mut a, "<Esc>:forget-server<CR>");
    assert!(matches!(a.floats.last(), Some((Float::Confirm(_), _))));
    assert_ne!(a.stage, Stage::Connect, "nothing forgotten before y");
    keys(&mut a, "y");
    frame(&mut a);
    assert_eq!(a.stage, Stage::Connect);
    assert_eq!(a.screens.connect.server.text(), fixture().url());
}
