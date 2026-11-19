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
//! - `:forget-server` forgets without a confirm, or leaves the stage off Connect, or drops the URL;
//! - cancelling the Confirm float leaves no field with the native focus, or gives it to the other
//!   field, or leaves Normal, or loses a value, so the next key types nowhere;
//! - an error about the server address stays after Server changes, or one about the code after
//!   the code changes, or either goes while its field is unchanged;
//! - an error about the address stands under Pairing code, or above the plain-HTTP warning, or
//!   one about the code stands under Server, or one about neither stands between the fields;
//! - at 800 x 600 with the warning and an error, the footer cannot be reached, or the field with
//!   the caret is out of view.

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
    assert_eq!(a.screens.connect.field(), Field::Server);
    assert_eq!(a.screens.connect.server.text(), "http://127.0.0.1:6283");
}

#[test]
fn tab_and_shift_tab_move_between_the_fields() {
    let (mut a, _) = unpaired();
    keys(&mut a, "<Tab>");
    assert_eq!(a.screens.connect.field(), Field::Code);
    keys(&mut a, "<S-Tab>");
    assert_eq!(a.screens.connect.field(), Field::Server);
}

#[test]
fn enter_without_a_code_sends_nothing_and_goes_to_the_code() {
    let (mut a, _) = unpaired();
    keys(&mut a, "<CR>");
    assert_eq!(a.link.status().phase, LinkPhase::NotPaired);
    assert_eq!(a.screens.connect.field(), Field::Code);
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

/// The window, with rune's focus and typing, from a named scene.
fn window(scene: &str) -> rune::Runtime<App> {
    window_sized(scene, (1512.0, 982.0))
}

fn window_sized(scene: &str, size: (f32, f32)) -> rune::Runtime<App> {
    let mut a = App::default();
    crate::app::scenes::all()
        .iter()
        .find(|(n, _)| *n == scene)
        .unwrap()
        .1(&mut a);
    let text = rune::text::TextSystem::new(crate::assets::fonts().unwrap(), crate::theme::FAMILIES);
    let mut rt =
        rune::Runtime::with_text(a, crate::app::view, size, rune::Env::new(true, 1.0), text);
    rt.on_key = Some(input::on_key);
    rt.poll = Some(crate::app::poll);
    rt.frame(0.1);
    rt
}

fn press(rt: &mut rune::Runtime<App>, key: rune::Key, cmd: bool) {
    let mods = rune::Mods {
        cmd,
        logo: cmd,
        ..Default::default()
    };
    rt.key(KeyEvent { key, mods });
    rt.frame(0.1);
}

fn type_text(rt: &mut rune::Runtime<App>, words: &str) {
    for c in words.chars() {
        press(rt, rune::Key::Text(c.to_string()), false);
    }
}

fn focused_field(rt: &rune::Runtime<App>) -> Option<String> {
    rt.elements()
        .into_iter()
        .find(|e| e.focused)
        .and_then(|e| e.key)
}

#[test]
fn cancelling_the_http_warning_gives_the_server_field_its_typing_back() {
    for cancel in [rune::Key::Escape, rune::Key::Text("n".into())] {
        let mut rt = window("connect-no-server");
        assert_eq!(focused_field(&rt).as_deref(), Some("Server"));
        assert!(rt.state.screens.connect.failure().is_some());
        press(&mut rt, rune::Key::Enter, false);
        assert!(matches!(
            rt.state.floats.last(),
            Some((Float::Confirm(_), _))
        ));
        press(&mut rt, cancel.clone(), false);
        assert!(rt.state.floats.is_empty(), "{cancel:?} closes Confirm");
        assert_eq!(rt.state.badge(), Badge::Insert);
        assert_eq!(focused_field(&rt).as_deref(), Some("Server"), "{cancel:?}");
        press(&mut rt, rune::Key::Command('a'), true);
        type_text(&mut rt, "https://tau.tail4e2f.ts.net");
        let s = &rt.state.screens.connect;
        assert_eq!(s.server.text(), "https://tau.tail4e2f.ts.net");
        assert_eq!(s.code.text(), "K7QM-3XRT");
        assert_eq!(s.failure(), None, "the old address's error went with it");
    }
}

#[test]
fn cancelling_the_http_warning_from_the_code_returns_to_the_code() {
    let mut rt = window("connect-http-warning");
    press(&mut rt, rune::Key::Escape, false);
    assert_eq!(focused_field(&rt).as_deref(), Some("Pairing code"));
    type_text(&mut rt, "Z");
    let s = &rt.state.screens.connect;
    assert_eq!(s.code.text(), "K7QM-3XRTZ");
    assert_eq!(s.server.text(), "http://192.168.1.20:6283");
}

#[test]
fn a_click_on_the_other_field_moves_the_caret_there() {
    let mut rt = window("connect-no-server");
    let code = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("Pairing code"))
        .unwrap()
        .rect;
    let (x, y) = (code[0] + code[2] - 4.0, code[1] + code[3] / 2.0);
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.1);
    assert_eq!(focused_field(&rt).as_deref(), Some("Pairing code"));
    type_text(&mut rt, "Z");
    assert_eq!(rt.state.screens.connect.code.text(), "K7QM-3XRTZ");
    press(&mut rt, rune::Key::Tab, false);
    assert_eq!(
        focused_field(&rt).as_deref(),
        Some("Server"),
        "Tab goes on from the code"
    );
}

#[test]
fn a_server_error_stays_until_the_server_changes() {
    let (mut a, wire) = unpaired();
    refused(
        &mut a,
        &wire,
        Outcome::Unknown(Unknown::Unreachable("Connection refused".into())),
    );
    keys(&mut a, "<Tab>9<S-Tab>");
    assert!(
        a.screens.connect.failure().is_some(),
        "the code is not the address"
    );
    keys(&mut a, "0");
    assert_eq!(a.screens.connect.failure(), None);
}

#[test]
fn a_code_error_stays_until_the_code_changes() {
    let (mut a, wire) = unpaired();
    let reply =
        br#"{"refused":{"code":"unauthorized","message":"The pairing code is wrong or expired."}}"#;
    refused(&mut a, &wire, parse_reply(401, Some("1"), reply));
    keys(&mut a, "0<Tab>");
    assert!(
        a.screens.connect.failure().is_some(),
        "the address is not the code"
    );
    keys(&mut a, "9");
    assert_eq!(a.screens.connect.failure(), None);
}

/// The top edge of the first element whose text starts with `words`, or that is keyed `words`.
fn top(rt: &rune::Runtime<App>, words: &str) -> f32 {
    rt.elements()
        .into_iter()
        .find(|e| {
            e.key.as_deref() == Some(words)
                || e.text.as_deref().is_some_and(|t| t.starts_with(words))
        })
        .unwrap_or_else(|| panic!("no {words:?} on screen"))
        .rect[1]
}

/// The window from `connect-no-server`, with `failure` in place of its error.
fn failed_with(failure: PairFailure) -> rune::Runtime<App> {
    let mut rt = window("connect-no-server");
    Connect.on_event(&mut rt.state, &LinkEvent::PairFailed(failure));
    rt.frame(0.1);
    rt
}

#[test]
fn an_error_about_the_address_stands_under_server_after_the_warning() {
    let rt = window("connect-no-server");
    let warning = top(&rt, "This address is not encrypted");
    let error = top(&rt, "No tau server answered");
    assert!(top(&rt, "Server") < warning, "the warning under Server");
    assert!(warning < error, "the error after the warning");
    assert!(
        error < top(&rt, "Pairing code"),
        "the error above Pairing code"
    );
}

#[test]
fn an_error_about_the_code_stands_under_pairing_code() {
    let Outcome::Refused(r) = parse_reply(
        401,
        Some("1"),
        br#"{"refused":{"code":"unauthorized","message":"The pairing code is wrong or expired."}}"#,
    ) else {
        panic!("a refusal");
    };
    let rt = failed_with(PairFailure::Refused(r));
    assert!(top(&rt, "Pairing code") < top(&rt, "The pairing code is wrong"));
}

#[test]
fn an_error_about_neither_field_stands_under_both() {
    let rt = failed_with(PairFailure::NoReply(Unknown::Timeout));
    assert!(top(&rt, "Pairing code") < top(&rt, "No reply from the server"));
}

#[test]
fn a_short_window_scrolls_to_the_footer_and_keeps_the_caret_in_view() {
    let bar = crate::theme::tokens().layout.height.bottombar;
    for failure in [None, Some(PairFailure::NoReply(Unknown::Timeout))] {
        let mut rt = window_sized("connect-no-server", (800.0, 600.0));
        if let Some(f) = failure {
            Connect.on_event(&mut rt.state, &LinkEvent::PairFailed(f));
            rt.frame(0.1);
        }
        let in_view = |rt: &rune::Runtime<App>, key: &str| {
            let e = rt
                .elements()
                .into_iter()
                .find(|e| e.key.as_deref() == Some(key) || e.text.as_deref() == Some(key))
                .unwrap();
            e.rect[1] >= 0.0 && e.rect[1] + e.rect[3] <= 600.0 - bar
        };
        assert!(in_view(&rt, "Server"), "the caret's field shows on open");
        rt.wheel(400.0, 300.0, -2000.0);
        rt.frame(0.1);
        rt.frame(1.0);
        assert!(in_view(&rt, "connect"), "the footer is reached");
        assert!(in_view(&rt, "Server"), "the caret's field stays in view");
        press(&mut rt, rune::Key::Tab, false);
        assert!(in_view(&rt, "Pairing code"), "Tab's field is in view");
    }
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
        a.screens.connect.failure(),
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
        a.screens.connect.failure(),
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
    let words = a.screens.connect.failure().unwrap_or_default();
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
